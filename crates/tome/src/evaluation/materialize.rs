//! Copy-only, hash-verified run workspace preparation.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use walkdir::WalkDir;

use crate::manifest::hash_directory;
use crate::validation::ContentHash;

use super::contract::Variant;
use super::suite::{EvaluationCase, validate_relative_path};

/// Paths copied into a fresh workspace. Canonical source paths are never passed to a runner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterializedWorkspace {
    pub artifact_path: Option<PathBuf>,
    pub fixture_path: Option<PathBuf>,
    pub artifact_hash: ContentHash,
    pub fixture_hash: Option<ContentHash>,
}

/// Compute artifact identity after rejecting symlinks and special files.
pub fn hash_artifact_directory(path: &Path) -> Result<ContentHash> {
    let root = canonical_directory(path, "artifact source")?;
    reject_links_and_special_files(&root)?;
    hash_directory(&root).context("failed to hash artifact directory")
}

/// Verify and copy the artifact and optional fixture into an existing empty workspace.
/// The caller owns workspace creation and cleanup.
pub fn materialize_run_workspace(
    suite_root: &Path,
    case: &EvaluationCase,
    variant: Variant,
    artifact_source: &Path,
    expected_artifact_hash: &ContentHash,
    workspace: &Path,
) -> Result<MaterializedWorkspace> {
    let workspace = canonical_empty_workspace(workspace)?;
    let artifact_source = canonical_directory(artifact_source, "artifact source")?;
    let suite_root = canonical_directory(suite_root, "suite directory")?;
    ensure_disjoint(&workspace, &artifact_source, "artifact source")?;
    ensure_disjoint(&workspace, &suite_root, "suite directory")?;

    let actual_artifact_hash = hash_artifact_directory(&artifact_source)?;
    ensure!(
        &actual_artifact_hash == expected_artifact_hash,
        "artifact hash does not match suite identity"
    );

    let artifact_path = if variant == Variant::Treatment {
        let destination = workspace.join("artifact");
        copy_tree(&artifact_source, &destination)?;
        ensure!(
            hash_directory(&destination)? == *expected_artifact_hash,
            "copied artifact hash changed during materialization"
        );
        Some(destination)
    } else {
        None
    };

    let (fixture_path, fixture_hash) = match &case.fixture {
        Some(relative) => {
            validate_relative_path(relative, "fixture path")?;
            let fixture_source = resolve_fixture(&suite_root, relative)?;
            ensure_disjoint(&workspace, &fixture_source, "fixture source")?;
            reject_links_and_special_files(&fixture_source)?;
            let source_hash = hash_directory(&fixture_source)?;
            let destination = workspace.join("fixtures");
            copy_tree(&fixture_source, &destination)?;
            ensure!(
                hash_directory(&destination)? == source_hash,
                "copied fixture hash changed during materialization"
            );
            (Some(destination), Some(source_hash))
        }
        None => (None, None),
    };

    Ok(MaterializedWorkspace {
        artifact_path,
        fixture_path,
        artifact_hash: actual_artifact_hash,
        fixture_hash,
    })
}

fn canonical_empty_workspace(path: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("workspace '{}' does not exist", path.display()))?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "workspace must be a real directory"
    );
    ensure!(
        fs::read_dir(path)?.next().is_none(),
        "workspace must be empty"
    );
    path.canonicalize()
        .with_context(|| format!("failed to resolve workspace '{}'", path.display()))
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("{label} '{}' does not exist", path.display()))?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "{label} must be a real directory"
    );
    path.canonicalize()
        .with_context(|| format!("failed to resolve {label} '{}'", path.display()))
}

fn ensure_disjoint(workspace: &Path, source: &Path, label: &str) -> Result<()> {
    ensure!(
        !workspace.starts_with(source) && !source.starts_with(workspace),
        "workspace must not overlap {label}"
    );
    Ok(())
}

fn resolve_fixture(suite_root: &Path, relative: &str) -> Result<PathBuf> {
    let mut current = suite_root.to_path_buf();
    for component in relative.split('/') {
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .with_context(|| format!("fixture path '{}' does not exist", current.display()))?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "fixture path must not contain symlinks"
        );
    }
    let canonical = current
        .canonicalize()
        .with_context(|| format!("failed to resolve fixture '{}'", current.display()))?;
    ensure!(
        canonical.starts_with(suite_root),
        "fixture path escapes the suite directory"
    );
    ensure!(canonical.is_dir(), "fixture path must name a directory");
    Ok(canonical)
}

fn reject_links_and_special_files(root: &Path) -> Result<()> {
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to walk '{}'", root.display()))?;
        let file_type = entry.file_type();
        ensure!(
            !file_type.is_symlink(),
            "source trees cannot contain symlinks"
        );
        ensure!(
            file_type.is_dir() || file_type.is_file(),
            "source trees cannot contain special files"
        );
        if entry.path() != root {
            let relative = entry
                .path()
                .strip_prefix(root)
                .context("walked path escaped its source root")?;
            let relative = relative
                .to_str()
                .context("source path is not valid UTF-8")?
                .replace(std::path::MAIN_SEPARATOR, "/");
            validate_relative_path(&relative, "source path")?;
        }
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    ensure!(!destination.exists(), "copy destination already exists");
    fs::create_dir(destination)
        .with_context(|| format!("failed to create '{}'", destination.display()))?;
    for entry in WalkDir::new(source).min_depth(1).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to walk '{}'", source.display()))?;
        let file_type = entry.file_type();
        ensure!(
            !file_type.is_symlink(),
            "source trees cannot contain symlinks"
        );
        let relative = entry
            .path()
            .strip_prefix(source)
            .context("walked path escaped its source root")?;
        let target = destination.join(relative);
        if file_type.is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("failed to create '{}'", target.display()))?;
        } else if file_type.is_file() {
            let parent = target.parent().context("copied file has no parent")?;
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
            fs::copy(entry.path(), &target)
                .with_context(|| format!("failed to copy '{}'", entry.path().display()))?;
        } else {
            bail!("source trees cannot contain special files");
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "materialize_tests.rs"]
mod tests;

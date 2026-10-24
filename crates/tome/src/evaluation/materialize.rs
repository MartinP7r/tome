//! Copy-only, hash-verified run workspace preparation.

use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

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

/// Compute an unambiguous, versioned SHA-256 identity for an artifact directory.
///
/// The tree encoding includes record types and length-delimited UTF-8 relative paths
/// and file contents. It is intentionally independent of Tome's manifest hash format.
pub fn hash_artifact_directory(path: &Path) -> Result<ContentHash> {
    let root = canonical_directory(path, "artifact source")?;
    hash_evaluation_tree(&root).context("failed to hash artifact directory")
}

/// Resolve and hash a suite fixture before any runs are scheduled.
pub(crate) fn hash_fixture_directory(suite_root: &Path, relative: &str) -> Result<ContentHash> {
    validate_relative_path(relative, "fixture path")?;
    let suite_root = canonical_directory(suite_root, "suite directory")?;
    let fixture = resolve_fixture(&suite_root, relative)?;
    hash_evaluation_tree(&fixture).context("failed to hash fixture directory")
}

/// Resolve a not-yet-created workspace path and reject overlap before filesystem writes.
pub(crate) fn validate_workspace_candidate(
    path: &Path,
    suite_root: &Path,
    artifact_source: &Path,
) -> Result<PathBuf> {
    let candidate = normalized_absolute_path(path)?;
    match fs::symlink_metadata(&candidate) {
        Ok(_) => bail!(
            "workspace candidate '{}' already exists",
            candidate.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect workspace '{}'", candidate.display()));
        }
    }

    let canonical_candidate = canonicalize_missing_path(&candidate)?;
    let artifact_source = canonical_directory(artifact_source, "artifact source")?;
    let suite_root = canonical_directory(suite_root, "suite directory")?;
    ensure_disjoint(&canonical_candidate, &artifact_source, "artifact source")?;
    ensure_disjoint(&canonical_candidate, &suite_root, "suite directory")?;
    Ok(canonical_candidate)
}

/// Verify and copy the artifact and optional fixture into an existing empty workspace.
/// A suite-level fixture hash is passed in so every repetition uses the same fixture revision.
/// Partial workspace contents are removed if materialization fails.
pub fn materialize_run_workspace(
    suite_root: &Path,
    case: &EvaluationCase,
    variant: Variant,
    artifact_source: &Path,
    expected_artifact_hash: &ContentHash,
    expected_fixture_hash: Option<&ContentHash>,
    workspace: &Path,
) -> Result<MaterializedWorkspace> {
    let workspace = canonical_empty_workspace(workspace)?;
    let result = materialize_run_workspace_inner(
        suite_root,
        case,
        variant,
        artifact_source,
        expected_artifact_hash,
        expected_fixture_hash,
        &workspace,
    );
    if result.is_err() {
        let _ = clear_workspace_contents(&workspace);
    }
    result
}

fn materialize_run_workspace_inner(
    suite_root: &Path,
    case: &EvaluationCase,
    variant: Variant,
    artifact_source: &Path,
    expected_artifact_hash: &ContentHash,
    expected_fixture_hash: Option<&ContentHash>,
    workspace: &Path,
) -> Result<MaterializedWorkspace> {
    let artifact_source = canonical_directory(artifact_source, "artifact source")?;
    let suite_root = canonical_directory(suite_root, "suite directory")?;
    ensure_disjoint(workspace, &artifact_source, "artifact source")?;
    ensure_disjoint(workspace, &suite_root, "suite directory")?;

    let artifact_path = if variant == Variant::Treatment {
        let source_hash = hash_artifact_directory(&artifact_source)?;
        ensure!(
            &source_hash == expected_artifact_hash,
            "artifact hash does not match suite identity"
        );
        let destination = workspace.join("artifact");
        copy_tree(&artifact_source, &destination)?;
        ensure!(
            hash_evaluation_tree(&destination)? == *expected_artifact_hash,
            "copied artifact hash changed during materialization"
        );
        Some(destination)
    } else {
        None
    };

    let (fixture_path, fixture_hash) = match (&case.fixture, expected_fixture_hash) {
        (Some(relative), Some(expected_hash)) => {
            validate_relative_path(relative, "fixture path")?;
            let fixture_source = resolve_fixture(&suite_root, relative)?;
            ensure_disjoint(workspace, &fixture_source, "fixture source")?;
            let destination = workspace.join("fixtures");
            copy_tree(&fixture_source, &destination)?;
            ensure!(
                hash_evaluation_tree(&destination)? == *expected_hash,
                "copied fixture hash changed from the suite snapshot"
            );
            (Some(destination), Some(expected_hash.clone()))
        }
        (None, None) => (None, None),
        (Some(_), None) => bail!("suite fixture is missing its preflight hash"),
        (None, Some(_)) => bail!("fixture hash supplied for a case without a fixture"),
    };

    Ok(MaterializedWorkspace {
        artifact_path,
        fixture_path,
        artifact_hash: expected_artifact_hash.clone(),
        fixture_hash,
    })
}

fn hash_evaluation_tree(root: &Path) -> Result<ContentHash> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(root).min_depth(1).follow_links(false) {
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
        let relative = entry
            .path()
            .strip_prefix(root)
            .context("walked path escaped its source root")?;
        let relative = relative
            .to_str()
            .context("source path is not valid UTF-8")?
            .replace(std::path::MAIN_SEPARATOR, "/");
        validate_relative_path(&relative, "source path")?;
        entries.push((relative, entry.path().to_path_buf(), file_type.is_dir()));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    hasher.update(b"tome-evaluation-tree-v1\0");
    for (relative, path, is_directory) in entries {
        let path_bytes = relative.as_bytes();
        hasher.update(if is_directory { b"D" } else { b"F" });
        hasher.update((path_bytes.len() as u64).to_be_bytes());
        hasher.update(path_bytes);
        if !is_directory {
            let mut file = fs::File::open(&path)
                .with_context(|| format!("failed to open source file '{}'", path.display()))?;
            let expected_size = file.metadata()?.len();
            hasher.update(expected_size.to_be_bytes());
            let mut actual_size = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                actual_size = actual_size
                    .checked_add(count as u64)
                    .context("file size overflow while hashing evaluation tree")?;
                hasher.update(&buffer[..count]);
            }
            ensure!(
                actual_size == expected_size,
                "source file changed while hashing: '{}'",
                path.display()
            );
        }
    }
    let digest = hasher.finalize();
    ContentHash::new(
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    )
}

fn normalized_absolute_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    ensure!(
        normalized.is_absolute(),
        "workspace path must resolve to an absolute path"
    );
    Ok(normalized)
}

fn canonicalize_missing_path(path: &Path) -> Result<PathBuf> {
    let mut ancestor = path;
    let mut missing_components = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => {
                let canonical_ancestor = ancestor.canonicalize().with_context(|| {
                    format!(
                        "failed to resolve workspace parent '{}'",
                        ancestor.display()
                    )
                })?;
                ensure!(
                    canonical_ancestor.is_dir(),
                    "workspace parent '{}' must be a directory",
                    ancestor.display()
                );
                let mut candidate = canonical_ancestor;
                for component in missing_components.iter().rev() {
                    candidate.push(component);
                }
                return Ok(candidate);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let component = ancestor
                    .file_name()
                    .context("workspace path has no existing parent")?;
                missing_components.push(component.to_os_string());
                ancestor = ancestor
                    .parent()
                    .context("workspace path has no filesystem root")?;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to resolve workspace '{}'", path.display()));
            }
        }
    }
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

fn clear_workspace_contents(workspace: &Path) -> Result<()> {
    for entry in fs::read_dir(workspace)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "materialize_tests.rs"]
mod tests;

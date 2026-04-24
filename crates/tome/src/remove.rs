//! Remove a directory entry from config and clean up all artifacts.
//!
//! Cleanup order:
//! 1. Remove symlinks from distribution directories
//! 2. Remove library entries for skills from this directory
//! 3. Remove manifest entries
//! 4. Remove cached git repo (if git-type directory)
//! 5. Remove directory entry from config
//! 6. Regenerate lockfile

use anyhow::{Context, Result};
use console::style;
use std::path::PathBuf;

use crate::config::{Config, DirectoryName, DirectoryRole, DirectoryType};
use crate::manifest::Manifest;
use crate::paths::TomePaths;

/// What will be removed.
#[derive(Debug)]
pub(crate) struct RemovePlan {
    /// Name of the directory to remove.
    pub directory_name: DirectoryName,
    /// Skills from this directory found in the manifest.
    pub skills: Vec<String>,
    /// Symlinks in distribution directories pointing to these skills.
    pub symlinks_to_remove: Vec<PathBuf>,
    /// Library directories for these skills.
    pub library_paths: Vec<PathBuf>,
    /// Cached git repo path (if git-type directory).
    pub git_cache_path: Option<PathBuf>,
}

impl RemovePlan {
    /// Returns true if there is nothing to clean up (directory entry still removed from config).
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
            && self.symlinks_to_remove.is_empty()
            && self.library_paths.is_empty()
            && self.git_cache_path.is_none()
    }
}

/// Which cleanup step produced a partial failure.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FailureKind {
    /// Distribution-dir symlink removal (step 1).
    Symlink,
    /// Local library directory removal (step 2, non-symlink branch).
    LibraryDir,
    /// Managed-skill library symlink removal (step 2, symlink branch).
    LibrarySymlink,
    /// Git repo cache removal (step 4).
    GitCache,
}

/// A single partial-cleanup failure aggregated from `execute`.
#[derive(Debug)]
pub(crate) struct RemoveFailure {
    pub path: PathBuf,
    pub op: FailureKind,
    pub error: std::io::Error,
}

/// Result of executing the remove plan.
pub(crate) struct RemoveResult {
    pub symlinks_removed: usize,
    pub library_entries_removed: usize,
    pub git_cache_removed: bool,
    /// Partial-cleanup failures that occurred during `execute`.
    ///
    /// Empty on full success. Caller is responsible for surfacing these
    /// (currently `Command::Remove` in `lib.rs`) — `execute` itself no
    /// longer prints per-failure warnings.
    pub failures: Vec<RemoveFailure>,
}

/// Build a plan describing what `tome remove <name>` will do.
pub(crate) fn plan(
    name: &str,
    config: &Config,
    paths: &TomePaths,
    manifest: &Manifest,
) -> Result<RemovePlan> {
    let dir_name =
        DirectoryName::new(name).with_context(|| format!("invalid directory name: {name}"))?;

    // Validate the directory exists in config
    let dir_config = config
        .directories
        .get(&dir_name)
        .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", name))?;

    // Find skills from this directory in the manifest
    let skills: Vec<String> = manifest
        .iter()
        .filter(|(_, entry)| entry.source_name == name)
        .map(|(skill_name, _)| skill_name.as_str().to_string())
        .collect();

    // Find symlinks to remove from distribution directories (Target or Synced role)
    let mut symlinks_to_remove = Vec::new();
    for (other_name, other_config) in &config.directories {
        let role = other_config.role();
        if role != DirectoryRole::Target && role != DirectoryRole::Synced {
            continue;
        }
        // Skip the directory being removed
        if *other_name == dir_name {
            continue;
        }
        let skills_dir = &other_config.path;
        if !skills_dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(skills_dir)
            .with_context(|| format!("failed to read {}", skills_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_symlink() {
                let link_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                if skills.iter().any(|s| s == link_name) {
                    symlinks_to_remove.push(path);
                }
            }
        }
    }

    // Find library directories to remove
    let library_paths: Vec<PathBuf> = skills
        .iter()
        .map(|s| paths.library_dir().join(s))
        .filter(|p| p.exists())
        .collect();

    // Check for cached git repo
    let git_cache_path = if dir_config.directory_type == DirectoryType::Git {
        let url_str = dir_config
            .path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("directory '{}' path is not valid UTF-8", name))?;
        let cache_dir = crate::git::repo_cache_dir(&paths.repos_dir(), url_str);
        if cache_dir.exists() {
            Some(cache_dir)
        } else {
            None
        }
    } else {
        None
    };

    Ok(RemovePlan {
        directory_name: dir_name,
        skills,
        symlinks_to_remove,
        library_paths,
        git_cache_path,
    })
}

/// Render the plan to stdout.
pub(crate) fn render_plan(plan: &RemovePlan) {
    println!(
        "Remove plan for directory '{}':",
        style(AsRef::<str>::as_ref(&plan.directory_name)).cyan()
    );

    if plan.skills.is_empty() {
        println!("  No skills found in library from this directory.");
    } else {
        println!(
            "  Skills to remove from library: {}",
            style(plan.skills.len()).bold()
        );
        for skill in &plan.skills {
            println!("    - {}", skill);
        }
    }

    if !plan.symlinks_to_remove.is_empty() {
        println!(
            "  Symlinks to remove: {}",
            style(plan.symlinks_to_remove.len()).bold()
        );
    }

    if !plan.library_paths.is_empty() {
        println!(
            "  Library directories to remove: {}",
            style(plan.library_paths.len()).bold()
        );
    }

    if plan.git_cache_path.is_some() {
        println!("  Git repo cache will be removed.");
    }

    println!("  Config entry will be removed.");
}

/// Execute the remove plan.
pub(crate) fn execute(
    plan: &RemovePlan,
    config: &mut Config,
    manifest: &mut Manifest,
    dry_run: bool,
) -> Result<RemoveResult> {
    let mut symlinks_removed = 0;
    let mut library_entries_removed = 0;
    let mut git_cache_removed = false;
    let mut failures: Vec<RemoveFailure> = Vec::new();

    // 1. Remove symlinks from distribution directories
    for symlink in &plan.symlinks_to_remove {
        if dry_run {
            symlinks_removed += 1;
        } else {
            match std::fs::remove_file(symlink) {
                Ok(_) => symlinks_removed += 1,
                Err(e) => failures.push(RemoveFailure {
                    path: symlink.clone(),
                    op: FailureKind::Symlink,
                    error: e,
                }),
            }
        }
    }

    // 2. Remove library directories
    for lib_path in &plan.library_paths {
        if dry_run {
            library_entries_removed += 1;
        } else if lib_path.is_symlink() {
            match std::fs::remove_file(lib_path) {
                Ok(_) => library_entries_removed += 1,
                Err(e) => failures.push(RemoveFailure {
                    path: lib_path.clone(),
                    op: FailureKind::LibrarySymlink,
                    error: e,
                }),
            }
        } else if lib_path.is_dir() {
            match std::fs::remove_dir_all(lib_path) {
                Ok(_) => library_entries_removed += 1,
                Err(e) => failures.push(RemoveFailure {
                    path: lib_path.clone(),
                    op: FailureKind::LibraryDir,
                    error: e,
                }),
            }
        }
    }

    // 3. Remove manifest entries
    if !dry_run {
        for skill in &plan.skills {
            manifest.remove(skill);
        }
    }

    // 4. Remove cached git repo
    if let Some(cache_path) = &plan.git_cache_path {
        if dry_run {
            git_cache_removed = true;
        } else {
            match std::fs::remove_dir_all(cache_path) {
                Ok(_) => git_cache_removed = true,
                Err(e) => failures.push(RemoveFailure {
                    path: cache_path.clone(),
                    op: FailureKind::GitCache,
                    error: e,
                }),
            }
        }
    }

    // 5. Remove directory entry from config
    if !dry_run {
        config.directories.remove(&plan.directory_name);
    }

    Ok(RemoveResult {
        symlinks_removed,
        library_entries_removed,
        git_cache_removed,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};
    use crate::discover::SkillName;
    use crate::manifest::{Manifest, SkillEntry};
    use crate::validation::ContentHash;
    use std::collections::BTreeMap;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    fn test_hash() -> ContentHash {
        ContentHash::new("a".repeat(64)).unwrap()
    }

    fn make_test_setup() -> (TempDir, Config, TomePaths, Manifest) {
        let tmp = TempDir::new().unwrap();
        let library_dir = tmp.path().join("library");
        std::fs::create_dir_all(&library_dir).unwrap();

        let source_dir = tmp.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();

        let target_dir = tmp.path().join("target");
        std::fs::create_dir_all(&target_dir).unwrap();

        // Create a skill in the library
        let skill_dir = library_dir.join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# my-skill").unwrap();

        // Create a symlink in the target
        unix_fs::symlink(&skill_dir, target_dir.join("my-skill")).unwrap();

        let mut directories = BTreeMap::new();
        directories.insert(
            DirectoryName::new("test-source").unwrap(),
            DirectoryConfig {
                path: source_dir,
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Source),
                branch: None,
                tag: None,
                rev: None,
                subdir: None,
            },
        );
        directories.insert(
            DirectoryName::new("test-target").unwrap(),
            DirectoryConfig {
                path: target_dir,
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Target),
                branch: None,
                tag: None,
                rev: None,
                subdir: None,
            },
        );

        let config = Config {
            library_dir: library_dir.clone(),
            directories,
            ..Default::default()
        };

        let paths = TomePaths::new(tmp.path().to_path_buf(), library_dir).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("my-skill").unwrap(),
            SkillEntry::new(
                tmp.path().join("source/my-skill"),
                "test-source".to_string(),
                test_hash(),
                false,
            ),
        );

        (tmp, config, paths, manifest)
    }

    #[test]
    fn plan_errors_on_nonexistent_directory() {
        let (_tmp, config, paths, manifest) = make_test_setup();
        let result = plan("nonexistent", &config, &paths, &manifest);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found in config")
        );
    }

    #[test]
    fn plan_finds_skills_and_symlinks() {
        let (_tmp, config, paths, manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();
        assert_eq!(p.skills.len(), 1);
        assert_eq!(p.skills[0], "my-skill");
        assert_eq!(p.symlinks_to_remove.len(), 1);
        assert_eq!(p.library_paths.len(), 1);
    }

    #[test]
    fn execute_removes_artifacts() {
        let (_tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        let result = execute(&p, &mut config, &mut manifest, false).unwrap();
        assert_eq!(result.symlinks_removed, 1);
        assert_eq!(result.library_entries_removed, 1);
        assert!(
            !config
                .directories
                .contains_key(&DirectoryName::new("test-source").unwrap())
        );
        assert!(manifest.is_empty());
    }

    #[test]
    fn partial_failure_aggregates_symlink_error() {
        let (tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        // Pre-delete the distribution symlink so std::fs::remove_file returns
        // ENOENT during execute's step 1 loop — forcing a FailureKind::Symlink
        // push without affecting the library-entry step which should still
        // succeed.
        let dist_symlink = tmp.path().join("target").join("my-skill");
        assert_eq!(
            p.symlinks_to_remove.len(),
            1,
            "fixture expected one dist symlink"
        );
        assert_eq!(p.symlinks_to_remove[0], dist_symlink);
        std::fs::remove_file(&dist_symlink).ok();

        let result = execute(&p, &mut config, &mut manifest, false).unwrap();

        // Assert: exactly one Symlink failure, path matches the pre-deleted link.
        assert!(
            result
                .failures
                .iter()
                .any(|f| f.op == FailureKind::Symlink),
            "expected a FailureKind::Symlink failure, got: {:?}",
            result.failures,
        );
        let symlink_failure = result
            .failures
            .iter()
            .find(|f| f.op == FailureKind::Symlink)
            .unwrap();
        assert_eq!(symlink_failure.path, dist_symlink);

        // Partial-failure semantics: the library entry (separate artifact)
        // should still have been cleaned up.
        assert_eq!(result.library_entries_removed, 1);
        assert_eq!(result.symlinks_removed, 0);
    }

    #[test]
    fn execute_dry_run_preserves_state() {
        let (_tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        let result = execute(&p, &mut config, &mut manifest, true).unwrap();
        assert_eq!(result.symlinks_removed, 1);
        assert_eq!(result.library_entries_removed, 1);
        // Config and manifest should not be modified
        assert!(
            config
                .directories
                .contains_key(&DirectoryName::new("test-source").unwrap())
        );
        assert!(!manifest.is_empty());
    }
}

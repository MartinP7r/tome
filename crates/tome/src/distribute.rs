//! Distribute library skills to configured directories via symlinks.

use anyhow::{Context, Result};
use std::os::unix::fs as unix_fs;
use std::path::Path;

use crate::config::{DirectoryConfig, DirectoryName};
use crate::machine::MachinePrefs;
use crate::manifest::Manifest;
use crate::paths::symlink_points_to;

/// Result of distributing skills to a single directory.
#[derive(Debug)]
pub struct DistributeResult {
    pub changed: usize,
    pub unchanged: usize,
    /// Skills skipped because a non-symlink file already exists at the destination.
    pub skipped: usize,
    /// Skills skipped because they are disabled in machine preferences.
    pub disabled: usize,
    /// Skills skipped because they originate from the same directory (prevents circular symlinks).
    pub skipped_managed: usize,
    pub directory_name: DirectoryName,
}

/// Distribute skills from the library to a configured directory.
///
/// Creates symlinks in `dir_config.path` pointing to library entries.
/// When `force` is true, all symlinks are recreated even if they already point to the correct target.
/// The `manifest` is used to check whether a skill's source originated from this directory
/// (to prevent circular symlinks when a directory is both a source and target).
pub fn distribute_to_directory(
    library_dir: &Path,
    dir_name: &DirectoryName,
    dir_config: &DirectoryConfig,
    manifest: &Manifest,
    machine_prefs: &MachinePrefs,
    dry_run: bool,
    force: bool,
) -> Result<DistributeResult> {
    let skills_dir = &dir_config.path;

    if !dry_run {
        std::fs::create_dir_all(skills_dir)
            .with_context(|| format!("failed to create target dir {}", skills_dir.display()))?;
    }

    let mut result = DistributeResult {
        directory_name: dir_name.clone(),
        changed: 0,
        unchanged: 0,
        skipped: 0,
        disabled: 0,
        skipped_managed: 0,
    };

    // Library may not exist yet on a first dry-run (consolidate skips creating it).
    if !library_dir.is_dir() {
        return Ok(result);
    }

    // Read all entries in library (may be real directories for local skills or symlinks for managed skills)
    let entries = std::fs::read_dir(library_dir)
        .with_context(|| format!("failed to read library dir {}", library_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", library_dir.display()))?;
        let skill_name = entry.file_name();
        let skill_name_str = skill_name.to_string_lossy();
        let library_skill_path = entry.path();
        let target_link = skills_dir.join(&skill_name);

        // Skip non-directory entries (e.g. .tome-manifest.json, .gitignore)
        if !library_skill_path.is_dir() {
            continue;
        }

        // Skip skills not allowed for this directory (global disabled + per-directory filtering)
        if !machine_prefs.is_skill_allowed(&skill_name_str, dir_name.as_str()) {
            result.disabled += 1;
            continue;
        }

        // Skip skills that originate from the same directory we're distributing to.
        // This prevents circular symlinks when a directory has a Synced role
        // (both discovery source and distribution target).
        if let Some(manifest_entry) = manifest.get(skill_name_str.as_ref())
            && manifest_entry.source_name == dir_name.as_str()
        {
            // Remove any existing symlink from a previous sync that
            // didn't have this check (cleans up legacy duplicates).
            if !dry_run
                && target_link.is_symlink()
                && let Err(e) = std::fs::remove_file(&target_link)
            {
                eprintln!(
                    "warning: failed to remove legacy symlink {}: {}",
                    target_link.display(),
                    e
                );
            }
            result.skipped_managed += 1;
            continue;
        }

        if target_link.is_symlink() {
            if symlink_points_to(&target_link, &library_skill_path) && !force {
                result.unchanged += 1;
                continue;
            }
            // Update stale link (or force-recreating)
            if !dry_run {
                std::fs::remove_file(&target_link).with_context(|| {
                    format!("failed to remove stale symlink {}", target_link.display())
                })?;
            }
        } else if target_link.exists() {
            eprintln!(
                "warning: {} exists in target and is not a symlink, skipping",
                target_link.display()
            );
            result.skipped += 1;
            continue;
        }

        if !dry_run {
            unix_fs::symlink(&library_skill_path, &target_link).with_context(|| {
                format!(
                    "failed to symlink {} -> {}",
                    target_link.display(),
                    library_skill_path.display()
                )
            })?;
        }
        result.changed += 1;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryConfig, DirectoryName, DirectoryType};
    use crate::machine::MachinePrefs;
    use crate::manifest::SkillEntry;
    use tempfile::TempDir;

    fn setup_library(dir: &std::path::Path, skill_names: &[&str]) {
        for name in skill_names {
            let skill_dir = dir.join(name);
            std::fs::create_dir_all(&skill_dir).unwrap();
            std::fs::write(skill_dir.join("SKILL.md"), "# test").unwrap();
        }
    }

    fn empty_manifest() -> Manifest {
        Manifest::default()
    }

    fn make_dir_config(path: std::path::PathBuf) -> DirectoryConfig {
        DirectoryConfig {
            path,
            directory_type: DirectoryType::Directory,
            role: None,
            branch: None,
            tag: None,
            rev: None,

            subdir: None,
            override_applied: false,
        }
    }

    #[test]
    fn distribute_creates_symlinks() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a", "skill-b"]);

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &empty_manifest(),
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 2);
        assert!(target_dir.path().join("skill-a").is_symlink());
        assert!(target_dir.path().join("skill-b").is_symlink());
    }

    #[test]
    fn distribute_idempotent() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());
        let manifest = empty_manifest();

        distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 0);
        assert_eq!(result.unchanged, 1);
    }

    #[test]
    fn distribute_force_recreates_links() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());
        let manifest = empty_manifest();

        distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &manifest,
            &MachinePrefs::default(),
            false,
            true,
        )
        .unwrap();
        assert_eq!(result.changed, 1, "force should recreate unchanged link");
        assert_eq!(result.unchanged, 0);
    }

    #[test]
    fn distribute_idempotent_with_canonicalized_paths() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        let target_dir = tmp.path().join("target");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::create_dir_all(&target_dir).unwrap();

        // Create a real library entry
        let skill_dir = lib_dir.join("skill-a");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# test").unwrap();

        // Manually create a relative symlink in target: ../library/skill-a
        unix_fs::symlink(
            std::path::Path::new("../library/skill-a"),
            target_dir.join("skill-a"),
        )
        .unwrap();

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.clone());

        let result = distribute_to_directory(
            &lib_dir,
            &dir_name,
            &dir_config,
            &empty_manifest(),
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(
            result.unchanged, 1,
            "relative symlink should be recognized as matching"
        );
        assert_eq!(result.changed, 0);
    }

    #[test]
    fn distribute_updates_stale_link() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());
        let manifest = empty_manifest();

        // First distribute: creates the link
        distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();

        // Simulate the target link now pointing somewhere else (stale)
        let stale_path = target_dir.path().join("skill-a");
        std::fs::remove_file(&stale_path).unwrap();
        let other = TempDir::new().unwrap();
        unix_fs::symlink(other.path(), &stale_path).unwrap();

        // Second distribute: should update the stale link
        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 1, "stale link should be updated");
        assert_eq!(result.unchanged, 0);

        // Link should now point to the library entry
        let link_target = std::fs::read_link(&stale_path).unwrap();
        assert_eq!(link_target, library.path().join("skill-a"));
    }

    #[test]
    fn distribute_dry_run_with_nonexistent_library() {
        let tmp = TempDir::new().unwrap();
        let nonexistent_library = tmp.path().join("library-never-created");
        let target_dir = TempDir::new().unwrap();

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let result = distribute_to_directory(
            &nonexistent_library,
            &dir_name,
            &dir_config,
            &empty_manifest(),
            &MachinePrefs::default(),
            true,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 0);
        assert_eq!(result.unchanged, 0);
    }

    #[test]
    fn distribute_dry_run_doesnt_create_dir() {
        let library = TempDir::new().unwrap();
        let tmp = TempDir::new().unwrap();
        let nonexistent_target = tmp.path().join("does-not-exist");
        setup_library(library.path(), &["skill-a"]);

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(nonexistent_target.clone());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &empty_manifest(),
            &MachinePrefs::default(),
            true,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 1);
        assert!(!nonexistent_target.exists());
    }

    #[test]
    fn distribute_skips_non_symlink_collision() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        std::fs::write(target_dir.path().join("skill-a"), "not a symlink").unwrap();

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &empty_manifest(),
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 0);
        assert_eq!(result.unchanged, 0);

        let content = std::fs::read_to_string(target_dir.path().join("skill-a")).unwrap();
        assert_eq!(content, "not a symlink");
    }

    #[test]
    fn distribute_skips_manifest_file() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        // Create a skill dir AND a manifest file in library
        setup_library(library.path(), &["skill-a"]);
        std::fs::write(library.path().join(".tome-manifest.json"), "{}").unwrap();

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &Manifest::default(),
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();

        assert_eq!(
            result.changed, 1,
            "only the skill dir should be distributed"
        );
        assert!(
            !target_dir.path().join(".tome-manifest.json").exists(),
            "manifest file should not be symlinked to target"
        );
    }

    #[test]
    fn distribute_skips_skills_from_same_directory() {
        // Skill discovered from directory "foo" should NOT be distributed back to "foo"
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        setup_library(library.path(), &["my-skill"]);

        // Manifest records this skill as originating from "my-dir"
        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            SkillEntry {
                source_path: target_dir.path().join("my-skill"),
                source_name: "my-dir".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );

        // Distribute to the SAME directory name → should skip
        let dir_name = DirectoryName::new("my-dir").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.skipped_managed, 1);
        assert_eq!(result.changed, 0);
        assert!(
            !target_dir.path().join("my-skill").exists(),
            "skill should NOT be distributed back to its own directory"
        );
    }

    #[test]
    fn distribute_allows_skills_to_different_directory() {
        // Skill discovered from "alpha" SHOULD be distributed to "beta"
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        setup_library(library.path(), &["my-skill"]);

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            SkillEntry {
                source_path: std::path::PathBuf::from("/some/alpha/my-skill"),
                source_name: "alpha".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: true,
            },
        );

        // Distribute to a DIFFERENT directory name → should succeed
        let dir_name = DirectoryName::new("beta").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 1);
        assert!(
            target_dir.path().join("my-skill").is_symlink(),
            "skill SHOULD be distributed to a different directory"
        );
    }

    #[test]
    fn distribute_skips_disabled_skills() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["enabled-skill", "disabled-skill"]);

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let mut prefs = MachinePrefs::default();
        prefs.disable(crate::discover::SkillName::new("disabled-skill").unwrap());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &empty_manifest(),
            &prefs,
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 1);
        assert_eq!(result.disabled, 1);
        assert!(target_dir.path().join("enabled-skill").is_symlink());
        assert!(!target_dir.path().join("disabled-skill").exists());
    }

    #[test]
    fn distribute_cleans_up_legacy_symlinks_for_same_dir_skills() {
        // If a skill was previously distributed to its own directory (before the
        // origin check), the legacy symlink should be cleaned up on re-sync.
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        setup_library(library.path(), &["my-skill"]);

        // Pre-create a legacy symlink in target
        unix_fs::symlink(
            library.path().join("my-skill"),
            target_dir.path().join("my-skill"),
        )
        .unwrap();
        assert!(target_dir.path().join("my-skill").is_symlink());

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            SkillEntry {
                source_path: target_dir.path().join("my-skill"),
                source_name: "my-dir".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: true,
            },
        );

        let dir_name = DirectoryName::new("my-dir").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.skipped_managed, 1);
        assert!(
            !target_dir.path().join("my-skill").exists(),
            "legacy symlink should be removed on re-sync"
        );
    }
}

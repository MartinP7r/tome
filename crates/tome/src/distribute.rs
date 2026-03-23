//! Distribute library skills to target tools via symlinks.

use anyhow::{Context, Result};
use std::os::unix::fs as unix_fs;
use std::path::Path;

use crate::config::{TargetConfig, TargetMethod, TargetName};
use crate::machine::MachinePrefs;
use crate::manifest::Manifest;
use crate::paths::symlink_points_to;

/// Result of distributing skills to a single target.
#[derive(Debug)]
pub struct DistributeResult {
    pub changed: usize,
    pub unchanged: usize,
    /// Skills skipped because a non-symlink file already exists at the destination.
    pub skipped: usize,
    /// Skills skipped because they are disabled in machine preferences.
    pub disabled: usize,
    pub target_name: TargetName,
}

/// Distribute skills from the library to a target tool.
/// When `force` is true, all symlinks are recreated even if they already point to the correct target.
/// The `manifest` is used to check whether a skill's source originated from the target dir
/// (to prevent circular symlinks when a directory is both a source and target).
pub fn distribute_to_target(
    library_dir: &Path,
    target_name: &str,
    target: &TargetConfig,
    manifest: &Manifest,
    machine_prefs: &MachinePrefs,
    dry_run: bool,
    force: bool,
) -> Result<DistributeResult> {
    if !target.enabled {
        return Ok(DistributeResult {
            target_name: TargetName::new(target_name)?,
            changed: 0,
            unchanged: 0,
            skipped: 0,
            disabled: 0,
        });
    }

    match &target.method {
        TargetMethod::Symlink { skills_dir } => distribute_symlinks(
            library_dir,
            target_name,
            skills_dir,
            manifest,
            machine_prefs,
            dry_run,
            force,
        ),
    }
}

/// Distribute via directory-level symlinks.
fn distribute_symlinks(
    library_dir: &Path,
    target_name: &str,
    skills_dir: &Path,
    manifest: &Manifest,
    machine_prefs: &MachinePrefs,
    dry_run: bool,
    force: bool,
) -> Result<DistributeResult> {
    if !dry_run {
        std::fs::create_dir_all(skills_dir)
            .with_context(|| format!("failed to create target dir {}", skills_dir.display()))?;
    }

    let mut result = DistributeResult {
        target_name: TargetName::new(target_name)?,
        changed: 0,
        unchanged: 0,
        skipped: 0,
        disabled: 0,
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

        // Skip non-directory entries (e.g. .tome-manifest.json)
        if !library_skill_path.is_dir() {
            continue;
        }

        // Skip skills disabled in machine preferences
        if machine_prefs.is_disabled(&skill_name_str) {
            result.disabled += 1;
            continue;
        }

        // Skip skills whose original source is already inside this target dir.
        // This prevents circular symlinks when a directory is both a source and target
        // (e.g. ~/.claude/skills used as both).
        if let Some(manifest_entry) = manifest.get(skill_name_str.as_ref()) {
            match (
                manifest_entry.source_path.canonicalize(),
                skills_dir.canonicalize(),
            ) {
                (Ok(source), Ok(target)) if source.starts_with(&target) => {
                    result.unchanged += 1;
                    continue;
                }
                // If either path can't be canonicalized (e.g. target dir doesn't
                // exist yet in dry-run mode), they can't be the same physical
                // directory — no circular symlink risk.
                (Err(_), _) | (_, Err(_)) => {}
                _ => {}
            }
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
    use crate::machine::MachinePrefs;
    use crate::manifest::SkillEntry;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_library(dir: &Path, skill_names: &[&str]) {
        for name in skill_names {
            let skill_dir = dir.join(name);
            std::fs::create_dir_all(&skill_dir).unwrap();
            std::fs::write(skill_dir.join("SKILL.md"), "# test").unwrap();
        }
    }

    fn empty_manifest() -> Manifest {
        Manifest::default()
    }

    #[test]
    fn distribute_symlinks_creates_links() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a", "skill-b"]);

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: target_dir.path().to_path_buf(),
            },
        };

        let manifest = empty_manifest();
        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
            &manifest,
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
    fn distribute_symlinks_idempotent() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: target_dir.path().to_path_buf(),
            },
        };

        let manifest = empty_manifest();
        distribute_to_target(
            library.path(),
            "test",
            &target,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
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
    fn distribute_symlinks_force_recreates_links() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: target_dir.path().to_path_buf(),
            },
        };

        let manifest = empty_manifest();
        distribute_to_target(
            library.path(),
            "test",
            &target,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
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

        // Create a real library entry (v0.2 style)
        let skill_dir = lib_dir.join("skill-a");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# test").unwrap();

        // Manually create a relative symlink in target: ../library/skill-a
        unix_fs::symlink(
            std::path::Path::new("../library/skill-a"),
            target_dir.join("skill-a"),
        )
        .unwrap();

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: target_dir.clone(),
            },
        };

        let manifest = empty_manifest();
        let result = distribute_to_target(
            &lib_dir,
            "test",
            &target,
            &manifest,
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
    fn distribute_symlinks_updates_stale_link() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: target_dir.path().to_path_buf(),
            },
        };

        let manifest = empty_manifest();
        // First distribute: creates the link
        distribute_to_target(
            library.path(),
            "test",
            &target,
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
        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
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
    fn distribute_disabled_target_is_noop() {
        let library = TempDir::new().unwrap();
        let target = TargetConfig {
            enabled: false,
            method: TargetMethod::Symlink {
                skills_dir: PathBuf::from("/unused"),
            },
        };

        let manifest = empty_manifest();
        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 0);
    }

    #[test]
    fn distribute_symlinks_dry_run_with_nonexistent_library() {
        let tmp = TempDir::new().unwrap();
        let nonexistent_library = tmp.path().join("library-never-created");
        let target_dir = TempDir::new().unwrap();

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: target_dir.path().to_path_buf(),
            },
        };

        let manifest = empty_manifest();
        let result = distribute_to_target(
            &nonexistent_library,
            "test",
            &target,
            &manifest,
            &MachinePrefs::default(),
            true,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 0);
        assert_eq!(result.unchanged, 0);
    }

    #[test]
    fn distribute_symlinks_dry_run_doesnt_create_dir() {
        let library = TempDir::new().unwrap();
        let tmp = TempDir::new().unwrap();
        let nonexistent_target = tmp.path().join("does-not-exist");
        setup_library(library.path(), &["skill-a"]);

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: nonexistent_target.clone(),
            },
        };

        let manifest = empty_manifest();
        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
            &manifest,
            &MachinePrefs::default(),
            true,
            false,
        )
        .unwrap();
        assert_eq!(result.changed, 1);
        assert!(!nonexistent_target.exists());
    }

    #[test]
    fn distribute_symlinks_skips_non_symlink_collision() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);

        std::fs::write(target_dir.path().join("skill-a"), "not a symlink").unwrap();

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: target_dir.path().to_path_buf(),
            },
        };

        let manifest = empty_manifest();
        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
            &manifest,
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
    fn distribute_symlinks_skips_manifest_file() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        // Create a skill dir AND a manifest file in library
        setup_library(library.path(), &["skill-a"]);
        std::fs::write(library.path().join(".tome-manifest.json"), "{}").unwrap();

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: target_dir.path().to_path_buf(),
            },
        };
        let manifest = Manifest::default();
        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
            &manifest,
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
    fn distribute_skips_skills_originating_from_target_dir() {
        // Simulate: ~/.claude/skills is both a source and a target.
        // The library has a real copy (v0.2), and the manifest records the source.
        let source_and_target = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        // Create a real skill in what will be both source and target
        let skill_dir = source_and_target.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# my-skill").unwrap();

        // Library has a real copy (v0.2 style)
        let lib_skill = library.path().join("my-skill");
        std::fs::create_dir_all(&lib_skill).unwrap();
        std::fs::write(lib_skill.join("SKILL.md"), "# my-skill").unwrap();

        // Manifest records the source origin
        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            SkillEntry {
                source_path: skill_dir.clone(),
                source_name: "test".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: source_and_target.path().to_path_buf(),
            },
        };

        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
            &manifest,
            &MachinePrefs::default(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.unchanged, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.changed, 0);
    }

    #[test]
    fn distribute_skips_disabled_skills() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        setup_library(library.path(), &["enabled-skill", "disabled-skill"]);

        let target = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: target_dir.path().to_path_buf(),
            },
        };

        let mut prefs = MachinePrefs::default();
        prefs.disable(crate::discover::SkillName::new("disabled-skill").unwrap());

        let manifest = empty_manifest();
        let result = distribute_to_target(
            library.path(),
            "test",
            &target,
            &manifest,
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
}

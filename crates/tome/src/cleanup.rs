//! Remove stale entries from the library and broken symlinks from target directories.
//!
//! Library cleanup compares manifest entries against currently discovered skill names.
//! Target cleanup still removes broken symlinks pointing into the library.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::Path;

use crate::discover::SkillName;
use crate::manifest::Manifest;
use crate::paths::resolve_symlink_target;

/// Result of cleanup operation.
#[derive(Debug, Default)]
pub struct CleanupResult {
    pub removed_from_library: usize,
}

/// Remove library entries whose skills are no longer present in any discovered source.
///
/// When stdin is a TTY and `quiet` is false, prompts the user before removing each
/// stale entry. Otherwise, warns to stderr and removes automatically.
pub fn cleanup_library(
    library_dir: &Path,
    discovered_names: &HashSet<String>,
    manifest: &mut Manifest,
    dry_run: bool,
    quiet: bool,
) -> Result<CleanupResult> {
    let mut result = CleanupResult::default();

    if !library_dir.is_dir() {
        return Ok(result);
    }

    let interactive = std::io::stdin().is_terminal() && !quiet;

    // Find manifest entries not in discovered_names
    let stale: Vec<SkillName> = manifest
        .keys()
        .filter(|name| !discovered_names.contains(name.as_str()))
        .cloned()
        .collect();

    for name in stale {
        let entry_path = library_dir.join(name.as_str());

        if interactive {
            let prompt = format!(
                "Skill '{}' was removed from sources. Delete from library?",
                name
            );
            let confirmed = dialoguer::Confirm::new()
                .with_prompt(prompt)
                .default(false)
                .interact_opt()?;

            if confirmed != Some(true) {
                continue;
            }
        } else {
            eprintln!(
                "warning: skill '{}' no longer in any source, removing from library",
                name
            );
        }

        if !dry_run {
            if entry_path.is_symlink() {
                // Managed skill — remove the symlink
                std::fs::remove_file(&entry_path).with_context(|| {
                    format!("failed to remove managed symlink {}", entry_path.display())
                })?;
            } else if entry_path.is_dir() {
                // Local skill — remove the directory
                std::fs::remove_dir_all(&entry_path).with_context(|| {
                    format!("failed to remove stale skill dir {}", entry_path.display())
                })?;
            }
            manifest.remove(name.as_str());
        }
        result.removed_from_library += 1;
    }

    // Also remove broken symlinks in the library (managed skill whose source was deleted, or orphan from a previous layout)
    let entries = std::fs::read_dir(library_dir)
        .with_context(|| format!("failed to read library dir {}", library_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", library_dir.display()))?;
        let path = entry.path();

        if path.is_symlink() {
            let raw_target = std::fs::read_link(&path)
                .with_context(|| format!("failed to read symlink {}", path.display()))?;
            let target = resolve_symlink_target(&path, &raw_target);
            if !target.exists() {
                if !dry_run {
                    std::fs::remove_file(&path).with_context(|| {
                        format!("failed to remove broken symlink {}", path.display())
                    })?;
                }
                result.removed_from_library += 1;
            }
        }
    }

    Ok(result)
}

/// Remove stale symlinks from a target directory.
pub fn cleanup_target(target_dir: &Path, library_dir: &Path, dry_run: bool) -> Result<usize> {
    if !target_dir.is_dir() {
        return Ok(0);
    }

    let mut removed = 0;

    // Canonicalize library_dir so that starts_with works when library_dir itself
    // contains a symlink component (e.g., /var -> /private/var on macOS).
    // We keep both forms so we can match symlinks created with either path variant.
    let canonical_library = std::fs::canonicalize(library_dir).unwrap_or_else(|e| {
        eprintln!(
            "warning: could not canonicalize library path {}: {} — symlinks using canonical paths may not be cleaned up",
            library_dir.display(),
            e
        );
        library_dir.to_path_buf()
    });

    let entries = std::fs::read_dir(target_dir)
        .with_context(|| format!("failed to read target dir {}", target_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", target_dir.display()))?;
        let path = entry.path();

        if path.is_symlink() {
            let raw_target = std::fs::read_link(&path)
                .with_context(|| format!("failed to read symlink {}", path.display()))?;
            let target = resolve_symlink_target(&path, &raw_target);

            // Match against both the original and canonical library path so we correctly
            // handle macOS /var -> /private/var symlinks and similar platform quirks.
            let points_into_library =
                target.starts_with(library_dir) || target.starts_with(&canonical_library);

            // Remove if it points into the library dir but the library entry is gone
            if points_into_library && !target.exists() {
                if !dry_run {
                    std::fs::remove_file(&path).with_context(|| {
                        format!("failed to remove stale symlink {}", path.display())
                    })?;
                }
                removed += 1;
            }
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    #[test]
    fn cleanup_removes_stale_manifest_entries() {
        let library = TempDir::new().unwrap();

        // Create a skill dir and manifest entry
        let skill_dir = library.path().join("old-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# old").unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("old-skill").unwrap(),
            crate::manifest::SkillEntry {
                source_path: std::path::PathBuf::from("/tmp/source/old-skill"),
                source_name: "test".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );

        // "old-skill" is NOT in discovered names — should be removed (non-interactive)
        let discovered: HashSet<String> = HashSet::new();
        let result =
            cleanup_library(library.path(), &discovered, &mut manifest, false, false).unwrap();

        assert_eq!(result.removed_from_library, 1);
        assert!(!library.path().join("old-skill").exists());
        assert!(!manifest.contains_key("old-skill"));
    }

    #[test]
    fn cleanup_preserves_current_skills() {
        let library = TempDir::new().unwrap();

        let skill_dir = library.path().join("keep-me");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("keep-me").unwrap(),
            crate::manifest::SkillEntry {
                source_path: std::path::PathBuf::from("/tmp/source/keep-me"),
                source_name: "test".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );

        let discovered: HashSet<String> = ["keep-me".to_string()].into();
        let result =
            cleanup_library(library.path(), &discovered, &mut manifest, false, false).unwrap();

        assert_eq!(result.removed_from_library, 0);
        assert!(library.path().join("keep-me").exists());
    }

    #[test]
    fn cleanup_dry_run_preserves_stale() {
        let library = TempDir::new().unwrap();

        let skill_dir = library.path().join("stale");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("stale").unwrap(),
            crate::manifest::SkillEntry {
                source_path: std::path::PathBuf::from("/tmp/source/stale"),
                source_name: "test".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );

        let discovered: HashSet<String> = HashSet::new();
        let result =
            cleanup_library(library.path(), &discovered, &mut manifest, true, false).unwrap();

        assert_eq!(result.removed_from_library, 1);
        // Should still exist in dry run
        assert!(library.path().join("stale").exists());
        // Manifest should still have the entry in dry run
        assert!(manifest.contains_key("stale"));
    }

    #[test]
    fn cleanup_removes_broken_legacy_symlinks() {
        let library = TempDir::new().unwrap();

        // Create a broken v0.1.x symlink
        unix_fs::symlink("/nonexistent/path", library.path().join("broken")).unwrap();

        let mut manifest = Manifest::default();
        let discovered: HashSet<String> = HashSet::new();
        let result =
            cleanup_library(library.path(), &discovered, &mut manifest, false, false).unwrap();

        assert_eq!(result.removed_from_library, 1);
        assert!(!library.path().join("broken").exists());
    }

    #[test]
    fn cleanup_target_removes_stale_links() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Symlink in target pointing to a non-existent library entry
        let phantom = library.path().join("deleted-skill");
        unix_fs::symlink(&phantom, target.path().join("deleted-skill")).unwrap();

        let removed = cleanup_target(target.path(), library.path(), false).unwrap();
        assert_eq!(removed, 1);
    }

    #[test]
    fn cleanup_target_dry_run_preserves_stale_links() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        let phantom = library.path().join("deleted-skill");
        unix_fs::symlink(&phantom, target.path().join("deleted-skill")).unwrap();

        let removed = cleanup_target(target.path(), library.path(), true).unwrap();
        assert_eq!(removed, 1, "dry-run should count the stale link");
        assert!(
            target.path().join("deleted-skill").is_symlink(),
            "dry-run should not remove the symlink"
        );
    }

    #[test]
    fn cleanup_target_preserves_external_symlinks() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Broken symlink pointing INTO library dir (should be removed)
        let library_phantom = library.path().join("deleted-skill");
        unix_fs::symlink(&library_phantom, target.path().join("library-link")).unwrap();

        // Broken symlink pointing OUTSIDE library dir (should be preserved)
        unix_fs::symlink("/some/external/path", target.path().join("external-link")).unwrap();

        let removed = cleanup_target(target.path(), library.path(), false).unwrap();
        assert_eq!(removed, 1);
        assert!(!target.path().join("library-link").exists());
        assert!(target.path().join("external-link").is_symlink());
    }

    #[test]
    fn cleanup_dry_run_preserves_managed_symlink() {
        let library = TempDir::new().unwrap();

        // Create a broken symlink simulating a managed skill whose source was removed
        unix_fs::symlink("/nonexistent", library.path().join("stale-skill")).unwrap();
        assert!(library.path().join("stale-skill").is_symlink());

        // Manifest has NO entry for stale-skill — it is stale
        let mut manifest = Manifest::default();
        let discovered: HashSet<String> = HashSet::new();

        let result =
            cleanup_library(library.path(), &discovered, &mut manifest, true, false).unwrap();

        // Dry-run should report it would clean up but not actually remove
        assert!(
            result.removed_from_library > 0,
            "dry-run should count the stale symlink as would-be-removed"
        );
        assert!(
            library.path().join("stale-skill").is_symlink(),
            "dry-run should preserve the symlink on disk"
        );
    }

    #[test]
    fn cleanup_removes_managed_symlink() {
        let library = TempDir::new().unwrap();
        let source = tempfile::TempDir::new().unwrap();

        // Create a managed skill symlink in the library
        let skill_source = source.path().join("plugin-skill");
        std::fs::create_dir_all(&skill_source).unwrap();
        std::fs::write(skill_source.join("SKILL.md"), "# test").unwrap();
        unix_fs::symlink(&skill_source, library.path().join("plugin-skill")).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("plugin-skill").unwrap(),
            crate::manifest::SkillEntry {
                source_path: skill_source,
                source_name: "plugins".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: true,
            },
        );

        // Skill not in discovered names — should be removed
        let discovered: HashSet<String> = HashSet::new();
        let result =
            cleanup_library(library.path(), &discovered, &mut manifest, false, false).unwrap();

        assert_eq!(result.removed_from_library, 1);
        assert!(
            !library.path().join("plugin-skill").exists(),
            "managed symlink should be removed"
        );
        assert!(!manifest.contains_key("plugin-skill"));
    }
}

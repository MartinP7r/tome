//! Consolidate discovered skills into the library directory by copying.
//! Idempotent — unchanged skills (by content hash) are skipped, changed skills are re-copied.
//! Transparent migration: v0.1.x symlinks are automatically converted to real directories.

use anyhow::{Context, Result};
use std::path::Path;

use crate::discover::DiscoveredSkill;
use crate::manifest::{self, Manifest, SkillEntry};

/// Result of a consolidation operation.
#[derive(Debug, Default)]
pub struct ConsolidateResult {
    pub created: usize,
    pub unchanged: usize,
    pub updated: usize,
    /// Skills skipped because a non-managed entry already exists at the library path.
    pub skipped: usize,
}

/// Consolidate discovered skills into the library directory by copying.
///
/// Each skill directory is copied into `library_dir/{skill_name}`.
/// A manifest tracks content hashes for idempotent updates.
/// When `force` is true, all skills are re-copied regardless of hash.
///
/// Returns both the operation result and the (possibly updated) manifest so the
/// caller can pass it directly to distribute/cleanup without a redundant disk read.
/// In dry-run mode the manifest is never written to disk, so returning it here is
/// the only way downstream steps see the would-be-updated state.
pub fn consolidate(
    skills: &[DiscoveredSkill],
    library_dir: &Path,
    dry_run: bool,
    force: bool,
) -> Result<(ConsolidateResult, Manifest)> {
    if !dry_run {
        std::fs::create_dir_all(library_dir)
            .with_context(|| format!("failed to create library dir {}", library_dir.display()))?;
    }

    let mut manifest = if library_dir.is_dir() {
        manifest::load(library_dir)?
    } else {
        Manifest::default()
    };

    let mut result = ConsolidateResult::default();

    for skill in skills {
        let dest = library_dir.join(skill.name.as_str());
        let content_hash = manifest::hash_directory(&skill.path)?;

        // Check for v0.1.x symlink — migrate transparently
        if dest.is_symlink() {
            if !dry_run {
                // Resolve the symlink target, remove the link, then copy
                let resolved = std::fs::read_link(&dest)
                    .with_context(|| format!("failed to read symlink {}", dest.display()))?;
                let abs_resolved = if resolved.is_relative() {
                    library_dir.join(&resolved)
                } else {
                    resolved
                };
                std::fs::remove_file(&dest)
                    .with_context(|| format!("failed to remove v0.1 symlink {}", dest.display()))?;
                // Copy from the resolved symlink target (the original source)
                if abs_resolved.is_dir() {
                    copy_dir_recursive(&abs_resolved, &dest)?;
                } else {
                    // Symlink target is gone — copy from the discovered source instead
                    eprintln!(
                        "warning: v0.1 symlink target for '{}' is gone, copying from current source",
                        skill.name
                    );
                    copy_dir_recursive(&skill.path, &dest)?;
                }
                manifest.insert(
                    skill.name.clone(),
                    SkillEntry::new(
                        skill.path.clone(),
                        skill.source_name.clone(),
                        content_hash.clone(),
                    ),
                );
            }
            result.updated += 1;
            continue;
        }

        // Check manifest for existing entry
        if let Some(entry) = manifest.get(skill.name.as_str()) {
            if entry.content_hash == content_hash && !force {
                result.unchanged += 1;
                continue;
            }
            // Content changed or force — re-copy
            if !dry_run {
                if dest.is_dir() {
                    std::fs::remove_dir_all(&dest).with_context(|| {
                        format!("failed to remove old skill dir {}", dest.display())
                    })?;
                }
                copy_dir_recursive(&skill.path, &dest)?;
                manifest.insert(
                    skill.name.clone(),
                    SkillEntry::new(
                        skill.path.clone(),
                        skill.source_name.clone(),
                        content_hash.clone(),
                    ),
                );
            }
            result.updated += 1;
        } else if dest.exists() && !manifest.contains_key(skill.name.as_str()) {
            // Something exists that's NOT in the manifest — skip with warning
            eprintln!(
                "warning: {} exists but is not in the manifest, skipping",
                dest.display()
            );
            result.skipped += 1;
        } else {
            // New skill — copy
            if !dry_run {
                copy_dir_recursive(&skill.path, &dest)?;
                manifest.insert(
                    skill.name.clone(),
                    SkillEntry::new(
                        skill.path.clone(),
                        skill.source_name.clone(),
                        content_hash.clone(),
                    ),
                );
            }
            result.created += 1;
        }
    }

    if !dry_run && library_dir.is_dir() {
        manifest::save(&manifest, library_dir)?;
    }

    Ok((result, manifest))
}

/// Recursively copy a directory from `src` to `dst`.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("failed to create {}", dst.display()))?;

    for entry in walkdir::WalkDir::new(src).follow_links(false).into_iter() {
        let entry = entry.with_context(|| format!("failed to walk directory {}", src.display()))?;
        let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
        let target = dst.join(rel);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)
                .with_context(|| format!("failed to create dir {}", target.display()))?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create dir {}", parent.display()))?;
            }
            std::fs::copy(entry.path(), &target).with_context(|| {
                format!(
                    "failed to copy {} -> {}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        } else if entry.file_type().is_symlink() {
            // Skip symlinks inside skill dirs — we don't follow them
            eprintln!(
                "warning: skipping symlink inside skill dir: {}",
                entry.path().display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_skill(dir: &Path, name: &str) -> DiscoveredSkill {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# test").unwrap();
        DiscoveredSkill {
            name: crate::discover::SkillName::new(name).unwrap(),
            path: skill_dir,
            source_name: "test".into(),
        }
    }

    #[test]
    fn consolidate_copies_skills() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let (result, _manifest) = consolidate(&[skill], library.path(), false, false).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.unchanged, 0);

        let dest = library.path().join("my-skill");
        assert!(dest.is_dir());
        assert!(!dest.is_symlink());
        assert!(dest.join("SKILL.md").is_file());
    }

    #[test]
    fn consolidate_idempotent() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        consolidate(std::slice::from_ref(&skill), library.path(), false, false).unwrap();
        let (result, _manifest) =
            consolidate(std::slice::from_ref(&skill), library.path(), false, false).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.unchanged, 1);
    }

    #[test]
    fn consolidate_force_recopies() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        consolidate(std::slice::from_ref(&skill), library.path(), false, false).unwrap();
        let (result, _manifest) =
            consolidate(std::slice::from_ref(&skill), library.path(), false, true).unwrap();
        assert_eq!(result.updated, 1, "force should recopy unchanged skill");
        assert_eq!(result.unchanged, 0);
    }

    #[test]
    fn consolidate_detects_content_change() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        consolidate(std::slice::from_ref(&skill), library.path(), false, false).unwrap();

        // Modify source content
        std::fs::write(source.path().join("my-skill/SKILL.md"), "# updated").unwrap();

        let (result, _manifest) =
            consolidate(std::slice::from_ref(&skill), library.path(), false, false).unwrap();
        assert_eq!(result.updated, 1);

        // Library copy should have the new content
        let content = std::fs::read_to_string(library.path().join("my-skill/SKILL.md")).unwrap();
        assert_eq!(content, "# updated");
    }

    #[test]
    fn consolidate_dry_run_no_changes() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let (result, _manifest) = consolidate(&[skill], library.path(), true, false).unwrap();
        assert_eq!(result.created, 1);

        // Directory should NOT exist
        assert!(!library.path().join("my-skill").exists());
    }

    #[test]
    fn consolidate_dry_run_doesnt_create_dir() {
        let tmp = TempDir::new().unwrap();
        let nonexistent_lib = tmp.path().join("does-not-exist");
        let source = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let (result, _manifest) = consolidate(&[skill], &nonexistent_lib, true, false).unwrap();
        assert_eq!(result.created, 1);
        assert!(!nonexistent_lib.exists());
    }

    #[test]
    fn consolidate_skips_unmanaged_collision() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        let skill = make_skill(source.path(), "my-skill");

        // Pre-create a directory at the library path (not in manifest)
        let collision = library.path().join("my-skill");
        std::fs::create_dir_all(&collision).unwrap();
        std::fs::write(collision.join("README.md"), "user-created").unwrap();

        let (result, _manifest) = consolidate(&[skill], library.path(), false, false).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.unchanged, 0);
        assert_eq!(result.skipped, 1);

        // User-created content should be untouched
        let content = std::fs::read_to_string(library.path().join("my-skill/README.md")).unwrap();
        assert_eq!(content, "user-created");
    }

    #[test]
    fn consolidate_migrates_v01_symlink() {
        use std::os::unix::fs as unix_fs;

        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        // Create the skill in the source
        let skill = make_skill(source.path(), "my-skill");

        // Simulate a v0.1.x library: symlink instead of directory
        unix_fs::symlink(&skill.path, library.path().join("my-skill")).unwrap();
        assert!(library.path().join("my-skill").is_symlink());

        let (result, _manifest) = consolidate(&[skill], library.path(), false, false).unwrap();
        assert_eq!(result.updated, 1, "symlink should be migrated");

        // Should now be a real directory, not a symlink
        let dest = library.path().join("my-skill");
        assert!(dest.is_dir());
        assert!(!dest.is_symlink());
        assert!(dest.join("SKILL.md").is_file());

        // Manifest should have the entry
        let manifest = manifest::load(library.path()).unwrap();
        assert!(manifest.contains_key("my-skill"));
    }

    #[test]
    fn consolidate_updates_changed_source() {
        let source1 = TempDir::new().unwrap();
        let source2 = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        let skill1 = make_skill(source1.path(), "my-skill");
        consolidate(&[skill1], library.path(), false, false).unwrap();

        // New skill from a different source with different content
        let skill2_dir = source2.path().join("my-skill");
        std::fs::create_dir_all(&skill2_dir).unwrap();
        std::fs::write(skill2_dir.join("SKILL.md"), "# different content").unwrap();
        let skill2 = DiscoveredSkill {
            name: crate::discover::SkillName::new("my-skill").unwrap(),
            path: skill2_dir,
            source_name: "test2".into(),
        };

        let (result, _manifest) =
            consolidate(std::slice::from_ref(&skill2), library.path(), false, false).unwrap();
        assert_eq!(result.updated, 1);

        let content = std::fs::read_to_string(library.path().join("my-skill/SKILL.md")).unwrap();
        assert_eq!(content, "# different content");
    }

    #[test]
    fn consolidate_manifest_persisted() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let (_, manifest) = consolidate(&[skill], library.path(), false, false).unwrap();

        assert_eq!(manifest.len(), 1);
        assert!(manifest.contains_key("my-skill"));
        let entry = manifest.get("my-skill").unwrap();
        assert!(!entry.content_hash.is_empty());
        assert!(!entry.synced_at.is_empty());
    }
}

//! Consolidate discovered skills into the library directory.
//!
//! Two consolidation strategies based on source type:
//! - **Managed** (ClaudePlugins): symlink in library → source dir (package manager owns the files)
//! - **Local** (Directory): copy into library (library is the canonical home)
//!
//! Idempotent — unchanged skills are skipped. Handles strategy transitions when a skill's
//! source type changes between syncs.

use anyhow::{Context, Result};
use std::os::unix::fs as unix_fs;
use std::path::Path;

use crate::discover::DiscoveredSkill;
use crate::manifest::{self, Manifest, SkillEntry};
use crate::paths::symlink_points_to;

/// What already exists at the library destination path.
enum DestinationState {
    Symlink,
    Directory,
    Empty,
    Other,
}

fn classify_destination(dest: &Path) -> DestinationState {
    if dest.is_symlink() {
        DestinationState::Symlink
    } else if dest.is_dir() {
        DestinationState::Directory
    } else if !dest.exists() {
        DestinationState::Empty
    } else {
        DestinationState::Other
    }
}

/// Result of a consolidation operation.
#[derive(Debug, Default)]
pub struct ConsolidateResult {
    pub created: usize,
    pub unchanged: usize,
    pub updated: usize,
    /// Skills skipped because a non-managed entry already exists at the library path.
    pub skipped: usize,
}

/// Create a symlink from `dest` pointing to `src`.
fn create_symlink(src: &Path, dest: &Path) -> Result<()> {
    unix_fs::symlink(src, dest)
        .with_context(|| format!("failed to symlink {} -> {}", dest.display(), src.display()))
}

/// Record a skill in the manifest after consolidation.
fn record_in_manifest(manifest: &mut Manifest, skill: &DiscoveredSkill, content_hash: String) {
    manifest.insert(
        skill.name.clone(),
        SkillEntry::new(
            skill.path.clone(),
            skill.source_name.clone(),
            content_hash,
            skill.managed,
        ),
    );
}

/// Consolidate discovered skills into the library directory.
///
/// Managed skills are symlinked; local skills are copied.
/// A manifest tracks content hashes and provenance for idempotent updates.
/// When `force` is true, all skills are re-synced regardless of state.
///
/// `tome_home` is the top-level `~/.tome/` directory where metadata files (manifest,
/// lockfile, config) are stored. `library_dir` is the subdirectory (typically
/// `~/.tome/skills/`) where skill contents actually live.
///
/// Returns both the operation result and the (possibly updated) manifest so the
/// caller can pass it directly to distribute/cleanup without a redundant disk read.
/// In dry-run mode the manifest is never written to disk, so returning it here is
/// the only way downstream steps see the would-be-updated state.
pub fn consolidate(
    skills: &[DiscoveredSkill],
    library_dir: &Path,
    tome_home: &Path,
    dry_run: bool,
    force: bool,
) -> Result<(ConsolidateResult, Manifest)> {
    if !dry_run {
        std::fs::create_dir_all(library_dir)
            .with_context(|| format!("failed to create library dir {}", library_dir.display()))?;
    }

    let mut manifest = if tome_home.is_dir() {
        manifest::load(tome_home)?
    } else {
        Manifest::default()
    };

    let mut result = ConsolidateResult::default();

    for skill in skills {
        let dest = library_dir.join(skill.name.as_str());

        if skill.managed {
            consolidate_managed(skill, &dest, &mut manifest, &mut result, dry_run, force)?;
        } else {
            consolidate_local(
                skill,
                &dest,
                library_dir,
                &mut manifest,
                &mut result,
                dry_run,
                force,
            )?;
        }
    }

    if !dry_run && tome_home.is_dir() {
        manifest::save(&manifest, tome_home)?;
    }

    Ok((result, manifest))
}

/// Consolidate a managed skill: create a symlink in the library pointing to the source.
fn consolidate_managed(
    skill: &DiscoveredSkill,
    dest: &Path,
    manifest: &mut Manifest,
    result: &mut ConsolidateResult,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    let content_hash = manifest::hash_directory(&skill.path)?;

    match classify_destination(dest) {
        DestinationState::Directory => {
            // A managed skill should always be a symlink. If a real directory exists
            // instead (e.g., from a prior local-to-managed transition or manual
            // intervention), replace it with a symlink.
            if manifest.contains_key(skill.name.as_str()) {
                if !dry_run {
                    std::fs::remove_dir_all(dest).with_context(|| {
                        format!(
                            "failed to remove stale dir for managed skill {}",
                            dest.display()
                        )
                    })?;
                    create_symlink(&skill.path, dest)?;
                }
                record_in_manifest(manifest, skill, content_hash.clone());
                result.updated += 1;
            } else {
                // Dir exists but not in manifest — skip with warning
                eprintln!(
                    "warning: {} exists but is not in the manifest, skipping",
                    dest.display()
                );
                result.skipped += 1;
            }
        }
        DestinationState::Symlink => {
            if symlink_points_to(dest, &skill.path) && !force {
                // Check if managed flag needs updating in manifest (v0.1 migration)
                if let Some(entry) = manifest.get(skill.name.as_str())
                    && !entry.managed
                {
                    record_in_manifest(manifest, skill, content_hash.clone());
                }
                result.unchanged += 1;
            } else {
                // Wrong target or force — remove and recreate
                if !dry_run {
                    std::fs::remove_file(dest)
                        .with_context(|| format!("failed to remove symlink {}", dest.display()))?;
                    create_symlink(&skill.path, dest)?;
                }
                record_in_manifest(manifest, skill, content_hash.clone());
                result.updated += 1;
            }
        }
        DestinationState::Empty => {
            if !dry_run {
                create_symlink(&skill.path, dest)?;
            }
            record_in_manifest(manifest, skill, content_hash.clone());
            result.created += 1;
        }
        DestinationState::Other => {
            eprintln!(
                "warning: {} exists but is not in the manifest, skipping",
                dest.display()
            );
            result.skipped += 1;
        }
    }

    Ok(())
}

/// Consolidate a local skill: copy the directory into the library.
fn consolidate_local(
    skill: &DiscoveredSkill,
    dest: &Path,
    library_dir: &Path,
    manifest: &mut Manifest,
    result: &mut ConsolidateResult,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    let content_hash = manifest::hash_directory(&skill.path)?;

    match classify_destination(dest) {
        DestinationState::Symlink => {
            // Strategy transition: managed (symlink) → local (dir)
            if let Some(entry) = manifest.get(skill.name.as_str())
                && entry.managed
            {
                // Was managed, now local — remove symlink, copy
                if !dry_run {
                    std::fs::remove_file(dest).with_context(|| {
                        format!("failed to remove managed symlink {}", dest.display())
                    })?;
                    copy_dir_recursive(&skill.path, dest)?;
                }
                record_in_manifest(manifest, skill, content_hash.clone());
                result.updated += 1;
                return Ok(());
            }
            // Legacy v0.1.x symlink — migrate to local copy
            if !dry_run {
                let resolved = std::fs::read_link(dest)
                    .with_context(|| format!("failed to read symlink {}", dest.display()))?;
                let abs_resolved = if resolved.is_relative() {
                    library_dir.join(&resolved)
                } else {
                    resolved
                };
                std::fs::remove_file(dest)
                    .with_context(|| format!("failed to remove v0.1 symlink {}", dest.display()))?;
                if abs_resolved.is_dir() {
                    copy_dir_recursive(&abs_resolved, dest)?;
                } else {
                    eprintln!(
                        "warning: v0.1 symlink target for '{}' is gone, copying from current source",
                        skill.name
                    );
                    copy_dir_recursive(&skill.path, dest)?;
                }
            }
            record_in_manifest(manifest, skill, content_hash.clone());
            result.updated += 1;
        }
        DestinationState::Directory | DestinationState::Empty | DestinationState::Other => {
            if let Some(entry) = manifest.get(skill.name.as_str()) {
                if entry.content_hash == content_hash && !force {
                    result.unchanged += 1;
                    return Ok(());
                }
                // Content changed or force — re-copy
                if !dry_run {
                    if dest.is_dir() {
                        std::fs::remove_dir_all(dest).with_context(|| {
                            format!("failed to remove old skill dir {}", dest.display())
                        })?;
                    }
                    copy_dir_recursive(&skill.path, dest)?;
                }
                record_in_manifest(manifest, skill, content_hash.clone());
                result.updated += 1;
            } else if dest.exists() {
                // Something exists that's NOT in the manifest — skip with warning
                eprintln!(
                    "warning: {} exists but is not in the manifest, skipping",
                    dest.display()
                );
                result.skipped += 1;
            } else {
                // New skill — copy
                if !dry_run {
                    copy_dir_recursive(&skill.path, dest)?;
                }
                record_in_manifest(manifest, skill, content_hash.clone());
                result.created += 1;
            }
        }
    }

    Ok(())
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

/// Generate or update `.gitignore` in the library directory.
///
/// Managed (symlinked) skill entries are gitignored — they are recreated by `tome sync`.
/// Local (copied) skill entries and `.tome-manifest.json` are tracked.
/// Only writes the file if the content would change, to avoid unnecessary git noise.
pub fn generate_gitignore(library_dir: &Path, manifest: &Manifest) -> Result<()> {
    let mut managed: Vec<&str> = manifest
        .iter()
        .filter(|(_, entry)| entry.managed)
        .map(|(name, _)| name.as_str())
        .collect();
    managed.sort();

    let mut content = String::from("# Auto-generated by tome — do not edit\n");

    if !managed.is_empty() {
        content.push_str("# Managed skills (recreated by `tome sync`)\n");
        for name in &managed {
            content.push_str(name);
            content.push_str("/\n");
        }
    }

    // No internal entries — manifest and lockfile now live at tome home, not in library

    let gitignore_path = library_dir.join(".gitignore");

    // Only write if content would change
    if gitignore_path.exists() {
        let existing = std::fs::read_to_string(&gitignore_path)
            .with_context(|| format!("failed to read {}", gitignore_path.display()))?;
        if existing == content {
            return Ok(());
        }
    }

    std::fs::write(&gitignore_path, &content)
        .with_context(|| format!("failed to write {}", gitignore_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_skill(dir: &Path, name: &str) -> DiscoveredSkill {
        make_skill_with_managed(dir, name, false)
    }

    fn make_managed_skill(dir: &Path, name: &str) -> DiscoveredSkill {
        make_skill_with_managed(dir, name, true)
    }

    fn make_skill_with_managed(dir: &Path, name: &str, managed: bool) -> DiscoveredSkill {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# test").unwrap();
        DiscoveredSkill {
            name: crate::discover::SkillName::new(name).unwrap(),
            path: skill_dir,
            source_name: "test".into(),
            managed,
            provenance: None,
        }
    }

    #[test]
    fn consolidate_copies_skills() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let (result, _manifest) =
            consolidate(&[skill], library.path(), library.path(), false, false).unwrap();
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

        consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        let (result, _manifest) = consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.unchanged, 1);
    }

    #[test]
    fn consolidate_force_recopies() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        let (result, _manifest) = consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            true,
        )
        .unwrap();
        assert_eq!(result.updated, 1, "force should recopy unchanged skill");
        assert_eq!(result.unchanged, 0);
    }

    #[test]
    fn consolidate_detects_content_change() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();

        // Modify source content
        std::fs::write(source.path().join("my-skill/SKILL.md"), "# updated").unwrap();

        let (result, _manifest) = consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
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

        let (result, _manifest) =
            consolidate(&[skill], library.path(), library.path(), true, false).unwrap();
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

        let (result, _manifest) =
            consolidate(&[skill], &nonexistent_lib, &nonexistent_lib, true, false).unwrap();
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

        let (result, _manifest) =
            consolidate(&[skill], library.path(), library.path(), false, false).unwrap();
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

        let (result, _manifest) =
            consolidate(&[skill], library.path(), library.path(), false, false).unwrap();
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
        consolidate(&[skill1], library.path(), library.path(), false, false).unwrap();

        // New skill from a different source with different content
        let skill2_dir = source2.path().join("my-skill");
        std::fs::create_dir_all(&skill2_dir).unwrap();
        std::fs::write(skill2_dir.join("SKILL.md"), "# different content").unwrap();
        let skill2 = DiscoveredSkill {
            name: crate::discover::SkillName::new("my-skill").unwrap(),
            path: skill2_dir,
            source_name: "test2".into(),
            managed: false,
            provenance: None,
        };

        let (result, _manifest) = consolidate(
            std::slice::from_ref(&skill2),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.updated, 1);

        let content = std::fs::read_to_string(library.path().join("my-skill/SKILL.md")).unwrap();
        assert_eq!(content, "# different content");
    }

    #[test]
    fn consolidate_manifest_persisted() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let (_, manifest) =
            consolidate(&[skill], library.path(), library.path(), false, false).unwrap();

        assert_eq!(manifest.len(), 1);
        assert!(manifest.contains_key("my-skill"));
        let entry = manifest.get("my-skill").unwrap();
        assert!(!entry.content_hash.is_empty());
        assert!(!entry.synced_at.is_empty());
    }

    #[test]
    fn consolidate_migrates_v01_symlink_with_broken_target() {
        use std::os::unix::fs as unix_fs;
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        // Create a symlink pointing to a nonexistent target (simulating gone original)
        unix_fs::symlink(
            "/nonexistent/original/path",
            library.path().join("my-skill"),
        )
        .unwrap();

        let (result, _manifest) =
            consolidate(&[skill], library.path(), library.path(), false, false).unwrap();
        assert_eq!(result.updated, 1);

        let dest = library.path().join("my-skill");
        assert!(dest.is_dir());
        assert!(!dest.is_symlink());
        assert!(dest.join("SKILL.md").is_file());
    }

    #[test]
    fn consolidate_copies_nested_subdirectories() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        let skill_dir = source.path().join("deep-skill");
        std::fs::create_dir_all(skill_dir.join("sub/nested")).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# test").unwrap();
        std::fs::write(skill_dir.join("sub/file.txt"), "content").unwrap();
        std::fs::write(skill_dir.join("sub/nested/deep.txt"), "deep").unwrap();

        let skill = DiscoveredSkill {
            name: crate::discover::SkillName::new("deep-skill").unwrap(),
            path: skill_dir,
            source_name: "test".into(),
            managed: false,
            provenance: None,
        };

        let (result, _) =
            consolidate(&[skill], library.path(), library.path(), false, false).unwrap();
        assert_eq!(result.created, 1);
        assert!(
            library
                .path()
                .join("deep-skill/sub/nested/deep.txt")
                .is_file()
        );
        let content =
            std::fs::read_to_string(library.path().join("deep-skill/sub/nested/deep.txt")).unwrap();
        assert_eq!(content, "deep");
    }

    #[test]
    fn consolidate_dry_run_no_manifest_written() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        // Create the library dir so it exists
        std::fs::create_dir_all(library.path()).unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let (result, _) =
            consolidate(&[skill], library.path(), library.path(), true, false).unwrap();
        assert_eq!(result.created, 1);
        assert!(
            !library.path().join(".tome-manifest.json").exists(),
            "dry-run should not write manifest"
        );
    }

    #[test]
    fn consolidate_dry_run_manifest_reflects_would_be_state() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        std::fs::create_dir_all(library.path()).unwrap();

        // First: consolidate as local (creates real copy + manifest entry)
        let local_skill = make_skill(source.path(), "my-skill");
        consolidate(&[local_skill], library.path(), library.path(), false, false).unwrap();

        // Now: dry-run consolidate the same skill as managed
        let managed_skill = make_managed_skill(source.path(), "my-skill");
        let (result, manifest) = consolidate(
            &[managed_skill],
            library.path(),
            library.path(),
            true,
            false,
        )
        .unwrap();
        assert_eq!(result.updated, 1);

        // In-memory manifest should reflect managed=true even though no disk changes
        let entry = manifest.get("my-skill").expect("should have entry");
        assert!(
            entry.managed,
            "dry-run manifest should reflect the would-be-updated managed flag"
        );

        // But disk should be unchanged (still a real dir, not a symlink)
        let dest = library.path().join("my-skill");
        assert!(dest.is_dir());
        assert!(!dest.is_symlink(), "dry-run should not change disk state");
    }

    #[test]
    fn consolidate_migrates_v01_symlink_records_discovered_source() {
        use std::os::unix::fs as unix_fs;
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        unix_fs::symlink(&skill.path, library.path().join("my-skill")).unwrap();

        let (_, manifest) = consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        let entry = manifest
            .get("my-skill")
            .expect("manifest should have entry");
        assert_eq!(
            entry.source_path, skill.path,
            "manifest source_path should point to discovered source"
        );
    }

    // -- Managed skill tests --

    #[test]
    fn consolidate_symlinks_managed_skill() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_managed_skill(source.path(), "plugin-skill");

        let (result, manifest) =
            consolidate(&[skill], library.path(), library.path(), false, false).unwrap();
        assert_eq!(result.created, 1);

        let dest = library.path().join("plugin-skill");
        assert!(dest.is_symlink(), "managed skill should be a symlink");
        assert!(dest.is_dir(), "symlink should point to a valid directory");

        let entry = manifest.get("plugin-skill").unwrap();
        assert!(entry.managed);
    }

    #[test]
    fn consolidate_managed_idempotent() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_managed_skill(source.path(), "plugin-skill");

        consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        let (result, _) = consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.unchanged, 1);
        assert_eq!(result.created, 0);
        assert_eq!(result.updated, 0);
    }

    #[test]
    fn consolidate_managed_path_changed() {
        let source1 = TempDir::new().unwrap();
        let source2 = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        let skill1 = make_managed_skill(source1.path(), "plugin-skill");
        consolidate(&[skill1], library.path(), library.path(), false, false).unwrap();

        // Same skill name from different path
        let skill2 = make_managed_skill(source2.path(), "plugin-skill");
        let (result, _) = consolidate(
            std::slice::from_ref(&skill2),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.updated, 1);

        // Should point to the new path
        let dest = library.path().join("plugin-skill");
        assert!(dest.is_symlink());
        let target = std::fs::read_link(&dest).unwrap();
        assert_eq!(target, skill2.path);
    }

    #[test]
    fn consolidate_strategy_transition_local_to_managed() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        // First: consolidate as local (copy)
        let local_skill = make_skill(source.path(), "my-skill");
        consolidate(&[local_skill], library.path(), library.path(), false, false).unwrap();
        let dest = library.path().join("my-skill");
        assert!(dest.is_dir());
        assert!(!dest.is_symlink(), "should be a real dir initially");

        // Now: same skill but managed
        let managed_skill = make_managed_skill(source.path(), "my-skill");
        let (result, manifest) = consolidate(
            &[managed_skill],
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.updated, 1);
        assert!(dest.is_symlink(), "should now be a symlink");
        assert!(manifest.get("my-skill").unwrap().managed);
    }

    #[test]
    fn consolidate_strategy_transition_managed_to_local() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        // First: consolidate as managed (symlink)
        let managed_skill = make_managed_skill(source.path(), "my-skill");
        consolidate(
            &[managed_skill],
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        let dest = library.path().join("my-skill");
        assert!(dest.is_symlink(), "should be a symlink initially");

        // Now: same skill but local
        let local_skill = make_skill(source.path(), "my-skill");
        let (result, manifest) =
            consolidate(&[local_skill], library.path(), library.path(), false, false).unwrap();
        assert_eq!(result.updated, 1);
        assert!(dest.is_dir());
        assert!(!dest.is_symlink(), "should now be a real directory");
        assert!(!manifest.get("my-skill").unwrap().managed);
    }

    #[test]
    fn consolidate_managed_manifest_records_managed_flag() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_managed_skill(source.path(), "plugin-skill");

        let (_, manifest) =
            consolidate(&[skill], library.path(), library.path(), false, false).unwrap();
        let entry = manifest.get("plugin-skill").unwrap();
        assert!(entry.managed);
        assert!(!entry.content_hash.is_empty());
    }

    // -- .gitignore generation tests --

    #[test]
    fn gitignore_lists_managed_skills() {
        let library = TempDir::new().unwrap();
        let source = TempDir::new().unwrap();

        let managed = make_managed_skill(source.path(), "plugin-a");
        let local = make_skill(source.path(), "user-skill");

        let (_, manifest) = consolidate(
            &[managed, local],
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        generate_gitignore(library.path(), &manifest).unwrap();

        let content = std::fs::read_to_string(library.path().join(".gitignore")).unwrap();
        assert!(
            content.contains("plugin-a/"),
            "managed skill should be gitignored"
        );
    }

    #[test]
    fn gitignore_does_not_list_local_skills() {
        let library = TempDir::new().unwrap();
        let source = TempDir::new().unwrap();

        let managed = make_managed_skill(source.path(), "plugin-a");
        let local = make_skill(source.path(), "user-skill");

        let (_, manifest) = consolidate(
            &[managed, local],
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        generate_gitignore(library.path(), &manifest).unwrap();

        let content = std::fs::read_to_string(library.path().join(".gitignore")).unwrap();
        assert!(
            !content.contains("user-skill"),
            "local skill should NOT be gitignored"
        );
    }

    #[test]
    fn gitignore_idempotent() {
        let library = TempDir::new().unwrap();
        let source = TempDir::new().unwrap();

        let managed = make_managed_skill(source.path(), "plugin-a");
        let (_, manifest) =
            consolidate(&[managed], library.path(), library.path(), false, false).unwrap();

        generate_gitignore(library.path(), &manifest).unwrap();
        let first = std::fs::read_to_string(library.path().join(".gitignore")).unwrap();

        generate_gitignore(library.path(), &manifest).unwrap();
        let second = std::fs::read_to_string(library.path().join(".gitignore")).unwrap();

        assert_eq!(
            first, second,
            "running twice should produce identical output"
        );
    }

    #[test]
    fn consolidate_managed_dry_run_no_symlink_created() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_managed_skill(source.path(), "plugin-skill");

        let (result, manifest) =
            consolidate(&[skill], library.path(), library.path(), true, false).unwrap();
        assert_eq!(result.created, 1);

        // Symlink should NOT exist on disk
        let dest = library.path().join("plugin-skill");
        assert!(!dest.exists(), "dry-run should not create symlink");
        assert!(!dest.is_symlink(), "dry-run should not create symlink");

        // But manifest should reflect the would-be state
        let entry = manifest.get("plugin-skill").expect("should have entry");
        assert!(entry.managed);
    }

    #[test]
    fn consolidate_managed_force_recreates_symlink() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_managed_skill(source.path(), "plugin-skill");

        consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        let (result, _) = consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            true,
        )
        .unwrap();
        assert_eq!(result.updated, 1, "force should recreate managed symlink");
        assert_eq!(result.unchanged, 0);

        let dest = library.path().join("plugin-skill");
        assert!(dest.is_symlink(), "should still be a symlink after force");
    }

    #[test]
    fn consolidate_managed_repairs_stale_directory() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_managed_skill(source.path(), "plugin-skill");

        // First: consolidate normally (creates symlink)
        consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        let dest = library.path().join("plugin-skill");
        assert!(dest.is_symlink(), "should be a symlink initially");

        // Replace symlink with a real directory (simulating stale state)
        std::fs::remove_file(&dest).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("SKILL.md"), "# stale").unwrap();
        assert!(
            dest.is_dir() && !dest.is_symlink(),
            "should be a real dir now"
        );

        // Re-consolidate — should repair by replacing dir with symlink
        let (result, _) = consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.updated, 1, "should repair stale directory");
        assert_eq!(result.unchanged, 0);
        assert!(dest.is_symlink(), "should be a symlink again after repair");
    }

    #[test]
    fn consolidate_managed_skips_non_manifest_dir_collision() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        let skill = make_managed_skill(source.path(), "plugin-skill");

        // Pre-create a real directory at the library path (not in manifest)
        let collision = library.path().join("plugin-skill");
        std::fs::create_dir_all(&collision).unwrap();
        std::fs::write(collision.join("README.md"), "user-created").unwrap();

        let (result, _) =
            consolidate(&[skill], library.path(), library.path(), false, false).unwrap();
        assert_eq!(result.skipped, 1);
        assert_eq!(result.created, 0);

        // User-created content should be untouched
        let content =
            std::fs::read_to_string(library.path().join("plugin-skill/README.md")).unwrap();
        assert_eq!(content, "user-created");
    }

    #[test]
    fn consolidate_local_manifest_reflects_update() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let (_, manifest1) = consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        let hash1 = manifest1.get("my-skill").unwrap().content_hash.clone();

        // Modify source content
        std::fs::write(source.path().join("my-skill/SKILL.md"), "# updated").unwrap();

        let (result, manifest2) = consolidate(
            std::slice::from_ref(&skill),
            library.path(),
            library.path(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.updated, 1);

        let entry = manifest2.get("my-skill").expect("should have entry");
        assert_ne!(entry.content_hash, hash1, "hash should change after update");
        assert_eq!(
            entry.source_path, skill.path,
            "source_path should be preserved"
        );
        assert!(!entry.managed, "local skill should not be managed");
    }

    #[test]
    fn gitignore_empty_manifest_no_tmp_entries() {
        let library = TempDir::new().unwrap();
        std::fs::create_dir_all(library.path()).unwrap();

        // Empty manifest — no managed skills
        let manifest = crate::manifest::Manifest::default();
        generate_gitignore(library.path(), &manifest).unwrap();

        let content = std::fs::read_to_string(library.path().join(".gitignore")).unwrap();
        assert!(
            !content.contains(".tome-manifest.tmp"),
            "manifest tmp files now live at tome home, not in library"
        );
        assert!(
            !content.contains("tome.lock.tmp"),
            "lockfile tmp files now live at tome home, not in library"
        );
    }
}

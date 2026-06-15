//! Distribute library skills to configured directories via symlinks.

use anyhow::{Context, Result};
use std::os::unix::fs as unix_fs;
use std::path::Path;
use tracing::{info, warn};

use crate::change_cause::ChangeCause;
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

    // Read all entries in library — all are real directory copies since
    // v0.10 (LIB-01). A symlink here indicates an un-migrated v0.9-shape
    // library that should have been refused at the sync gate; see
    // `library::consolidate_managed` for the refusal logic.
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
            && manifest_entry
                .source_name()
                .is_some_and(|s| s == dir_name.as_str())
        {
            // Remove any existing symlink from a previous sync that
            // didn't have this check (cleans up legacy duplicates).
            if !dry_run
                && target_link.is_symlink()
                && let Err(e) = std::fs::remove_file(&target_link)
            {
                warn!(
                    "failed to remove legacy symlink {}: {}",
                    target_link.display(),
                    e
                );
            }
            result.skipped_managed += 1;
            continue;
        }

        // OBS-04 state snapshot — sample BEFORE any remove/create happens so
        // the cause classification is faithful to the world at iteration start.
        let was_symlink = target_link.is_symlink();
        let in_manifest = manifest.get(skill_name_str.as_ref()).is_some();

        if target_link.is_symlink() {
            if symlink_points_to(&target_link, &library_skill_path) && !force {
                result.unchanged += 1;
                continue;
            }
            // HARD-09 / D-DIST-1: foreign-symlink protection. If the
            // existing symlink points OUTSIDE the current `library_dir`,
            // it was almost certainly placed there by a different tome
            // install (or a hand-edited dotfiles workflow). Refuse to
            // clobber it unless `force` is set; the existing `force`
            // semantic ("recreate stale links") is extended to also
            // mean "yes, clobber foreign symlinks", consistent with the
            // existing flag's meaning. No new CLI surface.
            //
            // Detection uses canonicalize so symlinks-in-the-middle of
            // the path resolve correctly (a target like
            // /var/lib/x → /private/var/lib/x on macOS still resolves
            // under the real library_dir if one is a prefix of the
            // other).
            if !force && is_foreign_symlink(&target_link, library_dir) {
                let actual_target =
                    std::fs::read_link(&target_link).unwrap_or_else(|_| target_link.clone());
                warn!(
                    "{} is a foreign symlink (→ {}); skipping. Pass --force to overwrite, or remove manually.",
                    target_link.display(),
                    actual_target.display(),
                );
                result.skipped += 1;
                continue;
            }
            // Update stale link (or force-recreating)
            if !dry_run {
                std::fs::remove_file(&target_link).with_context(|| {
                    format!("failed to remove stale symlink {}", target_link.display())
                })?;
            }
        } else if target_link.exists() {
            warn!(
                "{} exists in target and is not a symlink, skipping",
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

        // OBS-04 emission. Classification per RESEARCH §Open Question 2:
        // - was_symlink: an existing symlink was replaced (stale link update) → HashChanged
        // - !was_symlink && in_manifest: skill already known to consolidate but no symlink
        //   in this directory; plausibly the directory was disabled previously and is now
        //   allowed (machine_prefs flip) → DirectoryNowAllowed inference
        // - !was_symlink && !in_manifest: cannot occur in practice because consolidate
        //   inserts the manifest entry BEFORE distribute runs; defensive NewlyAdded fallback
        let cause = if was_symlink {
            ChangeCause::HashChanged
        } else if in_manifest {
            ChangeCause::DirectoryNowAllowed
        } else {
            ChangeCause::NewlyAdded
        };
        info!(
            skill = %skill_name_str,
            directory = %dir_name,
            cause = %cause,
            "re-emitted",
        );
    }

    Ok(result)
}

/// HARD-09 / D-DIST-1: classify whether `link_path` is a symlink whose
/// target resolves OUTSIDE `library_dir`. Returns false when the link
/// is missing, can't be read, or points anywhere under (or equal to)
/// the library directory under either its raw or canonicalised spelling.
///
/// Canonicalization handles symlinks-in-the-middle of the prefix path
/// (e.g. /var → /private/var on macOS), so a link target physically
/// inside the library is not mis-classified as foreign just because
/// the user's library_dir was given as the symlinked spelling.
///
/// Stale-but-in-library links (target missing, but the lexical path is
/// rooted under library_dir) are NOT classified as foreign — the
/// existing in-library staleness path handles them.
///
/// Doctor (D-DIST-2) reuses this predicate to surface ForeignSymlink
/// diagnostics without depending on a sync run.
pub(crate) fn is_foreign_symlink(link_path: &Path, library_dir: &Path) -> bool {
    if !link_path.is_symlink() {
        return false;
    }

    // Read the raw symlink target. If the read fails we conservatively
    // say "not foreign" — the surrounding sync code will surface the
    // I/O failure through its own error path.
    let raw_target = match std::fs::read_link(link_path) {
        Ok(t) => t,
        Err(_) => return false,
    };

    // Build a lexical target. For absolute symlink targets that's just
    // the raw target; for relative targets it's resolved against the
    // link's parent.
    let lexical_target = if raw_target.is_absolute() {
        raw_target.clone()
    } else {
        link_path
            .parent()
            .map(|p| p.join(&raw_target))
            .unwrap_or_else(|| raw_target.clone())
    };

    // We accept FOUR potential prefixes for "in-library" before
    // classifying as foreign — any match means not-foreign:
    //   1. raw library_dir vs lexical target (both un-resolved)
    //   2. canonicalised library_dir vs lexical target
    //   3. raw library_dir vs canonicalised target (link followed)
    //   4. canonicalised library_dir vs canonicalised target
    //
    // The pair-matrix avoids false-foreign reports when one side
    // canonicalises through symlinks (e.g. /var → /private/var) and
    // the other doesn't, AND when the link target is missing
    // (canonicalize fails, lexical is the only signal).
    let canonical_library = std::fs::canonicalize(library_dir).ok();
    let canonical_target = std::fs::canonicalize(link_path).ok();

    let prefixes = [Some(library_dir.to_path_buf()), canonical_library];
    let candidates = [Some(lexical_target), canonical_target];

    for prefix in prefixes.iter().flatten() {
        for candidate in candidates.iter().flatten() {
            if candidate.starts_with(prefix) {
                return false;
            }
        }
    }
    true
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
            git_ref: None,

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

        // Simulate the target link now pointing somewhere else WITHIN the
        // current library (stale-but-not-foreign). HARD-09 / D-DIST-1 only
        // protects symlinks that point OUTSIDE library_dir; intra-library
        // staleness keeps the original "auto-recreate" behaviour.
        //
        // The stale target deliberately does NOT exist on disk — we want
        // to test the "wrong but in-library" branch without inflating the
        // library entry count distribute_to_directory walks.
        let stale_path = target_dir.path().join("skill-a");
        std::fs::remove_file(&stale_path).unwrap();
        let stale_target = library.path().join("skill-stale-target-missing");
        unix_fs::symlink(&stale_target, &stale_path).unwrap();

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
                ownership: crate::manifest::SkillOwnership::Owned {
                    source: DirectoryName::new("my-dir").unwrap(),
                },
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
                ownership: crate::manifest::SkillOwnership::Owned {
                    source: DirectoryName::new("alpha").unwrap(),
                },
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
                ownership: crate::manifest::SkillOwnership::Owned {
                    source: DirectoryName::new("my-dir").unwrap(),
                },
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

    // ---------------------------------------------------------------------
    // HARD-09 / D-DIST-1: foreign-symlink protection.
    // ---------------------------------------------------------------------

    #[test]
    fn is_foreign_symlink_returns_false_for_non_symlink() {
        let tmp = TempDir::new().unwrap();
        let regular = tmp.path().join("real.txt");
        std::fs::write(&regular, "x").unwrap();
        assert!(!is_foreign_symlink(&regular, tmp.path()));
    }

    #[test]
    fn is_foreign_symlink_returns_false_when_pointing_into_library() {
        let tmp = TempDir::new().unwrap();
        let library = tmp.path().join("library");
        std::fs::create_dir_all(&library).unwrap();
        let real_skill = library.join("foo");
        std::fs::create_dir_all(&real_skill).unwrap();

        let link = tmp.path().join("link");
        unix_fs::symlink(&real_skill, &link).unwrap();

        assert!(!is_foreign_symlink(&link, &library));
    }

    #[test]
    fn is_foreign_symlink_returns_true_when_pointing_outside_library() {
        let tmp = TempDir::new().unwrap();
        let library = tmp.path().join("library");
        std::fs::create_dir_all(&library).unwrap();
        let elsewhere = tmp.path().join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let foreign_target = elsewhere.join("foo");
        std::fs::create_dir_all(&foreign_target).unwrap();

        let link = tmp.path().join("link");
        unix_fs::symlink(&foreign_target, &link).unwrap();

        assert!(is_foreign_symlink(&link, &library));
    }

    /// D-DIST-1 default behaviour: pre-existing foreign symlink at the
    /// destination is warn-and-skipped; result.skipped is incremented;
    /// the foreign symlink stays on disk untouched.
    #[test]
    fn distribute_warns_and_skips_foreign_symlink() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        let other_library = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);
        // Stage a foreign symlink at target/skill-a pointing INTO a
        // different library entirely.
        let other_skill = other_library.path().join("skill-a");
        std::fs::create_dir_all(&other_skill).unwrap();
        unix_fs::symlink(&other_skill, target_dir.path().join("skill-a")).unwrap();

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &empty_manifest(),
            &MachinePrefs::default(),
            false, // dry_run
            false, // force
        )
        .unwrap();

        assert_eq!(
            result.skipped, 1,
            "foreign symlink must be counted as skipped"
        );
        assert_eq!(
            result.changed, 0,
            "no symlink should be created/updated when foreign skip fires"
        );

        // Foreign symlink unchanged on disk.
        let actual = std::fs::read_link(target_dir.path().join("skill-a")).unwrap();
        assert_eq!(actual, other_skill);
    }

    /// D-DIST-1 force opt-out: with force=true the foreign symlink IS
    /// clobbered (consistent with the existing `force` semantic of
    /// "recreate stale links").
    #[test]
    fn distribute_force_clobbers_foreign_symlink() {
        let library = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        let other_library = TempDir::new().unwrap();
        setup_library(library.path(), &["skill-a"]);
        let other_skill = other_library.path().join("skill-a");
        std::fs::create_dir_all(&other_skill).unwrap();
        unix_fs::symlink(&other_skill, target_dir.path().join("skill-a")).unwrap();

        let dir_name = DirectoryName::new("test").unwrap();
        let dir_config = make_dir_config(target_dir.path().to_path_buf());

        let result = distribute_to_directory(
            library.path(),
            &dir_name,
            &dir_config,
            &empty_manifest(),
            &MachinePrefs::default(),
            false, // dry_run
            true,  // force
        )
        .unwrap();

        assert_eq!(result.skipped, 0, "force must bypass the foreign skip");
        assert_eq!(result.changed, 1, "force must recreate the symlink");

        // Symlink now points into the current library, not the foreign one.
        let actual = std::fs::read_link(target_dir.path().join("skill-a")).unwrap();
        assert_eq!(actual, library.path().join("skill-a"));
    }
}

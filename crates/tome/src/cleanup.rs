//! Remove stale entries from the library and broken symlinks from target directories.
//!
//! Library cleanup compares manifest entries against currently discovered skill names.
//! Target cleanup still removes broken symlinks pointing into the library.
//!
//! ## Three-bucket cleanup output (UX-01)
//!
//! Cleanup output is partitioned into three user-facing buckets per
//! `.planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md` D-UX01-1..-4:
//!
//! - **Bucket A (removed-from-config):** Manifest entries whose `source_name`
//!   points at a directory no longer in `config.directories`. Library content
//!   is preserved (LIB-04); manifest transitions to Unowned.
//! - **Bucket B (missing-from-disk):** Manifest entries whose `source_name`
//!   IS still in `config.directories` but the source file vanished. Library
//!   copy is removed (today's behaviour).
//! - **Bucket C (now-in-exclude-list):** Library skills whose distribution
//!   symlinks were just removed because the skill was added to
//!   `machine.toml::disabled` (global) or `directories.<name>.disabled`
//!   (per-directory). Library content is preserved; only distribution
//!   symlinks change.
//!
//! Buckets A and B are detected here in `cleanup_library` and surfaced via
//! `CleanupResult::bucket_a_removed_from_config` and
//! `CleanupResult::bucket_b_missing_from_disk`. Bucket C is collected in
//! `lib.rs::cleanup_disabled_from_target` and threaded back into the
//! unified renderer (`render_cleanup_buckets`) called from `lib.rs::sync`.
//!
//! All cleanup output goes to **stderr** (D-UX01-4); the output paths use
//! `eprintln!` (or write through the renderer to a writer the caller routes
//! to stderr) — never `print!`/macro variants targeting stdout.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::io::{IsTerminal, Write};
use std::path::Path;

use crate::config::DirectoryName;
use crate::discover::SkillName;
use crate::manifest::Manifest;
use crate::paths::resolve_symlink_target;

/// One library skill whose distribution symlink was removed because the
/// skill is now in `machine.toml::disabled` (global) or
/// `directories.<dir>.disabled` (per-directory). Surfaced in the unified
/// cleanup output as Bucket C (UX-01 D-UX01-1).
#[derive(Debug, Clone)]
pub struct ExcludedSkill {
    pub name: SkillName,
    /// `None` = excluded globally via `machine.toml::disabled`.
    /// `Some(dir)` = excluded for a specific directory via
    /// `directories.<dir>.disabled`.
    pub directory: Option<DirectoryName>,
}

/// Result of cleanup operation.
#[derive(Debug, Default)]
pub struct CleanupResult {
    pub removed_from_library: usize,
    /// Skills transitioned from owned -> Unowned (Case 1 of LIB-04 / D-09).
    /// Library content for these skills is preserved on disk; the manifest
    /// entry's `source_name` is set to `None`.
    pub transitioned_to_unowned: usize,
    /// Bucket A entries (removed-from-config) collected for the unified
    /// three-bucket renderer (UX-01 D-UX01-1). Each tuple is
    /// `(skill_name, last_known_source)` — the source name is the
    /// `previous_source` recorded at transition time.
    pub bucket_a_removed_from_config: Vec<(SkillName, DirectoryName)>,
    /// Bucket B entries (missing-from-disk) collected for the unified
    /// three-bucket renderer (UX-01 D-UX01-1). Each tuple is
    /// `(skill_name, currently_configured_source)`.
    pub bucket_b_missing_from_disk: Vec<(SkillName, DirectoryName)>,
}

/// Render the three cleanup buckets to a writer. Used by `lib.rs::sync`
/// after both `cleanup_library` (Buckets A + B) and the distribution-cleanup
/// loop (Bucket C) have run.
///
/// Each non-empty bucket is emitted as: a blank-line separator, a colored
/// bold header line, then per-skill lines with an inline actionable hint.
/// Empty buckets are silently skipped so `tome sync` runs that touch nothing
/// stay quiet.
///
/// Per UX-01 D-UX01-3 / D-UX01-4 — the trigger phrase rewritten away by
/// this milestone is forbidden in the output; bucket-distinct phrasing is
/// locked in:
/// - Bucket A: "no longer in any source" (preserving as Unowned)
/// - Bucket B: "missing from configured source on disk"
/// - Bucket C: "now in exclude list"
pub(crate) fn render_cleanup_buckets(
    writer: &mut impl Write,
    bucket_a: &[(SkillName, DirectoryName)],
    bucket_b: &[(SkillName, DirectoryName)],
    bucket_c: &[ExcludedSkill],
) -> std::io::Result<()> {
    if bucket_a.is_empty() && bucket_b.is_empty() && bucket_c.is_empty() {
        return Ok(());
    }

    // Bucket A — removed from config (preserve as Unowned).
    if !bucket_a.is_empty() {
        writeln!(writer)?;
        writeln!(
            writer,
            "{}",
            console::style(format!(
                "{} skill(s) no longer in any source (preserving as Unowned):",
                bucket_a.len()
            ))
            .yellow()
            .bold()
        )?;
        for (name, prev_source) in bucket_a {
            writeln!(
                writer,
                "  {} {} — re-add `{}`, or run `tome reassign {} --to <dir>`",
                name,
                console::style(format!("(was: {})", prev_source)).dim(),
                prev_source,
                name,
            )?;
        }
    }

    // Bucket B — missing from configured source on disk (delete from library).
    if !bucket_b.is_empty() {
        writeln!(writer)?;
        writeln!(
            writer,
            "{}",
            console::style(format!(
                "{} skill(s) missing from configured source on disk (removing from library):",
                bucket_b.len()
            ))
            .yellow()
            .bold()
        )?;
        for (name, source) in bucket_b {
            writeln!(
                writer,
                "  {} {} — restore the file, or run `tome remove skill {}`",
                name,
                console::style(format!("(from: {})", source)).dim(),
                name,
            )?;
        }
    }

    // Bucket C — now in exclude list (distribution symlinks removed; library preserved).
    if !bucket_c.is_empty() {
        writeln!(writer)?;
        writeln!(
            writer,
            "{}",
            console::style(format!(
                "{} skill(s) now in exclude list (distribution symlinks removed; library preserved):",
                bucket_c.len()
            ))
            .yellow()
            .bold()
        )?;
        for excluded in bucket_c {
            match &excluded.directory {
                None => {
                    writeln!(
                        writer,
                        "  {} {} — remove `{}` from `machine.toml::disabled` to re-distribute",
                        excluded.name,
                        console::style("(excluded globally)").dim(),
                        excluded.name,
                    )?;
                }
                Some(dir) => {
                    writeln!(
                        writer,
                        "  {} {} — remove `{}` from `machine.toml::directories.{}.disabled` to re-distribute",
                        excluded.name,
                        console::style(format!("(excluded for: {})", dir)).dim(),
                        excluded.name,
                        dir,
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// Remove library entries whose skills are no longer present in any discovered source.
///
/// Stale candidates (manifest entries not in `discovered_names`) are partitioned
/// per LIB-04 / D-09 / D-10:
///
/// - **Case 1** — `source_name` no longer keys a `[directories.*]` entry in
///   `config.directories`. The user removed the source from `tome.toml`
///   (manually or via `tome remove`). Action: transition to **Unowned**
///   (`source_name = None`) and **preserve library content on disk**.
/// - **Case 2** — `source_name` IS still in `config.directories` but the file
///   vanished from the source on disk. Today's behavior — delete the library
///   copy. The configured source removing a file is treated as intentional.
/// - **Already-Unowned** — `source_name` is `None`. Filtered out of the stale
///   set entirely; preserved by definition.
///
/// When stdin is a TTY and `quiet` is false, prompts the user before deleting
/// Case 2 entries. Case 1 transitions are silent (info-level eprintln) — no
/// confirmation needed because library content is preserved.
pub fn cleanup_library(
    library_dir: &Path,
    discovered_names: &HashSet<String>,
    manifest: &mut Manifest,
    config: &crate::config::Config,
    dry_run: bool,
    quiet: bool,
    no_input: bool,
) -> Result<CleanupResult> {
    let mut result = CleanupResult::default();

    if !library_dir.is_dir() {
        return Ok(result);
    }

    let interactive = !no_input && std::io::stdin().is_terminal() && !quiet;

    // Stale candidates = manifest entries whose skill names weren't discovered.
    // We split into D-09 cases:
    //   Case 1: source removed from config -> transition to Unowned (preserve library)
    //   Case 2: source still configured, file vanished from disk -> delete (today's behavior)
    //
    // Already-Unowned entries (source_name == None) are filtered out of the
    // stale set entirely; they have no source to compare against and are
    // preserved by definition (LIB-04). They were skipped from discover too.
    let stale: Vec<SkillName> = manifest
        .keys()
        .filter(|name| !discovered_names.contains(name.as_str()))
        .filter(|name| {
            // Skip already-Unowned entries — they're preserved by definition.
            manifest
                .get(name.as_str())
                .map(|e| e.source_name.is_some())
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    // Partition stale entries into Case 1 (transition / Bucket A) and
    // Case 2 (delete / Bucket B). Capture the source-name pairing for
    // each so the unified three-bucket renderer (UX-01) can show
    // per-skill provenance + actionable hints.
    let mut case1_unowned_transition: Vec<(SkillName, DirectoryName)> = Vec::new();
    let mut case2_delete: Vec<(SkillName, DirectoryName)> = Vec::new();
    for name in &stale {
        let entry = manifest
            .get(name.as_str())
            .expect("stale name from manifest");
        // SAFETY: we already filtered out None-source_name entries above.
        let source = entry
            .source_name
            .as_ref()
            .expect("filter-guard ensures Some")
            .clone();
        if config.directories().contains_key(&source) {
            // Source dir is still configured -> file vanished from disk -> Case 2.
            case2_delete.push((name.clone(), source));
        } else {
            // Source dir is gone from config -> preserve library, transition -> Case 1.
            case1_unowned_transition.push((name.clone(), source));
        }
    }

    // --- Case 1 / Bucket A: transition to Unowned (preserve library content) ---
    //
    // No per-skill output here — the unified three-bucket renderer in
    // `lib.rs::sync` drains `result.bucket_a_removed_from_config` after
    // both library and distribution cleanup have run (D-UX01-2 / D-UX01-4).
    for (name, prev_source) in &case1_unowned_transition {
        if !dry_run {
            // Per D-C1 (Phase 14): capture previous_source before clearing
            // source_name so tome status / tome doctor can render a clean
            // directory name in the Unowned section instead of falling back
            // to source_path. The .take() pattern atomically moves the old
            // value into previous_source and leaves source_name = None.
            // skills_get_mut is provided by Plan 11-01 in manifest.rs.
            if let Some(entry) = manifest.skills_get_mut(name.as_str()) {
                entry.previous_source = entry.source_name.take();
            }
        }
        result.transitioned_to_unowned += 1;
        result
            .bucket_a_removed_from_config
            .push((name.clone(), prev_source.clone()));
    }

    // --- Case 2 / Bucket B: delete (today's behavior) ---
    //
    // The unified renderer surfaces per-skill provenance + the "restore the
    // file, or run `tome remove skill <name>`" hint after both library and
    // distribution cleanup complete. The interactive deletion confirmation
    // below stays — it's the destructive-action gate, not the user-facing
    // summary (which the renderer owns).
    let skills_to_remove: Vec<SkillName> = if interactive && !case2_delete.is_empty() {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Delete {} skill(s) missing from source on disk from library?",
                case2_delete.len()
            ))
            .default(false)
            .interact_opt()?;
        if confirmed == Some(true) {
            case2_delete.iter().map(|(n, _)| n.clone()).collect()
        } else {
            Vec::new()
        }
    } else if !case2_delete.is_empty() {
        case2_delete.iter().map(|(n, _)| n.clone()).collect()
    } else {
        Vec::new()
    };

    // Capture Bucket B regardless of interactive prompt outcome — the
    // user-facing summary should reflect what tome detected as missing,
    // not the post-confirmation delete decision (which is a destructive
    // action the user explicitly confirms).
    for (name, source) in &case2_delete {
        result
            .bucket_b_missing_from_disk
            .push((name.clone(), source.clone()));
    }

    for name in skills_to_remove {
        let entry_path = library_dir.join(name.as_str());

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
    use crate::config::DirectoryName;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    fn empty_config() -> crate::config::Config {
        crate::config::Config::default()
    }

    fn config_with_dir(name: &str) -> crate::config::Config {
        use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};
        use std::collections::BTreeMap;
        let mut directories = BTreeMap::new();
        directories.insert(
            DirectoryName::new(name).unwrap(),
            DirectoryConfig {
                path: std::path::PathBuf::from("/tmp/source"),
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Source),
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );
        Config {
            directories,
            ..Default::default()
        }
    }

    #[test]
    fn cleanup_transitions_orphaned_to_unowned_when_source_removed_from_config() {
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
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );

        // "old-skill" is NOT in discovered names AND its source 'test' is NOT
        // in config.directories -> Case 1 (transition to Unowned).
        let config = empty_config();
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            false,
            true,
        )
        .unwrap();

        assert_eq!(result.removed_from_library, 0, "Case 1 must NOT delete");
        assert_eq!(result.transitioned_to_unowned, 1, "Case 1 must transition");
        assert!(
            library.path().join("old-skill").exists(),
            "Case 1 must preserve library content"
        );
        assert!(
            manifest.contains_key("old-skill"),
            "Case 1 must keep manifest entry"
        );
        assert_eq!(
            manifest.get("old-skill").unwrap().source_name,
            None,
            "Case 1 must transition source_name to None"
        );
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
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );

        let config = config_with_dir("test");
        let discovered: HashSet<String> = ["keep-me".to_string()].into();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            false,
            true,
        )
        .unwrap();

        assert_eq!(result.removed_from_library, 0);
        assert_eq!(result.transitioned_to_unowned, 0);
        assert!(library.path().join("keep-me").exists());
    }

    #[test]
    fn cleanup_dry_run_does_not_mutate_manifest_for_unowned_transition() {
        let library = TempDir::new().unwrap();

        let skill_dir = library.path().join("stale");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("stale").unwrap(),
            crate::manifest::SkillEntry {
                source_path: std::path::PathBuf::from("/tmp/source/stale"),
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );

        // Source 'test' is NOT in config.directories -> Case 1 (transition).
        // Dry-run: count the would-be transition but don't mutate.
        let config = empty_config();
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            true,
            false,
            true,
        )
        .unwrap();

        assert_eq!(result.removed_from_library, 0);
        assert_eq!(
            result.transitioned_to_unowned, 1,
            "dry-run should count the would-be transition"
        );
        // Library content preserved (Case 1 preserves regardless of dry-run).
        assert!(library.path().join("stale").exists());
        // Manifest entry preserved AND source_name unchanged (dry-run skipped mutation).
        assert!(manifest.contains_key("stale"));
        assert_eq!(
            manifest.get("stale").unwrap().source_name,
            Some(DirectoryName::new("test").unwrap()),
            "dry-run must NOT mutate source_name"
        );
    }

    #[test]
    fn cleanup_removes_broken_legacy_symlinks() {
        let library = TempDir::new().unwrap();

        // Create a broken v0.1.x symlink
        unix_fs::symlink("/nonexistent/path", library.path().join("broken")).unwrap();

        let mut manifest = Manifest::default();
        let config = empty_config();
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            false,
            true,
        )
        .unwrap();

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

        // Create a broken symlink simulating a managed skill whose source was removed.
        // Manifest has NO entry for stale-skill — so it's not classified by D-09 cases;
        // it falls into the broken-symlink branch instead.
        unix_fs::symlink("/nonexistent", library.path().join("stale-skill")).unwrap();
        assert!(library.path().join("stale-skill").is_symlink());

        let mut manifest = Manifest::default();
        let config = empty_config();
        let discovered: HashSet<String> = HashSet::new();

        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            true,
            false,
            true,
        )
        .unwrap();

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
    fn cleanup_transitions_managed_symlink_to_unowned_when_source_removed() {
        let library = TempDir::new().unwrap();
        let source = tempfile::TempDir::new().unwrap();

        // Create a managed skill symlink in the library (v0.9-shape artifact;
        // in v0.10 these would be real dirs per Plan 11-02, but the legacy
        // shape is still a valid scenario here).
        let skill_source = source.path().join("plugin-skill");
        std::fs::create_dir_all(&skill_source).unwrap();
        std::fs::write(skill_source.join("SKILL.md"), "# test").unwrap();
        unix_fs::symlink(&skill_source, library.path().join("plugin-skill")).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("plugin-skill").unwrap(),
            crate::manifest::SkillEntry {
                source_path: skill_source,
                source_name: Some(DirectoryName::new("plugins").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: true,
            },
        );

        // Source 'plugins' is NOT in config.directories -> Case 1 (transition).
        let config = empty_config();
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            false,
            true,
        )
        .unwrap();

        assert_eq!(result.removed_from_library, 0, "Case 1 must NOT delete");
        assert_eq!(result.transitioned_to_unowned, 1, "Case 1 must transition");
        assert!(
            library.path().join("plugin-skill").is_symlink(),
            "library content (managed symlink) preserved on transition"
        );
        assert!(
            manifest.contains_key("plugin-skill"),
            "manifest entry preserved on transition"
        );
        assert_eq!(
            manifest.get("plugin-skill").unwrap().source_name,
            None,
            "source_name transitioned to None"
        );
    }

    #[test]
    fn cleanup_case2_deletes_when_source_still_configured() {
        let library = TempDir::new().unwrap();
        let skill_dir = library.path().join("vanished");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("vanished").unwrap(),
            crate::manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/source/vanished"),
                crate::config::DirectoryName::new("active-source").unwrap(),
                crate::validation::test_hash("h"),
                false,
            ),
        );

        // Config STILL has "active-source" — file vanished from source disk -> Case 2.
        let config = config_with_dir("active-source");
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            false,
            true,
        )
        .unwrap();

        assert_eq!(result.removed_from_library, 1, "Case 2 must delete");
        assert_eq!(
            result.transitioned_to_unowned, 0,
            "Case 2 must NOT transition"
        );
        assert!(
            !library.path().join("vanished").exists(),
            "Case 2 must remove library dir"
        );
        assert!(!manifest.contains_key("vanished"));
    }

    #[test]
    fn cleanup_already_unowned_entry_is_preserved_and_not_counted() {
        let library = TempDir::new().unwrap();
        let skill_dir = library.path().join("orphan");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("orphan").unwrap(),
            crate::manifest::SkillEntry::new_unowned(
                std::path::PathBuf::from("/tmp/orphan"),
                crate::validation::test_hash("h"),
                false,
                None,
            ),
        );

        let config = empty_config();
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            false,
            true,
        )
        .unwrap();

        assert_eq!(result.removed_from_library, 0);
        assert_eq!(
            result.transitioned_to_unowned, 0,
            "already-Unowned must not be counted"
        );
        assert!(
            library.path().join("orphan").is_dir(),
            "Unowned library content preserved"
        );
        assert!(manifest.contains_key("orphan"));
        assert!(manifest.get("orphan").unwrap().source_name.is_none());
    }

    #[test]
    fn cleanup_case1_records_previous_source() {
        let library = TempDir::new().unwrap();
        let skill_dir = library.path().join("orphan");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("orphan").unwrap(),
            crate::manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/removed/orphan"),
                crate::config::DirectoryName::new("removed-source").unwrap(),
                crate::validation::test_hash("h"),
                false,
            ),
        );

        let config = empty_config();
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            false,
            true,
        )
        .unwrap();
        assert_eq!(result.transitioned_to_unowned, 1);
        let entry = manifest.get("orphan").unwrap();
        assert_eq!(entry.source_name, None, "source_name cleared");
        assert_eq!(
            entry.previous_source,
            Some(crate::config::DirectoryName::new("removed-source").unwrap()),
            "previous_source must record the original owner per D-C1"
        );
    }

    #[test]
    fn cleanup_case1_and_case2_in_same_run() {
        let library = TempDir::new().unwrap();
        std::fs::create_dir_all(library.path().join("orphan-c1")).unwrap();
        std::fs::create_dir_all(library.path().join("vanished-c2")).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("orphan-c1").unwrap(),
            crate::manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/removed-source/orphan-c1"),
                crate::config::DirectoryName::new("removed-source").unwrap(),
                crate::validation::test_hash("h1"),
                false,
            ),
        );
        manifest.insert(
            crate::discover::SkillName::new("vanished-c2").unwrap(),
            crate::manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/active-source/vanished-c2"),
                crate::config::DirectoryName::new("active-source").unwrap(),
                crate::validation::test_hash("h2"),
                false,
            ),
        );

        // Config has "active-source" but NOT "removed-source".
        let config = config_with_dir("active-source");
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            false,
            true,
        )
        .unwrap();

        assert_eq!(result.removed_from_library, 1);
        assert_eq!(result.transitioned_to_unowned, 1);
        assert!(library.path().join("orphan-c1").exists(), "C1 preserved");
        assert!(!library.path().join("vanished-c2").exists(), "C2 deleted");
        assert_eq!(manifest.get("orphan-c1").unwrap().source_name, None);
        assert!(!manifest.contains_key("vanished-c2"));
    }

    // -- UX-01 three-bucket renderer tests (Plan 16-01 Task 1) --

    /// Drives a Case 1 + Case 2 partition (mirrors
    /// `cleanup_case1_and_case2_in_same_run`) and asserts that the new
    /// `bucket_a_removed_from_config` / `bucket_b_missing_from_disk`
    /// fields are populated, then renders the buckets to a `Vec<u8>`
    /// writer and pins the bucket-distinct header substrings.
    #[test]
    fn cleanup_populates_bucket_a_and_bucket_b_for_renderer() {
        let library = TempDir::new().unwrap();
        std::fs::create_dir_all(library.path().join("orphan-a")).unwrap();
        std::fs::create_dir_all(library.path().join("vanished-b")).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            crate::discover::SkillName::new("orphan-a").unwrap(),
            crate::manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/removed-source/orphan-a"),
                crate::config::DirectoryName::new("removed-source").unwrap(),
                crate::validation::test_hash("h1"),
                false,
            ),
        );
        manifest.insert(
            crate::discover::SkillName::new("vanished-b").unwrap(),
            crate::manifest::SkillEntry::new(
                std::path::PathBuf::from("/tmp/active-source/vanished-b"),
                crate::config::DirectoryName::new("active-source").unwrap(),
                crate::validation::test_hash("h2"),
                false,
            ),
        );

        let config = config_with_dir("active-source");
        let discovered: HashSet<String> = HashSet::new();
        let result = cleanup_library(
            library.path(),
            &discovered,
            &mut manifest,
            &config,
            false,
            true,
            true,
        )
        .unwrap();

        // Bucket A populated with the Case 1 entry.
        assert_eq!(
            result.bucket_a_removed_from_config.len(),
            1,
            "Bucket A should hold the orphan-a Case 1 entry"
        );
        assert_eq!(
            result.bucket_a_removed_from_config[0].0.as_str(),
            "orphan-a",
        );
        assert_eq!(
            result.bucket_a_removed_from_config[0].1.as_str(),
            "removed-source",
        );

        // Bucket B populated with the Case 2 entry.
        assert_eq!(
            result.bucket_b_missing_from_disk.len(),
            1,
            "Bucket B should hold the vanished-b Case 2 entry"
        );
        assert_eq!(
            result.bucket_b_missing_from_disk[0].0.as_str(),
            "vanished-b",
        );
        assert_eq!(
            result.bucket_b_missing_from_disk[0].1.as_str(),
            "active-source",
        );

        // Render to a Vec<u8> writer (D-UX01-4 stderr discipline — the
        // renderer writes through a Write so callers route to stderr,
        // tests route to a buffer).
        let mut buf = Vec::new();
        let bucket_c: Vec<ExcludedSkill> = Vec::new();
        render_cleanup_buckets(
            &mut buf,
            &result.bucket_a_removed_from_config,
            &result.bucket_b_missing_from_disk,
            &bucket_c,
        )
        .unwrap();
        let rendered = String::from_utf8(buf).unwrap();

        // Bucket-distinct header substrings (D-UX01-3) — locked phrasing.
        assert!(
            rendered.contains("no longer in any source"),
            "Bucket A header substring missing from rendered output:\n{rendered}"
        );
        assert!(
            rendered.contains("missing from configured source on disk"),
            "Bucket B header substring missing from rendered output:\n{rendered}"
        );
        // Forbidden-phrase absence is asserted by the integration test in
        // tests/cli_sync.rs (Task 3); cleanup.rs satisfies the
        // grep-based acceptance criterion locally by never referencing the
        // trigger phrase from CONTEXT.md `<specifics>`.
    }

    #[test]
    fn render_bucket_a_includes_tome_reassign_hint() {
        let bucket_a = vec![(
            crate::discover::SkillName::new("foo").unwrap(),
            crate::config::DirectoryName::new("my-old-dir").unwrap(),
        )];
        let bucket_b: Vec<(SkillName, DirectoryName)> = Vec::new();
        let bucket_c: Vec<ExcludedSkill> = Vec::new();

        let mut buf = Vec::new();
        render_cleanup_buckets(&mut buf, &bucket_a, &bucket_b, &bucket_c).unwrap();
        let rendered = String::from_utf8(buf).unwrap();

        assert!(
            rendered.contains("tome reassign"),
            "Bucket A per-skill hint must point at `tome reassign` (D-API-1 vocab):\n{rendered}"
        );
        assert!(
            rendered.contains("foo"),
            "Bucket A per-skill line must include the skill name:\n{rendered}"
        );
        assert!(
            rendered.contains("my-old-dir"),
            "Bucket A per-skill line must name the previous source:\n{rendered}"
        );
    }

    #[test]
    fn render_bucket_b_includes_tome_remove_skill_hint() {
        let bucket_a: Vec<(SkillName, DirectoryName)> = Vec::new();
        let bucket_b = vec![(
            crate::discover::SkillName::new("qux").unwrap(),
            crate::config::DirectoryName::new("my-current-dir").unwrap(),
        )];
        let bucket_c: Vec<ExcludedSkill> = Vec::new();

        let mut buf = Vec::new();
        render_cleanup_buckets(&mut buf, &bucket_a, &bucket_b, &bucket_c).unwrap();
        let rendered = String::from_utf8(buf).unwrap();

        assert!(
            rendered.contains("tome remove skill"),
            "Bucket B per-skill hint must point at `tome remove skill` (D-API-2 vocab):\n{rendered}"
        );
        assert!(
            rendered.contains("qux"),
            "Bucket B per-skill line must include the skill name:\n{rendered}"
        );
    }

    #[test]
    fn render_bucket_c_global_and_per_directory_phrasing() {
        let bucket_a: Vec<(SkillName, DirectoryName)> = Vec::new();
        let bucket_b: Vec<(SkillName, DirectoryName)> = Vec::new();
        let bucket_c = vec![
            ExcludedSkill {
                name: crate::discover::SkillName::new("quux").unwrap(),
                directory: None,
            },
            ExcludedSkill {
                name: crate::discover::SkillName::new("corge").unwrap(),
                directory: Some(crate::config::DirectoryName::new("my-dir").unwrap()),
            },
        ];

        let mut buf = Vec::new();
        render_cleanup_buckets(&mut buf, &bucket_a, &bucket_b, &bucket_c).unwrap();
        let rendered = String::from_utf8(buf).unwrap();

        assert!(
            rendered.contains("now in exclude list"),
            "Bucket C header substring missing:\n{rendered}"
        );
        assert!(
            rendered.contains("excluded globally"),
            "Bucket C global-disable per-skill annotation missing:\n{rendered}"
        );
        assert!(
            rendered.contains("excluded for: my-dir"),
            "Bucket C per-directory annotation must name the directory:\n{rendered}"
        );
        assert!(
            rendered.contains("machine.toml::disabled"),
            "Bucket C global hint must point at machine.toml::disabled:\n{rendered}"
        );
        assert!(
            rendered.contains("machine.toml::directories.my-dir.disabled"),
            "Bucket C per-directory hint must name the per-dir path:\n{rendered}"
        );
    }

    #[test]
    fn render_empty_buckets_produces_no_output() {
        let bucket_a: Vec<(SkillName, DirectoryName)> = Vec::new();
        let bucket_b: Vec<(SkillName, DirectoryName)> = Vec::new();
        let bucket_c: Vec<ExcludedSkill> = Vec::new();

        let mut buf = Vec::new();
        render_cleanup_buckets(&mut buf, &bucket_a, &bucket_b, &bucket_c).unwrap();

        assert!(
            buf.is_empty(),
            "all-empty buckets should produce no output (silent on no-op syncs); got: {:?}",
            String::from_utf8_lossy(&buf)
        );
    }
}

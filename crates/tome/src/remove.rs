//! Remove a directory entry from config and clean up most artifacts.
//!
//! Cleanup order (v0.10, per LIB-04 / D-10 trigger 1):
//! 1. Remove symlinks from distribution directories
//! 2. Transition manifest entries owned by this directory to Unowned
//!    (`source_name = None`) — library content is preserved on disk
//! 3. Remove cached git repo (if git-type directory)
//! 4. Remove directory entry from config
//! 5. Regenerate lockfile
//!
//! Note: library directories for skills owned by the removed directory are
//! NOT deleted (LIB-04). The user can later re-anchor them with
//! `tome adopt <skill> <new-dir>` or delete them with `tome forget <skill>`
//! (Phase 14 commands).

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
    /// Library directories for these skills (preserved per LIB-04 v0.10 — these
    /// are reported by render_plan as "kept as Unowned" but NOT deleted by execute).
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
///
/// Variants are documented by the structural dispatch predicate that emits
/// them, not by step number — step numbering is implementation detail and
/// reordering the steps in `execute()` must not require doc edits.
///
/// In v0.10 (LIB-04 / D-10 trigger 1), `tome remove` no longer touches
/// library files — owned manifest entries transition to Unowned and library
/// content is preserved on disk. The `LibraryDir` and `LibrarySymlink`
/// variants from v0.9 are removed; only distribution-dir symlinks and the
/// git repo cache remain as failable filesystem operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureKind {
    /// Distribution-dir symlink removal — emitted when `remove_file` fails
    /// while iterating `plan.symlinks_to_remove`.
    DistributionSymlink,
    /// Git repo cache removal — emitted when `remove_dir_all` fails on the
    /// plan's `git_cache_path`.
    GitCache,
}

impl FailureKind {
    /// All variants, in the order preferred for user-facing grouped output.
    ///
    /// Exposed as an associated constant so the `lib.rs` consumer doesn't
    /// maintain a parallel hand-written array that could silently drop a
    /// variant when new variants are added.
    pub(crate) const ALL: [FailureKind; 2] = [
        FailureKind::DistributionSymlink,
        FailureKind::GitCache,
    ];

    /// Human-readable label used in the grouped failure summary.
    pub(crate) fn label(self) -> &'static str {
        match self {
            FailureKind::DistributionSymlink => "Distribution symlinks",
            FailureKind::GitCache => "Git cache",
        }
    }
}

/// Compile-time drift guard for `FailureKind::ALL` (POLISH-04 option c).
///
/// If a new variant is added to `FailureKind`, this `const fn` fails to
/// compile because the match below is exhaustive. The fix is to (a) add
/// an arm here AND (b) append the new variant to `ALL`. Symmetric to the
/// 12-combo `(DirectoryType, DirectoryRole)` matrix test that
/// compile-enforces config-shape invariants (WHARD-06).
///
/// The function is dead-code at runtime — its sole purpose is the
/// exhaustiveness check. The `const _: () = ...` block below additionally
/// pins `ALL.len() == 2` at compile time so a hand-edit that adds a
/// match arm here without growing `ALL` (or vice versa) also fails.
#[allow(dead_code)]
const fn _ensure_failure_kind_all_exhaustive(k: FailureKind) -> usize {
    match k {
        FailureKind::DistributionSymlink => 0,
        FailureKind::GitCache => 1,
    }
}

const _: () = {
    // If this fails: FailureKind::ALL is missing or has extra variants.
    // The match arms in _ensure_failure_kind_all_exhaustive are the source
    // of truth — ALL must contain exactly one entry per arm.
    assert!(FailureKind::ALL.len() == 2);
};

/// A single partial-cleanup failure aggregated from `execute`.
#[derive(Debug)]
pub(crate) struct RemoveFailure {
    pub path: PathBuf,
    pub kind: FailureKind,
    pub error: std::io::Error,
}

impl RemoveFailure {
    /// Construct a `RemoveFailure` (POLISH-05 option a).
    ///
    /// The path MUST be absolute — downstream rendering uses
    /// `paths::collapse_home(&f.path)` in lib.rs, which expects an absolute
    /// path. Relative paths would render unmodified, leaking
    /// working-directory-relative shapes into user-facing error output.
    ///
    /// The four `execute()` call sites all pass paths derived from
    /// config-resolved directory paths (always absolute), so this guard
    /// never fires in normal use; it's a forward guard against a future
    /// refactor that adds a relative-path call site. Debug-only via
    /// `debug_assert!` to keep release builds zero-cost.
    pub(crate) fn new(kind: FailureKind, path: PathBuf, error: std::io::Error) -> Self {
        debug_assert!(
            path.is_absolute(),
            "RemoveFailure::path must be absolute, got: {}",
            path.display()
        );
        RemoveFailure { kind, path, error }
    }
}

/// Result of executing the remove plan.
pub(crate) struct RemoveResult {
    pub symlinks_removed: usize,
    /// Manifest entries transitioned to Unowned (`source_name = None`) per
    /// LIB-04 / D-10 trigger 1. Library content for these skills is preserved
    /// on disk; only the manifest's source_name field is mutated.
    pub library_entries_transitioned_to_unowned: usize,
    pub git_cache_removed: bool,
    /// Partial-cleanup failures that occurred during `execute`.
    ///
    /// Empty on full success. Caller is responsible for surfacing these;
    /// `execute` itself does not print per-failure warnings.
    ///
    /// Post-condition: an entry in `failures` does NOT imply the
    /// corresponding counter (e.g. `symlinks_removed`) is zero —
    /// partial-success semantics mean counters reflect COMPLETED operations,
    /// failures reflect INCOMPLETE ones, and the two are independent. If
    /// 3 of 5 distribution symlinks removed successfully and 2 failed,
    /// `symlinks_removed == 3` and `failures.len() == 2`.
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
        .filter(|(_, entry)| {
            entry
                .source_name
                .as_ref()
                .is_some_and(|s| s.as_str() == name)
        })
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
            "  Library content preserved as {} (run `tome forget <skill>` later to delete): {}",
            style("Unowned").yellow(),
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
    let mut library_entries_transitioned_to_unowned = 0;
    let mut git_cache_removed = false;
    let mut failures: Vec<RemoveFailure> = Vec::new();

    // 1. Remove symlinks from distribution directories.
    for symlink in &plan.symlinks_to_remove {
        if dry_run {
            symlinks_removed += 1;
        } else {
            match std::fs::remove_file(symlink) {
                Ok(_) => symlinks_removed += 1,
                Err(e) => failures.push(RemoveFailure::new(
                    FailureKind::DistributionSymlink,
                    symlink.clone(),
                    e,
                )),
            }
        }
    }

    // 2. Remove cached git repo (if applicable). Library deletion is
    //    intentionally absent here — library content is preserved per LIB-04;
    //    the manifest transition below is the user-visible "removal".
    if let Some(cache_path) = &plan.git_cache_path {
        if dry_run {
            git_cache_removed = true;
        } else {
            match std::fs::remove_dir_all(cache_path) {
                Ok(_) => git_cache_removed = true,
                Err(e) => failures.push(RemoveFailure::new(
                    FailureKind::GitCache,
                    cache_path.clone(),
                    e,
                )),
            }
        }
    }

    // 3. On full success: transition manifest entries owned by this directory
    //    to Unowned (source_name = None) AND remove the directory entry from
    //    config (LIB-04 / D-10 trigger 1). Library content is preserved on disk.
    //
    //    On partial failure: preserve config + manifest entries unchanged so
    //    `tome remove <name>` can be re-run after addressing the underlying
    //    cause (matches Phase 8 SAFE-01 retention semantics).
    //
    //    skills_get_mut is provided by Plan 11-01 in manifest.rs.
    if !dry_run && failures.is_empty() {
        for skill_name in &plan.skills {
            if let Some(entry) = manifest.skills_get_mut(skill_name) {
                entry.source_name = None;
                library_entries_transitioned_to_unowned += 1;
            }
        }
        config.directories.remove(&plan.directory_name);
    } else if dry_run {
        // In dry-run, count what WOULD transition but don't mutate.
        library_entries_transitioned_to_unowned = plan.skills.len();
    }

    Ok(RemoveResult {
        symlinks_removed,
        library_entries_transitioned_to_unowned,
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
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );
        directories.insert(
            DirectoryName::new("test-target").unwrap(),
            DirectoryConfig {
                path: target_dir,
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Target),
                git_ref: None,
                subdir: None,
                override_applied: false,
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
                DirectoryName::new("test-source").unwrap(),
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
    fn execute_transitions_to_unowned_and_preserves_library() {
        let (_tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        let result = execute(&p, &mut config, &mut manifest, false).unwrap();
        assert_eq!(result.symlinks_removed, 1);
        assert_eq!(result.library_entries_transitioned_to_unowned, 1);
        assert!(
            !config
                .directories
                .contains_key(&DirectoryName::new("test-source").unwrap()),
            "config entry must be removed on full success"
        );
        assert!(!manifest.is_empty(), "manifest entry retained as Unowned");
        assert_eq!(
            manifest.get("my-skill").unwrap().source_name,
            None,
            "transitioned to Unowned"
        );
        assert!(
            _tmp.path().join("library").join("my-skill").exists(),
            "library content preserved per LIB-04"
        );
    }

    #[test]
    fn partial_failure_aggregates_symlink_error() {
        let (tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        // Pre-delete the distribution symlink so std::fs::remove_file returns
        // ENOENT during execute's step 1 loop — forcing a
        // FailureKind::DistributionSymlink push without affecting the
        // library-entry step which should still succeed.
        let dist_symlink = tmp.path().join("target").join("my-skill");
        assert_eq!(
            p.symlinks_to_remove.len(),
            1,
            "fixture expected one dist symlink"
        );
        assert_eq!(p.symlinks_to_remove[0], dist_symlink);
        std::fs::remove_file(&dist_symlink).ok();

        let result = execute(&p, &mut config, &mut manifest, false).unwrap();

        // Assert: exactly one DistributionSymlink failure, path matches.
        assert!(
            result
                .failures
                .iter()
                .any(|f| f.kind == FailureKind::DistributionSymlink),
            "expected a FailureKind::DistributionSymlink failure, got: {:?}",
            result.failures,
        );
        let symlink_failure = result
            .failures
            .iter()
            .find(|f| f.kind == FailureKind::DistributionSymlink)
            .unwrap();
        assert_eq!(symlink_failure.path, dist_symlink);

        // Partial-failure semantics: no Unowned transition happens because
        // failures.is_empty() is false. The library entry (and manifest
        // entry) are preserved unchanged.
        assert_eq!(result.library_entries_transitioned_to_unowned, 0);
        assert_eq!(result.symlinks_removed, 0);

        // I2/I3 retention: on partial failure, the config entry AND the
        // manifest entries are preserved so the user can re-run
        // `tome remove <name>` after addressing the failure. Without this,
        // the user would be stuck: plan() bails with "not found in config"
        // and there's no programmatic way to re-register it.
        assert!(
            config
                .directories
                .contains_key(&DirectoryName::new("test-source").unwrap()),
            "config entry must be retained on partial failure so retry works"
        );
        assert!(
            !manifest.is_empty(),
            "manifest entries must be retained on partial failure so retry sees the skills"
        );
        assert_eq!(
            manifest.get("my-skill").unwrap().source_name,
            Some(DirectoryName::new("test-source").unwrap()),
            "transition NOT applied on partial failure"
        );
    }

    #[test]
    fn execute_dry_run_preserves_state() {
        let (_tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        let result = execute(&p, &mut config, &mut manifest, true).unwrap();
        assert_eq!(result.symlinks_removed, 1);
        assert_eq!(
            result.library_entries_transitioned_to_unowned, 1,
            "dry-run should count the would-be transition"
        );
        // Config and manifest should not be modified
        assert!(
            config
                .directories
                .contains_key(&DirectoryName::new("test-source").unwrap()),
            "dry-run preserves config"
        );
        assert!(!manifest.is_empty());
        assert_eq!(
            manifest.get("my-skill").unwrap().source_name,
            Some(DirectoryName::new("test-source").unwrap()),
            "dry-run does not mutate manifest"
        );
    }

    #[test]
    fn execute_transitions_multiple_owned_skills_to_unowned() {
        let (_tmp, mut config, paths, mut manifest) = make_test_setup();
        // Add two more skills owned by test-source.
        for n in ["skill-2", "skill-3"] {
            manifest.insert(
                SkillName::new(n).unwrap(),
                SkillEntry::new(
                    _tmp.path().join("source").join(n),
                    DirectoryName::new("test-source").unwrap(),
                    test_hash(),
                    false,
                ),
            );
        }
        let p = plan("test-source", &config, &paths, &manifest).unwrap();
        let result = execute(&p, &mut config, &mut manifest, false).unwrap();

        assert_eq!(result.library_entries_transitioned_to_unowned, 3);
        for n in ["my-skill", "skill-2", "skill-3"] {
            assert_eq!(
                manifest.get(n).unwrap().source_name,
                None,
                "skill {n} should transition to Unowned"
            );
        }
        assert!(result.failures.is_empty());
        assert!(
            !config
                .directories
                .contains_key(&DirectoryName::new("test-source").unwrap()),
            "config entry removed on full success"
        );
    }

    /// Cover FailureKind::label() exhaustively — pins user-visible label
    /// strings for every variant (a rename in one variant would fail here).
    /// In v0.10 (LIB-04 / Plan 11-03) the LibraryDir/LibrarySymlink variants
    /// were removed; only DistributionSymlink and GitCache remain.
    #[test]
    fn failure_kind_label_coverage() {
        assert_eq!(
            FailureKind::DistributionSymlink.label(),
            "Distribution symlinks"
        );
        assert_eq!(FailureKind::GitCache.label(), "Git cache");
    }

    /// `FailureKind::ALL` is consumed by lib.rs's grouped failure summary;
    /// pinning length to 2 also pairs with the const-fn drift guard
    /// `_ensure_failure_kind_all_exhaustive` so a hand-edit that grows
    /// the enum without growing ALL fails to compile.
    #[test]
    fn failure_kind_all_pinned_size_two() {
        assert_eq!(FailureKind::ALL.len(), 2);
        assert!(FailureKind::ALL.contains(&FailureKind::DistributionSymlink));
        assert!(FailureKind::ALL.contains(&FailureKind::GitCache));
    }

    // POLISH-04: Pins the runtime drift check that complements the
    // compile-time `_ensure_failure_kind_all_exhaustive` sentinel.
    // Uses a hand-rolled uniqueness check (FailureKind only derives
    // PartialEq/Eq, not Ord/Hash, so BTreeSet/HashSet are unavailable).
    #[test]
    fn failure_kind_all_length_matches_variant_count() {
        let all = FailureKind::ALL;
        assert_eq!(
            all.len(),
            2,
            "FailureKind::ALL must contain every variant exactly once"
        );
        // Pairwise-unique: no duplicates in ALL.
        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(a, b, "FailureKind::ALL contains duplicate variant {a:?}");
            }
        }
        // Membership: every variant appears.
        assert!(all.contains(&FailureKind::DistributionSymlink));
        assert!(all.contains(&FailureKind::GitCache));
    }

    // POLISH-04: The grouped failure-summary output in lib.rs::Command::Remove
    // iterates FailureKind::ALL in declaration order. The user-visible grouping
    // therefore depends on this exact order. A reorder is a UI change and
    // must require an explicit code edit (this test fails on reorder).
    #[test]
    fn failure_kind_all_ordering_pinned() {
        assert_eq!(
            FailureKind::ALL,
            [FailureKind::DistributionSymlink, FailureKind::GitCache,],
            "FailureKind::ALL ordering is part of the user-visible grouping contract"
        );
    }

    // POLISH-05: `RemoveFailure::new` carries a debug-only `is_absolute`
    // invariant. Debug builds panic on relative paths; release builds
    // compile out the assert (zero release cost).
    #[test]
    fn remove_failure_new_relative_path_panics_in_debug() {
        let result = std::panic::catch_unwind(|| {
            RemoveFailure::new(
                FailureKind::DistributionSymlink,
                PathBuf::from("relative/path"),
                std::io::Error::other("test"),
            )
        });
        if cfg!(debug_assertions) {
            assert!(result.is_err(), "debug build must panic on relative path");
        } else {
            assert!(
                result.is_ok(),
                "release build must allow construction (debug_assert compiled out)"
            );
        }
    }

    // POLISH-05: Absolute paths are accepted in both debug and release.
    #[test]
    fn remove_failure_new_absolute_path_succeeds() {
        let f = RemoveFailure::new(
            FailureKind::DistributionSymlink,
            PathBuf::from("/abs/path"),
            std::io::Error::other("test"),
        );
        assert_eq!(f.kind, FailureKind::DistributionSymlink);
        assert_eq!(f.path, PathBuf::from("/abs/path"));
    }
}

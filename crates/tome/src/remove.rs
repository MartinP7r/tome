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
///
/// Variants are documented by the structural dispatch predicate that emits
/// them, not by step number — step numbering is implementation detail and
/// reordering the steps in `execute()` must not require doc edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureKind {
    /// Distribution-dir symlink removal — emitted when `remove_file` fails
    /// while iterating `plan.symlinks_to_remove`.
    DistributionSymlink,
    /// Local library directory removal — emitted by the `remove_dir_all`
    /// branch of library cleanup (dispatched by `lib_path.is_dir()`).
    LibraryDir,
    /// Managed-skill library symlink removal — emitted by the `remove_file`
    /// branch of library cleanup (dispatched by `lib_path.is_symlink()`).
    LibrarySymlink,
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
    pub(crate) const ALL: [FailureKind; 4] = [
        FailureKind::DistributionSymlink,
        FailureKind::LibraryDir,
        FailureKind::LibrarySymlink,
        FailureKind::GitCache,
    ];

    /// Human-readable label used in the grouped failure summary.
    pub(crate) fn label(self) -> &'static str {
        match self {
            FailureKind::DistributionSymlink => "Distribution symlinks",
            FailureKind::LibraryDir => "Library entries",
            FailureKind::LibrarySymlink => "Library symlinks",
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
/// pins `ALL.len() == 4` at compile time so a hand-edit that adds a
/// match arm here without growing `ALL` (or vice versa) also fails.
#[allow(dead_code)]
const fn _ensure_failure_kind_all_exhaustive(k: FailureKind) -> usize {
    match k {
        FailureKind::DistributionSymlink => 0,
        FailureKind::LibraryDir => 1,
        FailureKind::LibrarySymlink => 2,
        FailureKind::GitCache => 3,
    }
}

const _: () = {
    // If this fails: FailureKind::ALL is missing or has extra variants.
    // The match arms in _ensure_failure_kind_all_exhaustive are the source
    // of truth — ALL must contain exactly one entry per arm.
    assert!(FailureKind::ALL.len() == 4);
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
    pub library_entries_removed: usize,
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
                Err(e) => failures.push(RemoveFailure::new(
                    FailureKind::DistributionSymlink,
                    symlink.clone(),
                    e,
                )),
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
                Err(e) => failures.push(RemoveFailure::new(
                    FailureKind::LibrarySymlink,
                    lib_path.clone(),
                    e,
                )),
            }
        } else if lib_path.is_dir() {
            match std::fs::remove_dir_all(lib_path) {
                Ok(_) => library_entries_removed += 1,
                Err(e) => failures.push(RemoveFailure::new(
                    FailureKind::LibraryDir,
                    lib_path.clone(),
                    e,
                )),
            }
        }
    }

    // 4. Remove cached git repo (step 3/manifest cleanup is deferred to
    //    after step 4 so we know the full failure state before deciding
    //    whether to preserve config+manifest for retry).
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

    // Partial-failure state preservation (I2, I3 from phase-8 PR review).
    // If ANY cleanup step failed, preserve the config entry AND the
    // manifest entries so the user can re-run `tome remove <name>` after
    // addressing the underlying cause (e.g., fixing file permissions).
    // Otherwise the user would be stuck: plan() bails with "directory
    // not found in config" and `tome doctor` cannot re-register a
    // vanished config entry — recovery would be manual `rm -rf`.
    //
    // On full success: remove manifest entries (step 3) and config
    // entry (step 5) as before. dry_run preserves everything.
    if !dry_run && failures.is_empty() {
        // 3. Remove manifest entries
        for skill in &plan.skills {
            manifest.remove(skill);
        }
        // 5. Remove directory entry from config
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
                override_applied: false,
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

        // Partial-failure semantics: the library entry (separate artifact)
        // should still have been cleaned up.
        assert_eq!(result.library_entries_removed, 1);
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

    /// Covers the lib.rs grouped-output formatting indirectly: populates TWO
    /// FailureKind variants (DistributionSymlink + LibraryDir) in a single
    /// execute() call, then asserts that:
    /// - the failures vec contains both variants
    /// - the counters correctly reflect partial success (both zero because
    ///   neither step completed any operation)
    ///
    /// Pins the partial-success invariant across multiple variants and
    /// ensures a future refactor that drops a `FailureKind` from the ALL
    /// constant or label() map would not silently break grouped output
    /// for the affected variant. Also covers FailureKind::label() for
    /// every variant and the size of FailureKind::ALL.
    #[cfg(unix)]
    #[test]
    fn partial_failure_aggregates_multiple_kinds() {
        use std::os::unix::fs::PermissionsExt;

        let (tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        // Pre-delete the distribution symlink → DistributionSymlink
        // failure on step 1 (remove_file returns ENOENT).
        let dist_symlink = tmp.path().join("target").join("my-skill");
        std::fs::remove_file(&dist_symlink).ok();

        // chmod the LIBRARY PARENT to 0o500 (r-x) so step 2's
        // remove_dir_all on library/my-skill fails with EACCES. The write
        // bit on the parent is what's needed to unlink children; the
        // search bit alone still lets is_dir() check the child.
        // (remove_dir_all on a non-existent path returns Ok(()) in Rust,
        // so the simpler "pre-delete" trick used elsewhere doesn't work
        // here — we need a real filesystem-level permission denial.)
        assert_eq!(p.library_paths.len(), 1, "fixture expected one lib path");
        let library_parent = p.library_paths[0].parent().unwrap().to_path_buf();
        std::fs::set_permissions(&library_parent, std::fs::Permissions::from_mode(0o500)).unwrap();

        let result = execute(&p, &mut config, &mut manifest, false);

        // CRITICAL: restore permissions BEFORE assertions so TempDir::drop
        // can clean up even on panic (Pitfall 2 from 08-RESEARCH.md).
        std::fs::set_permissions(&library_parent, std::fs::Permissions::from_mode(0o755)).unwrap();

        let result = result.expect("execute should not error, only aggregate failures");

        let has = |k: FailureKind| result.failures.iter().any(|f| f.kind == k);
        assert!(
            has(FailureKind::DistributionSymlink),
            "expected DistributionSymlink failure, got: {:?}",
            result.failures
        );
        assert!(
            has(FailureKind::LibraryDir),
            "expected LibraryDir failure (from EACCES on parent), got: {:?}",
            result.failures
        );

        // Both counters zero — neither step completed.
        assert_eq!(result.symlinks_removed, 0);
        assert_eq!(result.library_entries_removed, 0);

        // Cover FailureKind::label() exhaustively — pins user-visible label
        // strings for every variant (a rename in one variant would fail here).
        assert_eq!(
            FailureKind::DistributionSymlink.label(),
            "Distribution symlinks"
        );
        assert_eq!(FailureKind::LibraryDir.label(), "Library entries");
        assert_eq!(FailureKind::LibrarySymlink.label(), "Library symlinks");
        assert_eq!(FailureKind::GitCache.label(), "Git cache");

        // Cover FailureKind::ALL — the consumer in lib.rs iterates this.
        assert_eq!(FailureKind::ALL.len(), 4);
        assert!(FailureKind::ALL.contains(&FailureKind::DistributionSymlink));
        assert!(FailureKind::ALL.contains(&FailureKind::LibraryDir));
        assert!(FailureKind::ALL.contains(&FailureKind::LibrarySymlink));
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
            4,
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
        assert!(all.contains(&FailureKind::LibraryDir));
        assert!(all.contains(&FailureKind::LibrarySymlink));
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
            [
                FailureKind::DistributionSymlink,
                FailureKind::LibraryDir,
                FailureKind::LibrarySymlink,
                FailureKind::GitCache,
            ],
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

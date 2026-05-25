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
use crate::discover::SkillName;
use crate::manifest::Manifest;
use crate::paths::TomePaths;

/// What will be removed.
///
/// `pub` (not `pub(crate)`) so the v1.0 `tome-desktop` crate can render a
/// plan-preview-confirm flow over the Tauri IPC boundary (OPS-* in Phase 29).
#[derive(Debug, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct RemovePlan {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
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
    pub(crate) const ALL: [FailureKind; 2] =
        [FailureKind::DistributionSymlink, FailureKind::GitCache];

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
///
/// The `error` field is a pre-stringified message (not a live
/// `std::io::Error`) so this type is `Serialize` + `specta::Type` and can
/// cross the Tauri IPC boundary (Pitfall 2). The GUI can't act on a live
/// `io::Error` anyway — the display string is the boundary-useful shape. This
/// is a deliberate field-shape sub-decision flagged in the plan SUMMARY.
#[derive(Debug, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub(crate) struct RemoveFailure {
    pub path: PathBuf,
    pub kind: FailureKind,
    pub error: String,
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
    ///
    /// `error` is stringified at construction (`error.to_string()`) so the
    /// stored field is the boundary-friendly `String` shape (Pitfall 2).
    pub(crate) fn new(kind: FailureKind, path: PathBuf, error: std::io::Error) -> Self {
        debug_assert!(
            path.is_absolute(),
            "RemoveFailure::path must be absolute, got: {}",
            path.display()
        );
        RemoveFailure {
            kind,
            path,
            error: error.to_string(),
        }
    }
}

/// Which step of `tome remove skill` produced a partial-cleanup failure.
///
/// Variants follow D-B1 cleanup scope (Phase 14):
/// 1. `LibraryDir` — `remove_dir_all` failed on `library_dir/<name>/`
/// 2. `DistributionSymlink` — `remove_file` failed on a per-skill
///    distribution symlink in some Target/Synced directory
/// 3. `Lockfile` — `lockfile::save` failed after removing the entry
/// 4. `MachineToml` — `machine::save` failed after removing memberships
///
/// Manifest mutation is in-memory and saves last; if `manifest::save`
/// fails the error propagates via `?` and never lands here. The aggregate
/// failure-summary semantic only kicks in for filesystem-touch steps that
/// need group reporting (Phase 8 SAFE-01 + Phase 10 POLISH-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub(crate) enum RemoveSkillFailureKind {
    LibraryDir,
    DistributionSymlink,
    Lockfile,
    MachineToml,
}

impl RemoveSkillFailureKind {
    /// All variants, in the order preferred for user-facing grouped output.
    pub(crate) const ALL: [RemoveSkillFailureKind; 4] = [
        RemoveSkillFailureKind::LibraryDir,
        RemoveSkillFailureKind::DistributionSymlink,
        RemoveSkillFailureKind::Lockfile,
        RemoveSkillFailureKind::MachineToml,
    ];

    /// Human-readable label used in the grouped failure summary.
    pub(crate) fn label(self) -> &'static str {
        match self {
            RemoveSkillFailureKind::LibraryDir => "Library directory",
            RemoveSkillFailureKind::DistributionSymlink => "Distribution symlinks",
            RemoveSkillFailureKind::Lockfile => "Lockfile",
            RemoveSkillFailureKind::MachineToml => "Machine prefs",
        }
    }
}

/// Compile-time drift guard for `RemoveSkillFailureKind::ALL` (POLISH-04 option c).
/// Mirrors `_ensure_failure_kind_all_exhaustive` for `FailureKind`.
#[allow(dead_code)]
const fn _ensure_remove_skill_failure_kind_all_exhaustive(k: RemoveSkillFailureKind) -> usize {
    match k {
        RemoveSkillFailureKind::LibraryDir => 0,
        RemoveSkillFailureKind::DistributionSymlink => 1,
        RemoveSkillFailureKind::Lockfile => 2,
        RemoveSkillFailureKind::MachineToml => 3,
    }
}

const _: () = {
    // If this fails: RemoveSkillFailureKind::ALL is missing or has extra
    // variants. The match arms in
    // _ensure_remove_skill_failure_kind_all_exhaustive are the source
    // of truth — ALL must contain exactly one entry per arm.
    assert!(RemoveSkillFailureKind::ALL.len() == 4);
};

/// A single partial-cleanup failure aggregated from `skill_execute`.
/// Mirror of `RemoveFailure` for the `skill` flavour. `error` is a
/// pre-stringified message so the type is `Serialize` + `specta::Type`
/// (Pitfall 2).
#[derive(Debug, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub(crate) struct RemoveSkillFailure {
    pub path: PathBuf,
    pub kind: RemoveSkillFailureKind,
    pub error: String,
}

impl RemoveSkillFailure {
    /// Construct a `RemoveSkillFailure` (POLISH-05 mirror).
    ///
    /// The path MUST be absolute — downstream rendering uses
    /// `paths::collapse_home(&f.path)` which expects an absolute path.
    /// Debug-only via `debug_assert!` to keep release builds zero-cost.
    ///
    /// `error` is stringified at construction (`error.to_string()`) so the
    /// stored field is the boundary-friendly `String` shape (Pitfall 2).
    pub(crate) fn new(kind: RemoveSkillFailureKind, path: PathBuf, error: std::io::Error) -> Self {
        debug_assert!(
            path.is_absolute(),
            "RemoveSkillFailure::path must be absolute, got: {}",
            path.display()
        );
        RemoveSkillFailure {
            kind,
            path,
            error: error.to_string(),
        }
    }
}

/// What `tome remove skill <name>` will do (per D-B1).
#[derive(Debug)]
pub(crate) struct RemoveSkillPlan {
    /// Skill name being deleted.
    pub skill_name: SkillName,
    /// Library directory path (`library_dir/<skill_name>/`). Absolute.
    pub library_path: PathBuf,
    /// Distribution symlinks pointing at this skill in target/synced dirs.
    /// Each path is absolute.
    pub symlinks_to_remove: Vec<PathBuf>,
    /// Whether the skill has a lockfile entry that needs deleting.
    pub has_lockfile_entry: bool,
    /// Whether the skill is in `machine.toml::disabled`.
    pub in_machine_disabled: bool,
    /// Per-directory machine.toml memberships to clean. Each tuple is
    /// `(directory_name, in_enabled, in_disabled)`. Empty when the skill
    /// isn't referenced by any per-directory list.
    pub per_directory_memberships: Vec<(DirectoryName, bool, bool)>,
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
///
/// `pub` (CORE-01 / D-GUI-08): the reference plan/preview/confirm fn that the
/// GUI's mutating-operations UI (Phase 29) calls directly to render the
/// `RemovePlan` before executing.
pub fn plan(
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
        .filter(|(_, entry)| entry.source_name().is_some_and(|s| s.as_str() == name))
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
            if let Some(entry) = manifest.skills_get_mut(skill_name)
                && let crate::manifest::SkillOwnership::Owned { source } = &entry.ownership
            {
                // Per D-C1 (Phase 14, transition site 2): capture the old
                // owning directory as the Unowned breadcrumb so the user can
                // see the original owner name in `tome status` after this
                // directory is gone from config.
                let last_owner = Some(source.clone());
                entry.ownership = crate::manifest::SkillOwnership::Unowned { last_owner };
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

// ============================================================================
// `tome remove skill <name>` — Phase 14 Plan 14-05 (UNOWN-02)
// ============================================================================
//
// Mirror of the `dir`-flavour `plan/render_plan/execute` triple above, for
// deleting an Unowned skill from the library. Cleanup scope per D-B1:
//
//   1. manifest[name] entry (Manifest::remove)
//   2. library_dir/<name>/ directory tree (std::fs::remove_dir_all)
//   3. Distribution symlinks for the skill in every distribution-role dir
//   4. tome.lock entry for the skill (LockEntry removal)
//   5. machine.toml::disabled set membership
//   6. machine.toml::directories.<dir>.enabled / .disabled list memberships
//
// D-B2: Owned skills are refused (no --force bypass).
// D-B3: caller (lib.rs::run) handles confirmation default-no, --yes bypass.
// SAFE-01 + POLISH-04: failures aggregate via RemoveSkillFailureKind::ALL
// with compile-time exhaustiveness pinning (above).

/// Result of `skill_execute`.
#[derive(Debug)]
pub(crate) struct RemoveSkillResult {
    pub library_removed: bool,
    pub symlinks_removed: usize,
    pub lockfile_entry_removed: bool,
    pub machine_disabled_removed: bool,
    pub per_directory_cleanups: usize,
    /// Partial-cleanup failures aggregated from `skill_execute`. Empty on
    /// full success. On any failure, in-memory state (manifest, lockfile,
    /// machine_prefs) is NOT mutated — matching the dir-flavour I2/I3
    /// retention semantic so the caller can retry after addressing the
    /// underlying cause without losing state.
    pub failures: Vec<RemoveSkillFailure>,
}

/// Build a plan for `tome remove skill <name>`. Refuses Owned skills (D-B2).
pub(crate) fn skill_plan(
    name: &str,
    config: &Config,
    paths: &TomePaths,
    manifest: &Manifest,
    lockfile: Option<&crate::lockfile::Lockfile>,
    machine_prefs: &crate::machine::MachinePrefs,
) -> Result<RemoveSkillPlan> {
    // Validate skill exists in manifest.
    let entry = manifest
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("skill '{}' not found in library", name))?;

    // D-B2 owned guard: refuse to operate on Owned skills. No --force bypass —
    // the source file is still on disk and the next `tome sync` would
    // re-discover the skill. The hint points at actionable paths.
    if let Some(owner) = entry.source_name() {
        anyhow::bail!(
            "skill '{}' is owned by directory '{}' (source_name = {}). \
             Remove the source directory with `tome remove dir {}` first, \
             or remove the file from disk and re-sync.",
            name,
            owner,
            owner,
            owner,
        );
    }

    let skill_name =
        SkillName::new(name).with_context(|| format!("invalid skill name in manifest: {name}"))?;
    let library_path = paths.library_dir().join(name);

    // Find distribution symlinks pointing at this skill across every
    // distribution-role directory (Target or Synced).
    let mut symlinks_to_remove = Vec::new();
    for other_config in config.directories.values() {
        let role = other_config.role();
        if !role.is_distribution() {
            continue;
        }
        let skills_dir = match crate::config::expand_tilde(&other_config.path) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !skills_dir.is_dir() {
            continue;
        }
        let candidate = skills_dir.join(name);
        // is_symlink() returns true for both intact and broken symlinks; both
        // should be cleaned up. A non-symlink with the same name (e.g. a real
        // directory the user created manually) is left alone.
        if candidate.is_symlink() {
            symlinks_to_remove.push(candidate);
        }
    }

    // Lockfile membership.
    let has_lockfile_entry = lockfile
        .map(|lf| lf.skills.contains_key(&skill_name))
        .unwrap_or(false);

    // machine.toml memberships.
    let in_machine_disabled = machine_prefs.is_disabled(name);

    let mut per_directory_memberships: Vec<(DirectoryName, bool, bool)> = Vec::new();
    for (dir_name, dir_prefs) in &machine_prefs.directory {
        let in_enabled = dir_prefs
            .enabled
            .as_ref()
            .map(|set| set.iter().any(|s| s.as_str() == name))
            .unwrap_or(false);
        let in_disabled = dir_prefs.disabled.iter().any(|s| s.as_str() == name);
        if in_enabled || in_disabled {
            per_directory_memberships.push((dir_name.clone(), in_enabled, in_disabled));
        }
    }

    Ok(RemoveSkillPlan {
        skill_name,
        library_path,
        symlinks_to_remove,
        has_lockfile_entry,
        in_machine_disabled,
        per_directory_memberships,
    })
}

/// Render the skill-removal plan to stdout.
pub(crate) fn skill_render_plan(plan: &RemoveSkillPlan) {
    println!(
        "Forget skill plan for '{}':",
        style(plan.skill_name.as_str()).cyan()
    );
    if plan.library_path.exists() {
        println!(
            "  Library directory will be removed: {}",
            style(crate::paths::collapse_home(&plan.library_path)).dim()
        );
    }
    if !plan.symlinks_to_remove.is_empty() {
        println!(
            "  Distribution symlinks to remove: {}",
            style(plan.symlinks_to_remove.len()).bold()
        );
    }
    if plan.has_lockfile_entry {
        println!("  Lockfile entry will be removed.");
    }
    if plan.in_machine_disabled {
        println!("  Membership in `machine.toml::disabled` will be removed.");
    }
    if !plan.per_directory_memberships.is_empty() {
        println!(
            "  Per-directory machine.toml memberships to clean: {}",
            style(plan.per_directory_memberships.len()).bold()
        );
        for (dir, in_e, in_d) in &plan.per_directory_memberships {
            let parts: Vec<&str> = match (in_e, in_d) {
                (true, true) => vec!["enabled", "disabled"],
                (true, false) => vec!["enabled"],
                (false, true) => vec!["disabled"],
                (false, false) => continue,
            };
            println!("    - {}: {}", dir, parts.join(", "));
        }
    }
}

/// Execute the skill-removal plan.
///
/// On full success, mutates manifest, lockfile, and machine_prefs in memory.
/// Caller is responsible for calling `manifest::save` / `lockfile::save` /
/// `machine::save` (atomic temp+rename) — this function does no disk writes
/// for those three artifacts.
///
/// On partial filesystem failure (LibraryDir or DistributionSymlink), returns
/// `failures` without mutating in-memory state — matching the dir-flavour
/// I2/I3 retention semantic so the caller can retry after addressing the
/// underlying cause.
///
/// `dry_run = true` skips all filesystem and in-memory mutation but still
/// reports would-be counters in the result.
pub(crate) fn skill_execute(
    plan: &RemoveSkillPlan,
    manifest: &mut Manifest,
    lockfile: &mut Option<crate::lockfile::Lockfile>,
    machine_prefs: &mut crate::machine::MachinePrefs,
    dry_run: bool,
) -> Result<RemoveSkillResult> {
    let mut failures: Vec<RemoveSkillFailure> = Vec::new();
    let mut library_removed = false;
    let mut symlinks_removed = 0usize;

    // 1. Remove library directory.
    if plan.library_path.exists() {
        if dry_run {
            library_removed = true;
        } else {
            match std::fs::remove_dir_all(&plan.library_path) {
                Ok(_) => library_removed = true,
                Err(e) => failures.push(RemoveSkillFailure::new(
                    RemoveSkillFailureKind::LibraryDir,
                    plan.library_path.clone(),
                    e,
                )),
            }
        }
    }

    // 2. Remove distribution symlinks.
    for symlink in &plan.symlinks_to_remove {
        if dry_run {
            symlinks_removed += 1;
        } else {
            match std::fs::remove_file(symlink) {
                Ok(_) => symlinks_removed += 1,
                Err(e) => failures.push(RemoveSkillFailure::new(
                    RemoveSkillFailureKind::DistributionSymlink,
                    symlink.clone(),
                    e,
                )),
            }
        }
    }

    // On partial filesystem failure: bail out before mutating in-memory state.
    // The caller will not call save() on this branch, so disk state remains
    // consistent (matches dir-flavour I2/I3 retention).
    let mut lockfile_entry_removed = false;
    let mut machine_disabled_removed = false;
    let mut per_directory_cleanups = 0usize;

    if failures.is_empty() && !dry_run {
        // 3. Remove lockfile entry (in-memory).
        if let Some(lf) = lockfile.as_mut()
            && lf.skills.remove(&plan.skill_name).is_some()
        {
            lockfile_entry_removed = true;
        }

        // 4. Remove machine.toml::disabled membership (in-memory).
        if machine_prefs
            .disabled
            .iter()
            .any(|s| s.as_str() == plan.skill_name.as_str())
        {
            machine_prefs
                .disabled
                .retain(|s| s.as_str() != plan.skill_name.as_str());
            machine_disabled_removed = true;
        }

        // 5. Remove per-directory memberships (in-memory).
        for (dir_name, _in_e, _in_d) in &plan.per_directory_memberships {
            if let Some(dir_prefs) = machine_prefs.directory.get_mut(dir_name) {
                let before_e = dir_prefs.enabled.as_ref().map(|s| s.len()).unwrap_or(0);
                if let Some(enabled) = dir_prefs.enabled.as_mut() {
                    enabled.retain(|s| s.as_str() != plan.skill_name.as_str());
                }
                let after_e = dir_prefs.enabled.as_ref().map(|s| s.len()).unwrap_or(0);
                let before_d = dir_prefs.disabled.len();
                dir_prefs
                    .disabled
                    .retain(|s| s.as_str() != plan.skill_name.as_str());
                let after_d = dir_prefs.disabled.len();
                if (before_e > after_e) || (before_d > after_d) {
                    per_directory_cleanups += 1;
                }
            }
        }

        // 6. Remove manifest entry (in-memory). Last so the in-memory mutation
        //    sequence matches the lockfile/machine.toml ordering and a panic
        //    mid-sequence still leaves the manifest entry available for retry.
        manifest.remove(plan.skill_name.as_str());
    } else if dry_run {
        lockfile_entry_removed = plan.has_lockfile_entry;
        machine_disabled_removed = plan.in_machine_disabled;
        per_directory_cleanups = plan.per_directory_memberships.len();
    }

    Ok(RemoveSkillResult {
        library_removed,
        symlinks_removed,
        lockfile_entry_removed,
        machine_disabled_removed,
        per_directory_cleanups,
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
            manifest.get("my-skill").unwrap().source_name(),
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
            manifest.get("my-skill").unwrap().source_name(),
            Some(&DirectoryName::new("test-source").unwrap()),
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
            manifest.get("my-skill").unwrap().source_name(),
            Some(&DirectoryName::new("test-source").unwrap()),
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
                manifest.get(n).unwrap().source_name(),
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

    #[test]
    fn execute_records_previous_source_on_unowned_transition() {
        let (_tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        let result = execute(&p, &mut config, &mut manifest, false).unwrap();
        assert_eq!(result.library_entries_transitioned_to_unowned, 1);

        let entry = manifest.get("my-skill").unwrap();
        assert_eq!(entry.source_name(), None);
        assert_eq!(
            entry.previous_source(),
            Some(&DirectoryName::new("test-source").unwrap()),
            "previous_source must record the original owner per D-C1"
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

    // === Phase 14 Plan 14-05 Task 1: RemoveSkillFailureKind tests ===
    //
    // Mirror of the `FailureKind` runtime tests above. Pin the variant
    // count, label coverage, ALL ordering, and pairwise uniqueness so a
    // hand-edit that grows the enum without growing ALL fails CI.

    #[test]
    fn remove_skill_failure_kind_all_pinned_size_four() {
        assert_eq!(RemoveSkillFailureKind::ALL.len(), 4);
        assert!(RemoveSkillFailureKind::ALL.contains(&RemoveSkillFailureKind::LibraryDir));
        assert!(RemoveSkillFailureKind::ALL.contains(&RemoveSkillFailureKind::DistributionSymlink));
        assert!(RemoveSkillFailureKind::ALL.contains(&RemoveSkillFailureKind::Lockfile));
        assert!(RemoveSkillFailureKind::ALL.contains(&RemoveSkillFailureKind::MachineToml));
    }

    #[test]
    fn remove_skill_failure_kind_label_coverage() {
        assert_eq!(
            RemoveSkillFailureKind::LibraryDir.label(),
            "Library directory"
        );
        assert_eq!(
            RemoveSkillFailureKind::DistributionSymlink.label(),
            "Distribution symlinks"
        );
        assert_eq!(RemoveSkillFailureKind::Lockfile.label(), "Lockfile");
        assert_eq!(RemoveSkillFailureKind::MachineToml.label(), "Machine prefs");
    }

    #[test]
    fn remove_skill_failure_kind_all_unique() {
        let all = RemoveSkillFailureKind::ALL;
        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(a, b, "ALL contains duplicate variant {a:?}");
            }
        }
    }

    /// `RemoveSkillFailureKind::ALL` ordering is part of the user-visible
    /// grouping contract (consumed by lib.rs::run in Task 3). A reorder is
    /// a UI change that must require an explicit code edit.
    #[test]
    fn remove_skill_failure_kind_all_ordering_pinned() {
        assert_eq!(
            RemoveSkillFailureKind::ALL,
            [
                RemoveSkillFailureKind::LibraryDir,
                RemoveSkillFailureKind::DistributionSymlink,
                RemoveSkillFailureKind::Lockfile,
                RemoveSkillFailureKind::MachineToml,
            ],
            "RemoveSkillFailureKind::ALL ordering is part of the user-visible grouping contract"
        );
    }

    /// POLISH-05 mirror: `RemoveSkillFailure::new` carries a debug-only
    /// `is_absolute` invariant. Debug builds panic on relative paths;
    /// release builds compile out the assert.
    #[test]
    fn remove_skill_failure_new_relative_path_panics_in_debug() {
        let result = std::panic::catch_unwind(|| {
            RemoveSkillFailure::new(
                RemoveSkillFailureKind::LibraryDir,
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

    #[test]
    fn remove_skill_failure_new_absolute_path_succeeds() {
        let f = RemoveSkillFailure::new(
            RemoveSkillFailureKind::Lockfile,
            PathBuf::from("/abs/lock.toml"),
            std::io::Error::other("test"),
        );
        assert_eq!(f.kind, RemoveSkillFailureKind::Lockfile);
        assert_eq!(f.path, PathBuf::from("/abs/lock.toml"));
    }

    // === Phase 14 Plan 14-05 Task 2: skill_plan / skill_execute tests ===

    /// D-B2: skill_plan refuses to operate on Owned skills with the verbatim
    /// error message containing the actionable hint.
    #[test]
    fn skill_plan_refuses_owned_skill() {
        let (_tmp, config, paths, manifest) = make_test_setup();
        // make_test_setup creates an Owned "my-skill" in test-source.
        let lockfile = None;
        let machine_prefs = crate::machine::MachinePrefs::default();
        let err = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile,
            &machine_prefs,
        )
        .expect_err("must refuse Owned per D-B2")
        .to_string();
        assert!(err.contains("is owned by directory"), "got: {err}");
        assert!(err.contains("Remove the source directory"), "got: {err}");
        assert!(err.contains("tome remove dir"), "got: {err}");
    }

    /// skill_plan errors when the named skill isn't in the manifest at all.
    #[test]
    fn skill_plan_skill_not_in_library() {
        let (_tmp, config, paths, manifest) = make_test_setup();
        let lockfile = None;
        let machine_prefs = crate::machine::MachinePrefs::default();
        let err = skill_plan(
            "nonexistent",
            &config,
            &paths,
            &manifest,
            lockfile,
            &machine_prefs,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(err.contains("not found in library"));
    }

    /// D-B1 happy path: skill_execute removes manifest, library directory,
    /// distribution symlinks, lockfile entry, and machine.toml memberships
    /// in one shot.
    #[test]
    fn skill_execute_full_cleanup_happy_path() {
        let (tmp, config, paths, mut manifest) = make_test_setup();
        // Transition my-skill to Unowned for this test (production callers
        // never pass an Owned skill to skill_execute — D-B2 guard above).
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.ownership = crate::manifest::SkillOwnership::Unowned { last_owner: None };

        // Build fake lockfile with my-skill.
        use crate::lockfile::{LockEntry, Lockfile};
        use std::collections::BTreeMap;
        let mut skills = BTreeMap::new();
        skills.insert(
            SkillName::new("my-skill").unwrap(),
            LockEntry {
                source_name: None,
                previous_source: None,
                content_hash: test_hash(),
                registry_id: None,
                version: None,
                git_commit_sha: None,
            },
        );
        let mut lockfile = Some(Lockfile { version: 1, skills });

        // Build machine_prefs with my-skill disabled.
        let mut machine_prefs = crate::machine::MachinePrefs::default();
        machine_prefs.disable(SkillName::new("my-skill").unwrap());

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();
        assert!(plan.has_lockfile_entry);
        assert!(plan.in_machine_disabled);
        assert_eq!(plan.symlinks_to_remove.len(), 1);

        let result = skill_execute(
            &plan,
            &mut manifest,
            &mut lockfile,
            &mut machine_prefs,
            false,
        )
        .unwrap();
        assert!(result.library_removed);
        assert_eq!(result.symlinks_removed, 1, "1 dist symlink in fixture");
        assert!(result.lockfile_entry_removed);
        assert!(result.machine_disabled_removed);
        assert!(result.failures.is_empty());

        // Verify in-memory state.
        assert!(!manifest.contains_key("my-skill"));
        assert!(
            !lockfile
                .as_ref()
                .unwrap()
                .skills
                .contains_key(&SkillName::new("my-skill").unwrap())
        );
        assert!(!machine_prefs.is_disabled("my-skill"));

        // Verify on-disk state.
        assert!(!tmp.path().join("library").join("my-skill").exists());
        assert!(!tmp.path().join("target").join("my-skill").exists());
    }

    /// D-B1 partial: skill not in lockfile is OK — just skip the lockfile
    /// step (no error).
    #[test]
    fn skill_execute_skill_not_in_lockfile_succeeds() {
        let (_tmp, config, paths, mut manifest) = make_test_setup();
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.ownership = crate::manifest::SkillOwnership::Unowned { last_owner: None };

        let mut lockfile: Option<crate::lockfile::Lockfile> = None;
        let mut machine_prefs = crate::machine::MachinePrefs::default();

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();
        assert!(!plan.has_lockfile_entry);

        let result = skill_execute(
            &plan,
            &mut manifest,
            &mut lockfile,
            &mut machine_prefs,
            false,
        )
        .unwrap();
        assert!(result.failures.is_empty());
        assert!(!result.lockfile_entry_removed);
        assert!(!manifest.contains_key("my-skill"));
    }

    /// D-B1 partial: skill not in machine.toml is OK — no error, no mutation
    /// of the machine_disabled bool.
    #[test]
    fn skill_execute_skill_not_in_machine_toml_succeeds() {
        let (_tmp, config, paths, mut manifest) = make_test_setup();
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.ownership = crate::manifest::SkillOwnership::Unowned { last_owner: None };

        let mut lockfile: Option<crate::lockfile::Lockfile> = None;
        let mut machine_prefs = crate::machine::MachinePrefs::default();

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();
        assert!(!plan.in_machine_disabled);
        assert!(plan.per_directory_memberships.is_empty());

        let result = skill_execute(
            &plan,
            &mut manifest,
            &mut lockfile,
            &mut machine_prefs,
            false,
        )
        .unwrap();
        assert!(result.failures.is_empty());
        assert!(!result.machine_disabled_removed);
        assert_eq!(result.per_directory_cleanups, 0);
    }

    /// SAFE-01 partial-failure aggregation: when a distribution-symlink
    /// delete fails, the failure is recorded and in-memory state retained.
    #[test]
    fn skill_execute_partial_failure_preserves_in_memory_state() {
        let (tmp, config, paths, mut manifest) = make_test_setup();
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.ownership = crate::manifest::SkillOwnership::Unowned { last_owner: None };

        let mut lockfile: Option<crate::lockfile::Lockfile> = None;
        let mut machine_prefs = crate::machine::MachinePrefs::default();

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();
        assert_eq!(plan.symlinks_to_remove.len(), 1);

        // Pre-delete the dist symlink so std::fs::remove_file fails with ENOENT
        // during execute's symlink loop.
        let dist_symlink = tmp.path().join("target").join("my-skill");
        std::fs::remove_file(&dist_symlink).ok();

        let result = skill_execute(
            &plan,
            &mut manifest,
            &mut lockfile,
            &mut machine_prefs,
            false,
        )
        .unwrap();
        assert!(
            !result.failures.is_empty(),
            "expected DistributionSymlink failure"
        );
        assert!(
            result
                .failures
                .iter()
                .any(|f| f.kind == RemoveSkillFailureKind::DistributionSymlink),
            "expected DistributionSymlink kind, got: {:?}",
            result.failures
        );

        // I2/I3 retention: manifest entry retained on partial failure so the
        // user can re-run after addressing the underlying cause.
        assert!(
            manifest.contains_key("my-skill"),
            "manifest entry must be preserved on partial failure for retry"
        );
    }

    /// SAFE-01 partial-failure on the library_dir step: when remove_dir_all
    /// fails (here simulated by pre-deleting the library dir so ENOENT is
    /// returned), the failure is recorded and in-memory state retained.
    #[test]
    fn skill_execute_library_dir_failure_preserves_state() {
        // Note: std::fs::remove_dir_all on a missing path returns Ok in
        // recent Rust. We synthesise a real failure by passing a path that
        // exists but is a regular file (remove_dir_all returns NotADirectory).
        let (tmp, config, paths, mut manifest) = make_test_setup();
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.ownership = crate::manifest::SkillOwnership::Unowned { last_owner: None };

        // Replace the library dir with a regular file so remove_dir_all errors.
        let lib_path = tmp.path().join("library").join("my-skill");
        std::fs::remove_dir_all(&lib_path).unwrap();
        std::fs::write(&lib_path, "not a dir").unwrap();

        let mut lockfile: Option<crate::lockfile::Lockfile> = None;
        let mut machine_prefs = crate::machine::MachinePrefs::default();

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();

        let result = skill_execute(
            &plan,
            &mut manifest,
            &mut lockfile,
            &mut machine_prefs,
            false,
        )
        .unwrap();

        assert!(
            result
                .failures
                .iter()
                .any(|f| f.kind == RemoveSkillFailureKind::LibraryDir),
            "expected LibraryDir failure, got: {:?}",
            result.failures
        );
        // I2/I3 retention.
        assert!(manifest.contains_key("my-skill"));
    }

    /// dry_run = true: nothing is mutated; counters reflect would-be ops.
    #[test]
    fn skill_execute_dry_run_no_mutation() {
        let (tmp, config, paths, mut manifest) = make_test_setup();
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.ownership = crate::manifest::SkillOwnership::Unowned { last_owner: None };
        let mut lockfile: Option<crate::lockfile::Lockfile> = None;
        let mut machine_prefs = crate::machine::MachinePrefs::default();

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();

        let result = skill_execute(
            &plan,
            &mut manifest,
            &mut lockfile,
            &mut machine_prefs,
            true,
        )
        .unwrap();
        // Counters reflect would-be operations.
        assert!(result.library_removed);
        assert_eq!(result.symlinks_removed, 1);
        // Manifest still has it.
        assert!(manifest.contains_key("my-skill"));
        // Library still on disk.
        assert!(tmp.path().join("library").join("my-skill").exists());
        // Distribution symlink still on disk.
        assert!(tmp.path().join("target").join("my-skill").is_symlink());
    }

    /// D-B1 step 6 (per-directory memberships): cleanup of both `enabled`
    /// and `disabled` lists across multiple directories.
    #[test]
    fn skill_execute_cleans_per_directory_memberships() {
        use crate::machine::DirectoryPrefs;

        let (_tmp, config, paths, mut manifest) = make_test_setup();
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.ownership = crate::manifest::SkillOwnership::Unowned { last_owner: None };

        let mut lockfile: Option<crate::lockfile::Lockfile> = None;
        let mut machine_prefs = crate::machine::MachinePrefs::default();

        // Add per-directory memberships across two directories: one in
        // `disabled`, one in `enabled`.
        machine_prefs.directory.insert(
            DirectoryName::new("test-source").unwrap(),
            DirectoryPrefs {
                disabled: [SkillName::new("my-skill").unwrap()].into_iter().collect(),
                ..Default::default()
            },
        );
        machine_prefs.directory.insert(
            DirectoryName::new("test-target").unwrap(),
            DirectoryPrefs {
                enabled: Some([SkillName::new("my-skill").unwrap()].into_iter().collect()),
                ..Default::default()
            },
        );

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();
        assert_eq!(plan.per_directory_memberships.len(), 2);

        let result = skill_execute(
            &plan,
            &mut manifest,
            &mut lockfile,
            &mut machine_prefs,
            false,
        )
        .unwrap();
        assert!(result.failures.is_empty());
        assert_eq!(result.per_directory_cleanups, 2);

        // Verify the lists are now empty.
        let src_prefs = machine_prefs
            .directory
            .get(&DirectoryName::new("test-source").unwrap())
            .unwrap();
        assert!(src_prefs.disabled.is_empty());
        let tgt_prefs = machine_prefs
            .directory
            .get(&DirectoryName::new("test-target").unwrap())
            .unwrap();
        assert!(
            tgt_prefs
                .enabled
                .as_ref()
                .map(|s| s.is_empty())
                .unwrap_or(true)
        );
    }

    /// Atomic save round-trip: after `skill_execute` mutates lockfile +
    /// machine_prefs in memory, calling lockfile::save and machine::save
    /// produces a clean on-disk state with the entries gone.
    #[test]
    fn skill_execute_save_round_trip() {
        use crate::lockfile::{LockEntry, Lockfile};
        use std::collections::BTreeMap;

        let (tmp, config, paths, mut manifest) = make_test_setup();
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.ownership = crate::manifest::SkillOwnership::Unowned { last_owner: None };

        // Lockfile with my-skill entry.
        let mut skills = BTreeMap::new();
        skills.insert(
            SkillName::new("my-skill").unwrap(),
            LockEntry {
                source_name: None,
                previous_source: None,
                content_hash: test_hash(),
                registry_id: None,
                version: None,
                git_commit_sha: None,
            },
        );
        let mut lockfile = Some(Lockfile { version: 1, skills });

        // Machine prefs with my-skill disabled.
        let mut machine_prefs = crate::machine::MachinePrefs::default();
        machine_prefs.disable(SkillName::new("my-skill").unwrap());

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();

        skill_execute(
            &plan,
            &mut manifest,
            &mut lockfile,
            &mut machine_prefs,
            false,
        )
        .unwrap();

        // Save and reload — verify atomic save round-trips cleanly.
        let machine_path = tmp.path().join("machine.toml");
        crate::machine::save(&machine_prefs, &machine_path).unwrap();
        let reloaded = crate::machine::load(&machine_path).unwrap();
        assert!(!reloaded.is_disabled("my-skill"));

        if let Some(lf) = &lockfile {
            crate::lockfile::save(lf, paths.config_dir()).unwrap();
            let reloaded_lf = crate::lockfile::load(paths.config_dir()).unwrap().unwrap();
            assert!(
                !reloaded_lf
                    .skills
                    .contains_key(&SkillName::new("my-skill").unwrap())
            );
        }
    }
}

//! Diagnose and optionally repair issues such as missing entries, orphan directories,
//! and stale directory symlinks.

use anyhow::{Context, Result};
use console::style;
use dialoguer::Confirm;
use std::io::IsTerminal;
use std::path::Path;
use tracing::debug;

use crate::cleanup;
use crate::config::Config;
use crate::manifest;
use crate::paths::{TomePaths, resolve_symlink_target};

// -- Data structs --

/// Severity of a diagnostic issue.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum IssueSeverity {
    /// Critical problem (e.g., missing directory, broken symlink).
    Error,
    /// Non-critical problem (e.g., orphan directory, missing source path).
    Warning,
}

/// Categorical classification for a [`DiagnosticIssue`].
///
/// Most existing diagnostic checks emit a free-form `message` string with
/// a [`IssueSeverity`]; this typed kind sits alongside that field for
/// issues whose call sites need to discriminate on the issue *category*
/// (e.g. doctor JSON output, future repair routines).
///
/// HARD-09 / D-DIST-2 introduces the first variant:
/// [`DiagnosticIssueKind::ForeignSymlink`].
///
/// Future variants must extend [`DiagnosticIssueKind::ALL`] and the
/// compile-time exhaustiveness sentinel below (POLISH-04 pattern).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum DiagnosticIssueKind {
    /// A distribution-directory entry is a symlink whose target lives
    /// outside the active `library_dir` — typically left behind by a
    /// different tome install or a hand-edited dotfiles workflow.
    /// Renders as [`IssueSeverity::Warning`] and contributes to
    /// [`DoctorReport::total_issues`].
    ForeignSymlink,
}

impl DiagnosticIssueKind {
    /// Compile-time-validated enumeration of every variant. Mirrors
    /// `crate::remove::FailureKind::ALL` and
    /// `crate::marketplace::InstallFailureKind::ALL`.
    pub const ALL: [DiagnosticIssueKind; 1] = [DiagnosticIssueKind::ForeignSymlink];
}

/// Compile-time drift guard for [`DiagnosticIssueKind::ALL`] (POLISH-04).
/// If a future variant is added without updating `ALL`, this match fails to
/// compile (`non-exhaustive patterns`) and the const-len assert fails
/// `cargo check`. Either failure forces the maintainer to update the array.
#[allow(dead_code)]
const fn _diagnostic_issue_kind_exhaustiveness_sentinel(kind: DiagnosticIssueKind) {
    match kind {
        // If this fails: DiagnosticIssueKind::ALL is missing or has extra
        // variants. Update the array and this match arm together.
        DiagnosticIssueKind::ForeignSymlink => {}
    }
}
const _: () = {
    assert!(DiagnosticIssueKind::ALL.len() == 1);
};

/// Category of a [`DiagnosticIssue`]. Derived at construction from the
/// [`DoctorReport`] field the issue lives in, with `ForeignSymlink`
/// promoted regardless of source field per D-CAT-1.
///
/// JSON serialisation is snake_case (`"library"`, `"directory"`,
/// `"config"`, `"foreign_symlink"`), matching the project convention.
///
/// Per POLISH-04: `ALL` array + compile-time exhaustiveness sentinel
/// keep every variant pinned. Adding a variant without updating `ALL`
/// is a `cargo check` failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    Library,
    Directory,
    Config,
    ForeignSymlink,
}

impl IssueCategory {
    /// Compile-time-validated enumeration of every variant. Mirrors
    /// `DiagnosticIssueKind::ALL` and other POLISH-04 patterns.
    pub const ALL: [Self; 4] = [
        Self::Library,
        Self::Directory,
        Self::Config,
        Self::ForeignSymlink,
    ];
}

/// Compile-time drift guard for [`IssueCategory::ALL`] (POLISH-04).
/// Adding a variant without updating `ALL` and this match fails to
/// compile (`non-exhaustive patterns`) or trips the const-len assert.
#[allow(dead_code)]
const fn _issue_category_exhaustiveness_sentinel(c: IssueCategory) {
    match c {
        IssueCategory::Library => {}
        IssueCategory::Directory => {}
        IssueCategory::Config => {}
        IssueCategory::ForeignSymlink => {}
    }
}
const _: () = {
    assert!(IssueCategory::ALL.len() == 4);
};

/// Categorises the auto-repair available for a [`DiagnosticIssue`]
/// (D-REPAIR-1).
///
/// `Some(kind)` on [`DiagnosticIssue::repair_kind`] ↔ the issue is
/// auto-fixable and the global repair dispatcher in [`diagnose`] has a
/// handler arm for `kind`. `None` means the issue requires user
/// interaction (e.g. orphan directories, which use a per-item Select
/// prompt) or is informational only.
///
/// JSON serialisation is snake_case
/// (`"remove_stale_manifest_entry"`, `"remove_broken_library_symlink"`,
/// `"remove_stale_target_symlink"`).
///
/// Per POLISH-04: `ALL` array + compile-time exhaustiveness sentinel
/// pin every variant. The repair dispatcher matches exhaustively on
/// `Option<RepairKind>` — adding a variant without a handler arm fails
/// to compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
// Every variant in this enum names a specific "Remove …" action. The
// shared `Remove` prefix is intentional (one variant per real
// auto-repair handler) and aids readability at call sites.
#[allow(clippy::enum_variant_names)]
pub enum RepairKind {
    /// Remove a manifest entry whose library directory is missing on
    /// disk OR whose managed symlink is broken. Emit sites in
    /// `check_library` (both cases share the action:
    /// `Manifest::remove(name)` plus `remove_file` if the entry is a
    /// symlink).
    RemoveStaleManifestEntry,
    /// Remove a broken legacy symlink in the library directory (not
    /// referenced by the manifest). Emit site: `check_library`
    /// "broken legacy symlink: X -> Y". Action: `remove_file(path)`.
    RemoveBrokenLibrarySymlink,
    /// Remove a stale symlink from a distribution directory. Emit
    /// site: `check_distribution_dir` "stale symlink X". Action:
    /// `cleanup::cleanup_target` removes broken symlinks pointing into
    /// the library.
    RemoveStaleTargetSymlink,
}

impl RepairKind {
    /// Compile-time-validated enumeration of every variant. Mirrors
    /// `DiagnosticIssueKind::ALL` and other POLISH-04 patterns.
    pub const ALL: [Self; 3] = [
        Self::RemoveStaleManifestEntry,
        Self::RemoveBrokenLibrarySymlink,
        Self::RemoveStaleTargetSymlink,
    ];
}

/// Compile-time drift guard for [`RepairKind::ALL`] (POLISH-04).
/// Adding a variant without updating `ALL` and this match fails to
/// compile (`non-exhaustive patterns`) or trips the const-len assert.
#[allow(dead_code)]
const fn _repair_kind_exhaustiveness_sentinel(k: RepairKind) {
    match k {
        RepairKind::RemoveStaleManifestEntry => {}
        RepairKind::RemoveBrokenLibrarySymlink => {}
        RepairKind::RemoveStaleTargetSymlink => {}
    }
}
const _: () = {
    assert!(RepairKind::ALL.len() == 3);
};

/// A single diagnostic issue found during a health check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagnosticIssue {
    pub severity: IssueSeverity,
    pub message: String,
    /// Optional typed classification. Existing diagnostic emit sites
    /// leave this `None` (the free-form `message` carries the detail);
    /// HARD-09 D-DIST-2 ForeignSymlink is the first emitter to set it.
    /// Serialised JSON shape: omitted when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<DiagnosticIssueKind>,
    /// Category bucket for the OBS-06 categorised summary line and
    /// `tome doctor --json` per-issue category field. Computed at
    /// construction from the [`DoctorReport`] field the issue lives
    /// in, with `ForeignSymlink` promoted regardless of source field
    /// per D-CAT-1. Always emits in JSON.
    pub category: IssueCategory,
    /// Auto-repair classifier (D-REPAIR-1). `Some(kind)` ↔ the
    /// repair dispatcher in [`diagnose`] has a handler arm for `kind`
    /// and the issue contributes to `auto_fixable_count`. `None`
    /// means interactive-only (orphan directories) or informational.
    /// Omitted from JSON when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_kind: Option<RepairKind>,
}

impl DiagnosticIssue {
    /// Build a Library-category issue with no auto-repair handler.
    /// Used by `check_library` for non-repairable findings (e.g.
    /// orphan directories — interactive-only) and the "library dir
    /// missing" warning.
    ///
    /// The pre-OBS-06 `untyped`/`typed` constructors are deleted; all
    /// emit sites now use one of the category-specific constructors
    /// (`library`/`library_repairable`/`directory`/`directory_repairable`
    /// /`directory_foreign_symlink`/`config`) so `category` and
    /// `repair_kind` are set at construction time.
    pub(crate) fn library(severity: IssueSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            kind: None,
            category: IssueCategory::Library,
            repair_kind: None,
        }
    }

    /// Build a Library-category issue that the dispatcher can
    /// auto-repair. The supplied [`RepairKind`] must correspond to a
    /// match arm in the dispatcher (exhaustive match enforces this at
    /// compile time).
    pub(crate) fn library_repairable(
        severity: IssueSeverity,
        message: impl Into<String>,
        repair_kind: RepairKind,
    ) -> Self {
        Self {
            severity,
            message: message.into(),
            kind: None,
            category: IssueCategory::Library,
            repair_kind: Some(repair_kind),
        }
    }

    /// Build a Directory-category issue with no auto-repair handler.
    pub(crate) fn directory(severity: IssueSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            kind: None,
            category: IssueCategory::Directory,
            repair_kind: None,
        }
    }

    /// Build a Directory-category issue that the dispatcher can
    /// auto-repair.
    pub(crate) fn directory_repairable(
        severity: IssueSeverity,
        message: impl Into<String>,
        repair_kind: RepairKind,
    ) -> Self {
        Self {
            severity,
            message: message.into(),
            kind: None,
            category: IssueCategory::Directory,
            repair_kind: Some(repair_kind),
        }
    }

    /// Build a Directory-emitted ForeignSymlink issue.
    /// Category is promoted to `ForeignSymlink` regardless of source
    /// field (D-CAT-1). `kind` is set so the existing
    /// `DiagnosticIssueKind` JSON surface stays consistent.
    pub(crate) fn directory_foreign_symlink(
        severity: IssueSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            message: message.into(),
            kind: Some(DiagnosticIssueKind::ForeignSymlink),
            category: IssueCategory::ForeignSymlink,
            repair_kind: None,
        }
    }

    /// Build a Config-category issue. Config issues are not
    /// auto-repairable (config edits require user action).
    pub(crate) fn config(severity: IssueSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            kind: None,
            category: IssueCategory::Config,
            repair_kind: None,
        }
    }
}

/// Per-directory diagnostic entry. Aggregates issues for one configured
/// directory and notes whether its `path` was rewritten by a `machine.toml`
/// `[directory_overrides.<name>]` entry (PORT-05).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectoryDiagnostic {
    pub name: String,
    pub issues: Vec<DiagnosticIssue>,
    /// True iff `directories.<name>.path` was rewritten by a `machine.toml`
    /// override during config load. Renders as ` (override)` after the
    /// directory name in text mode; appears as `override_applied: true` in
    /// `tome doctor --json`.
    pub override_applied: bool,
}

/// Complete diagnostic report for the tome system.
#[derive(Debug, serde::Serialize)]
pub struct DoctorReport {
    pub configured: bool,
    pub library_issues: Vec<DiagnosticIssue>,
    pub directory_issues: Vec<DirectoryDiagnostic>,
    pub config_issues: Vec<DiagnosticIssue>,
    /// Unowned skills (UNOWN-03 / D-D3). INFORMATIONAL section — these
    /// entries do NOT contribute to `total_issues` and do NOT affect
    /// `tome doctor` exit code. They surface in text rendering as a
    /// parallel "Unowned skills" section after the issue checks.
    pub unowned_skills: Vec<crate::summary::SkillSummary>,
}

impl DoctorReport {
    /// Sum of actionable diagnostic issues. Per D-D3, `unowned_skills`
    /// is INTENTIONALLY excluded — Unowned is an informational state
    /// (the user removed a directory), not a malfunction. `tome doctor`
    /// exit code is unaffected by the Unowned set.
    pub fn total_issues(&self) -> usize {
        self.library_issues.len()
            + self
                .directory_issues
                .iter()
                .map(|d| d.issues.len())
                .sum::<usize>()
            + self.config_issues.len()
    }

    /// Flatten the three issue buckets into a single iterator.
    /// Used by the OBS-06 categorised summary and the FIX-01 repair
    /// dispatcher (D-REPAIR-3 — replaces substring matching).
    pub fn all_issues(&self) -> impl Iterator<Item = &DiagnosticIssue> {
        self.library_issues
            .iter()
            .chain(self.directory_issues.iter().flat_map(|d| d.issues.iter()))
            .chain(self.config_issues.iter())
    }

    /// Number of issues for which the dispatcher has an auto-repair
    /// handler. D-REPAIR-2: when this is zero, the global
    /// "Apply N auto-fixable repairs?" prompt is skipped entirely.
    pub fn auto_fixable_count(&self) -> usize {
        self.all_issues()
            .filter(|i| i.repair_kind.is_some())
            .count()
    }

    /// Per-category count of issues with [`Self::all_issues`]. Used
    /// by the OBS-06 categorised summary and the JSON `summary`
    /// object.
    pub fn count_by_category(&self, category: IssueCategory) -> usize {
        self.all_issues().filter(|i| i.category == category).count()
    }

    /// Per-category count of auto-fixable issues. Used by the D-CAT-3
    /// breakdown line and the JSON `summary.auto_fixable_by_category`
    /// map.
    pub fn auto_fixable_count_by_category(&self, category: IssueCategory) -> usize {
        self.all_issues()
            .filter(|i| i.category == category && i.repair_kind.is_some())
            .count()
    }
}

// -- Data gathering (pure computation, no I/O) --

/// Run all diagnostic checks and return a structured report.
pub fn check(config: &Config, paths: &TomePaths) -> Result<DoctorReport> {
    let configured = paths.library_dir().is_dir() || !config.directories.is_empty();

    if !configured {
        return Ok(DoctorReport {
            configured: false,
            library_issues: Vec::new(),
            directory_issues: Vec::new(),
            config_issues: Vec::new(),
            unowned_skills: Vec::new(),
        });
    }

    let library_issues = check_library(paths)?;

    let mut directory_issues = Vec::new();
    for (name, dir_config) in config.distribution_dirs() {
        let issues = check_distribution_dir(name.as_str(), &dir_config.path, paths.library_dir())?;
        directory_issues.push(DirectoryDiagnostic {
            name: name.as_str().to_string(),
            issues,
            override_applied: dir_config.override_applied,
        });
    }

    let config_issues = check_config(config)?;

    // UNOWN-03 / D-D3: collect Unowned skills from the manifest.
    // Manifest read errors degrade gracefully to an empty Vec — the
    // separate library_issues section reports the underlying read
    // failure if there is one (see `check_library`).
    let unowned_skills = match manifest::load(paths.config_dir()) {
        Ok(m) => m
            .iter()
            .filter(|(_, e)| e.source_name.is_none())
            .map(|(n, e)| crate::summary::SkillSummary::from_entry(n, e))
            .collect(),
        Err(_) => Vec::new(),
    };

    Ok(DoctorReport {
        configured: true,
        library_issues,
        directory_issues,
        config_issues,
        unowned_skills,
    })
}

// -- Rendering + control flow --

/// Diagnose and optionally repair issues.
pub fn diagnose(
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
    no_input: bool,
    json: bool,
) -> Result<()> {
    let report = check(config, paths)?;

    if json {
        // OBS-06: emit the report alongside a `summary` object that
        // exposes total + per-category + auto-fixable counts. Helper
        // builds a JSON `Value` so the per-issue `category` /
        // `repair_kind` fields (struct-derived) compose with the
        // computed summary in one document.
        let payload = serde_json::json!({
            "configured": report.configured,
            "library_issues": report.library_issues,
            "directory_issues": report.directory_issues,
            "config_issues": report.config_issues,
            "unowned_skills": report.unowned_skills,
            "summary": render_summary_json(&report),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if !report.configured {
        println!("Not configured yet. Run `tome init` to get started.");
        return Ok(());
    }

    if dry_run {
        eprintln!(
            "{}",
            style("[dry-run] No changes will be made").yellow().bold()
        );
    }

    // Render results
    println!("{}", style("Checking library...").bold());
    render_issues(&report.library_issues, "library");

    println!("{}", style("Checking directories...").bold());
    for d in &report.directory_issues {
        render_issues_for_directory(&d.name, &d.issues, d.override_applied);
    }

    println!("{}", style("Checking config...").bold());
    render_issues(&report.config_issues, "config");

    // UNOWN-03 / D-D3: parallel informational section. Does NOT affect
    // `total_issues` or `tome doctor` exit code. Section omits cleanly
    // when the Unowned set is empty.
    render_unowned_skills(&report.unowned_skills);

    let total = report.total_issues();
    let auto_fixable = report.auto_fixable_count();

    println!();
    if total == 0 {
        println!("{}", style("No issues found.").green().bold());
    } else {
        // D-CAT-3: render the summary line with per-category breakdown
        // of auto-fixable issues. Only categories with >0 auto-fixable
        // issues appear in the breakdown.
        println!("{}", render_summary_line(&report));

        let interactive = !no_input && std::io::stdin().is_terminal();

        if !dry_run && interactive {
            // Collect orphan-directory issues (interactive-only, no
            // repair_kind). Routed through the per-item Select prompt
            // below.
            let orphan_dirs: Vec<&DiagnosticIssue> = report
                .library_issues
                .iter()
                .filter(|i| i.repair_kind.is_none() && is_orphan_directory(i))
                .collect();

            // D-REPAIR-2: skip the global "Apply N auto-fixable
            // repairs?" prompt entirely when there is nothing to
            // auto-repair. The pre-FIX-01 code printed
            // "(no auto-repair available)" lines under a non-zero
            // count — gone. See GitHub #530.
            if auto_fixable > 0 {
                println!();
                println!("{} auto-fixable issue(s):", style(auto_fixable).bold());
                render_repair_plan_auto(&report);

                let confirmed = Confirm::new()
                    .with_prompt("Proceed with auto-repair?")
                    .default(true)
                    .interact()?;

                if confirmed {
                    println!();
                    dispatch_repairs(&report, config, paths)?;
                } else {
                    // D-REPAIR-3 / OBS-01-shaped tracing: user
                    // declined. Logged so `tome doctor --verbose`
                    // surfaces why repairs were skipped.
                    debug!(
                        target: "doctor::repair",
                        fixable = auto_fixable,
                        reason = "user_declined",
                        "skipped repair"
                    );
                }
            }

            // Handle orphan directories interactively — one at a time
            if !orphan_dirs.is_empty() {
                println!();
                println!(
                    "{} orphan director{} in library (on disk but not in manifest):",
                    style(orphan_dirs.len()).bold(),
                    if orphan_dirs.len() == 1 { "y" } else { "ies" }
                );
                for issue in &orphan_dirs {
                    // Extract path from message: "orphan directory: <path> (not in manifest)"
                    let path_str = issue
                        .message
                        .strip_prefix("orphan directory: ")
                        .and_then(|s| s.strip_suffix(" (not in manifest)"))
                        .unwrap_or(&issue.message);
                    println!("  {}", path_str);
                }
                println!();
                println!(
                    "  {} — hash the dir + register it in the manifest as Unowned (proper fix; \
                     v0.14+)",
                    style("claim").cyan(),
                );
                println!(
                    "  {} — run {} to re-register them in the manifest (only works if a configured \
                     source contains them)",
                    style("keep").cyan(),
                    style("tome sync").bold()
                );
                println!(
                    "  {} — delete from disk permanently",
                    style("delete").cyan()
                );
                println!("  {} — leave as-is for now", style("skip").cyan());

                for issue in &orphan_dirs {
                    let path_str = issue
                        .message
                        .strip_prefix("orphan directory: ")
                        .and_then(|s| s.strip_suffix(" (not in manifest)"))
                        .unwrap_or(&issue.message);

                    let items = [
                        "claim (register in manifest as Unowned)",
                        "keep (try to re-register on next sync)",
                        "delete from disk",
                        "skip",
                    ];
                    let selection = dialoguer::Select::new()
                        .with_prompt(path_str)
                        .items(items)
                        .default(3)
                        .interact()?;

                    match selection {
                        0 => {
                            // Phase 21 (v0.14): claim the orphan into the
                            // manifest as Unowned. Hashes the directory,
                            // writes a SkillEntry with source_name=None.
                            // Subsequent `tome sync` will distribute it to
                            // configured target dirs like any other Unowned
                            // skill (LIB-04 lifecycle). Closes the dead-end
                            // where "keep" was a no-op when no source could
                            // re-discover the orphan (v0.12 dogfooding).
                            let path = std::path::Path::new(path_str);
                            claim_orphan_directory(path, paths)?;
                        }
                        1 => {
                            println!(
                                "  {} Keeping — run {} to re-register",
                                style("ok").green(),
                                style("tome sync").bold()
                            );
                        }
                        2 => {
                            let path = std::path::Path::new(path_str);
                            if path.is_dir() {
                                std::fs::remove_dir_all(path).with_context(|| {
                                    format!("failed to delete {}", path.display())
                                })?;
                                println!("  {} Deleted {}", style("fixed").green(), path.display());
                            }
                        }
                        _ => {
                            println!("  {} Skipped", style("—").dim());
                        }
                    }
                }
            }
        } else if !dry_run {
            eprintln!("info: non-interactive mode — skipping repair prompt");
        } else {
            println!("  (dry run — no changes made)");
        }
    }

    Ok(())
}

/// Human-readable label for a category, used in the D-CAT-3 summary
/// breakdown (`"Foreign-symlink"` uses a hyphen even though JSON wire
/// form is `"foreign_symlink"`).
fn category_display_name(c: IssueCategory) -> &'static str {
    match c {
        IssueCategory::Library => "Library",
        IssueCategory::Directory => "Directory",
        IssueCategory::Config => "Config",
        IssueCategory::ForeignSymlink => "Foreign-symlink",
    }
}

/// Action description for a repair kind. Used by
/// `render_repair_plan_auto` so each auto-fixable issue gets a typed
/// description (no substring matching). New `RepairKind` variants get
/// a new arm here automatically via the exhaustive match.
fn repair_kind_action_label(k: RepairKind) -> &'static str {
    match k {
        RepairKind::RemoveStaleManifestEntry => {
            "will remove entry from manifest file (and broken symlink, if any)"
        }
        RepairKind::RemoveBrokenLibrarySymlink => "will delete broken symlink",
        RepairKind::RemoveStaleTargetSymlink => "will delete stale symlink from distribution dir",
    }
}

/// Identify orphan-directory issues for the interactive Select prompt.
///
/// Orphan directories live in `library_issues` with `repair_kind:
/// None` and a message prefix of `"orphan directory:"`. Carrying a
/// message-prefix check inside the orphan-only handler is acceptable
/// per the D-REPAIR-3 contract — the bug class #530 was about
/// substring matching at the DISPATCHER level (replaced above by
/// `repair_kind`-based discrimination). The orphan-only matcher
/// stays scoped to one render path.
fn is_orphan_directory(issue: &DiagnosticIssue) -> bool {
    issue.category == IssueCategory::Library
        && issue.repair_kind.is_none()
        && issue.message.starts_with("orphan directory:")
}

/// Claim an orphan library directory into the manifest as an Unowned skill
/// (Phase 21 / v0.14).
///
/// Closes the dead-end where a library entry existed on disk but had no
/// manifest registration — and the "keep" option's "run `tome sync` to
/// re-register" hint was misleading because sync can only re-register
/// orphans whose content gets re-discovered from a configured source.
/// Library-canonical orphans (no upstream source) had no path to recovery
/// in the CLI; the user had to hand-edit `.tome-manifest.json`.
///
/// What this does:
///
/// 1. Hash the directory contents via `manifest::hash_directory` (same
///    `ContentHash` that consolidate writes).
/// 2. Construct a `SkillEntry::new_unowned` (source_name: None,
///    previous_source: None — there is no prior source for a true
///    orphan).
/// 3. Insert into the manifest under the directory's basename (validated
///    as a `SkillName`).
/// 4. Save the manifest atomically.
///
/// On next `tome sync`:
/// - The skill stays in the library (Unowned content preserved per LIB-04).
/// - Distribute pushes symlinks to every configured `target` / `synced`
///   directory.
/// - `tome doctor` no longer flags it as orphan (it's now in the manifest).
fn claim_orphan_directory(path: &Path, paths: &TomePaths) -> Result<()> {
    let skill_name_str = path.file_name().and_then(|n| n.to_str()).with_context(|| {
        format!(
            "could not extract a skill name from path '{}'",
            path.display()
        )
    })?;

    let skill_name = crate::discover::SkillName::new(skill_name_str).with_context(|| {
        format!(
            "directory name '{skill_name_str}' is not a valid skill identifier \
             (must be non-empty, no path separators, no `.` or `..`)"
        )
    })?;

    let content_hash = manifest::hash_directory(path)
        .with_context(|| format!("failed to hash directory {}", path.display()))?;

    let mut man = manifest::load(paths.config_dir())?;
    if man.contains_key(skill_name.as_str()) {
        // Defensive: shouldn't happen because is_orphan_directory filters
        // exactly entries that are NOT in the manifest. But the check
        // makes the error mode explicit if a future refactor changes the
        // filter contract.
        anyhow::bail!(
            "skill '{}' is already in the manifest — refusing to clobber its entry",
            skill_name
        );
    }
    let entry = manifest::SkillEntry::new_unowned(
        path.to_path_buf(),
        content_hash,
        false, // managed: false (orphans have no upstream package manager)
        None,  // previous_source: None (true orphan, never owned)
    );
    man.insert(skill_name.clone(), entry);
    manifest::save(&man, paths.config_dir())
        .with_context(|| "failed to save manifest after claiming orphan")?;

    println!(
        "  {} Claimed {} into manifest as Unowned skill '{}'",
        style("fixed").green(),
        path.display(),
        style(skill_name.as_str()).cyan()
    );
    println!(
        "    {}",
        style("→ tome sync will now distribute it to your target directories").dim()
    );
    Ok(())
}

/// Build the OBS-06 `summary` JSON object exposed in
/// `tome doctor --json` output. Shape:
///
/// ```json
/// {
///   "total_issues": 5,
///   "by_category": { "library": 2, "directory": 1, "config": 1, "foreign_symlink": 1 },
///   "auto_fixable_count": 3,
///   "auto_fixable_by_category": { "library": 2, "directory": 1 }
/// }
/// ```
///
/// Every `IssueCategory` variant appears in `by_category` (zero
/// values included so consumers can iterate without per-variant
/// nil-checks). `auto_fixable_by_category` is sparse — only
/// categories with at least one auto-fixable issue.
fn render_summary_json(report: &DoctorReport) -> serde_json::Value {
    use serde_json::{Map, Value, json};

    let mut by_category = Map::new();
    let mut auto_fixable_by_category = Map::new();
    for c in IssueCategory::ALL {
        let n = report.count_by_category(c);
        // `IssueCategory` derives `Serialize` and renders as a JSON string —
        // any failure here is a programming error (e.g. a new variant added
        // without `#[serde(rename_all = "snake_case")]`), not a runtime
        // condition we should silently mask. Panicking with a clear message
        // beats emitting `"": <count>` and corrupting machine-readable output.
        let key = serde_json::to_value(c)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .expect("IssueCategory serializes to a JSON string");
        by_category.insert(key.clone(), Value::from(n));
        let nf = report.auto_fixable_count_by_category(c);
        if nf > 0 {
            auto_fixable_by_category.insert(key, Value::from(nf));
        }
    }

    json!({
        "total_issues": report.total_issues(),
        "by_category": Value::Object(by_category),
        "auto_fixable_count": report.auto_fixable_count(),
        "auto_fixable_by_category": Value::Object(auto_fixable_by_category),
    })
}

/// Render the D-CAT-3 summary line, e.g.:
///
/// ```text
/// Found 5 issue(s). (3 auto-fixable: Library 2, Foreign-symlink 1)
/// ```
///
/// Only categories with non-zero auto-fixable counts appear. When
/// `auto_fixable_count == 0`, the trailing parenthetical is omitted.
fn render_summary_line(report: &DoctorReport) -> String {
    let total = report.total_issues();
    let auto_fixable = report.auto_fixable_count();

    let head = style(format!("Found {total} issue(s).")).yellow().bold();

    if auto_fixable == 0 {
        return head.to_string();
    }

    let breakdown = IssueCategory::ALL
        .iter()
        .filter_map(|c| {
            let n = report.auto_fixable_count_by_category(*c);
            if n > 0 {
                Some(format!("{} {n}", category_display_name(*c)))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("{head} ({auto_fixable} auto-fixable: {breakdown})")
}

/// Dispatch auto-repairs via exhaustive match on `Option<RepairKind>`.
///
/// D-REPAIR-3: substring matching is gone. Adding a `RepairKind`
/// variant without an arm here is a compile-time error.
fn dispatch_repairs(report: &DoctorReport, config: &Config, paths: &TomePaths) -> Result<()> {
    // Track which kinds we've seen so we only call the
    // batch-repair helpers once per kind. The handlers operate over
    // the whole report (e.g. `repair_library` processes every stale
    // manifest entry in one pass) so we don't re-enter them per
    // issue.
    let mut ran_library_repair = false;
    let mut ran_target_cleanup = false;

    for issue in report.all_issues() {
        match issue.repair_kind {
            Some(RepairKind::RemoveStaleManifestEntry)
            | Some(RepairKind::RemoveBrokenLibrarySymlink) => {
                if !ran_library_repair {
                    repair_library(paths)?;
                    ran_library_repair = true;
                }
            }
            Some(RepairKind::RemoveStaleTargetSymlink) => {
                if !ran_target_cleanup {
                    for (name, dir_config) in config.distribution_dirs() {
                        let removed =
                            cleanup::cleanup_target(&dir_config.path, paths.library_dir(), false)?;
                        if removed > 0 {
                            println!(
                                "  {} Removed {} stale symlink(s) from {}",
                                style("fixed").green(),
                                removed,
                                name
                            );
                        }
                    }
                    ran_target_cleanup = true;
                }
            }
            None => {
                // Interactive-only or informational. The orphan-dir
                // and (still-present, deleted in Task 3) git-tracked
                // paths handle these elsewhere.
                debug!(
                    target: "doctor::repair",
                    category = ?issue.category,
                    reason = "no_repair_kind",
                    "skipped repair"
                );
            }
        }
    }

    Ok(())
}

/// Show auto-fixable repair actions. Each auto-fixable issue prints
/// its repair-kind action label (typed dispatch; no substring
/// matching). Non-auto-fixable issues are skipped (they're rendered
/// in interactive prompts elsewhere).
fn render_repair_plan_auto(report: &DoctorReport) {
    for issue in report.all_issues() {
        let Some(kind) = issue.repair_kind else {
            continue;
        };
        println!(
            "  → {} ({})",
            issue.message,
            style(repair_kind_action_label(kind)).cyan()
        );
    }
}

fn render_issues(issues: &[DiagnosticIssue], section: &str) {
    if issues.is_empty() {
        println!("  {} {} OK", style("ok").green(), section);
    } else {
        for issue in issues {
            let marker = match issue.severity {
                IssueSeverity::Error => style("x").red(),
                IssueSeverity::Warning => style("!").yellow(),
            };
            println!("  {} {}", marker, issue.message);
        }
    }
}

/// Format the directory header (name plus optional override marker) used by
/// `render_issues_for_directory`. Extracted as a helper so the override
/// annotation can be unit-tested without capturing stdout (PORT-05).
fn format_dir_diagnostic_header(name: &str, override_applied: bool) -> String {
    if override_applied {
        format!("{} {}", name, style("(override)").cyan())
    } else {
        name.to_string()
    }
}

fn render_issues_for_directory(name: &str, issues: &[DiagnosticIssue], override_applied: bool) {
    let display_name = format_dir_diagnostic_header(name, override_applied);
    if issues.is_empty() {
        println!("  {} {}: OK", style("ok").green(), display_name);
    } else {
        for issue in issues {
            let marker = match issue.severity {
                IssueSeverity::Error => style("x").red(),
                IssueSeverity::Warning => style("!").yellow(),
            };
            println!("  {} {}: {}", marker, display_name, issue.message);
        }
    }
}

/// Render the Unowned skills section (UNOWN-03 / D-D3 / D-D1).
///
/// INFORMATIONAL — this section is parallel to library/directory/config
/// issue sections. It does NOT contribute to `DoctorReport::total_issues`
/// and does NOT affect `tome doctor` exit code. Mirrors the column
/// layout used by `tome status` (D-D1: NAME / LAST-KNOWN SOURCE / SYNCED).
/// Section omits cleanly when the Unowned set is empty.
fn render_unowned_skills(unowned: &[crate::summary::SkillSummary]) {
    use tabled::settings::{Modify, Style, object::Rows};

    if unowned.is_empty() {
        return;
    }

    println!();
    println!("{} ({}):", style("Unowned skills").bold(), unowned.len());
    let mut rows: Vec<[String; 3]> = Vec::with_capacity(unowned.len() + 1);
    rows.push([
        "NAME".to_string(),
        "LAST-KNOWN SOURCE".to_string(),
        "SYNCED".to_string(),
    ]);
    for s in unowned {
        // D-D1: render previous_source when present (D-C1), fall back to
        // source_path_display (D-C2) for pre-Phase-14 Unowned entries.
        let last_known = s
            .previous_source
            .clone()
            .unwrap_or_else(|| s.source_path_display.clone());
        rows.push([s.name.clone(), last_known, s.synced_at.clone()]);
    }
    let table = tabled::Table::from_iter(rows)
        .with(Style::blank())
        .with(
            Modify::new(Rows::first()).with(tabled::settings::Format::content(|s| {
                style(s).bold().to_string()
            })),
        )
        .to_string();
    println!("{table}");
}

// -- Check functions (return structured data) --

fn check_library(paths: &TomePaths) -> Result<Vec<DiagnosticIssue>> {
    let library_dir = paths.library_dir();
    let config_dir = paths.config_dir();
    let mut issues = Vec::new();

    if !library_dir.is_dir() {
        issues.push(DiagnosticIssue::library(
            IssueSeverity::Warning,
            "library directory does not exist",
        ));
        return Ok(issues);
    }

    let m = match manifest::load(config_dir) {
        Ok(m) => m,
        Err(e) => {
            issues.push(DiagnosticIssue::library(
                IssueSeverity::Error,
                format!("manifest is corrupted or unreadable: {}", e),
            ));
            return Ok(issues);
        }
    };

    // Check manifest entries exist on disk
    for name in m.keys() {
        let entry_path = library_dir.join(name.as_str());
        if !entry_path.is_dir() {
            let entry = m.get(name.as_str());
            let is_managed = entry.is_some_and(|e| e.managed);
            if is_managed && entry_path.is_symlink() {
                // Broken managed symlink — same action as "missing
                // directory" (remove manifest entry + delete symlink),
                // so it shares the RemoveStaleManifestEntry handler.
                issues.push(DiagnosticIssue::library_repairable(
                    IssueSeverity::Error,
                    format!(
                        "managed skill '{}' has a broken symlink (source may have been uninstalled)",
                        name
                    ),
                    RepairKind::RemoveStaleManifestEntry,
                ));
            } else {
                issues.push(DiagnosticIssue::library_repairable(
                    IssueSeverity::Error,
                    format!("manifest entry '{}' has no directory on disk", name),
                    RepairKind::RemoveStaleManifestEntry,
                ));
            }
        }
    }

    // Check disk entries are in manifest (orphans)
    let entries = std::fs::read_dir(library_dir)
        .with_context(|| format!("failed to read library dir {}", library_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", library_dir.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() && !name.starts_with('.') && !m.contains_key(&name) {
            // Orphan directories are interactive-only — the user
            // decides keep/delete/skip per item. No `repair_kind` so
            // the global "Apply N auto-fixable repairs?" prompt does
            // not include orphan directories.
            issues.push(DiagnosticIssue::library(
                IssueSeverity::Warning,
                format!("orphan directory: {} (not in manifest)", path.display()),
            ));
        }

        // Check for broken symlinks — managed skill whose source was deleted, or orphan from a previous layout
        if path.is_symlink() && !path.exists() {
            let is_managed = m.get(&name).is_some_and(|e| e.managed);
            if !is_managed {
                let raw_target = std::fs::read_link(&path)
                    .with_context(|| format!("failed to read symlink {}", path.display()))?;
                issues.push(DiagnosticIssue::library_repairable(
                    IssueSeverity::Error,
                    format!(
                        "broken legacy symlink: {} -> {}",
                        path.display(),
                        raw_target.display()
                    ),
                    RepairKind::RemoveBrokenLibrarySymlink,
                ));
            }
        }
    }

    // FIX-03 (#532): v0.10 made managed skills real directory copies,
    // so the pre-v0.10 git-tracking detection check is obsolete
    // (managed skills cannot be symlinks any more — the detection
    // criterion can never fire on a clean v0.10 library). The check,
    // its render/Confirm flow, and the supporting git-shellout
    // helper are deleted entirely. If a real failure mode emerges, a
    // new ticket will scope it.

    Ok(issues)
}

fn check_distribution_dir(
    _name: &str,
    skills_dir: &Path,
    library_dir: &Path,
) -> Result<Vec<DiagnosticIssue>> {
    let mut issues = Vec::new();

    if !skills_dir.is_dir() {
        issues.push(DiagnosticIssue::directory(
            IssueSeverity::Warning,
            format!("directory path does not exist ({})", skills_dir.display()),
        ));
        return Ok(issues);
    }

    // Canonicalize library_dir so starts_with works when library_dir contains
    // a symlink component (e.g., /var -> /private/var on macOS).
    let canonical_library = std::fs::canonicalize(library_dir).unwrap_or_else(|e| {
        eprintln!(
            "warning: could not canonicalize library path {}: {}",
            library_dir.display(),
            e
        );
        library_dir.to_path_buf()
    });

    let entries = std::fs::read_dir(skills_dir)
        .with_context(|| format!("failed to read target dir {}", skills_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", skills_dir.display()))?;
        let path = entry.path();

        if path.is_symlink() {
            let raw_target = std::fs::read_link(&path)
                .with_context(|| format!("failed to read symlink {}", path.display()))?;
            let target = resolve_symlink_target(&path, &raw_target);
            let points_into_library =
                target.starts_with(library_dir) || target.starts_with(&canonical_library);
            if points_into_library && !target.exists() {
                issues.push(DiagnosticIssue::directory_repairable(
                    IssueSeverity::Error,
                    format!("stale symlink {}", path.display()),
                    RepairKind::RemoveStaleTargetSymlink,
                ));
            }
            // HARD-09 / D-DIST-2: surface foreign symlinks so they show
            // up in `tome doctor` even when the user hasn't run `sync`
            // recently. Reuses the canonical-path predicate from
            // `crate::distribute::is_foreign_symlink` so detection
            // semantics stay in lockstep across the two emit sites.
            // Renders as Warning per D-DIST-2; contributes to
            // `total_issues` via the existing summing logic.
            // Category is promoted to ForeignSymlink (D-CAT-1).
            if crate::distribute::is_foreign_symlink(&path, library_dir) {
                issues.push(DiagnosticIssue::directory_foreign_symlink(
                    IssueSeverity::Warning,
                    format!(
                        "foreign symlink: {} -> {} (points outside library_dir; tome will skip on sync unless --force)",
                        path.display(),
                        raw_target.display(),
                    ),
                ));
            }
        }
    }

    Ok(issues)
}

fn check_config(config: &Config) -> Result<Vec<DiagnosticIssue>> {
    let mut issues = Vec::new();

    for (name, dir_config) in &config.directories {
        if !dir_config.path.exists() {
            issues.push(DiagnosticIssue::config(
                IssueSeverity::Warning,
                format!(
                    "directory '{}' path does not exist: {}",
                    name,
                    dir_config.path.display()
                ),
            ));
        }
    }

    Ok(issues)
}

/// Repair library issues: remove orphan manifest entries and broken symlinks.
fn repair_library(paths: &TomePaths) -> Result<()> {
    let library_dir = paths.library_dir();
    let config_dir = paths.config_dir();
    let mut m = manifest::load(config_dir).with_context(|| {
        format!(
            "cannot repair: manifest is unreadable. Back up {} and run sync --force",
            crate::manifest::MANIFEST_FILENAME
        )
    })?;
    let mut fixed = 0;

    // Remove manifest entries missing from disk (includes managed broken symlinks)
    let missing: Vec<String> = m
        .keys()
        .filter(|name| !library_dir.join(name.as_str()).is_dir())
        .map(|name| name.as_str().to_string())
        .collect();
    for name in &missing {
        let entry_path = library_dir.join(name.as_str());
        // Clean up broken managed symlinks
        if entry_path.is_symlink() {
            std::fs::remove_file(&entry_path).with_context(|| {
                format!("failed to remove broken symlink {}", entry_path.display())
            })?;
        }
        m.remove(name);
        println!(
            "  {} Removed manifest entry '{}' (directory missing)",
            style("fixed").green(),
            name
        );
        fixed += 1;
    }

    // Remove broken legacy symlinks (not in manifest)
    let entries = std::fs::read_dir(library_dir)
        .with_context(|| format!("failed to read library dir {}", library_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", library_dir.display()))?;
        let path = entry.path();

        if path.is_symlink() && !path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove broken symlink {}", path.display()))?;
            println!(
                "  {} Removed broken symlink {}",
                style("fixed").green(),
                path.display()
            );
            fixed += 1;
        }
    }

    if fixed > 0 {
        manifest::save(&m, config_dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};
    use std::collections::BTreeMap;
    use std::os::unix::fs as unix_fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // -- check() tests --

    #[test]
    fn check_unconfigured_returns_not_configured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/library"),
            ..Config::default()
        };

        let tmp = TempDir::new().unwrap();
        let report = check(
            &config,
            &TomePaths::new(tmp.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(!report.configured);
        assert_eq!(report.total_issues(), 0);
    }

    #[test]
    fn check_healthy_library_returns_no_issues() {
        let lib = TempDir::new().unwrap();
        let skill_dir = lib.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/my-skill"),
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        let config = Config {
            library_dir: lib.path().to_path_buf(),
            ..Config::default()
        };

        let report = check(
            &config,
            &TomePaths::new(lib.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(report.configured);
        assert_eq!(report.total_issues(), 0);
    }

    #[test]
    fn check_detects_orphan_directory() {
        let lib = TempDir::new().unwrap();
        std::fs::create_dir_all(lib.path().join("orphan")).unwrap();

        let config = Config {
            library_dir: lib.path().to_path_buf(),
            ..Config::default()
        };

        let report = check(
            &config,
            &TomePaths::new(lib.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.library_issues.len(), 1);
        assert_eq!(report.library_issues[0].severity, IssueSeverity::Warning);
        assert!(report.library_issues[0].message.contains("orphan"));
    }

    #[test]
    fn check_detects_missing_directory_path() {
        let lib = TempDir::new().unwrap();

        let config = Config {
            library_dir: lib.path().to_path_buf(),
            directories: BTreeMap::from([(
                DirectoryName::new("gone").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/nonexistent/source"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    git_ref: None,

                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Config::default()
        };

        let report = check(
            &config,
            &TomePaths::new(lib.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.config_issues.len(), 1);
        assert!(report.config_issues[0].message.contains("gone"));
    }

    // -- check_library --

    #[test]
    fn check_library_missing_dir() {
        let tmp = TempDir::new().unwrap();
        let result = check_library(
            &TomePaths::new(
                tmp.path().to_path_buf(),
                Path::new("/nonexistent/library").to_path_buf(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn check_library_no_issues() {
        let lib = TempDir::new().unwrap();
        let skill_dir = lib.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/my-skill"),
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        let result = check_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn check_library_missing_manifest_entry() {
        let lib = TempDir::new().unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("gone").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/gone"),
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        let result = check_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, IssueSeverity::Error);
    }

    #[test]
    fn check_library_orphan_directory() {
        let lib = TempDir::new().unwrap();
        std::fs::create_dir_all(lib.path().join("orphan")).unwrap();

        let result = check_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn check_library_broken_legacy_symlink() {
        let lib = TempDir::new().unwrap();
        unix_fs::symlink("/nonexistent/target", lib.path().join("broken")).unwrap();

        let result = check_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, IssueSeverity::Error);
    }

    // -- check_distribution_dir --

    #[test]
    fn check_distribution_dir_missing_dir() {
        let lib = TempDir::new().unwrap();
        let result =
            check_distribution_dir("test-dir", Path::new("/nonexistent/dir"), lib.path()).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn check_distribution_dir_stale_symlink() {
        let lib = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        let stale_target = lib.path().join("deleted-skill");
        unix_fs::symlink(&stale_target, target_dir.path().join("skill-link")).unwrap();

        let result = check_distribution_dir("test", target_dir.path(), lib.path()).unwrap();
        assert_eq!(result.len(), 1);
    }

    /// HARD-09 / D-DIST-2 BEHAVIOUR CHANGE: external (foreign) symlinks
    /// in distribution directories now surface as ForeignSymlink
    /// Warnings instead of being silently ignored. The pre-HARD-09
    /// "silent ignore" assertion is replaced with a typed-issue
    /// assertion so the new contract is pinned.
    #[test]
    fn check_distribution_dir_surfaces_external_symlinks_as_foreign() {
        let lib = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        unix_fs::symlink("/some/other/place", target_dir.path().join("external")).unwrap();

        let result = check_distribution_dir("test", target_dir.path(), lib.path()).unwrap();
        let foreign: Vec<_> = result
            .iter()
            .filter(|i| i.kind == Some(DiagnosticIssueKind::ForeignSymlink))
            .collect();
        assert_eq!(
            foreign.len(),
            1,
            "external symlink must surface as one ForeignSymlink Warning, got: {result:?}"
        );
        assert_eq!(foreign[0].severity, IssueSeverity::Warning);
    }

    // -- check_config --

    #[test]
    fn check_config_missing_directory() {
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("gone").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/nonexistent/source"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    git_ref: None,

                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Config::default()
        };

        let result = check_config(&config).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn check_config_valid_directories() {
        let source_dir = TempDir::new().unwrap();
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("real").unwrap(),
                DirectoryConfig {
                    path: source_dir.path().to_path_buf(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    git_ref: None,

                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Config::default()
        };

        let result = check_config(&config).unwrap();
        assert!(result.is_empty());
    }

    // -- diagnose (pre-init guard) --

    #[test]
    fn diagnose_shows_init_prompt_when_unconfigured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/library"),
            ..Config::default()
        };

        let tmp = TempDir::new().unwrap();
        let result = diagnose(
            &config,
            &TomePaths::new(tmp.path().to_path_buf(), config.library_dir.clone()).unwrap(),
            true,
            true,
            false,
        );
        assert!(result.is_ok());
    }

    // -- repair_library --

    #[test]
    fn check_library_uses_tome_home_for_manifest() {
        let tome_home = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        // Create a skill directory in the library
        let skill_dir = library.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        // Save manifest at tome_home (not library_dir)
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/my-skill"),
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, tome_home.path()).unwrap();

        let issues = check_library(
            &TomePaths::new(tome_home.path().to_path_buf(), library.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert!(
            issues.is_empty(),
            "should find no issues when manifest is at tome_home and skill exists in library"
        );

        let issues = check_library(
            &TomePaths::new(library.path().to_path_buf(), library.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            issues.len(),
            1,
            "should detect orphan when manifest is not at the given tome_home"
        );
    }

    #[test]
    fn repair_library_uses_tome_home_for_manifest() {
        let tome_home = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("orphan-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/orphan-skill"),
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, tome_home.path()).unwrap();

        repair_library(
            &TomePaths::new(tome_home.path().to_path_buf(), library.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        let after = manifest::load(tome_home.path()).unwrap();
        assert!(
            !after.contains_key("orphan-skill"),
            "repair should remove orphan manifest entry when using separate tome_home"
        );
    }

    #[test]
    fn repair_library_removes_orphan_manifest_entry() {
        let lib = TempDir::new().unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("ghost").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/ghost"),
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        repair_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        let after = manifest::load(lib.path()).unwrap();
        assert!(
            !after.contains_key("ghost"),
            "repair should remove manifest entry without directory"
        );
    }

    #[test]
    fn repair_library_removes_broken_managed_symlink() {
        let lib = TempDir::new().unwrap();

        unix_fs::symlink("/nonexistent/source", lib.path().join("broken-plugin")).unwrap();
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("broken-plugin").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/nonexistent/source"),
                source_name: Some(DirectoryName::new("plugins").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: true,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        repair_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        assert!(
            !lib.path().join("broken-plugin").exists(),
            "broken managed symlink should be removed"
        );
        let after = manifest::load(lib.path()).unwrap();
        assert!(!after.contains_key("broken-plugin"));
    }

    #[test]
    fn repair_library_removes_broken_legacy_symlink() {
        let lib = TempDir::new().unwrap();

        unix_fs::symlink("/nonexistent/v01/skill", lib.path().join("legacy")).unwrap();

        repair_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        assert!(
            !lib.path().join("legacy").exists(),
            "broken legacy symlink should be removed"
        );
    }

    // -- PORT-05: override_applied surfacing --

    #[test]
    fn check_with_no_overrides_sets_flags_false() {
        let lib = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();
        let config = Config {
            library_dir: lib.path().to_path_buf(),
            directories: BTreeMap::from([(
                DirectoryName::new("plain").unwrap(),
                DirectoryConfig {
                    path: target.path().to_path_buf(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Target),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Config::default()
        };
        let report = check(
            &config,
            &TomePaths::new(lib.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.directory_issues.len(), 1);
        assert!(
            !report.directory_issues[0].override_applied,
            "override_applied should default to false"
        );
        assert_eq!(report.directory_issues[0].name, "plain");
    }

    #[test]
    fn check_with_override_applied_sets_flag_true() {
        let lib = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();
        let config = Config {
            library_dir: lib.path().to_path_buf(),
            directories: BTreeMap::from([(
                DirectoryName::new("work").unwrap(),
                DirectoryConfig {
                    path: target.path().to_path_buf(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Target),
                    git_ref: None,
                    subdir: None,
                    override_applied: true,
                },
            )]),
            ..Config::default()
        };
        let report = check(
            &config,
            &TomePaths::new(lib.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.directory_issues.len(), 1);
        assert_eq!(report.directory_issues[0].name, "work");
        assert!(
            report.directory_issues[0].override_applied,
            "override_applied should be true when the config flag is set"
        );
    }

    #[test]
    fn render_issues_for_directory_appends_override_marker_when_set() {
        let s = format_dir_diagnostic_header("work", true);
        assert!(s.contains("work"), "name missing: {s}");
        assert!(s.contains("(override)"), "override marker missing: {s}");
    }

    #[test]
    fn render_issues_for_directory_omits_marker_when_unset() {
        let s = format_dir_diagnostic_header("work", false);
        assert!(s.contains("work"), "name missing: {s}");
        assert!(
            !s.contains("(override)"),
            "override marker should NOT appear when flag is false: {s}"
        );
    }

    #[test]
    fn doctor_json_includes_override_applied_per_directory() {
        let dd = DirectoryDiagnostic {
            name: "work".to_string(),
            issues: Vec::new(),
            override_applied: true,
        };
        let json = serde_json::to_string(&dd).unwrap();
        assert!(
            json.contains("\"override_applied\":true"),
            "JSON output should include override_applied field, got: {json}"
        );
        assert!(
            json.contains("\"name\":\"work\""),
            "JSON output should include name field, got: {json}"
        );
    }

    #[test]
    fn total_issues_unchanged_by_directory_diagnostic_shape() {
        let report = DoctorReport {
            configured: true,
            library_issues: vec![DiagnosticIssue::library(IssueSeverity::Warning, "lib")],
            directory_issues: vec![
                DirectoryDiagnostic {
                    name: "a".to_string(),
                    issues: vec![DiagnosticIssue::directory(IssueSeverity::Error, "x")],
                    override_applied: true,
                },
                DirectoryDiagnostic {
                    name: "b".to_string(),
                    issues: vec![
                        DiagnosticIssue::directory(IssueSeverity::Error, "y"),
                        DiagnosticIssue::directory(IssueSeverity::Warning, "z"),
                    ],
                    override_applied: false,
                },
            ],
            config_issues: vec![DiagnosticIssue::config(IssueSeverity::Warning, "cfg")],
            unowned_skills: Vec::new(),
        };
        // 1 (lib) + 1 (a) + 2 (b) + 1 (cfg) = 5
        assert_eq!(report.total_issues(), 5);
    }

    #[test]
    fn repair_library_healthy_is_noop() {
        let lib = TempDir::new().unwrap();
        let skill_dir = lib.path().join("healthy-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("healthy-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/healthy-skill"),
                source_name: Some(DirectoryName::new("test").unwrap()),
                previous_source: None,
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        repair_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        let after = manifest::load(lib.path()).unwrap();
        assert!(after.contains_key("healthy-skill"));
        assert!(skill_dir.exists());
    }

    // -- UNOWN-03 / D-D3: unowned_skills section --

    /// Build a tome_home directory with a manifest containing the given
    /// (name, source_name) pairs. `source_name = None` produces an
    /// Unowned entry with `previous_source = Some("removed")`.
    fn write_manifest_with(entries: Vec<(&str, Option<&str>)>) -> TempDir {
        let tome_home = TempDir::new().unwrap();
        let library = tome_home.path().join("library");
        std::fs::create_dir_all(&library).unwrap();
        let mut m = manifest::Manifest::default();
        for (name, source_opt) in entries {
            std::fs::create_dir_all(library.join(name)).unwrap();
            let entry = match source_opt {
                Some(src) => manifest::SkillEntry::new(
                    PathBuf::from(format!("/tmp/src/{name}")),
                    DirectoryName::new(src).unwrap(),
                    crate::validation::test_hash(name),
                    false,
                ),
                None => manifest::SkillEntry::new_unowned(
                    PathBuf::from(format!("/tmp/old/{name}")),
                    crate::validation::test_hash(name),
                    false,
                    Some(DirectoryName::new("removed").unwrap()),
                ),
            };
            m.insert(crate::discover::SkillName::new(name).unwrap(), entry);
        }
        manifest::save(&m, tome_home.path()).unwrap();
        tome_home
    }

    #[test]
    fn check_populates_unowned_skills() {
        let tome_home = write_manifest_with(vec![("kept", Some("active")), ("orphan", None)]);
        let library = tome_home.path().join("library");
        let config = Config {
            library_dir: library.clone(),
            ..Config::default()
        };
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

        let report = check(&config, &paths).unwrap();
        assert_eq!(report.unowned_skills.len(), 1);
        assert_eq!(report.unowned_skills[0].name, "orphan");
        assert_eq!(
            report.unowned_skills[0].previous_source,
            Some("removed".to_string()),
            "previous_source must be surfaced via SkillSummary projection"
        );
    }

    #[test]
    fn unowned_skills_do_not_contribute_to_total_issues() {
        let tome_home = write_manifest_with(vec![("orphan-1", None), ("orphan-2", None)]);
        let library = tome_home.path().join("library");
        let config = Config {
            library_dir: library.clone(),
            ..Config::default()
        };
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

        let report = check(&config, &paths).unwrap();
        assert_eq!(report.unowned_skills.len(), 2, "fixture sanity");
        assert_eq!(
            report.total_issues(),
            0,
            "unowned skills must NOT contribute to total_issues per D-D3"
        );
    }

    #[test]
    fn check_empty_unowned_skills_when_all_owned() {
        let tome_home = write_manifest_with(vec![("kept", Some("active"))]);
        let library = tome_home.path().join("library");
        let config = Config {
            library_dir: library.clone(),
            ..Config::default()
        };
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

        let report = check(&config, &paths).unwrap();
        assert!(
            report.unowned_skills.is_empty(),
            "no Unowned entries in manifest → empty unowned_skills"
        );
    }

    #[test]
    fn json_doctor_always_includes_unowned_skills_field() {
        // Stable JSON shape: the key must be present even when the
        // Unowned set is empty (no skip_serializing_if). Programmatic
        // consumers can rely on the field existing.
        let report = DoctorReport {
            configured: false,
            library_issues: Vec::new(),
            directory_issues: Vec::new(),
            config_issues: Vec::new(),
            unowned_skills: Vec::new(),
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(
            json.contains("\"unowned_skills\""),
            "JSON must include 'unowned_skills' key for stable shape: {json}"
        );
    }

    // -- OBS-06 / D-CAT-1: IssueCategory enum --

    #[test]
    fn issue_category_all_len_4() {
        // POLISH-04 ALL-array contract: every variant enumerated.
        assert_eq!(IssueCategory::ALL.len(), 4);
        assert!(IssueCategory::ALL.contains(&IssueCategory::Library));
        assert!(IssueCategory::ALL.contains(&IssueCategory::Directory));
        assert!(IssueCategory::ALL.contains(&IssueCategory::Config));
        assert!(IssueCategory::ALL.contains(&IssueCategory::ForeignSymlink));
    }

    #[test]
    fn issue_category_serializes_snake_case() {
        // JSON wire-form: snake_case matches project convention
        // (override_applied, skill_count, source_path, etc).
        assert_eq!(
            serde_json::to_string(&IssueCategory::Library).unwrap(),
            "\"library\""
        );
        assert_eq!(
            serde_json::to_string(&IssueCategory::Directory).unwrap(),
            "\"directory\""
        );
        assert_eq!(
            serde_json::to_string(&IssueCategory::Config).unwrap(),
            "\"config\""
        );
        assert_eq!(
            serde_json::to_string(&IssueCategory::ForeignSymlink).unwrap(),
            "\"foreign_symlink\""
        );
    }

    // -- FIX-01 / D-REPAIR-1: RepairKind enum --

    #[test]
    fn repair_kind_all_len_3() {
        // POLISH-04 ALL-array contract: every variant enumerated.
        assert_eq!(RepairKind::ALL.len(), 3);
        assert!(RepairKind::ALL.contains(&RepairKind::RemoveStaleManifestEntry));
        assert!(RepairKind::ALL.contains(&RepairKind::RemoveBrokenLibrarySymlink));
        assert!(RepairKind::ALL.contains(&RepairKind::RemoveStaleTargetSymlink));
    }

    #[test]
    fn repair_kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&RepairKind::RemoveStaleManifestEntry).unwrap(),
            "\"remove_stale_manifest_entry\""
        );
        assert_eq!(
            serde_json::to_string(&RepairKind::RemoveBrokenLibrarySymlink).unwrap(),
            "\"remove_broken_library_symlink\""
        );
        assert_eq!(
            serde_json::to_string(&RepairKind::RemoveStaleTargetSymlink).unwrap(),
            "\"remove_stale_target_symlink\""
        );
    }

    // -- D-CAT-2: category-count invariant --

    #[test]
    fn category_counts_sum_to_total_issues() {
        // D-CAT-2: every DiagnosticIssue belongs to exactly one
        // category. Sum of per-category counts MUST equal
        // report.total_issues(). The ForeignSymlink issue (which
        // lives in directory_issues by container) counts ONLY in
        // the ForeignSymlink bucket — the promotion in D-CAT-1
        // shifts it out of Directory.
        let report = DoctorReport {
            configured: true,
            library_issues: vec![DiagnosticIssue::library_repairable(
                IssueSeverity::Error,
                "lib repairable",
                RepairKind::RemoveStaleManifestEntry,
            )],
            directory_issues: vec![DirectoryDiagnostic {
                name: "claude".to_string(),
                issues: vec![
                    DiagnosticIssue::directory_repairable(
                        IssueSeverity::Error,
                        "dir repairable",
                        RepairKind::RemoveStaleTargetSymlink,
                    ),
                    DiagnosticIssue::directory_foreign_symlink(
                        IssueSeverity::Warning,
                        "foreign symlink",
                    ),
                ],
                override_applied: false,
            }],
            config_issues: vec![DiagnosticIssue::config(IssueSeverity::Warning, "cfg")],
            unowned_skills: Vec::new(),
        };

        let total = report.total_issues();
        let sum: usize = IssueCategory::ALL
            .iter()
            .map(|c| report.count_by_category(*c))
            .sum();
        assert_eq!(sum, total, "category counts must sum to total_issues");
        assert_eq!(total, 4);

        // ForeignSymlink bucket contains the foreign symlink and only
        // the foreign symlink.
        assert_eq!(report.count_by_category(IssueCategory::ForeignSymlink), 1);
        // Directory bucket holds the directory_repairable but NOT the
        // foreign symlink (promoted).
        assert_eq!(report.count_by_category(IssueCategory::Directory), 1);
        assert_eq!(report.count_by_category(IssueCategory::Library), 1);
        assert_eq!(report.count_by_category(IssueCategory::Config), 1);
    }

    // -- D-REPAIR-2: zero-prompt skip --

    #[test]
    fn auto_fixable_count_is_zero_when_no_repair_kind() {
        // D-REPAIR-2: when the report has issues but none carry a
        // repair_kind, the dispatcher's global "Apply N auto-fixable
        // repairs?" prompt is skipped. The easiest contract pin is
        // auto_fixable_count == 0 in this state. (#530 — the
        // pre-FIX-01 code printed
        // "(no auto-repair available)" lines under a non-zero
        // count; that contradiction is fixed by skipping the prompt
        // at zero entirely.)
        let report = DoctorReport {
            configured: true,
            library_issues: vec![DiagnosticIssue::library(
                IssueSeverity::Warning,
                "orphan directory: /tmp/foo (not in manifest)",
            )],
            directory_issues: Vec::new(),
            config_issues: vec![DiagnosticIssue::config(
                IssueSeverity::Warning,
                "directory 'x' path does not exist",
            )],
            unowned_skills: Vec::new(),
        };
        assert!(report.total_issues() > 0, "fixture sanity");
        assert_eq!(report.auto_fixable_count(), 0);
    }

    #[test]
    fn auto_fixable_count_matches_repairable_issues() {
        let report = DoctorReport {
            configured: true,
            library_issues: vec![
                DiagnosticIssue::library_repairable(
                    IssueSeverity::Error,
                    "stale a",
                    RepairKind::RemoveStaleManifestEntry,
                ),
                DiagnosticIssue::library(IssueSeverity::Warning, "orphan directory: /tmp/x"),
            ],
            directory_issues: vec![DirectoryDiagnostic {
                name: "claude".to_string(),
                issues: vec![DiagnosticIssue::directory_repairable(
                    IssueSeverity::Error,
                    "stale symlink /tmp/x",
                    RepairKind::RemoveStaleTargetSymlink,
                )],
                override_applied: false,
            }],
            config_issues: Vec::new(),
            unowned_skills: Vec::new(),
        };
        assert_eq!(report.auto_fixable_count(), 2);
    }

    // -- D-CAT-3: summary breakdown rendering --

    #[test]
    fn summary_line_omits_breakdown_when_no_auto_fixable() {
        let report = DoctorReport {
            configured: true,
            library_issues: vec![DiagnosticIssue::library(
                IssueSeverity::Warning,
                "orphan directory: /tmp/foo",
            )],
            directory_issues: Vec::new(),
            config_issues: Vec::new(),
            unowned_skills: Vec::new(),
        };
        let line = render_summary_line(&report);
        assert!(line.contains("Found 1 issue(s)."), "{line}");
        assert!(
            !line.contains("auto-fixable"),
            "no auto-fixable issues → no breakdown: {line}"
        );
    }

    #[test]
    fn summary_line_renders_per_category_breakdown() {
        let report = DoctorReport {
            configured: true,
            library_issues: vec![
                DiagnosticIssue::library_repairable(
                    IssueSeverity::Error,
                    "a",
                    RepairKind::RemoveStaleManifestEntry,
                ),
                DiagnosticIssue::library_repairable(
                    IssueSeverity::Error,
                    "b",
                    RepairKind::RemoveBrokenLibrarySymlink,
                ),
            ],
            directory_issues: vec![DirectoryDiagnostic {
                name: "claude".to_string(),
                issues: vec![DiagnosticIssue::directory_foreign_symlink(
                    IssueSeverity::Warning,
                    "foreign",
                )],
                override_applied: false,
            }],
            config_issues: Vec::new(),
            unowned_skills: Vec::new(),
        };
        let line = render_summary_line(&report);
        // Only categories with non-zero auto-fixable counts appear.
        // Library has 2 auto-fixable; ForeignSymlink has 0 (foreign
        // symlinks aren't auto-repairable). Directory has 0.
        assert!(line.contains("Library 2"), "missing 'Library 2': {line}");
        assert!(
            !line.contains("Foreign-symlink"),
            "ForeignSymlink not auto-fixable, must be omitted from breakdown: {line}"
        );
        assert!(
            line.contains("(2 auto-fixable"),
            "auto_fixable_count must equal 2: {line}"
        );
    }

    #[test]
    fn summary_json_includes_categories_and_auto_fixable_breakdown() {
        let report = DoctorReport {
            configured: true,
            library_issues: vec![DiagnosticIssue::library_repairable(
                IssueSeverity::Error,
                "x",
                RepairKind::RemoveStaleManifestEntry,
            )],
            directory_issues: vec![DirectoryDiagnostic {
                name: "claude".to_string(),
                issues: vec![DiagnosticIssue::directory_foreign_symlink(
                    IssueSeverity::Warning,
                    "foreign",
                )],
                override_applied: false,
            }],
            config_issues: Vec::new(),
            unowned_skills: Vec::new(),
        };
        let summary = render_summary_json(&report);
        assert_eq!(summary["total_issues"], 2);
        assert_eq!(summary["auto_fixable_count"], 1);
        assert_eq!(summary["by_category"]["library"], 1);
        assert_eq!(summary["by_category"]["foreign_symlink"], 1);
        assert_eq!(summary["by_category"]["directory"], 0);
        assert_eq!(summary["by_category"]["config"], 0);
        // Sparse map: only categories with auto-fixable > 0 appear.
        assert_eq!(summary["auto_fixable_by_category"]["library"], 1);
        assert!(
            summary["auto_fixable_by_category"]
                .get("foreign_symlink")
                .is_none(),
            "foreign_symlink has zero auto-fixable; must be absent from sparse map"
        );
    }

    // -- D-CAT-1: per-issue JSON category field --

    #[test]
    fn diagnostic_issue_serialises_category_in_json() {
        let issue = DiagnosticIssue::library(IssueSeverity::Warning, "x");
        let json = serde_json::to_string(&issue).unwrap();
        assert!(
            json.contains("\"category\":\"library\""),
            "per-issue category must always be present: {json}"
        );
    }

    #[test]
    fn foreign_symlink_issue_has_promoted_category() {
        let issue = DiagnosticIssue::directory_foreign_symlink(IssueSeverity::Warning, "x");
        assert_eq!(issue.category, IssueCategory::ForeignSymlink);
        let json = serde_json::to_string(&issue).unwrap();
        assert!(json.contains("\"category\":\"foreign_symlink\""), "{json}");
    }

    // -- HARD-09 / D-DIST-2: DiagnosticIssueKind::ForeignSymlink --

    #[test]
    fn diagnostic_issue_kind_all_contains_foreign_symlink() {
        // POLISH-04 ALL-array contract: ForeignSymlink is enumerated
        // exactly once.
        assert_eq!(DiagnosticIssueKind::ALL.len(), 1);
        assert!(DiagnosticIssueKind::ALL.contains(&DiagnosticIssueKind::ForeignSymlink));
    }

    #[test]
    fn foreign_symlink_renders_as_warning_severity() {
        // D-DIST-2: the ForeignSymlink variant always emits as Warning
        // (NOT Error) — the user has a healthy alternative tome install
        // sharing the directory; this is informational, not a fault.
        let issue = DiagnosticIssue::directory_foreign_symlink(
            IssueSeverity::Warning,
            "foreign symlink: ~/.claude/skills/foo -> /other/library/foo",
        );
        assert_eq!(issue.severity, IssueSeverity::Warning);
        assert_eq!(issue.kind, Some(DiagnosticIssueKind::ForeignSymlink));
        assert_eq!(issue.category, IssueCategory::ForeignSymlink);
    }

    #[test]
    fn foreign_symlink_contributes_to_total_issues() {
        // D-DIST-2: ForeignSymlink contributes to total_issues via the
        // existing summing logic (no separate accounting). One per
        // affected directory entry.
        let report = DoctorReport {
            configured: true,
            library_issues: Vec::new(),
            directory_issues: vec![DirectoryDiagnostic {
                name: "claude".to_string(),
                issues: vec![DiagnosticIssue::directory_foreign_symlink(
                    IssueSeverity::Warning,
                    "foreign symlink",
                )],
                override_applied: false,
            }],
            config_issues: Vec::new(),
            unowned_skills: Vec::new(),
        };
        assert_eq!(report.total_issues(), 1);
    }

    #[test]
    fn foreign_symlink_serialises_kind_in_json() {
        // JSON shape: typed `kind` field appears for ForeignSymlink
        // emissions; absent for untyped category constructors.
        let typed = DiagnosticIssue::directory_foreign_symlink(IssueSeverity::Warning, "msg");
        let json = serde_json::to_string(&typed).unwrap();
        assert!(
            json.contains("\"kind\":\"ForeignSymlink\""),
            "typed issue must serialise kind: {json}"
        );

        let untyped = DiagnosticIssue::library(IssueSeverity::Warning, "msg");
        let json = serde_json::to_string(&untyped).unwrap();
        assert!(
            !json.contains("\"kind\""),
            "untyped issue must omit kind via skip_serializing_if: {json}"
        );
    }

    #[test]
    fn check_distribution_dir_surfaces_foreign_symlink() {
        // End-to-end: stage a foreign symlink under a distribution dir,
        // run check_distribution_dir, assert one ForeignSymlink issue.
        let tmp = TempDir::new().unwrap();
        let library = tmp.path().join("library");
        std::fs::create_dir_all(&library).unwrap();
        let dist = tmp.path().join("dist");
        std::fs::create_dir_all(&dist).unwrap();
        let other_library = tmp.path().join("other-library");
        std::fs::create_dir_all(&other_library).unwrap();
        let foreign_target = other_library.join("foo");
        std::fs::create_dir_all(&foreign_target).unwrap();
        std::os::unix::fs::symlink(&foreign_target, dist.join("foo")).unwrap();

        let issues = super::check_distribution_dir("test", &dist, &library).unwrap();
        let foreign: Vec<_> = issues
            .iter()
            .filter(|i| i.kind == Some(DiagnosticIssueKind::ForeignSymlink))
            .collect();
        assert_eq!(
            foreign.len(),
            1,
            "expected one ForeignSymlink diagnostic, got: {issues:?}"
        );
        assert_eq!(foreign[0].severity, IssueSeverity::Warning);
        assert!(
            foreign[0].message.contains("foreign symlink"),
            "message must use the 'foreign symlink' wording: {}",
            foreign[0].message
        );
    }

    // -- Phase 21 (v0.14): claim_orphan_directory tests --

    #[test]
    fn claim_orphan_adds_unowned_entry_to_manifest() {
        // Plant an orphan dir in the library, run claim_orphan_directory,
        // verify the manifest now has an Unowned entry for it.
        let tmp = tempfile::TempDir::new().unwrap();
        let lib = tmp.path().join("library");
        std::fs::create_dir_all(&lib).unwrap();
        let paths = TomePaths::new(tmp.path().to_path_buf(), lib.clone()).unwrap();
        std::fs::create_dir_all(paths.config_dir()).unwrap();

        // Create the orphan: library/test-skill/SKILL.md
        let orphan = lib.join("test-skill");
        std::fs::create_dir_all(&orphan).unwrap();
        std::fs::write(
            orphan.join("SKILL.md"),
            "---\nname: test-skill\n---\n# Test\nContent body.\n",
        )
        .unwrap();

        // Manifest starts empty (orphan = not in manifest).
        let man = manifest::load(paths.config_dir()).unwrap();
        assert!(man.is_empty(), "manifest should start empty");

        claim_orphan_directory(&orphan, &paths).unwrap();

        // After claim: manifest has one entry, Unowned (source_name=None),
        // content_hash matches what hash_directory would compute now.
        let man = manifest::load(paths.config_dir()).unwrap();
        assert_eq!(man.len(), 1, "claim should add one manifest entry");
        let entry = man
            .get("test-skill")
            .expect("test-skill should be in manifest");
        assert_eq!(
            entry.source_name, None,
            "claimed orphan must be Unowned (source_name=None)"
        );
        assert_eq!(
            entry.previous_source, None,
            "true orphan has no previous_source"
        );
        assert!(!entry.managed, "claimed orphan is not managed");

        // Hash matches a fresh hash of the dir.
        let expected_hash = manifest::hash_directory(&orphan).unwrap();
        assert_eq!(entry.content_hash, expected_hash);
    }

    #[test]
    fn claim_orphan_refuses_to_clobber_existing_manifest_entry() {
        // If a manifest entry already exists for the skill name (defensive
        // check; shouldn't happen in production because is_orphan_directory
        // filters to NOT-in-manifest entries — but the explicit guard
        // documents the invariant).
        let tmp = tempfile::TempDir::new().unwrap();
        let lib = tmp.path().join("library");
        std::fs::create_dir_all(&lib).unwrap();
        let paths = TomePaths::new(tmp.path().to_path_buf(), lib.clone()).unwrap();
        std::fs::create_dir_all(paths.config_dir()).unwrap();

        let orphan = lib.join("dup-skill");
        std::fs::create_dir_all(&orphan).unwrap();
        std::fs::write(orphan.join("SKILL.md"), "---\nname: dup-skill\n---\n").unwrap();

        // Pre-populate the manifest with a same-name entry.
        let mut man = manifest::Manifest::default();
        let existing_hash = manifest::hash_directory(&orphan).unwrap();
        man.insert(
            crate::discover::SkillName::new("dup-skill").unwrap(),
            manifest::SkillEntry::new_unowned(
                orphan.clone(),
                existing_hash,
                false,
                Some(crate::config::DirectoryName::new("ghost-source").unwrap()),
            ),
        );
        manifest::save(&man, paths.config_dir()).unwrap();

        // Now try to claim — should bail.
        let err = claim_orphan_directory(&orphan, &paths).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("dup-skill") && msg.contains("already in the manifest"),
            "error should name the conflict + manifest-presence; got: {msg}"
        );
    }

    #[test]
    fn claim_orphan_distributes_on_next_sync() {
        // After claim, the orphan should be findable in the manifest with
        // the same shape any other Unowned skill has — meaning subsequent
        // sync code paths (distribute, lockfile generation, doctor) treat
        // it identically. This test pins the contract that the entry is
        // a "real" Unowned entry, not a partial / placeholder shape.
        let tmp = tempfile::TempDir::new().unwrap();
        let lib = tmp.path().join("library");
        std::fs::create_dir_all(&lib).unwrap();
        let paths = TomePaths::new(tmp.path().to_path_buf(), lib.clone()).unwrap();
        std::fs::create_dir_all(paths.config_dir()).unwrap();

        let orphan = lib.join("shape-test");
        std::fs::create_dir_all(&orphan).unwrap();
        std::fs::write(orphan.join("SKILL.md"), "---\nname: shape-test\n---\n").unwrap();

        claim_orphan_directory(&orphan, &paths).unwrap();

        let man = manifest::load(paths.config_dir()).unwrap();
        let entry = man.get("shape-test").expect("entry present");

        // Shape parity with Unowned entries created via the LIB-04
        // source-removal transition: source_name=None, previous_source=None
        // (or Some, but here it's None for true orphans), managed=false,
        // synced_at populated. The content_hash is what makes downstream
        // sync distribute the entry like any other.
        assert_eq!(entry.source_name, None);
        assert!(!entry.managed);
        assert!(
            !entry.synced_at.is_empty(),
            "synced_at must be populated (used by tome status' Last sync line)"
        );
        // source_path points at the library copy (the canonical home for
        // Unowned skills).
        assert_eq!(entry.source_path, orphan);
    }
}

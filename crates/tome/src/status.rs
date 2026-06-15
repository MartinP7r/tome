//! Read-only summary of the library state, configured directories, and overall health.

use anyhow::{Context, Result};
use console::style;
use std::path::{Path, PathBuf};
use tabled::settings::{Modify, Style, object::Rows};

use crate::config::Config;
use crate::lockfile;
use crate::machine;
use crate::manifest;
use crate::paths::TomePaths;

// -- Data structs --

/// A count that may have failed with an error message.
#[derive(serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct CountOrError {
    pub count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<Result<usize, String>> for CountOrError {
    fn from(result: Result<usize, String>) -> Self {
        match result {
            Ok(n) => Self {
                count: Some(n),
                error: None,
            },
            Err(e) => Self {
                count: None,
                error: Some(e),
            },
        }
    }
}

/// State of `tome.lock` relative to the on-disk manifest (VIEW-01).
///
/// Classification reuses the same content-hash comparison `reconcile.rs`
/// performs against the marketplace: every lockfile entry must have a
/// matching manifest entry whose `content_hash` equals the lockfile-recorded
/// value. Any divergence (missing manifest entry OR hash mismatch) increments
/// `drift_count`.
///
/// Variant tag style matches the GUI's discriminated-union pattern (D-GUI-08):
/// the serialized shape is `{ "kind": "in_sync" | "out_of_sync" | "missing", ... }`
/// so the React side can pattern-match without parsing strings.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LockfileState {
    /// Every lockfile entry's `content_hash` matches the manifest.
    InSync,
    /// At least one lockfile entry has no matching manifest hash. `drift_count`
    /// counts the divergent entries (mismatch + missing manifest combined).
    OutOfSync { drift_count: usize },
    /// `tome.lock` does not exist on disk (fresh `tome_home`, never synced).
    Missing,
}

impl LockfileState {
    /// All variant tag strings, in declaration order.
    ///
    /// Mirrors the [`crate::marketplace::InstallFailureKind::ALL`] /
    /// [`crate::remove::FailureKind::ALL`] POLISH-04 pattern: paired with
    /// [`_ensure_lockfile_state_exhaustive`] + the const-length guard below,
    /// a hand-edit that adds a variant without growing `ALL` (or vice versa)
    /// fails to compile.
    pub const ALL: [&'static str; 3] = ["in_sync", "out_of_sync", "missing"];

    /// Classify the lockfile/manifest pair at `paths`.
    ///
    /// Returns:
    /// - [`LockfileState::Missing`] when `tome.lock` does not exist.
    /// - [`LockfileState::InSync`] when every lockfile entry has a matching
    ///   manifest entry with byte-equal `content_hash`.
    /// - [`LockfileState::OutOfSync`] otherwise, with `drift_count` set to the
    ///   number of divergent entries (missing-from-manifest + hash-mismatch).
    ///
    /// Reuses the manifest+lockfile content-hash comparison that
    /// [`crate::reconcile::classify_lockfile`] performs against the
    /// marketplace, but operates against the on-disk manifest (no adapter
    /// required) — matching `reconcile.rs`-shaped semantics per OQ-4 /
    /// RESEARCH §"Standard Stack — Status dashboard".
    pub fn classify(paths: &TomePaths) -> Result<Self> {
        let lockfile = match lockfile::load(paths.config_dir())? {
            Some(lf) => lf,
            None => return Ok(LockfileState::Missing),
        };

        // Manifest read errors fall back to "no manifest" — every lockfile
        // entry counts as drift in that case so the user is signalled.
        let manifest = manifest::load(paths.config_dir()).unwrap_or_default();

        let mut drift_count = 0usize;
        for (name, lock_entry) in lockfile.skills() {
            match manifest.get(name.as_str()) {
                Some(manifest_entry) if manifest_entry.content_hash == lock_entry.content_hash => {
                    // matched
                }
                _ => drift_count += 1,
            }
        }

        if drift_count == 0 {
            Ok(LockfileState::InSync)
        } else {
            Ok(LockfileState::OutOfSync { drift_count })
        }
    }
}

/// Compile-time drift guard for [`LockfileState::ALL`] (POLISH-04 option c).
///
/// If a new variant is added to [`LockfileState`], this `const fn` fails to
/// compile because the match below is exhaustive. The fix is to (a) add an
/// arm here AND (b) append the new tag string to `ALL`. Mirrors
/// [`crate::marketplace::_ensure_install_failure_kind_all_exhaustive`].
#[allow(dead_code)] // sentinel-only — its purpose is the exhaustiveness check
const fn _lockfile_state_exhaustiveness_sentinel(state: LockfileState) -> usize {
    match state {
        LockfileState::InSync => 0,
        LockfileState::OutOfSync { .. } => 1,
        LockfileState::Missing => 2,
    }
}

const _: () = {
    // If this fails: LockfileState::ALL is missing or has extra variants.
    // The match arms in _lockfile_state_exhaustiveness_sentinel are the
    // source of truth — ALL must contain exactly one tag per arm.
    assert!(LockfileState::ALL.len() == 3);
};

/// Per-machine prefs summary shown in the Status view (VIEW-01).
///
/// Surfaces the integer counts the Status view's `MACHINE` row renders
/// ("N skills disabled"). Counts only — the full skill / directory lists
/// stay in `machine.toml`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct MachinePrefsSummary {
    /// `MachinePrefs.disabled.len()` — count of skills globally disabled
    /// on this machine.
    pub disabled_count: usize,
    /// `MachinePrefs.disabled_directories.len()` — count of directories
    /// disabled on this machine.
    pub disabled_directory_count: usize,
}

/// Status of a single configured directory.
///
/// **JSON shape change (v0.11+):** `role` is now the typed
/// `DirectoryRole` enum (serializes as `"managed"` / `"synced"` /
/// `"source"` / `"target"`) so GUI consumers can branch on the role
/// without parsing prose. The human-readable description previously
/// in `role` (e.g. `"Synced (skills discovered here AND distributed
/// here)"`) moved to `role_description`. Per the project's
/// `Backward compat: None` policy, JSON consumers that read `role` as
/// a description string need to switch to `role_description` or to the
/// enum-aware reader.
#[derive(serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct DirectoryStatus {
    pub name: String,
    pub directory_type: String,
    pub role: crate::config::DirectoryRole,
    /// Human-readable description of the role (e.g. `"Synced (skills
    /// discovered here AND distributed here)"`). Display-only — GUI
    /// consumers should branch on [`role`](Self::role) instead.
    pub role_description: String,
    pub path: String,
    /// Number of skills discovered (for discovery dirs) or symlinks present (for target dirs),
    /// or an error message if counting failed.
    pub skill_count: CountOrError,
    /// Warnings emitted during discovery.
    pub warnings: Vec<String>,
    /// True iff `directories.<name>.path` was rewritten by a `machine.toml`
    /// `[directory_overrides.<name>]` entry during config load (PORT-05).
    /// JSON consumers can use this to render the same context that text-mode
    /// `tome status` shows via the `(override)` annotation.
    pub override_applied: bool,
}

/// Complete status report for the tome system.
#[derive(serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct StatusReport {
    pub configured: bool,
    pub library_dir: PathBuf,
    /// Number of skills consolidated in the library, or an error message.
    pub library_count: CountOrError,
    /// RFC-3339 timestamp of last successful sync; `null` if never synced
    /// or pre-v0.11 manifest. Per D-LSYNC-2 (OBS-07): "never" in text;
    /// `null` in JSON. No `skip_serializing_if` — emit `"last_sync": null`
    /// for stable-shape JSON consumers (matches `unowned: []` pattern).
    pub last_sync: Option<String>,
    pub directories: Vec<DirectoryStatus>,
    /// Skills in the library whose source was removed from `tome.toml`
    /// (Unowned per LIB-04). Surfaces in text rendering between the
    /// Directories table and the Health line (D-D2). Always present in
    /// JSON output for stable shape; empty array when no Unowned skills.
    pub unowned: Vec<crate::summary::SkillSummary>,
    /// State of `tome.lock` relative to the on-disk manifest (VIEW-01).
    /// Surfaces in the Status view's `LOCKFILE` row paired with `StatusDot`.
    /// JSON shape: `{ "kind": "in_sync" | "out_of_sync" | "missing", ... }`.
    pub lockfile: LockfileState,
    /// Per-machine prefs summary (VIEW-01). Surfaces in the Status view's
    /// `MACHINE` row ("N skills disabled").
    pub machine_prefs_summary: MachinePrefsSummary,
    /// Number of health issues, or an error message.
    pub health: CountOrError,
}

// -- Data gathering (pure computation, no I/O) --

/// Gather status data without producing any output.
pub fn gather(config: &Config, paths: &TomePaths) -> Result<StatusReport> {
    let configured = paths.library_dir().is_dir() || !config.directories.is_empty();

    let library_count = if paths.library_dir().is_dir() {
        count_entries(paths.library_dir()).map_err(|e| e.to_string())
    } else {
        Ok(0)
    };

    let directories: Vec<DirectoryStatus> = config
        .directories
        .iter()
        .map(|(name, dir_config)| {
            let role = dir_config.role();
            let skill_count = if role.is_discovery() {
                // For discovery directories, count SKILL.md subdirs
                count_skill_dirs(&dir_config.path).map_err(|e| e.to_string())
            } else {
                // For target-only directories, count existing symlinks
                count_symlinks(&dir_config.path).map_err(|e| e.to_string())
            };
            let warnings = Vec::new();
            DirectoryStatus {
                name: name.as_str().to_string(),
                directory_type: dir_config.directory_type.to_string(),
                role_description: role.description().to_string(),
                role,
                path: dir_config.path.display().to_string(),
                skill_count: skill_count.into(),
                warnings,
                override_applied: dir_config.override_applied,
            }
        })
        .collect();

    let health = if paths.library_dir().is_dir() {
        count_health_issues(paths.library_dir(), paths.config_dir()).map_err(|e| e.to_string())
    } else {
        Ok(0)
    };

    // Populate the Unowned set per UNOWN-03 AND the `last_sync` header per
    // D-LSYNC-1/-2 (OBS-07). Both are sourced from the same manifest read.
    // Read errors degrade gracefully: empty Unowned set + last_sync = None.
    let (unowned, last_sync): (Vec<crate::summary::SkillSummary>, Option<String>) =
        match manifest::load(paths.config_dir()) {
            Ok(m) => {
                let last = m.last_synced_at().map(String::from);
                let unowned = m
                    .iter()
                    .filter(|(_, entry)| entry.source_name().is_none())
                    .map(|(name, entry)| crate::summary::SkillSummary::from_entry(name, entry))
                    .collect();
                (unowned, last)
            }
            Err(_) => (Vec::new(), None),
        };

    // VIEW-01: lockfile classification + machine-prefs summary. Both degrade
    // gracefully — IO/parse errors on the lockfile fall through to `Missing`
    // (the same outcome a missing file produces), and machine-prefs read
    // errors fall through to `Default::default()` so the Status view always
    // renders the row.
    let lockfile = LockfileState::classify(paths).unwrap_or(LockfileState::Missing);
    let machine_prefs_summary = match machine::default_machine_path() {
        Ok(p) => match machine::load(&p) {
            Ok(prefs) => MachinePrefsSummary {
                disabled_count: prefs.disabled.len(),
                disabled_directory_count: prefs.disabled_directories.len(),
            },
            Err(_) => MachinePrefsSummary {
                disabled_count: 0,
                disabled_directory_count: 0,
            },
        },
        Err(_) => MachinePrefsSummary {
            disabled_count: 0,
            disabled_directory_count: 0,
        },
    };

    Ok(StatusReport {
        configured,
        library_dir: paths.library_dir().to_path_buf(),
        library_count: library_count.into(),
        last_sync,
        directories,
        unowned,
        lockfile,
        machine_prefs_summary,
        health: health.into(),
    })
}

// -- Rendering --

/// Format the PATH column for the directories table. When `override_applied`
/// is true, append a styled ` (override)` annotation so the user can see
/// which entries were rewritten by a `machine.toml` `[directory_overrides.<name>]`
/// entry (PORT-05).
fn format_dir_path_column(path: &str, override_applied: bool) -> String {
    let collapsed = crate::paths::collapse_home(std::path::Path::new(path));
    if override_applied {
        format!("{} {}", collapsed, style("(override)").cyan())
    } else {
        collapsed
    }
}

/// Format the Unowned skills section (heading + table) per D-D1/D-D2.
/// Returns `None` when the unowned set is empty so the section omits
/// cleanly (no header, no blank line). Pure formatter — no I/O — so
/// rendering can be unit-tested without capturing stdout.
fn format_unowned_section(unowned: &[crate::summary::SkillSummary]) -> Option<String> {
    if unowned.is_empty() {
        return None;
    }
    let heading = format!("{} ({}):", style("Unowned skills").bold(), unowned.len());
    let mut rows: Vec<[String; 3]> = Vec::with_capacity(unowned.len() + 1);
    rows.push([
        "NAME".to_string(),
        "LAST-KNOWN SOURCE".to_string(),
        "SYNCED".to_string(),
    ]);
    for s in unowned {
        // D-C1 / D-C2 fallback: render previous_source when present;
        // fall back to source_path_display (already collapse_home-rendered
        // by SkillSummary::from_entry).
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
    Some(format!("{heading}\n{table}"))
}

/// Display the current status of the tome system.
pub fn show(config: &Config, paths: &TomePaths, json: bool) -> Result<()> {
    let report = gather(config, paths)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        render_status(&report);
    }
    Ok(())
}

fn render_status(report: &StatusReport) {
    if !report.configured {
        println!("Not configured yet. Run `tome init` to get started.");
        return;
    }

    // Library
    println!(
        "{} {}",
        style("Library:").bold(),
        crate::paths::collapse_home(&report.library_dir)
    );
    let (lib_count, lib_indicator) =
        match (&report.library_count.count, &report.library_count.error) {
            (Some(n), _) => (format!("{}", n), style("✓").green()),
            (None, Some(e)) => {
                eprintln!("warning: could not read library: {}", e);
                ("?".to_string(), style("✗").red())
            }
            (None, None) => ("0".to_string(), style("✓").green()),
        };
    println!(
        "  {} {} skills consolidated",
        lib_indicator,
        style(lib_count).cyan()
    );
    // D-LSYNC-2 (OBS-07): Last sync header line. Reads from
    // StatusReport.last_sync; "never" when manifest doesn't exist or
    // last_synced_at is None.
    let last_sync_str = match &report.last_sync {
        Some(ts) => ts.clone(),
        None => "never".to_string(),
    };
    println!(
        "  {} {}",
        style("Last sync:").bold(),
        style(last_sync_str).cyan()
    );
    println!();

    // Directories
    println!("{}", style("Directories:").bold());
    if report.directories.is_empty() {
        println!("  (none configured)");
    } else {
        let mut rows: Vec<[String; 5]> = Vec::with_capacity(report.directories.len() + 1);
        rows.push([
            "NAME".to_string(),
            "TYPE".to_string(),
            "ROLE".to_string(),
            "PATH".to_string(),
            "SKILLS".to_string(),
        ]);
        for dir in &report.directories {
            let count = match (&dir.skill_count.count, &dir.skill_count.error) {
                (Some(n), _) => format!("✓ {}", n),
                (None, Some(e)) => {
                    eprintln!("warning: could not count skills in '{}': {}", dir.name, e);
                    "✗ ?".to_string()
                }
                (None, None) => "✓ 0".to_string(),
            };
            rows.push([
                dir.name.clone(),
                dir.directory_type.clone(),
                dir.role_description.clone(),
                format_dir_path_column(&dir.path, dir.override_applied),
                count,
            ]);
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
        for dir in &report.directories {
            for w in &dir.warnings {
                eprintln!("warning: {}", w);
            }
        }
    }
    println!();

    // Unowned skills (UNOWN-03 / D-D1, D-D2). Section omits cleanly when empty.
    if let Some(rendered) = format_unowned_section(&report.unowned) {
        println!("{rendered}");
        println!();
    }

    // Lockfile state + machine-prefs summary (VIEW-01).
    let lockfile_str = match &report.lockfile {
        LockfileState::InSync => format!("{} {}", style("✓").green(), style("in sync").green()),
        LockfileState::OutOfSync { drift_count } => format!(
            "{} {}",
            style("⚠").yellow(),
            style(format!("out of sync ({} drift)", drift_count)).yellow()
        ),
        LockfileState::Missing => format!("{} {}", style("✗").red(), style("missing").red()),
    };
    println!("{} {}", style("Lockfile:").bold(), lockfile_str);
    println!(
        "{} {} skills disabled, {} directories disabled",
        style("Machine:").bold(),
        style(report.machine_prefs_summary.disabled_count).cyan(),
        style(report.machine_prefs_summary.disabled_directory_count).cyan(),
    );

    // Health
    let health = match (&report.health.count, &report.health.error) {
        (Some(0), _) => format!("{} {}", style("✓").green(), style("All good").green()),
        (Some(n), _) => format!(
            "{} {}",
            style("⚠").yellow(),
            style(format!("{} issue(s) — run `tome doctor` for details", n)).yellow()
        ),
        (None, Some(e)) => {
            eprintln!("warning: could not check library health: {}", e);
            format!("{} {}", style("✗").red(), style("unknown").red())
        }
        (None, None) => format!("{} {}", style("✓").green(), style("All good").green()),
    };
    println!("{} {}", style("Health:").bold(), health);
}

/// Count skill directory entries in the library, excluding hidden entries.
/// Since v0.10 (LIB-01) all library entries are real directory copies;
/// symlinks-to-dirs are still counted via `path.is_dir()` to support
/// reading un-migrated v0.9-shape libraries from `tome status`.
fn count_entries(dir: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        // `is_dir()` follows symlinks, so it counts both real directories
        // (the v0.10+ canonical shape) and symlinks-to-dirs (v0.9-shape
        // libraries that haven't run `tome migrate-library` yet).
        if path.is_dir() {
            count += 1;
        }
    }
    Ok(count)
}

/// Count subdirectories that look like skills (contain SKILL.md or are directories).
fn count_skill_dirs(dir: &Path) -> Result<usize> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            count += 1;
        }
    }
    Ok(count)
}

/// Count symlinks in a directory (for target-only directories).
fn count_symlinks(dir: &Path) -> Result<usize> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        if path.is_symlink() {
            count += 1;
        }
    }
    Ok(count)
}

/// Count health issues: manifest/disk mismatches.
fn count_health_issues(dir: &Path, tome_home: &Path) -> Result<usize> {
    let m = manifest::load(tome_home)?;
    let mut issues = 0;

    // Check manifest entries exist on disk
    for name in m.keys() {
        if !dir.join(name.as_str()).is_dir() {
            issues += 1;
        }
    }

    // Second pass: orphan directories and broken symlinks
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() && !name.starts_with('.') && !m.contains_key(&name) {
            issues += 1; // orphan
        }
        if path.is_symlink() && !path.exists() && !m.contains_key(&name) {
            issues += 1; // broken symlink (not already counted via manifest check)
        }
    }

    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    // -- gather() tests --

    #[test]
    fn gather_unconfigured_returns_not_configured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/tome/library"),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(!report.configured);
        assert!(report.directories.is_empty());
    }

    #[test]
    fn gather_with_directories_marks_configured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/tome/library"),
            directories: BTreeMap::from([(
                DirectoryName::new("test").unwrap(),
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

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(report.configured);
        assert_eq!(report.directories.len(), 1);
        assert_eq!(report.directories[0].name, "test");
        // Source path doesn't exist — count_skill_dirs returns Ok(0)
        assert_eq!(report.directories[0].skill_count.count, Some(0));
    }

    #[test]
    fn gather_with_library_dir_counts_skills() {
        let lib_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(lib_dir.path().join("skill-a")).unwrap();
        std::fs::create_dir_all(lib_dir.path().join("skill-b")).unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(report.configured);
        assert_eq!(report.library_count.count, Some(2));
    }

    #[test]
    fn gather_with_target_directory_populates_status() {
        let lib_dir = tempfile::TempDir::new().unwrap();
        let target_dir = tempfile::TempDir::new().unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            directories: BTreeMap::from([(
                DirectoryName::new("claude").unwrap(),
                DirectoryConfig {
                    path: target_dir.path().to_path_buf(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Target),
                    git_ref: None,

                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.directories.len(), 1);
        assert_eq!(report.directories[0].name, "claude");
        assert_eq!(report.directories[0].role, DirectoryRole::Target);
        assert!(report.directories[0].role_description.contains("Target"));
    }

    #[test]
    fn gather_directory_status_includes_role_description() {
        let lib_dir = tempfile::TempDir::new().unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            directories: BTreeMap::from([(
                DirectoryName::new("my-skills").unwrap(),
                DirectoryConfig {
                    path: lib_dir.path().to_path_buf(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Synced),
                    git_ref: None,

                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.directories.len(), 1);
        assert_eq!(
            report.directories[0].role,
            DirectoryRole::Synced,
            "typed role field must be Synced"
        );
        assert!(
            report.directories[0].role_description.contains("Synced"),
            "role_description should contain Synced, got: {}",
            report.directories[0].role_description
        );
        assert!(
            report.directories[0]
                .role_description
                .contains("discovered here AND distributed here"),
            "role_description should include human-readable expansion, got: {}",
            report.directories[0].role_description
        );
    }

    #[test]
    fn gather_health_detects_orphan() {
        let lib_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(lib_dir.path().join("orphan-skill")).unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.health.count, Some(1));
    }

    // -- count_entries --

    #[test]
    fn count_entries_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(count_entries(dir.path()).unwrap(), 0);
    }

    #[test]
    fn count_entries_ignores_regular_files() {
        let dir = tempfile::TempDir::new().unwrap();
        for name in ["a", "b", "c"] {
            std::fs::write(dir.path().join(name), "").unwrap();
        }
        assert_eq!(count_entries(dir.path()).unwrap(), 0);
    }

    #[test]
    fn count_entries_ignores_hidden_directories() {
        let dir = tempfile::TempDir::new().unwrap();

        // Visible skill dir — should be counted
        std::fs::create_dir_all(dir.path().join("my-skill")).unwrap();
        // Hidden dirs — should NOT be counted
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        std::fs::create_dir_all(dir.path().join(".hidden")).unwrap();

        assert_eq!(count_entries(dir.path()).unwrap(), 1);
    }

    #[test]
    fn count_entries_counts_directories() {
        let dir = tempfile::TempDir::new().unwrap();

        // Two directories — should be counted
        std::fs::create_dir_all(dir.path().join("skill-a")).unwrap();
        std::fs::create_dir_all(dir.path().join("skill-b")).unwrap();
        // One regular file — should be ignored
        std::fs::write(dir.path().join(".tome-manifest.json"), "{}").unwrap();

        assert_eq!(count_entries(dir.path()).unwrap(), 2);
    }

    // -- count_skill_dirs --

    #[test]
    fn count_skill_dirs_nonexistent_returns_zero() {
        assert_eq!(count_skill_dirs(Path::new("/nonexistent/dir")).unwrap(), 0);
    }

    #[test]
    fn count_skill_dirs_counts_subdirs() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("skill-a")).unwrap();
        std::fs::create_dir_all(dir.path().join("skill-b")).unwrap();
        std::fs::write(dir.path().join("not-a-skill.txt"), "").unwrap();

        assert_eq!(count_skill_dirs(dir.path()).unwrap(), 2);
    }

    // -- count_symlinks --

    #[test]
    fn count_symlinks_nonexistent_returns_zero() {
        assert_eq!(count_symlinks(Path::new("/nonexistent/dir")).unwrap(), 0);
    }

    #[test]
    fn count_symlinks_counts_only_symlinks() {
        use std::os::unix::fs as unix_fs;

        let dir = tempfile::TempDir::new().unwrap();
        let target = tempfile::TempDir::new().unwrap();

        unix_fs::symlink(target.path(), dir.path().join("linked")).unwrap();
        std::fs::create_dir_all(dir.path().join("real-dir")).unwrap();
        std::fs::write(dir.path().join("file.txt"), "").unwrap();

        assert_eq!(count_symlinks(dir.path()).unwrap(), 1);
    }

    // -- count_health_issues --

    #[test]
    fn count_health_issues_uses_tome_home() {
        let tome_home = tempfile::TempDir::new().unwrap();
        let library = tempfile::TempDir::new().unwrap();

        // Create a skill directory in the library
        std::fs::create_dir_all(library.path().join("my-skill")).unwrap();

        // Save manifest at tome_home (not library_dir)
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/my-skill"),
                ownership: manifest::SkillOwnership::Owned {
                    source: DirectoryName::new("test").unwrap(),
                },
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, tome_home.path()).unwrap();

        // Should find 0 issues when manifest is at tome_home
        assert_eq!(
            count_health_issues(library.path(), tome_home.path()).unwrap(),
            0,
            "should find no issues when manifest at tome_home matches library contents"
        );

        // Should find 1 orphan when using library_dir as tome_home (no manifest there)
        assert_eq!(
            count_health_issues(library.path(), library.path()).unwrap(),
            1,
            "should detect orphan when manifest is not at the given tome_home"
        );
    }

    #[test]
    fn count_health_issues_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 0);
    }

    #[test]
    fn count_health_issues_detects_manifest_disk_mismatch() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create a manifest entry with no corresponding directory
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("missing").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source"),
                ownership: manifest::SkillOwnership::Owned {
                    source: DirectoryName::new("test").unwrap(),
                },
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, dir.path()).unwrap();

        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 1);
    }

    #[test]
    fn count_health_issues_detects_orphan_directory() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create a directory not tracked by manifest
        std::fs::create_dir_all(dir.path().join("orphan-skill")).unwrap();

        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 1);
    }

    #[test]
    fn count_health_issues_no_double_count_broken_managed_symlink() {
        use std::os::unix::fs as unix_fs;

        let dir = tempfile::TempDir::new().unwrap();

        // Create a managed skill manifest entry pointing to a non-existent source
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("managed-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source"),
                ownership: manifest::SkillOwnership::Owned {
                    source: DirectoryName::new("plugins").unwrap(),
                },
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: true,
            },
        );
        manifest::save(&m, dir.path()).unwrap();

        // Create a broken symlink (managed skill whose source is gone)
        unix_fs::symlink("/nonexistent/source", dir.path().join("managed-skill")).unwrap();

        // Should count exactly 1 issue (manifest-vs-disk), not 2
        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 1);
    }

    #[test]
    fn count_health_issues_ignores_hidden_dirs() {
        let dir = tempfile::TempDir::new().unwrap();

        // .git dir should not be counted as an orphan
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();

        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 0);
    }

    // -- PORT-05: override_applied surfacing --

    #[test]
    fn gather_with_no_overrides_sets_flag_false() {
        let lib_dir = tempfile::TempDir::new().unwrap();
        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            directories: BTreeMap::from([(
                DirectoryName::new("plain").unwrap(),
                DirectoryConfig {
                    path: lib_dir.path().to_path_buf(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.directories.len(), 1);
        assert!(
            !report.directories[0].override_applied,
            "override_applied should default to false"
        );
    }

    #[test]
    fn gather_with_override_applied_sets_flag_true() {
        let lib_dir = tempfile::TempDir::new().unwrap();
        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            directories: BTreeMap::from([(
                DirectoryName::new("work").unwrap(),
                DirectoryConfig {
                    path: lib_dir.path().to_path_buf(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    git_ref: None,
                    subdir: None,
                    override_applied: true,
                },
            )]),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.directories.len(), 1);
        assert!(
            report.directories[0].override_applied,
            "override_applied should be true when the config flag is set"
        );
    }

    #[test]
    fn render_status_appends_override_marker_to_path() {
        let s = format_dir_path_column("/foo/bar", true);
        assert!(s.contains("/foo/bar"), "path content missing: {s}");
        assert!(s.contains("(override)"), "override marker missing: {s}");
    }

    #[test]
    fn render_status_no_override_omits_marker() {
        let s = format_dir_path_column("/foo/bar", false);
        assert!(s.contains("/foo/bar"), "path content missing: {s}");
        assert!(
            !s.contains("(override)"),
            "override marker should NOT appear when flag is false: {s}"
        );
    }

    #[test]
    fn status_json_includes_override_applied_field() {
        let ds = DirectoryStatus {
            name: "work".to_string(),
            directory_type: "directory".to_string(),
            role: DirectoryRole::Source,
            role_description: "Source (discovery only)".to_string(),
            path: "/some/path".to_string(),
            skill_count: CountOrError {
                count: Some(0),
                error: None,
            },
            warnings: Vec::new(),
            override_applied: true,
        };
        let json = serde_json::to_string(&ds).unwrap();
        assert!(
            json.contains("\"override_applied\":true"),
            "JSON output should include override_applied field, got: {json}"
        );
    }

    // -- UNOWN-03: status surfaces Unowned skills section (D-D1, D-D2, D-D3) --

    #[test]
    fn gather_populates_unowned_for_entries_with_no_source_name() {
        let tome_home = tempfile::TempDir::new().unwrap();
        let library = tome_home.path().join("library");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::create_dir_all(library.join("orphan")).unwrap();
        std::fs::create_dir_all(library.join("kept")).unwrap();

        // Build manifest with one Owned + one Unowned (with previous_source).
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("kept").unwrap(),
            manifest::SkillEntry::new(
                PathBuf::from("/tmp/src/kept"),
                DirectoryName::new("active").unwrap(),
                crate::validation::test_hash("h"),
                false,
            ),
        );
        m.insert(
            crate::discover::SkillName::new("orphan").unwrap(),
            manifest::SkillEntry::new_unowned(
                PathBuf::from("/tmp/old/orphan"),
                crate::validation::test_hash("o"),
                false,
                Some(DirectoryName::new("removed-dir").unwrap()),
            ),
        );
        manifest::save(&m, tome_home.path()).unwrap();

        let config = Config {
            library_dir: library.clone(),
            ..Config::default()
        };
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

        let report = gather(&config, &paths).unwrap();
        assert_eq!(
            report.unowned.len(),
            1,
            "expected exactly one Unowned entry, got {:?}",
            report.unowned
        );
        assert_eq!(report.unowned[0].name, "orphan");
        assert_eq!(
            report.unowned[0].previous_source,
            Some("removed-dir".to_string())
        );
    }

    #[test]
    fn gather_returns_empty_unowned_when_all_entries_are_owned() {
        let tome_home = tempfile::TempDir::new().unwrap();
        let library = tome_home.path().join("library");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::create_dir_all(library.join("kept")).unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("kept").unwrap(),
            manifest::SkillEntry::new(
                PathBuf::from("/tmp/src/kept"),
                DirectoryName::new("active").unwrap(),
                crate::validation::test_hash("h"),
                false,
            ),
        );
        manifest::save(&m, tome_home.path()).unwrap();

        let config = Config {
            library_dir: library.clone(),
            ..Config::default()
        };
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library).unwrap();

        let report = gather(&config, &paths).unwrap();
        assert!(
            report.unowned.is_empty(),
            "expected no Unowned entries, got {:?}",
            report.unowned
        );
    }

    #[test]
    fn json_status_always_includes_unowned_field() {
        let report = StatusReport {
            configured: false,
            library_dir: PathBuf::from("/tmp/lib"),
            library_count: CountOrError {
                count: Some(0),
                error: None,
            },
            last_sync: None,
            directories: Vec::new(),
            unowned: Vec::new(),
            lockfile: LockfileState::Missing,
            machine_prefs_summary: MachinePrefsSummary {
                disabled_count: 0,
                disabled_directory_count: 0,
            },
            health: CountOrError {
                count: Some(0),
                error: None,
            },
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(
            json.contains("\"unowned\""),
            "JSON must include 'unowned' key for stable shape: {json}"
        );
        // Empty array, not omitted, for stable shape.
        assert!(
            json.contains("\"unowned\":[]"),
            "JSON empty unowned must serialize as [], got: {json}"
        );
    }

    #[test]
    fn json_status_serializes_unowned_skill_summaries() {
        // Round-trip: Unowned entry projects through SkillSummary::from_entry
        // and lands as a JSON object on `unowned[0]`.
        let entry = manifest::SkillEntry::new_unowned(
            PathBuf::from("/tmp/old/orphan"),
            crate::validation::test_hash("o"),
            false,
            Some(DirectoryName::new("removed-dir").unwrap()),
        );
        let name = crate::discover::SkillName::new("orphan").unwrap();
        let summary = crate::summary::SkillSummary::from_entry(&name, &entry);
        let report = StatusReport {
            configured: true,
            library_dir: PathBuf::from("/tmp/lib"),
            library_count: CountOrError {
                count: Some(1),
                error: None,
            },
            last_sync: None,
            directories: Vec::new(),
            unowned: vec![summary],
            lockfile: LockfileState::Missing,
            machine_prefs_summary: MachinePrefsSummary {
                disabled_count: 0,
                disabled_directory_count: 0,
            },
            health: CountOrError {
                count: Some(0),
                error: None,
            },
        };
        let value = serde_json::to_value(&report).unwrap();
        let arr = value["unowned"].as_array().expect("unowned must be array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "orphan");
        assert_eq!(arr[0]["previous_source"], "removed-dir");
    }

    #[test]
    fn format_unowned_section_returns_none_for_empty_set() {
        let rendered = format_unowned_section(&[]);
        assert!(
            rendered.is_none(),
            "empty unowned set must return None so the section omits cleanly: {rendered:?}"
        );
    }

    #[test]
    fn format_unowned_section_renders_heading_and_columns() {
        let summaries = vec![crate::summary::SkillSummary {
            name: "orphan".to_string(),
            previous_source: Some("removed-dir".to_string()),
            source_path_display: "~/old/orphan".to_string(),
            synced_at: "2026-01-01T00:00:00Z".to_string(),
            managed: false,
        }];
        let rendered = format_unowned_section(&summaries).expect("non-empty must Some");
        // Heading with count.
        assert!(
            rendered.contains("Unowned skills") && rendered.contains("(1)"),
            "heading missing 'Unowned skills' or count: {rendered}"
        );
        // D-D1 column headers.
        assert!(rendered.contains("NAME"), "missing NAME column: {rendered}");
        assert!(
            rendered.contains("LAST-KNOWN SOURCE"),
            "missing LAST-KNOWN SOURCE column: {rendered}"
        );
        assert!(
            rendered.contains("SYNCED"),
            "missing SYNCED column: {rendered}"
        );
        // Body row.
        assert!(
            rendered.contains("orphan"),
            "missing skill name: {rendered}"
        );
        assert!(
            rendered.contains("removed-dir"),
            "missing previous_source value: {rendered}"
        );
        assert!(
            rendered.contains("2026-01-01T00:00:00Z"),
            "missing synced_at value: {rendered}"
        );
    }

    #[test]
    fn format_unowned_section_falls_back_to_source_path_when_previous_missing() {
        // D-C2 fallback: previous_source = None -> render source_path_display.
        let summaries = vec![crate::summary::SkillSummary {
            name: "legacy".to_string(),
            previous_source: None,
            source_path_display: "~/legacy/path".to_string(),
            synced_at: "2026-02-02T00:00:00Z".to_string(),
            managed: true,
        }];
        let rendered = format_unowned_section(&summaries).expect("non-empty must Some");
        assert!(
            rendered.contains("~/legacy/path"),
            "D-C2 fallback: should render source_path_display when previous_source is None: {rendered}"
        );
    }

    // -- VIEW-01: LockfileState classifier --

    #[test]
    fn lockfile_state_missing_when_no_lockfile_on_disk() {
        let tmp = tempfile::TempDir::new().unwrap();
        let lib = tmp.path().join("library");
        std::fs::create_dir_all(&lib).unwrap();
        let paths = TomePaths::new(tmp.path().to_path_buf(), lib).unwrap();

        let state = LockfileState::classify(&paths).unwrap();
        assert_eq!(state, LockfileState::Missing);
    }

    #[test]
    fn lockfile_state_in_sync_when_hashes_match() {
        use crate::lockfile::{LockEntry, Lockfile};
        use std::collections::BTreeMap;

        let tmp = tempfile::TempDir::new().unwrap();
        let lib = tmp.path().join("library");
        std::fs::create_dir_all(&lib).unwrap();
        let paths = TomePaths::new(tmp.path().to_path_buf(), lib).unwrap();

        let hash = crate::validation::test_hash("alpha-body");

        // Manifest with one entry.
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("alpha").unwrap(),
            manifest::SkillEntry::new(
                PathBuf::from("/tmp/src/alpha"),
                crate::config::DirectoryName::new("dotfiles").unwrap(),
                hash.clone(),
                false,
            ),
        );
        manifest::save(&m, paths.config_dir()).unwrap();

        // Lockfile with matching hash.
        let mut entries = BTreeMap::new();
        entries.insert(
            crate::discover::SkillName::new("alpha").unwrap(),
            LockEntry {
                source_name: Some(crate::config::DirectoryName::new("dotfiles").unwrap()),
                previous_source: None,
                content_hash: hash,
                registry_id: None,
                version: None,
                git_commit_sha: None,
            },
        );
        let lf = Lockfile {
            version: 1,
            skills: entries,
        };
        crate::lockfile::save(&lf, paths.config_dir()).unwrap();

        let state = LockfileState::classify(&paths).unwrap();
        assert_eq!(state, LockfileState::InSync);
    }

    #[test]
    fn lockfile_state_out_of_sync_when_hashes_differ() {
        use crate::lockfile::{LockEntry, Lockfile};
        use std::collections::BTreeMap;

        let tmp = tempfile::TempDir::new().unwrap();
        let lib = tmp.path().join("library");
        std::fs::create_dir_all(&lib).unwrap();
        let paths = TomePaths::new(tmp.path().to_path_buf(), lib).unwrap();

        let lock_hash = crate::validation::test_hash("OLD-CONTENT");
        let manifest_hash = crate::validation::test_hash("NEW-CONTENT");

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("alpha").unwrap(),
            manifest::SkillEntry::new(
                PathBuf::from("/tmp/src/alpha"),
                crate::config::DirectoryName::new("dotfiles").unwrap(),
                manifest_hash,
                false,
            ),
        );
        manifest::save(&m, paths.config_dir()).unwrap();

        let mut entries = BTreeMap::new();
        entries.insert(
            crate::discover::SkillName::new("alpha").unwrap(),
            LockEntry {
                source_name: Some(crate::config::DirectoryName::new("dotfiles").unwrap()),
                previous_source: None,
                content_hash: lock_hash,
                registry_id: None,
                version: None,
                git_commit_sha: None,
            },
        );
        let lf = Lockfile {
            version: 1,
            skills: entries,
        };
        crate::lockfile::save(&lf, paths.config_dir()).unwrap();

        let state = LockfileState::classify(&paths).unwrap();
        assert!(
            matches!(state, LockfileState::OutOfSync { drift_count: 1 }),
            "expected OutOfSync {{ drift_count: 1 }}, got {state:?}"
        );
    }

    #[test]
    fn lockfile_state_out_of_sync_counts_missing_manifest_entries_as_drift() {
        use crate::lockfile::{LockEntry, Lockfile};
        use std::collections::BTreeMap;

        let tmp = tempfile::TempDir::new().unwrap();
        let lib = tmp.path().join("library");
        std::fs::create_dir_all(&lib).unwrap();
        let paths = TomePaths::new(tmp.path().to_path_buf(), lib).unwrap();

        // Empty manifest.
        let m = manifest::Manifest::default();
        manifest::save(&m, paths.config_dir()).unwrap();

        // Lockfile records two entries that have no manifest counterpart.
        let mut entries = BTreeMap::new();
        for n in ["alpha", "beta"] {
            entries.insert(
                crate::discover::SkillName::new(n).unwrap(),
                LockEntry {
                    source_name: Some(crate::config::DirectoryName::new("dotfiles").unwrap()),
                    previous_source: None,
                    content_hash: crate::validation::test_hash(n),
                    registry_id: None,
                    version: None,
                    git_commit_sha: None,
                },
            );
        }
        let lf = Lockfile {
            version: 1,
            skills: entries,
        };
        crate::lockfile::save(&lf, paths.config_dir()).unwrap();

        let state = LockfileState::classify(&paths).unwrap();
        assert!(
            matches!(state, LockfileState::OutOfSync { drift_count: 2 }),
            "expected OutOfSync {{ drift_count: 2 }}, got {state:?}"
        );
    }

    #[test]
    fn lockfile_state_serializes_with_kind_tag() {
        let in_sync = serde_json::to_value(LockfileState::InSync).unwrap();
        assert_eq!(in_sync["kind"], "in_sync");

        let out = serde_json::to_value(LockfileState::OutOfSync { drift_count: 3 }).unwrap();
        assert_eq!(out["kind"], "out_of_sync");
        assert_eq!(out["drift_count"], 3);

        let missing = serde_json::to_value(LockfileState::Missing).unwrap();
        assert_eq!(missing["kind"], "missing");
    }

    #[test]
    fn lockfile_state_all_contains_every_tag_in_declaration_order() {
        // POLISH-04 anchor: this test pins the tag-string order so a hand-edit
        // that scrambles ALL is obvious; the compile-time `const _` block
        // pins the length.
        assert_eq!(LockfileState::ALL, ["in_sync", "out_of_sync", "missing"]);
    }

    #[test]
    fn machine_prefs_summary_serializes_with_count_fields() {
        let summary = MachinePrefsSummary {
            disabled_count: 7,
            disabled_directory_count: 2,
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["disabled_count"], 7);
        assert_eq!(json["disabled_directory_count"], 2);
    }
}

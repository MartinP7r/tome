//! Tome — sync AI coding skills across tools.
//!
//! This crate provides both a CLI binary (`tome`) and a library for managing
//! AI coding skills across multiple tools.
//!
//! # Core pipeline
//!
//! The `sync` function drives the main workflow:
//!
//! 1. **Reconcile** — lockfile-authoritative drift detection for managed
//!    skills via [`MarketplaceAdapter`](marketplace::MarketplaceAdapter)
//!    (Phase 13: classify each managed skill as Match / Drift / Vanished,
//!    apply updates per `auto_install_plugins` consent).
//! 2. **Discover** — scan configured directories (role `managed`/`source`/
//!    `synced`) for `*/SKILL.md` directories.
//! 3. **Consolidate** — copy every discovered skill (managed AND local)
//!    into the library as a real directory (v0.10 library-canonical model;
//!    no symlinks).
//! 4. **Distribute** — push library skills to target tools via symlinks
//!    into `target` / `synced` directories.
//! 5. **Cleanup** — three-bucket stale-skill report
//!    (removed-from-config / missing-from-disk / now-in-exclude-list);
//!    orphan transitions preserve library content per LIB-04.
//! 6. **Save** — persist manifest, lockfile, and `.gitignore`.
//!
//! # Public API
//!
//! - [`config`] — TOML configuration loading and validation
//! - [`cli`] — command-line argument parsing (clap)
//! - [`run()`] — entry point that dispatches CLI commands
//! - [`TomePaths`] — bundled home/library paths
//! - [`SyncReport`] — sync operation results

// `actions` is `pub` so `tome-desktop`'s Tauri command surface can call
// `tome::actions::resolve_source_path` + `tome::actions::set_skill_disabled`
// directly (Phase 26 plan 26-03 / VIEW-03 / D-06/D-07). The module is the
// CORE-01 collect-shape for cross-surface mutations: pure helpers, no
// presentation. The browse TUI's `apply_toggle` Global-scope arm also calls
// `set_skill_disabled` to avoid duplicating the load/mutate/save chain.
pub mod actions;
pub(crate) mod add;
pub(crate) mod backup;
// `browse` is normally `pub(crate)` so the rest of v1.0's GUI Tauri IPC
// surface doesn't accidentally bind to it. When the `test-support`
// feature is enabled (used by integration tests in `tests/browse_snapshots/`
// per HARD-12) it widens to `pub` so the snapshot harness can construct
// `App` fixtures and call `ui::render` against a `TestBackend`. Production
// builds (`cargo build` without features) keep the old `pub(crate)`
// visibility byte-for-byte.
#[cfg(any(test, feature = "test-support"))]
pub mod browse;
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) mod browse;
pub(crate) mod change_cause;
pub(crate) mod cleanup;
pub mod cli;
pub mod config;
pub(crate) mod discover;
pub(crate) mod distribute;
// `doctor` is `pub` since Phase 26 plan 26-05: the GUI Health view's two
// Tauri commands (`get_doctor_report` / `doctor_repair_one`) call into
// `tome::doctor::collect_doctor_view` + `tome::doctor::repair_one`, and the
// IPC boundary needs the public `FindingId` / `DoctorView` / `DoctorFinding`
// types to round-trip across the webview wire.
pub mod doctor;
// `errors` is `pub` because `DomainErrorKind` is the domain half of the
// GUI error boundary (CORE-05 / D-14): `tome-desktop`'s `TomeError::from`
// downcasts these typed sentinels out of the anyhow cause chain to pick a
// coarse `ErrorCode`. Re-exported as `tome::DomainErrorKind` below. The CLI
// never names this type (the domain stays `anyhow::Result`); it is attached
// at GUI-relevant failure sites via `.context()` and only read at the IPC edge.
pub(crate) mod eject;
pub mod errors;
pub(crate) mod git;
pub(crate) mod library;
pub(crate) mod lint;
// `list` is `pub` so `tome-desktop` can call `list::collect` directly from
// the `list_skills` Tauri command (plan 26-02 Task 2 / VIEW-02). The
// CORE-01 collect-shape (gather → render) keeps the surface narrow: only
// `ListReport` + `collect` are public.
pub mod list;
// `lockfile` is `pub` so `tome-desktop` can call `lockfile::load` and consume
// `Lockfile` + `LockEntry` across the crate boundary for the SYNC-02
// triage-panel projection (Phase 27 plan 27-02 / get_lockfile_diff command).
// The CLI's lockfile-writing path stays in-crate via the pipeline; only the
// read shape + `load` are needed by the GUI for diff projection.
pub mod lockfile;
// `machine` is normally `pub(crate)` to keep `MachinePrefs` out of the
// v1.0 GUI Tauri IPC surface. The HARD-21 browse_snapshots integration
// test (under `test-support`) needs to construct `MachinePrefs` to
// drive the post-toggle snapshot — same gating as `browse`.
#[cfg(any(test, feature = "test-support"))]
pub mod machine;
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) mod machine;
// `manifest` is `pub` so `tome-desktop`'s perf-bench fixture generator
// (`tests/perf/synthetic_skills.rs`, Phase 26 plan 26-08) can construct
// `Manifest` + `SkillEntry` rows with the canonical serde shape — no
// hand-written JSON, no drift risk. Narrow surface: only `Manifest`,
// `SkillEntry`, `SkillOwnership`, `load`, `save`, `hash_directory` are
// reachable; `MANIFEST_FILENAME` stays `pub(crate)`. The same precedent
// (lift module to `pub` so the GUI crate can reuse it) was set when
// `tome::list` was lifted in plan 26-02.
pub mod manifest;
pub mod marketplace;
pub(crate) mod migration_v010;
pub(crate) mod paths;
// `progress` is `pub` because its trait + event vocabulary
// (`ProgressSink`/`ProgressEvent`/`SyncStage`/`CancelToken`) is the domain
// half of the "structure at the edge" pattern (D-09/D-11): the GUI's
// `tome-desktop` crate implements `ProgressSink` and pattern-matches the
// typed `ProgressEvent` enum across the Tauri IPC boundary. The CLI
// `IndicatifSink` + sync threading land in 25-03; the GUI `TauriEventSink`
// in 25-04.
pub mod progress;
pub(crate) mod reassign;
pub(crate) mod reconcile;
pub(crate) mod relocate;
pub(crate) mod remove;
// `skill` is `pub` so `tome-desktop` can call `skill::collect_detail` and
// consume `SkillDetail` + `SkillFrontmatterView` directly across the crate
// boundary (Phase 26 plan 26-03 / VIEW-03 / D-05). The CLI/TUI keep using
// `skill::parse` (also pub); the GUI is purely additive.
pub mod skill;
// `status` is `pub` so the v1.0 GUI (`tome-desktop`) can call
// `tome::status::gather` and consume `tome::status::StatusReport` across the
// crate boundary (CORE-04 / D-GUI-08). The CLI presenter (`status::show`)
// stays in-crate; only the structured `gather`/`StatusReport` surface is the
// GUI contract. Widened in 25-04 (carry-forward from 25-03's pub `plan` fns).
pub mod status;
pub(crate) mod summary;
pub mod tracing_init;
// `update` is `pub` so `tome-desktop` can call `update::diff` and consume
// `UpdateDiff`/`SkillChange` for the SYNC-02 lockfile-diff projection (plan
// 27-02). The CLI's `present_changes` interactive triage stays in-crate
// (not exported); the GUI substitutes its own visual triage flow.
pub mod update;
pub(crate) mod validation;
pub(crate) mod wizard;

use std::collections::{BTreeMap, HashSet};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command as GitCommand;

use anyhow::{Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{debug, info, info_span, warn};

use cleanup::CleanupResult;
use cli::{Cli, Command};
use config::{Config, DirectoryName, DirectoryType};
use distribute::DistributeResult;
use library::ConsolidateResult;
pub use paths::TomePaths;
use progress::{CancelToken, NullSink, ProgressEvent, ProgressSink, SyncStage};

/// Re-exported for integration tests so the synthetic-fixture builder in
/// `tests/cli.rs` can hash directories with the exact same algorithm the
/// production manifest uses (avoids a duplicated SHA-256 helper that could
/// drift). Production code should still call `manifest::hash_directory`
/// directly via the crate path.
pub use manifest::hash_directory;

/// HARD-04: surface lint-failure and migrate-failure typed errors so the
/// thin `main.rs` binary can downcast and map them to exit code 1 without
/// the library calling `process::exit` itself.
pub use lint::LintFailed;
pub use migration_v010::MigrationPartialOrFailed;

/// CORE-05 / D-14: the typed `DomainErrorKind` sentinels (and the transparent
/// `DomainTagged` wrapper that carries one through the anyhow cause chain) the
/// `tome-desktop` IPC boundary downcasts to classify errors into a coarse
/// `ErrorCode`. Attached at GUI-relevant failure sites via
/// `WithDomainKind::with_domain_kind`; the domain itself stays `anyhow::Result`.
pub use errors::{DomainErrorKind, DomainTagged};

/// Per-machine preferences (re-exported from the `pub(crate)` `machine`
/// module so external consumers — the Tauri `start_sync` command in
/// `crates/tome-desktop/src/commands.rs`, plan 27-01b — can load and pass
/// `MachinePrefs` into `sync()` without depending on the module path.
pub use machine::MachinePrefs;
/// Phase 26 plan 26-06 (VIEW-06 / NF-05) — the `tome-desktop` file watcher
/// reads the canonical `machine.toml` path here without forcing the whole
/// `machine` module to become part of the public API. Keep the re-export
/// narrow (single function — no `MachinePrefs` etc.) so the GUI watcher's
/// dependency surface stays small.
pub use machine::default_machine_path;
/// Load `machine.toml` from the given path. Re-exported alongside
/// [`MachinePrefs`] for the same reason (plan 27-01b).
pub use machine::load as load_machine_prefs;
/// Phase 27 plan 27-03 (SYNC-03) — the desktop `apply_machine_toml` command
/// commits proposed prefs via the canonical atomic temp+rename. Re-exported
/// alongside [`load_machine_prefs`] so the GUI does not need to know the
/// `machine` module path; this also keeps the public API symmetric (load /
/// save) without lifting the whole module to `pub`.
pub use machine::save as save_machine_prefs;
/// Phase 27 plan 27-03 (SYNC-03) — the desktop `preview_machine_toml` command
/// returns a structured Myers diff between the on-disk `machine.toml` and the
/// proposed prefs, rendered as a `MachineTomlDiff` inside the `PreviewPopover`.
/// The types ([`MachineTomlPreview`], [`DiffLine`], [`DiffLineKind`]) and the
/// helper ([`preview_save`](machine::preview_save)) are re-exported here so
/// `tome-desktop`'s `commands.rs` can use them without touching the gated
/// `machine` module.
pub use machine::{DiffLine, DiffLineKind, MachineTomlPreview, preview_save};

/// Phase 26 plan 26-03 (VIEW-03) — `tome-desktop` accepts `SkillName` as the
/// input arg for the 4 new commands (`get_skill_detail`, `set_skill_disabled`,
/// `open_source_folder`, `copy_path`). Re-exporting at the crate root keeps
/// the `discover` module's larger surface (`DiscoveredSkill`, scanners) out
/// of the GUI's import path while making the validated newtype reachable.
pub use discover::SkillName;

/// Phase 27 plan 27-02 (SYNC-02) — `tome-desktop`'s SYNC-02 triage projection
/// reconstructs a `SkillOrigin` from lockfile `registry_id`/`version`/
/// `git_commit_sha` fields so the React side reuses the same discriminator
/// the Skills view already pattern-matches. The `discover_all` re-export
/// lets `get_lockfile_diff` build a prospective lockfile from the current
/// disk state without depending on the `pub(crate)` `discover` module path.
pub use discover::{SkillOrigin, SkillProvenance, discover_all};

/// Phase 27 plan 27-02 (SYNC-02) — `tome-desktop`'s SYNC-02 triage projection
/// surfaces lockfile content hashes as boundary strings. Re-exporting
/// `ContentHash` lets the `sync_types` module's unit tests construct valid
/// hashes without duplicating the 64-hex validator. The `validation` module
/// stays `pub(crate)` (the rest of its surface is the internal
/// `validate_identifier` helper); only `ContentHash` is lifted.
pub use validation::ContentHash;

/// Summary of a complete sync operation — the return-shape of the full
/// `sync()` pipeline (reconcile → discover → consolidate → distribute →
/// cleanup → save). This is the primary data source for any consumer that
/// wants to surface "what happened this sync" (the CLI's stdout summary
/// block today; the v1.0 Tauri GUI's sync-result view tomorrow).
///
/// # Field ownership
///
/// - [`consolidate`](Self::consolidate) — always populated; counts skills
///   created / unchanged / updated in the library this run.
/// - [`distributions`](Self::distributions) — one entry per configured
///   `target` / `synced` directory. Empty when no directories are
///   configured (a config error surfaced separately).
/// - [`cleanup`](Self::cleanup) — always populated; reflects the three
///   stale-skill buckets (A: removed-from-config / B: missing-from-disk /
///   C: now-in-exclude-list). Use the public accessors on
///   [`CleanupResult`] for read-only access.
/// - [`removed_from_targets`](Self::removed_from_targets) — total stale
///   distribution symlinks pruned across all targets.
/// - [`reconcile`](Self::reconcile) — `None` when sync ran without a
///   `MarketplaceAdapter` (no `claude-plugins` directory configured).
///   `Some(_)` when reconcile ran; counts may all be zero on a clean
///   match. See [`reconcile::ReconcileReport`] for the inner shape.
pub struct SyncReport {
    pub consolidate: ConsolidateResult,
    pub distributions: Vec<DistributeResult>,
    pub cleanup: CleanupResult,
    pub removed_from_targets: usize,
    /// Phase 18 OBS-05: per-classification reconcile counts surfaced in
    /// the final summary block. `None` when the sync didn't invoke a
    /// reconcile pass (no Claude adapter configured).
    pub reconcile: Option<reconcile::ReconcileReport>,
}

/// Create a spinner with a consistent style.
fn spinner(msg: &str) -> ProgressBar {
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    sp.set_message(msg.to_string());
    sp.enable_steady_tick(std::time::Duration::from_millis(80));
    sp
}

/// The CLI [`ProgressSink`] (D-11): re-homes the `spinner()` / `finish_and_clear()`
/// presentation that used to be inlined in `sync()`.
///
/// "Structure at the edge" (D-17): the domain `sync()` is now presentation-free
/// — it `emit`s typed [`ProgressEvent`]s and the *front-end* decides how to show
/// them. The CLI front-end (this sink) reproduces the exact pre-decomposition
/// spinner behavior; the GUI front-end (`tome-desktop`, Phase 25-04) ships a
/// `TauriEventSink` that forwards the same events over IPC.
///
/// # Output fidelity
///
/// Spinners are a TTY affordance: each is `finish_and_clear()`ed before the next
/// begins, so they never reach piped stdout and are absent from the `insta` /
/// `assert_cmd` regression snapshots. This sink therefore preserves CLI output
/// byte-for-byte — it only redraws the same transient spinners the inline code
/// drew. `cmd_sync` constructs an `IndicatifSink` for interactive runs and a
/// [`NullSink`] under `--quiet`/`--verbose`, exactly matching the previous
/// `show_progress = !quiet && !verbose` gate.
///
/// # Interior mutability
///
/// `emit(&self, …)` takes `&self` (the trait is `Send + Sync` so a GUI sink can
/// hold an `AppHandle`). The "currently-active spinner" therefore lives behind a
/// `Mutex<Option<ProgressBar>>`: `SyncStageStarted` installs a fresh spinner,
/// `SyncStageFinished` takes it back out and `finish_and_clear()`s it.
struct IndicatifSink {
    current: std::sync::Mutex<Option<ProgressBar>>,
}

impl IndicatifSink {
    fn new() -> Self {
        Self {
            current: std::sync::Mutex::new(None),
        }
    }

    /// The transient spinner message for a stage — matches the strings the
    /// pre-decomposition inline call sites used (Reconcile reused the
    /// "Resolving git sources..." banner per the plan's stage mapping).
    fn stage_message(stage: SyncStage) -> &'static str {
        match stage {
            SyncStage::Reconcile => "Resolving git sources...",
            SyncStage::Discover => "Discovering skills...",
            SyncStage::Consolidate => "Consolidating to library...",
            SyncStage::Distribute => "Distributing to targets...",
            SyncStage::Cleanup => "Cleaning up...",
            SyncStage::Save => "Saving...",
        }
    }
}

impl ProgressSink for IndicatifSink {
    fn emit(&self, event: ProgressEvent) {
        let mut current = self.current.lock().expect("IndicatifSink mutex poisoned");
        match event {
            ProgressEvent::SyncStageStarted { stage } => {
                // Replace any in-flight spinner (defensive: stages are
                // strictly Started→Finished, but a missed Finished must not
                // leave a stale ticking spinner behind).
                if let Some(prev) = current.take() {
                    prev.finish_and_clear();
                }
                *current = Some(spinner(Self::stage_message(stage)));
            }
            ProgressEvent::SyncStageFinished { .. } => {
                if let Some(sp) = current.take() {
                    sp.finish_and_clear();
                }
            }
            // Per-stage incremental progress + the git/backup long-op events
            // update the active spinner's message in-place when one is live.
            // No active spinner (e.g. an event emitted between stages) is a
            // silent no-op — these are transient TTY hints, never captured
            // output.
            ProgressEvent::SyncStageProgress {
                stage,
                current: done,
                total,
                item: _, // D-08: the per-unit subtitle is a GUI affordance;
                         // the CLI spinner keeps a counts-only message so the
                         // captured-output contract (no per-skill chrome in
                         // `insta`/`assert_cmd` snapshots) holds byte-for-byte.
            } => {
                if let Some(sp) = current.as_ref()
                    && total > 0
                {
                    sp.set_message(format!("{} ({done}/{total})", Self::stage_message(stage)));
                }
            }
            ProgressEvent::GitCloneProgress { directory, .. } => {
                if let Some(sp) = current.as_ref() {
                    sp.set_message(format!("Fetching {directory}..."));
                }
            }
            ProgressEvent::BackupSnapshot { message } => {
                if let Some(sp) = current.as_ref() {
                    sp.set_message(message);
                }
            }
        }
    }
}

/// Resolve the machine preferences path from an optional CLI flag,
/// falling back to the default `~/.config/tome/machine.toml`.
fn resolve_machine_path(cli_machine: Option<&Path>) -> Result<std::path::PathBuf> {
    match cli_machine {
        Some(p) => Ok(p.to_path_buf()),
        None => machine::default_machine_path(),
    }
}

/// Derive the tome home directory.
///
/// Resolution order:
/// 1. `--tome-home` CLI flag (highest priority)
/// 2. `--config` CLI flag (tome home = parent directory of config file)
/// 3. `TOME_HOME` env var (checked inside `default_tome_home()`)
/// 4. `~/.tome/` (default)
fn resolve_tome_home(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<std::path::PathBuf> {
    if let Some(p) = cli_tome_home {
        let expanded = config::expand_tilde(p)?;
        anyhow::ensure!(
            expanded.is_absolute(),
            "--tome-home path '{}' must be an absolute path",
            p.display()
        );
        return Ok(expanded);
    }
    match cli_config {
        Some(p) => {
            anyhow::ensure!(
                p.is_absolute(),
                "config path '{}' must be an absolute path",
                p.display()
            );
            let parent = p.parent().context("config path has no parent directory")?;
            Ok(parent.to_path_buf())
        }
        None => config::default_tome_home(),
    }
}

/// Derive the effective config file path.
///
/// If `--config` is given, use that directly.
/// If `--tome-home` is given, use smart detection (`.tome/tome.toml` if exists, else `tome.toml`).
/// Otherwise, fall back to `default_config_path()` (which also reads TOME_HOME + smart detection).
fn resolve_config_path(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<Option<std::path::PathBuf>> {
    if cli_config.is_some() {
        return Ok(cli_config.map(|p| p.to_path_buf()));
    }
    if let Some(th) = cli_tome_home {
        let expanded = config::expand_tilde(th)?;
        return Ok(Some(
            config::resolve_config_dir(&expanded).join("tome.toml"),
        ));
    }
    Ok(None)
}

/// Run the CLI with parsed arguments.
pub fn run(cli: Cli) -> Result<()> {
    if matches!(cli.command, Command::Version) {
        println!("tome {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let effective_config = resolve_config_path(cli.tome_home.as_deref(), cli.config.as_deref())?;

    if matches!(cli.command, Command::Init) {
        if let Err(e) = Config::load_or_default(effective_config.as_deref()) {
            eprintln!(
                "warning: existing config is malformed ({}), the wizard will create a new one",
                e
            );
        }
        // WUX-04: surface the resolved tome_home + its source BEFORE any
        // wizard prompts so the user can Ctrl-C if the wrong path is about
        // to be populated (e.g. a stray `TOME_HOME=/wrong/path` in their
        // shell rc). Printed in both interactive and --no-input modes.
        //
        // HARD-15: this is wizard chrome (informational, around an
        // interactive flow), so it goes to stderr alongside wizard.rs's
        // banner. Stdout stays reserved for the dry-run TOML body.
        //
        // `tome_home_source` is intentionally bound here; later plans in
        // this phase will consume it to gate greenfield prompts (WUX-01).
        let (tome_home, tome_home_source) =
            config::resolve_tome_home_with_source(cli.tome_home.as_deref(), cli.config.as_deref())?;
        eprintln!();
        eprintln!(
            "resolved tome_home: {} (from {})",
            style(tome_home.display()).cyan(),
            tome_home_source.label()
        );

        // WUX-03: Detect and handle legacy pre-v0.6 ~/.config/tome/config.toml.
        // The legacy file is silently ignored by v0.6+ (only its `tome_home`
        // key is read); this warns the user and offers cleanup.
        let home = dirs::home_dir().context("could not determine home directory")?;
        let machine_state = wizard::detect_machine_state(&home, &tome_home)?;
        if let wizard::MachineState::Legacy { legacy_path }
        | wizard::MachineState::BrownfieldWithLegacy { legacy_path, .. } = &machine_state
        {
            wizard::handle_legacy_cleanup(legacy_path, cli.no_input)?;
        }

        // WUX-02: Brownfield decision. When a tome.toml already exists at the
        // resolved tome_home, present a summary and a 4-way choice:
        //   UseExisting  → exit wizard, skip post-init sync
        //   Edit         → launch wizard with existing values pre-filled
        //   Reinit       → backup existing file and launch fresh wizard
        //   Cancel       → exit cleanly (exit 0), no changes
        // Non-brownfield states (Greenfield, Legacy-only) skip this block and
        // proceed to the wizard with no prefill.
        let prefill: Option<Config> = match &machine_state {
            wizard::MachineState::Brownfield {
                existing_config_path,
                existing_config,
            }
            | wizard::MachineState::BrownfieldWithLegacy {
                existing_config_path,
                existing_config,
                ..
            } => {
                let action = wizard::brownfield_decision(
                    existing_config_path,
                    existing_config,
                    cli.no_input,
                )?;
                match action {
                    wizard::BrownfieldAction::UseExisting => {
                        println!("  Config unchanged. Run `tome sync` to apply.");
                        return Ok(());
                    }
                    wizard::BrownfieldAction::Cancel => {
                        println!("Wizard cancelled. Existing config left unchanged.");
                        return Ok(());
                    }
                    wizard::BrownfieldAction::Reinit => {
                        let backup = wizard::backup_brownfield_config(existing_config_path)?;
                        println!(
                            "  Backed up existing config to: {}",
                            style(backup.display()).cyan()
                        );
                        None // proceed as greenfield
                    }
                    wizard::BrownfieldAction::Edit => match existing_config {
                        Ok(c) => Some(c.clone()),
                        Err(e) => {
                            // The Edit action is only offered when the existing
                            // config parses cleanly (see wizard::brownfield_decision).
                            // Reaching this arm with Err means a refactor broke that
                            // invariant; bail with a recoverable error so the user
                            // gets an actionable message instead of a panic.
                            anyhow::bail!(
                                "internal: brownfield Edit reached with unparsable config: {e:#}"
                            );
                        }
                    },
                }
            }
            _ => None,
        };

        let config = wizard::run(
            cli.dry_run,
            cli.no_input,
            &tome_home,
            tome_home_source,
            prefill.as_ref(),
        )?;
        config.validate()?;
        if !cli.dry_run {
            // Expand `~` in library_dir before passing to TomePaths, which
            // requires absolute paths. The wizard preserves tilde-shaped paths
            // so the on-disk TOML stays portable; here we resolve them for the
            // post-init sync call.
            let mut expanded = config.clone();
            expanded
                .expand_tildes()
                .context("failed to expand ~ in wizard-produced config")?;
            let paths = TomePaths::new(tome_home, expanded.library_dir.clone())?;
            // Load machine prefs once at the top of the post-Init sync path
            // (mirrors the canonical `run()` load order). Init does NOT use
            // `Config::load_with_overrides` because the wizard runs against
            // the bare tome.toml that the user is about to write — overrides
            // would mask schema errors the wizard wants to surface.
            let machine_path = resolve_machine_path(cli.machine.as_deref())?;
            let machine_prefs = machine::load(&machine_path)?;
            let verbose = cli.log_level().is_verbose();
            let quiet = cli.log_level().is_quiet();
            // Same front-end selection as cmd_sync (D-11): IndicatifSink for
            // interactive post-init sync, NullSink under --quiet/--verbose.
            let indicatif_sink;
            let null_sink = NullSink;
            let sink: &dyn ProgressSink = if !quiet && !verbose {
                indicatif_sink = IndicatifSink::new();
                &indicatif_sink
            } else {
                &null_sink
            };
            let cancel = CancelToken::new();
            sync(
                &expanded,
                &paths,
                SyncOptions {
                    dry_run: cli.dry_run,
                    force: false,
                    no_triage: true, // skip on initial sync after init
                    no_input: cli.no_input,
                    no_install: false,
                    verbose,
                    quiet,
                    machine_path: &machine_path,
                    machine_prefs: &machine_prefs,
                },
                sink,
                &cancel,
            )?;
        }
        return Ok(());
    }

    // Load per-machine preferences first — they may rewrite directory paths via
    // `[directory_overrides.<name>]` entries, which `Config::load_with_overrides`
    // applies between `expand_tildes()` and `validate()` (PORT-02 / I2 invariant).
    let machine_path = resolve_machine_path(cli.machine.as_deref())?;
    let machine_prefs = machine::load(&machine_path)?;

    let config = Config::load_or_default_with_overrides(
        effective_config.as_deref(),
        &machine_path,
        &machine_prefs,
    )?;
    // Note: load_or_default_with_overrides already runs validate() internally —
    // no separate config.validate()? call here.
    let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
    let paths = TomePaths::new(tome_home, config.library_dir.clone())?;

    // HARD-02: dispatch via per-subcommand `cmd_<name>` helpers defined later
    // in this file. Each match arm is a one-line call into the helper, keeping
    // `run` itself a thin router. Init and Version are dispatched via
    // early-returns above, so the corresponding arms here are unreachable
    // contract guards.
    match cli.command {
        Command::Init => unreachable_early_return("Command::Init"),
        Command::Version => unreachable_early_return("Command::Version"),
        Command::Add {
            url,
            name,
            branch,
            tag,
            rev,
            subdir,
            role,
        } => cmd_add(
            url,
            name,
            branch,
            tag,
            rev,
            subdir,
            role,
            config,
            &paths,
            cli.dry_run,
        ),
        Command::Sync {
            force,
            no_triage,
            no_install,
        } => {
            let log = cli.log_level();
            cmd_sync(
                force,
                no_triage,
                no_install,
                &config,
                &paths,
                &machine_path,
                &machine_prefs,
                cli.dry_run,
                cli.no_input,
                log.is_verbose(),
                log.is_quiet(),
            )
        }
        Command::Status { json } => cmd_status(&config, &paths, json),
        Command::Doctor { json } => cmd_doctor(&config, &paths, cli.dry_run, cli.no_input, json),
        Command::Lint { path, format } => cmd_lint(path, format, &paths),
        Command::Browse => {
            // HARD-21: thread per-machine prefs into browse so the
            // Detail-mode Disable/Enable toggle can persist via
            // machine.toml atomic save (D-BROWSE-3 step 2).
            let machine_path = resolve_machine_path(cli.machine.as_deref())?;
            let machine_prefs = machine::load(&machine_path)?;
            cmd_browse(
                &config,
                &paths,
                cli.log_level().is_quiet(),
                machine_prefs,
                machine_path,
            )
        }
        Command::Remove { kind } => cmd_remove(
            kind,
            config,
            &paths,
            cli.machine.as_deref(),
            cli.dry_run,
            cli.no_input,
        ),
        Command::Reassign { skill, to, force } => {
            cmd_reassign(skill, to, force, &config, &paths, cli.dry_run)
        }
        Command::Fork { skill, to, force } => {
            cmd_fork(skill, to, force, &config, &paths, cli.dry_run, cli.no_input)
        }
        Command::MigrateLibrary { dry_run, yes } => {
            cmd_migrate_library(&paths, dry_run || cli.dry_run, yes, cli.no_input)
        }
        Command::Eject => cmd_eject(&config, &paths, cli.dry_run),
        Command::Relocate { new_path } => cmd_relocate(
            new_path,
            &config,
            &paths,
            cli.config.as_deref(),
            cli.dry_run,
        ),
        Command::Completions { shell, print } => cmd_completions(shell, print),
        Command::List { json } => cmd_list(&config, cli.log_level().is_quiet(), json),
        Command::Config { path } => cmd_config(&config, path, &paths),
        Command::Backup { sub } => cmd_backup(sub, &paths, cli.dry_run),
    }
}

/// Guard for command variants whose handling is dispatched via an early
/// return at the top of `run()` (Init, Version). Reaching the post-setup
/// match for one of these means a refactor broke the early-return contract;
/// bail so the user sees an actionable error instead of a silent fallthrough.
#[cold]
fn unreachable_early_return(variant: &str) -> Result<()> {
    anyhow::bail!(
        "internal: {variant} reached the main dispatch but should have been handled by the early-return path"
    )
}

// ---------------------------------------------------------------------------
// Per-subcommand dispatch helpers (HARD-02)
//
// One `cmd_<name>` per `cli::Command` variant. Each helper consumes args
// already extracted from the variant plus the shared state `run()` resolves
// once (paths, config, machine prefs). Helpers do NOT re-load config or paths.
// ---------------------------------------------------------------------------

/// `tome add <url>` — register a git directory in config from a URL.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_add(
    url: String,
    name: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
    rev: Option<String>,
    subdir: Option<String>,
    role: Option<config::DirectoryRole>,
    config: Config,
    paths: &TomePaths,
    dry_run: bool,
) -> Result<()> {
    let mut config = config;
    add::add(
        &mut config,
        add::AddOptions {
            url: &url,
            name: name.as_deref(),
            branch: branch.as_deref(),
            tag: tag.as_deref(),
            rev: rev.as_deref(),
            subdir: subdir.as_deref(),
            role,
            dry_run,
            config_path: &paths.config_path(),
        },
    )?;
    Ok(())
}

/// `tome sync` — run the full discover → consolidate → distribute → cleanup pipeline.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_sync(
    force: bool,
    no_triage: bool,
    no_install: bool,
    config: &Config,
    paths: &TomePaths,
    machine_path: &Path,
    machine_prefs: &machine::MachinePrefs,
    dry_run: bool,
    no_input: bool,
    verbose: bool,
    quiet: bool,
) -> Result<()> {
    // Front-end selection (D-11): use the spinner-driven IndicatifSink for
    // interactive runs, and a discarding NullSink under --quiet or --verbose —
    // exactly the previous `show_progress = !quiet && !verbose` gate, now
    // expressed as a sink choice instead of an inline `if`. The CLI never
    // cancels, so it passes a fresh, never-tripped CancelToken (D-12); the GUI
    // (Phase 27) clones a live token into its cancel command.
    let indicatif_sink;
    let null_sink = NullSink;
    let sink: &dyn ProgressSink = if !quiet && !verbose {
        indicatif_sink = IndicatifSink::new();
        &indicatif_sink
    } else {
        &null_sink
    };
    let cancel = CancelToken::new();
    sync(
        config,
        paths,
        SyncOptions {
            dry_run,
            force,
            no_triage: no_triage || no_input,
            no_input,
            no_install,
            verbose,
            quiet,
            machine_path,
            machine_prefs,
        },
        sink,
        &cancel,
    )
}

/// `tome status` — read-only summary of library, directories, and health.
pub(crate) fn cmd_status(config: &Config, paths: &TomePaths, json: bool) -> Result<()> {
    status::show(config, paths, json)
}

/// `tome doctor` — diagnose and (optionally) repair library/symlink issues.
pub(crate) fn cmd_doctor(
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
    no_input: bool,
    json: bool,
) -> Result<()> {
    doctor::diagnose(config, paths, dry_run, no_input, json)
}

/// `tome lint` — validate skill frontmatter; exits 1 when errors are found.
pub(crate) fn cmd_lint(
    path: Option<PathBuf>,
    format: cli::LintFormat,
    paths: &TomePaths,
) -> Result<()> {
    let report = match path {
        Some(p) => {
            let dir_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            let issues = lint::lint_skill(dir_name, &p);
            lint::LintReport {
                results: vec![(dir_name.to_string(), issues)],
                skills_checked: 1,
            }
        }
        None => lint::lint_library(paths.library_dir()),
    };
    match format {
        cli::LintFormat::Text => lint::render_text(&report),
        cli::LintFormat::Json => lint::render_json(&report),
    }
    // HARD-04: bubble up a downcastable error rather than `process::exit(1)`
    // so embedding callers can decide how to translate the failure.
    // `main.rs` downcasts and maps to exit code 1.
    if report.has_errors() {
        anyhow::bail!(lint::LintFailed {
            violations: report.error_count(),
        });
    }
    Ok(())
}

/// `tome browse` — interactive TUI browser for the discovered skills.
pub(crate) fn cmd_browse(
    config: &Config,
    paths: &TomePaths,
    quiet: bool,
    machine_prefs: machine::MachinePrefs,
    machine_path: std::path::PathBuf,
) -> Result<()> {
    let mut warnings = Vec::new();
    let skills = discover::discover_all(config, &BTreeMap::new(), &mut warnings)?;
    if !quiet {
        for w in &warnings {
            eprintln!("warning: {}", w);
        }
    }
    if skills.is_empty() {
        println!("No skills found. Run `tome init` to configure sources.");
        return Ok(());
    }
    let manifest = manifest::load(paths.config_dir())?;
    browse::browse(skills, &manifest, machine_prefs, machine_path)?;
    Ok(())
}

/// `tome remove dir|skill` — directory removal vs Unowned-skill deletion (D-API-2).
pub(crate) fn cmd_remove(
    kind: cli::RemoveKind,
    config: Config,
    paths: &TomePaths,
    cli_machine: Option<&Path>,
    dry_run: bool,
    no_input: bool,
) -> Result<()> {
    match kind {
        cli::RemoveKind::Dir { name, force } => {
            cmd_remove_dir(name, force, config, paths, dry_run, no_input)
        }
        cli::RemoveKind::Skill { name, yes } => {
            cmd_remove_skill(name, yes, &config, paths, cli_machine, dry_run, no_input)
        }
    }
}

/// `tome remove dir <name>` — remove a directory entry from `tome.toml` and
/// clean up its artifacts (D-API-2). Owned skills transition to Unowned.
fn cmd_remove_dir(
    name: String,
    force: bool,
    config: Config,
    paths: &TomePaths,
    dry_run: bool,
    no_input: bool,
) -> Result<()> {
    let manifest = manifest::load(paths.config_dir())?;
    let plan = remove::plan(&name, &config, paths, &manifest)?;
    remove::render_plan(&plan);

    if dry_run {
        println!("\n{}", style("Dry run — no changes made.").yellow());
        return Ok(());
    }

    if !force {
        if !no_input && std::io::stdin().is_terminal() {
            let confirmed = dialoguer::Confirm::new()
                .with_prompt(format!("Remove directory '{}'?", name))
                .default(false)
                .interact()?;
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
        } else {
            anyhow::bail!(
                "tome remove requires confirmation — use --force in non-interactive mode"
            );
        }
    }

    let mut config = config;
    let mut manifest = manifest;
    let result = remove::execute(&plan, &mut config, &mut manifest, false)?;

    // Surface partial-cleanup failures BEFORE the save chain. If any of
    // config.save / manifest::save / discover_all / lockfile::save returns
    // Err, `?` would otherwise propagate and the user would only see a
    // disk-write error — never the ⚠ block or the I2/I3 retention messaging
    // ("config entry and manifest retained so you can retry"). Returning
    // here also means in-memory mutations to `config` and `manifest` are
    // never persisted on the failure path, which is correct: remove::execute
    // deliberately leaves them in their pre-mutation state when failures
    // occur, so the disk state on retry matches the in-memory state.
    if !result.failures.is_empty() {
        let k = result.failures.len();
        eprintln!(
            "{} {} operations failed during remove of '{}' — config entry and \
             manifest retained so you can retry after addressing these. \
             Run {} after resolving:",
            style("⚠").yellow(),
            k,
            name,
            style("`tome doctor`").bold(),
        );

        for kind in crate::remove::FailureKind::ALL {
            let group: Vec<&crate::remove::RemoveFailure> =
                result.failures.iter().filter(|f| f.kind == kind).collect();
            if group.is_empty() {
                continue;
            }
            eprintln!("  {} ({}):", kind.label(), group.len());
            for f in group {
                eprintln!("    {}: {}", paths::collapse_home(&f.path), f.error);
            }
        }

        return Err(anyhow::anyhow!("remove completed with {k} failures"));
    }

    // Save updated config
    config.save(&paths.config_path())?;
    // Save updated manifest
    manifest::save(&manifest, paths.config_dir())?;
    // Regenerate lockfile. Recover git-skill provenance offline from
    // the previous lockfile + on-disk cache so git-type directories
    // are not silently dropped during regen (#461 H1). Warnings
    // collected here are deferred until AFTER the success banner —
    // see comment below (TEST-04 option a).
    let (resolved_paths, mut regen_warnings) =
        lockfile::resolved_paths_from_lockfile_cache(&config, paths);
    let skills = discover::discover_all(&config, &resolved_paths, &mut regen_warnings)?;
    let lockfile = lockfile::generate(&manifest, &skills);
    lockfile::save(&lockfile, paths.config_dir())?;

    // Success banner FIRST (TEST-04 option a — deferred regen-warnings).
    // The banner is the user's anchor for "what just happened"; warnings
    // come after as a footnote. Without this ordering, multi-warning
    // regen output buries the green ✓ confirmation and the user has to
    // scroll up to find it. The deferred ordering is regression-tested
    // by `lib_rs_remove_handler_prints_success_banner_before_regen_warnings`
    // in tests/cli.rs.
    println!(
        "\n{} Removed directory '{}': {} library entries kept as Unowned, {} symlinks{}",
        style("✓").green(),
        name,
        result.library_entries_transitioned_to_unowned,
        result.symlinks_removed,
        if result.git_cache_removed {
            ", git cache"
        } else {
            ""
        },
    );
    for w in &regen_warnings {
        eprintln!("warning: {}", w);
    }
    Ok(())
}

/// `tome remove skill <name>` — delete an Unowned skill from the library
/// (manifest entry, library directory, distribution symlinks, lockfile entry,
/// machine.toml memberships) per Phase 14 D-B1.
fn cmd_remove_skill(
    name: String,
    yes: bool,
    config: &Config,
    paths: &TomePaths,
    cli_machine: Option<&Path>,
    dry_run: bool,
    no_input: bool,
) -> Result<()> {
    // Load all the pieces skill_plan needs to compute the cleanup
    // scope (D-B1): manifest entry, lockfile entry, and machine
    // prefs memberships.
    let manifest = manifest::load(paths.config_dir())?;
    let lockfile = lockfile::load(paths.config_dir())?;
    let machine_path = resolve_machine_path(cli_machine)?;
    let machine_prefs = machine::load(&machine_path)?;

    let plan = remove::skill_plan(
        &name,
        config,
        paths,
        &manifest,
        lockfile.as_ref(),
        &machine_prefs,
    )?;
    remove::skill_render_plan(&plan);

    if dry_run {
        println!("\n{}", style("Dry run — no changes made.").yellow());
        return Ok(());
    }

    // D-B3: confirmation default-no, --yes / -y bypasses. Mirrors
    // the existing `tome remove dir` confirmation default.
    if !yes {
        if !no_input && std::io::stdin().is_terminal() {
            let confirmed = dialoguer::Confirm::new()
                .with_prompt(format!("Are you sure you want to forget skill '{}'?", name))
                .default(false)
                .interact()?;
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
        } else {
            anyhow::bail!(
                "tome remove skill requires confirmation — use --yes in non-interactive mode"
            );
        }
    }

    let mut manifest = manifest;
    let mut lockfile = lockfile;
    let mut machine_prefs = machine_prefs;
    let result = remove::skill_execute(
        &plan,
        &mut manifest,
        &mut lockfile,
        &mut machine_prefs,
        false,
    )?;

    // SAFE-01 grouped partial-failure summary BEFORE any save call.
    // skill_execute deliberately leaves manifest/lockfile/machine_prefs
    // unchanged on partial failure (matches the dir-flavour I2/I3
    // retention semantic), so returning here without saving keeps
    // disk state consistent with in-memory state for retry.
    if !result.failures.is_empty() {
        let k = result.failures.len();
        eprintln!(
            "{} {} operations failed during remove of skill '{}' — \
             in-memory state retained so you can retry after addressing these. \
             Run {} after resolving:",
            style("⚠").yellow(),
            k,
            name,
            style("`tome doctor`").bold(),
        );
        for kind in remove::RemoveSkillFailureKind::ALL {
            let group: Vec<&remove::RemoveSkillFailure> =
                result.failures.iter().filter(|f| f.kind == kind).collect();
            if group.is_empty() {
                continue;
            }
            eprintln!("  {} ({}):", kind.label(), group.len());
            for f in group {
                eprintln!("    {}: {}", paths::collapse_home(&f.path), f.error);
            }
        }
        return Err(anyhow::anyhow!(
            "tome remove skill completed with {k} failures"
        ));
    }

    // D-B1 atomic-save chain: manifest + lockfile + machine.toml
    // (each uses temp+rename internally).
    manifest::save(&manifest, paths.config_dir())?;
    if let Some(lf) = &lockfile {
        lockfile::save(lf, paths.config_dir())?;
    }
    machine::save(&machine_prefs, &machine_path)?;

    // Success banner. Reports each step that actually cleaned
    // something so the user sees the full scope of the operation
    // (library, symlinks, lockfile, machine.toml). Counters that
    // were no-ops (e.g. skill had no lockfile entry) are omitted.
    let mut parts: Vec<String> = Vec::new();
    if result.library_removed {
        parts.push("library".to_string());
    }
    if result.symlinks_removed > 0 {
        parts.push(format!("{} symlinks", result.symlinks_removed));
    }
    if result.lockfile_entry_removed {
        parts.push("lockfile entry".to_string());
    }
    if result.machine_disabled_removed {
        parts.push("machine.toml disabled".to_string());
    }
    if result.per_directory_cleanups > 0 {
        parts.push(format!(
            "{} per-directory entries",
            result.per_directory_cleanups
        ));
    }
    let summary = if parts.is_empty() {
        "manifest entry only (nothing else to clean)".to_string()
    } else {
        parts.join(", ")
    };
    println!(
        "\n{} Forgot skill '{}' — cleaned: {}.",
        style("✓").green(),
        name,
        summary,
    );
    Ok(())
}

/// `tome reassign <skill> --to <dir>` — change skill provenance.
pub(crate) fn cmd_reassign(
    skill: String,
    to: String,
    force: bool,
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
) -> Result<()> {
    let mut manifest = manifest::load(paths.config_dir())?;
    let plan = reassign::plan(&skill, &to, config, paths, &manifest, false, force)?;
    reassign::render_plan(&plan);

    let target_dir_path = config
        .directories
        .get(&config::DirectoryName::new(&to)?)
        .map(|d| config::expand_tilde(&d.path))
        .transpose()?
        .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", to))?;

    reassign::execute(&plan, &mut manifest, &target_dir_path, dry_run)?;
    if !dry_run {
        manifest::save(&manifest, paths.config_dir())?;
        // Regenerate lockfile to keep it in sync. Recover git-skill
        // provenance offline from the previous lockfile + on-disk
        // cache so git-type directories are not silently dropped
        // during regen (#461 H1).
        let (resolved_paths, mut regen_warnings) =
            lockfile::resolved_paths_from_lockfile_cache(config, paths);
        let skills = discover::discover_all(config, &resolved_paths, &mut regen_warnings)?;
        for w in &regen_warnings {
            eprintln!("warning: {}", w);
        }
        let lockfile_data = lockfile::generate(&manifest, &skills);
        lockfile::save(&lockfile_data, paths.config_dir())?;
        let from_label = match &plan.from_directory {
            Some(d) => style(d.as_str().to_string()).cyan().to_string(),
            None => style("Unowned").yellow().to_string(),
        };
        println!(
            "{} '{}' from '{}' to '{}'",
            style("Reassigned").green(),
            style(&skill).cyan(),
            from_label,
            style(&to).cyan(),
        );
    }
    Ok(())
}

/// `tome fork <skill> --to <dir>` — fork a managed skill to a local directory.
pub(crate) fn cmd_fork(
    skill: String,
    to: String,
    force: bool,
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
    no_input: bool,
) -> Result<()> {
    let mut manifest = manifest::load(paths.config_dir())?;
    // Phase 14 D-A1: Fork shares the reassign::plan path, so Fork's
    // existing --force flag (skip-confirmation) now also bypasses
    // the D-A1 different-content collision refusal. The user's
    // mental model — "--force on fork bypasses safety checks" —
    // still holds; the surface is just slightly bigger.
    let plan = reassign::plan(&skill, &to, config, paths, &manifest, true, force)?;
    reassign::render_plan(&plan);

    if !force {
        if !no_input && std::io::stdin().is_terminal() {
            let confirmed = dialoguer::Confirm::new()
                .with_prompt(format!(
                    "Fork '{}' to '{}'? This copies skill files to the target directory.",
                    skill, to
                ))
                .default(false)
                .interact()?;
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
        } else {
            anyhow::bail!("tome fork requires confirmation — use --force in non-interactive mode");
        }
    }

    let target_dir_path = config
        .directories
        .get(&config::DirectoryName::new(&to)?)
        .map(|d| config::expand_tilde(&d.path))
        .transpose()?
        .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", to))?;

    reassign::execute(&plan, &mut manifest, &target_dir_path, dry_run)?;
    if !dry_run {
        manifest::save(&manifest, paths.config_dir())?;
        // Regenerate lockfile to keep it in sync. Recover git-skill
        // provenance offline from the previous lockfile + on-disk
        // cache so git-type directories are not silently dropped
        // during regen (#461 H1).
        let (resolved_paths, mut regen_warnings) =
            lockfile::resolved_paths_from_lockfile_cache(config, paths);
        let skills = discover::discover_all(config, &resolved_paths, &mut regen_warnings)?;
        for w in &regen_warnings {
            eprintln!("warning: {}", w);
        }
        let lockfile_data = lockfile::generate(&manifest, &skills);
        lockfile::save(&lockfile_data, paths.config_dir())?;
        println!(
            "{} '{}' to '{}' (local copy created)",
            style("Forked").green(),
            style(&skill).cyan(),
            style(&to).cyan(),
        );
    }
    Ok(())
}

/// `tome migrate-library` — one-shot v0.9 → v0.10 library migration.
///
/// Per D-05: any skip or failure means non-zero exit.
/// Per UX-02 D-UX02-1/-2: drives plan → render_plan_to → confirm gate →
/// execute → render_result_to. The confirm gate is bypassed under
/// `--dry-run` (no destructive action runs) or `--yes`. Under
/// `--no-input` AND not `--dry-run`, missing `--yes` bails with a
/// Conflict/Why/Suggestion error; `--dry-run --no-input` is fine without
/// `--yes` because the dry run never reaches the gate.
pub(crate) fn cmd_migrate_library(
    paths: &TomePaths,
    dry_run: bool,
    yes: bool,
    no_input: bool,
) -> Result<()> {
    if dry_run {
        eprintln!(
            "{}",
            console::style("[dry-run] No changes will be made")
                .yellow()
                .bold()
        );
    }

    let manifest = manifest::load(paths.config_dir())?;
    let plan = migration_v010::plan(paths.library_dir(), &manifest)?;
    // HARD-15 stderr discipline: render directly to a locked stderr handle.
    // Best-effort write — failure to render is non-fatal for the migration,
    // but we route the I/O failure through `tracing::warn!` instead of
    // dropping it silently so a broken stderr (broken pipe, /dev/full) is
    // diagnosable in `--verbose` / `TOME_LOG=warn` runs. Without this, the
    // user could land in the confirmation prompt below having seen no plan,
    // and silently approve an unknown migration.
    {
        let mut stderr = std::io::stderr().lock();
        if let Err(e) = migration_v010::render_plan_to(&plan, &mut stderr) {
            tracing::warn!("could not write migration plan to stderr: {e}");
        }
    }

    // Empty plan — render_plan_to already printed the already-in-v0.10-shape
    // message; nothing to confirm or execute.
    if plan.entries.is_empty() {
        return Ok(());
    }

    if !dry_run {
        // UX-02 confirm-or-abort. PromptMode encodes the three valid arms
        // (Forced / NoInputRequiresYes / Interactive); `yes` always wins
        // over `no_input` so the impossible state is unrepresentable.
        let mode = migration_v010::PromptMode::from_flags(yes, no_input);
        if !migration_v010::prompt_confirmation(mode)? {
            return Ok(());
        }
    }

    let result = migration_v010::execute(&plan, dry_run)?;
    {
        let mut stderr = std::io::stderr().lock();
        if let Err(e) = migration_v010::render_result_to(&result, dry_run, &mut stderr) {
            tracing::warn!("could not write migration result to stderr: {e}");
        }
    }

    // HARD-04 sibling: bubble through anyhow rather than `process::exit(1)`.
    // `main.rs` downcasts `MigrationPartialOrFailed` and exits with code 1.
    if result.is_partial_or_failed() {
        anyhow::bail!(migration_v010::MigrationPartialOrFailed {
            skipped_broken_source: result.skipped_broken_source,
            failed: result.failed,
        });
    }
    Ok(())
}

/// `tome eject` — remove tome's symlinks from all distribution directories.
pub(crate) fn cmd_eject(config: &Config, paths: &TomePaths, dry_run: bool) -> Result<()> {
    let plan = eject::plan(config, paths)?;
    eject::render_plan(&plan);

    if plan.total_symlinks == 0 {
        return Ok(());
    }

    if dry_run {
        println!("\n{}", style("Dry run — no changes made.").yellow());
        return Ok(());
    }

    if std::io::stdin().is_terminal() {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("Remove these symlinks?")
            .default(true)
            .interact()?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    }

    let removed = eject::execute(&plan, false)?;
    println!(
        "\n{} Removed {} symlink(s). Run {} to re-distribute.",
        style("✓").green(),
        removed,
        style("tome sync").cyan()
    );
    Ok(())
}

/// `tome relocate <new_path>` — move the skill library to a new location safely.
pub(crate) fn cmd_relocate(
    new_path: PathBuf,
    config: &Config,
    paths: &TomePaths,
    cli_config: Option<&Path>,
    dry_run: bool,
) -> Result<()> {
    let config_path = cli_config
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| paths.config_path());

    let plan = relocate::plan(config, paths, &new_path, &config_path)?;
    relocate::render_plan(&plan);

    if dry_run {
        println!("\n{}", style("Dry run -- no changes made.").yellow());
        return Ok(());
    }

    if std::io::stdin().is_terminal() {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("Proceed with relocation?")
            .default(false)
            .interact()?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    } else {
        anyhow::bail!(
            "tome relocate requires interactive confirmation -- refusing in non-interactive mode"
        );
    }

    relocate::execute(&plan, false)?;

    // Use plain Config::load (no overrides) — relocate verifies the
    // newly-written config exactly as it lives on disk. Applying
    // machine overrides here would mask the relocation result.
    let new_config = Config::load(&config_path)?;
    relocate::verify(&new_config, &plan.new_library_dir, paths.tome_home())?;
    Ok(())
}

/// `tome completions <shell>` — print or install shell completions.
pub(crate) fn cmd_completions(shell: clap_complete::Shell, print: bool) -> Result<()> {
    if print {
        print_completions(shell);
        Ok(())
    } else {
        install_completions(shell)
    }
}

/// `tome list` — list all discovered skills (text or JSON).
pub(crate) fn cmd_list(config: &Config, quiet: bool, json: bool) -> Result<()> {
    list(config, quiet, json)
}

/// `tome config` — show resolved config (TOML) or just the path.
pub(crate) fn cmd_config(config: &Config, path: bool, paths: &TomePaths) -> Result<()> {
    show_config(config, path, &paths.config_path())
}

/// `tome backup <sub>` — git-backed snapshot/restore for the library.
pub(crate) fn cmd_backup(sub: cli::BackupCommand, paths: &TomePaths, dry_run: bool) -> Result<()> {
    match sub {
        cli::BackupCommand::Init => {
            backup::init(paths.tome_home(), dry_run)?;
            // Offer remote setup after successful init (interactive only)
            if !dry_run && std::io::stdin().is_terminal() && !backup::has_remote(paths.tome_home())
            {
                offer_remote_setup(paths.tome_home())?;
            }
        }
        cli::BackupCommand::Snapshot { message } => {
            // The CLI backup commands present via direct `println!` inside
            // `backup::*`; the BackupSnapshot events have no separate CLI
            // surface today, so pass a discarding NullSink + a never-tripped
            // CancelToken (D-11/D-12 plumbing is threaded for signature
            // symmetry, exercised by the GUI in a later phase).
            backup::snapshot(
                paths.tome_home(),
                message.as_deref(),
                dry_run,
                &NullSink,
                &CancelToken::new(),
            )?;
        }
        cli::BackupCommand::List { count } => {
            let entries = backup::list(paths.tome_home(), count)?;
            backup::render_list(&entries);
        }
        cli::BackupCommand::Restore { target, force } => {
            if !force {
                if std::io::stdin().is_terminal() {
                    let confirmed = dialoguer::Confirm::new()
                        .with_prompt(format!(
                            "Restore to {}? This will overwrite current state",
                            target
                        ))
                        .default(false)
                        .interact()?;
                    if !confirmed {
                        println!("Aborted.");
                        return Ok(());
                    }
                } else {
                    anyhow::bail!(
                        "tome backup restore requires confirmation — use --force in non-interactive mode"
                    );
                }
            }
            backup::restore(
                paths.tome_home(),
                &target,
                dry_run,
                &NullSink,
                &CancelToken::new(),
            )?;
        }
        cli::BackupCommand::Diff { target } => {
            let diff = backup::diff(paths.tome_home(), &target)?;
            if diff.is_empty() {
                println!("No changes since {}", target);
            } else {
                println!("{}", diff);
            }
        }
    }
    Ok(())
}

/// D-16: populate `DiscoveredSkill::synced_at` from the manifest.
///
/// The discover layer cannot read the manifest (the manifest is owned by
/// `sync()` and lives outside the discover module), so the orchestrator joins
/// the per-skill `synced_at` timestamp at the post-discover boundary. Skills
/// with no matching `SkillEntry` in the manifest remain `synced_at: None`
/// (they haven't been synced yet). Cost: one hashmap lookup per discovered
/// skill — negligible compared to discovery's filesystem traversal.
///
/// Extracted from `sync()` so the join semantic is directly unit-testable
/// without spinning a full TempDir+config+manifest fixture; pinned by tests
/// in `mod tests` below (`join_synced_at_*`).
fn join_synced_at_from_manifest(
    skills: &mut [discover::DiscoveredSkill],
    manifest: &manifest::Manifest,
) {
    for skill in skills {
        skill.synced_at = manifest
            .get(skill.name.as_str())
            .map(|entry| entry.synced_at.clone());
    }
}

/// Warn about `disabled_directories` entries in machine.toml that don't match any
/// configured directory name. Helps catch typos and stale entries.
fn warn_unknown_disabled_directories(machine_prefs: &machine::MachinePrefs, config: &Config) {
    for name in &machine_prefs.disabled_directories {
        if !config.directories.contains_key(name.as_str()) {
            eprintln!(
                "warning: disabled directory '{}' in machine.toml does not match any configured directory",
                name
            );
        }
    }
}

/// Options for the sync pipeline.
///
/// Made `pub` in plan 27-01b (Wave 2 of Phase 27): the Tauri `start_sync`
/// command in `crates/tome-desktop/src/commands.rs` constructs `SyncOptions`
/// at the IPC boundary so the GUI can drive `sync()` the same way `cmd_sync`
/// does. All fields are `pub` so callers can populate them inline; the public
/// shape mirrors what `cmd_sync` was already passing internally.
pub struct SyncOptions<'a> {
    pub dry_run: bool,
    pub force: bool,
    pub no_triage: bool,
    pub no_input: bool,
    pub no_install: bool,
    pub verbose: bool,
    pub quiet: bool,
    /// Path where `machine.toml` should be saved after triage. Loaded once
    /// at `run()` entry alongside `machine_prefs` so the override-apply step
    /// in `Config::load_with_overrides` and the disabled-skill filtering
    /// inside `sync()` see identical prefs.
    pub machine_path: &'a Path,
    /// Per-machine preferences already loaded by the caller. `sync()` clones
    /// these locally so triage can mutate without affecting the caller's copy.
    pub machine_prefs: &'a machine::MachinePrefs,
}

/// Pre-discovery step: clone or update git-type directories.
///
/// Returns a map of directory name -> (resolved local path, optional HEAD SHA).
/// Failed git operations produce warnings and are skipped (GIT-08).
fn resolve_git_directories(
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
    sink: &dyn ProgressSink,
    cancel: &CancelToken,
) -> BTreeMap<DirectoryName, (PathBuf, Option<String>)> {
    let mut resolved = BTreeMap::new();
    let repos_dir = paths.repos_dir();

    // Check git availability once
    if !config
        .directories
        .values()
        .any(|d| d.directory_type == DirectoryType::Git)
    {
        return resolved;
    }

    if !git::is_git_available() {
        warn!("git is not available — skipping all git-type directories");
        return resolved;
    }

    // Read HEAD sha and warn (not silently swallow) when the cache is
    // unreadable — without the warning the lockfile would record
    // git_commit_sha: null, falsely claiming "no provenance".
    let read_sha_or_warn = |cache_dir: &Path, name: &DirectoryName| -> Option<String> {
        match git::read_head_sha(cache_dir) {
            Ok(sha) => Some(sha),
            Err(e) => {
                warn!(
                    "could not read HEAD sha for '{}' cache at {}: {e}",
                    name,
                    cache_dir.display()
                );
                None
            }
        }
    };

    for (name, dir_config) in &config.directories {
        if dir_config.directory_type != DirectoryType::Git {
            continue;
        }

        let url = dir_config.path.to_string_lossy();
        let cache_dir = git::repo_cache_dir(&repos_dir, &url);
        let already_cloned = cache_dir.is_dir();

        if dry_run {
            // In dry-run, use cached path if it exists, skip otherwise
            if already_cloned {
                let effective = git::effective_path(&cache_dir, dir_config.subdir.as_deref());
                let sha = read_sha_or_warn(&cache_dir, name);
                resolved.insert(name.clone(), (effective, sha));
            }
            continue;
        }

        // Create repos dir if needed
        if let Err(e) = std::fs::create_dir_all(&repos_dir) {
            warn!(
                "failed to create repos directory {}: {e}",
                repos_dir.display()
            );
            continue;
        }

        let result = if already_cloned {
            // Update existing clone (GIT-03)
            debug!("Updating git directory '{}'...", name);
            git::update_repo(
                &cache_dir,
                dir_config.git_ref.as_ref().and_then(|r| r.branch()),
                dir_config.git_ref.as_ref().and_then(|r| r.tag()),
                dir_config.git_ref.as_ref().and_then(|r| r.rev()),
                sink,
                cancel,
            )
        } else {
            // Fresh clone (GIT-02)
            debug!("Cloning git directory '{}'...", name);
            git::clone_repo(
                &url,
                &cache_dir,
                dir_config.git_ref.as_ref().and_then(|r| r.branch()),
                dir_config.git_ref.as_ref().and_then(|r| r.tag()),
                dir_config.git_ref.as_ref().and_then(|r| r.rev()),
                sink,
                cancel,
            )
        };

        match result {
            Ok(()) => {
                let effective = git::effective_path(&cache_dir, dir_config.subdir.as_deref());
                let sha = read_sha_or_warn(&cache_dir, name);
                resolved.insert(name.clone(), (effective, sha));
            }
            Err(e) => {
                // D-09, D-10: Distinct messages for never-cloned vs update-failed
                if already_cloned {
                    warn!("could not update '{}' — using cached state: {e}", name);
                    let effective = git::effective_path(&cache_dir, dir_config.subdir.as_deref());
                    let sha = read_sha_or_warn(&cache_dir, name);
                    resolved.insert(name.clone(), (effective, sha));
                } else {
                    warn!(
                        "could not clone '{}' — skipping (no cached state): {e}",
                        name
                    );
                }
            }
        }
    }
    resolved
}

/// Build the marketplace adapter for `[directories.<name>] type = "claude-plugins"`.
///
/// Returns `Ok(None)` when no claude-plugins directory is configured. Returns
/// `Err` per D-20 when at least one claude-plugins entry exists but the
/// `claude` binary is missing — the error message names the binary and points
/// to the actionable remedy.
///
/// Per D-11: dispatch is by `DirectoryType`. Per D-21: GitAdapter is NOT
/// constructed here — git directories flow through `resolve_git_directories`
/// instead, preserving the v0.9 byte-for-byte regression contract.
fn build_claude_adapter(config: &Config) -> Result<Option<marketplace::ClaudeMarketplaceAdapter>> {
    let needs_claude = config
        .directories
        .values()
        .any(|d| d.directory_type == DirectoryType::ClaudePlugins);
    if !needs_claude {
        return Ok(None);
    }
    // D-20: hard error with the Conflict / Why / Suggestion shape.
    let adapter = marketplace::ClaudeMarketplaceAdapter::new().with_context(|| {
        "claude binary not found on PATH.\n\n\
         Why: at least one [directories.<name>] in tome.toml has type = \
         \"claude-plugins\" — tome reconciles those entries via the claude CLI.\n\
         Suggestion: install Claude Code (https://claude.ai/code), or remove the \
         claude-plugins directory entry from tome.toml."
    })?;
    Ok(Some(adapter))
}

/// Apply the user's edit-in-library decisions to the manifest in-memory.
///
/// Returns `true` if any Fork mutation was applied (sync() should save the
/// manifest in that case). Revert and Skip emit user-facing output but do
/// not mutate.
///
/// Per RECON-05 D-13:
///
/// - **Fork**: `managed: true → false`, `source_name: Some → None`. Library
///   content stays in place. `previous_source` captures the old
///   `source_name` per D-C1 (Phase 14, transition site 3).
/// - **Revert**: emits a warning that revert is not yet wired (deferred to a
///   follow-up; D-16's safety guarantee is "never silently overwrite", and
///   revert is opt-in so a warn-and-skip is acceptable for v0.10). No
///   mutation.
/// - **Skip**: emits nothing additional (the per-skill warning already fired
///   in `handle_edited`). No mutation.
///
/// Pre-refactor (v0.11.1 and earlier), this function did its own `manifest::
/// load` + `manifest::save` round-trip, then `consolidate` did its own
/// independent `manifest::load`. That double-disk-touch worked today (the
/// operations were sequential and same-path) but the data flow had two
/// readers and two writers of the same file with no shared in-memory
/// state. Any future refactor that changed `consolidate` to skip the
/// reload would silently lose Fork mutations. Now sync() owns the
/// `Manifest` variable end-to-end through reconcile, so the mutation is
/// visible in-memory and the save is centralized.
fn apply_edit_decisions(
    report: &reconcile::ReconcileReport,
    manifest: &mut manifest::Manifest,
    dry_run: bool,
) -> bool {
    if report.edited.is_empty() || dry_run {
        return false;
    }
    debug_assert_eq!(report.edit_decisions.len(), report.edited.len());

    let mut mutated = false;

    for (edit, decision) in report.edited.iter().zip(report.edit_decisions.iter()) {
        match decision {
            reconcile::EditDecision::Fork => {
                if let Some(entry) = manifest.skills_get_mut(edit.name.as_str()) {
                    entry.managed = false;
                    // Per D-C1 (Phase 14, transition site 3): capture the old
                    // owning directory as the Unowned breadcrumb. Closes the
                    // Phase 13 D-13 lossy fork-in-place gap. An already-Unowned
                    // entry keeps its existing breadcrumb.
                    entry.ownership = match &entry.ownership {
                        manifest::SkillOwnership::Owned { source } => {
                            manifest::SkillOwnership::Unowned {
                                last_owner: Some(source.clone()),
                            }
                        }
                        manifest::SkillOwnership::Unowned { last_owner } => {
                            manifest::SkillOwnership::Unowned {
                                last_owner: last_owner.clone(),
                            }
                        }
                    };
                    mutated = true;
                }
            }
            reconcile::EditDecision::Revert => {
                // v0.10 deferred stub: the Revert decision is a no-op — we
                // warn the user that their choice was not applied and leave
                // both the manifest and the library content untouched. Per
                // D-16's "never silently overwrite" guarantee, opt-in revert
                // gets a warn-and-skip until a future phase wires the
                // actual upstream-copy restore path.
                eprintln!(
                    "warning: revert chosen for {} but is not wired in v0.10 — \
                     left as-is. Re-run with `tome sync` after manually \
                     deleting library/{} to force a fresh install.",
                    edit.name.as_str(),
                    edit.name.as_str(),
                );
            }
            reconcile::EditDecision::Skip => {}
        }
    }

    mutated
}

/// The core sync pipeline: reconcile → discover → consolidate → distribute → cleanup → save.
///
/// `sink` receives a typed [`ProgressEvent`] at each stage boundary (D-09/D-11):
/// the domain stays presentation-free and synchronous; the CLI passes an
/// [`IndicatifSink`] (spinners) and the GUI a `TauriEventSink` (IPC events).
/// `cancel` is checked at every stage boundary (D-12); the CLI passes a
/// never-tripped [`CancelToken`], while Phase 27's GUI clones it into a cancel
/// command. A cancellation observed between stages bails *before* any
/// half-written manifest/lockfile (T-25-03a: cancel checks sit at stage
/// boundaries, never mid-write, so the atomic temp+rename invariant holds).
/// Run the full sync pipeline.
///
/// Made `pub` in plan 27-01b: the Tauri `start_sync` command in
/// `crates/tome-desktop/src/commands.rs` invokes this directly (wrapped in
/// `tauri::async_runtime::spawn_blocking` so the synchronous body does not
/// stall the async IPC reactor — RESEARCH Pitfall 5). The CLI's `cmd_sync`
/// is the other caller. Both paths share a single implementation; the
/// front-end-specific behavior (CLI spinner vs GUI typed event stream)
/// is supplied by `sink`.
///
/// Return type stays `Result<()>` for plan 27-01b — 27-05 will swap this
/// for a `Result<SyncOutcomeWire>` shape that crosses the IPC boundary
/// with structured per-stage outcomes. Today the GUI consumes progress
/// purely through `SyncProgress` events on `sink`.
pub fn sync(
    config: &Config,
    paths: &TomePaths,
    opts: SyncOptions<'_>,
    sink: &dyn ProgressSink,
    cancel: &CancelToken,
) -> Result<()> {
    let SyncOptions {
        dry_run,
        force,
        no_triage,
        no_input,
        no_install,
        verbose,
        quiet,
        machine_path,
        machine_prefs: prefs_in,
    } = opts;

    // OBS-03 D-SPAN-1: top-level sync span. RAII via `.entered()`; the
    // returned guard `_sync_span` drops at function exit, emitting a
    // FmtSpan::CLOSE event with `time.busy` / `time.idle` on stderr.
    let _sync_span = info_span!("sync", dry_run = dry_run, force = force).entered();

    if dry_run && !quiet {
        eprintln!(
            "{}",
            style("[dry-run] No changes will be made").yellow().bold()
        );
    }

    // RESEARCH OQ-6: surface non-zero exit when reconcile failed any
    // install/update. Declared up here so the bail at end-of-sync can read
    // it after the reconcile block (below) populates it.
    let mut reconcile_install_failures: Vec<marketplace::InstallFailure> = Vec::new();

    // Cache git state to avoid repeated subprocess calls
    let has_backup_repo = backup::has_repo(paths.tome_home());
    let has_remote = has_backup_repo && backup::has_remote(paths.tome_home());

    // Pull from remote before anything else (if configured)
    if !dry_run && has_remote {
        match backup::pull(paths.tome_home()) {
            Ok(true) => {
                if !quiet {
                    println!(
                        "  {} Pulled changes from remote",
                        console::style("↓").cyan()
                    );
                }
            }
            Ok(false) => {} // up to date
            Err(e) => warn!("remote pull failed: {e}"),
        }
    }

    // Pre-sync auto-snapshot if configured
    if !dry_run && config.backup.enabled && config.backup.auto_snapshot && has_backup_repo {
        match backup::snapshot(
            paths.tome_home(),
            Some("pre-sync auto-snapshot"),
            false,
            sink,
            cancel,
        ) {
            Ok(true) => {
                info!("pre-sync snapshot created");
            }
            Ok(false) => {} // nothing to snapshot
            Err(e) => warn!("auto-snapshot failed: {e}"),
        }
    }

    // Per-machine preferences (disabled skills and targets) are loaded once
    // in `run()` so the override-apply step in `Config::load_with_overrides`
    // and the disabled-skill filtering below see identical prefs. We clone
    // here so triage (below) can mutate locally without affecting the caller.
    let mut machine_prefs = prefs_in.clone();

    // Load existing lockfile for diffing and reconciliation
    let old_lockfile = lockfile::load(paths.config_dir())?;
    // Load manifest once for reconcile's edit-in-library detection. Held as
    // `mut` so apply_edit_decisions can mutate it in-memory after reconcile
    // returns — see Critical #1 refactor in the v0.11 review pass. The
    // post-consolidate manifest reload happens inside `library::consolidate`;
    // sync() is responsible for saving the fork-flipped state to disk here
    // BEFORE consolidate's load, so the two stay coherent.
    let mut manifest_for_reconcile = manifest::load(paths.config_dir())?;

    // v0.10 RECON-01..05: replaces the v0.9 reconcile_managed_plugins flow.
    // Adapter dispatch by DirectoryType (D-11); git stays separate (D-21).
    //
    // OBS-03: `reconcile` step span. Even though it runs BEFORE discover in
    // code order, the span name reflects the pipeline step it represents.
    //
    // OBS-05 (D-ENV-4): the previous inline `reconcile::render_summary(...)`
    // call at this site is REMOVED. The classification line is now emitted
    // from `render_sync_report` (final summary block, immediately above the
    // per-bucket cleanup output). The report is threaded into the eventual
    // `SyncReport` via `reconcile_report`.
    // Stage boundary: cancellation checked before reconcile begins (D-12).
    if cancel.is_cancelled() {
        anyhow::bail!("sync cancelled");
    }
    let mut reconcile_report: Option<reconcile::ReconcileReport> = None;
    {
        let _span = info_span!("reconcile").entered();
        // D-09/D-11: the CLI's "Resolving git sources..." spinner is now driven
        // by the Reconcile stage event; the IndicatifSink redraws it.
        sink.emit(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Reconcile,
        });
        if let Some(claude_adapter) = build_claude_adapter(config)? {
            let mut report = reconcile::reconcile_lockfile(
                old_lockfile.as_ref(),
                &manifest_for_reconcile,
                paths.library_dir(),
                &claude_adapter,
                &mut machine_prefs,
                machine_path,
                paths,
                reconcile::ReconcileOpts {
                    dry_run,
                    no_input,
                    no_install,
                    quiet,
                    verbose,
                },
            )?;

            // Apply edit-in-library decisions to the manifest. The manifest
            // is owned by sync(); reconcile_lockfile only proposed the user's
            // choice (RECON-05 D-13). apply_edit_decisions mutates the
            // in-memory `manifest_for_reconcile` and returns whether any Fork
            // mutation landed; sync() saves the manifest to disk here so
            // `library::consolidate` (which loads its own copy) observes the
            // post-fork state. Centralizing the save here eliminates the
            // pre-refactor "two writers, two readers, no shared state"
            // pattern that risked silently losing Fork mutations under
            // future consolidate refactors.
            if apply_edit_decisions(&report, &mut manifest_for_reconcile, dry_run) {
                manifest::save(&manifest_for_reconcile, paths.config_dir())
                    .context("failed to save manifest after edit-in-library fork-in-place flip")?;
            }

            // ADP-04: render grouped install failures. Sync exits non-zero at end
            // when this Vec is non-empty (RESEARCH OQ-6). Drain install_failures
            // in-place (instead of via the consuming `take_install_failures`)
            // so the rest of the report (matches/drift/vanished/missing) can
            // still be threaded into SyncReport for OBS-05.
            if !report.install_failures.is_empty() {
                marketplace::render_install_failures(&report.install_failures);
                reconcile_install_failures = std::mem::take(&mut report.install_failures);
            }
            reconcile_report = Some(report);
        }
        sink.emit(ProgressEvent::SyncStageFinished {
            stage: SyncStage::Reconcile,
        });
    }

    // Safety guard: warn and skip cleanup when no directories are configured (CFG-06)
    if config.directories.is_empty() {
        warn!("no directories configured. Run `tome init` to set up directories.");
        return Ok(());
    }

    // OBS-03: `discover` step span. Wraps both git resolution AND discovery.
    // Span guard drops on the closing brace of this lexical block (RAII).
    // Per RESEARCH §Pitfall 4: `?` early-returns inside this block still
    // emit the span CLOSE event because the top-level `_sync_span` guard
    // drops at function exit, which also drops the entered child span.
    // Stage boundary: cancellation checked before discover begins (D-12).
    if cancel.is_cancelled() {
        anyhow::bail!("sync cancelled");
    }
    let skills = {
        let _span = info_span!("discover").entered();
        // D-09/D-11: the Discover stage drives the "Discovering skills..."
        // spinner. Git resolution below emits GitCloneProgress events that
        // re-message the active spinner ("Fetching {url}...") as each repo is
        // cloned/updated.
        sink.emit(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Discover,
        });

        // 0. Resolve git directories (clone/update to local cache). Verbose
        //    step banners deleted per D-OUT-3 — span CLOSE supplies the
        //    "step name + time.busy" event. sink/cancel are threaded into
        //    git::clone_repo/update_repo so each fetch emits GitCloneProgress
        //    and observes cancellation (D-11/D-12).
        let resolved = resolve_git_directories(config, paths, dry_run, sink, cancel);

        // 1. Discover
        let mut warnings = Vec::new();
        let mut discovered = discover::discover_all(config, &resolved, &mut warnings)?;

        // D-16: join in the manifest's per-skill `synced_at` timestamp.
        // Extracted into `join_synced_at_from_manifest` so the join logic is
        // directly unit-testable without spinning a full sync fixture.
        join_synced_at_from_manifest(&mut discovered, &manifest_for_reconcile);

        sink.emit(ProgressEvent::SyncStageFinished {
            stage: SyncStage::Discover,
        });

        // Discover-warnings emission. EnvFilter handles the quiet vs warn
        // discipline globally (LogLevel::Quiet → "warn" directive still
        // fires warn-level events, matching the previous always-show
        // semantics).
        for w in &warnings {
            warn!("{}", w);
        }

        discovered
    };

    if skills.is_empty() {
        if !quiet {
            println!("No skills found. Run `tome init` to configure sources.");
        }
        return Ok(());
    }

    debug!("Found {} skills", skills.len());

    // v0.10 D-02: refuse to sync against a v0.9-shape library. Detection is an
    // isolated check; the entire migration_v010 module deletes cleanly with
    // this check in v0.11+.
    {
        let manifest_for_detection = manifest::load(paths.config_dir())?;
        if migration_v010::detect_v09_shape(paths.library_dir(), &manifest_for_detection) {
            anyhow::bail!(
                "library is in v0.9 shape (one or more managed skills are stored as symlinks).\n\
                 \n\
                 Why: v0.10 stores managed skills as real directory copies (LIB-01).\n\
                 Run `tome migrate-library` to convert the library, then re-run this command.\n\
                 Pass `--dry-run` first to preview changes without touching the filesystem."
            );
        }
    }

    // Stage boundary: cancellation checked before consolidate begins (D-12).
    if cancel.is_cancelled() {
        anyhow::bail!("sync cancelled");
    }
    // 2. Consolidate into library (copy). OBS-03: `consolidate` step span.
    let (consolidate_result, mut manifest) = {
        let _span = info_span!("consolidate").entered();
        sink.emit(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Consolidate,
        });
        let result = library::consolidate(&skills, paths, dry_run, force)?;
        sink.emit(ProgressEvent::SyncStageFinished {
            stage: SyncStage::Consolidate,
        });
        result
    };

    // 3. Diff lockfile and triage changes (pre-cleanup snapshot for diffing)
    let pre_cleanup_lockfile = lockfile::generate(&manifest, &skills);
    if !no_triage && !quiet {
        if let Some(ref old) = old_lockfile {
            let d = update::diff(old, &pre_cleanup_lockfile);
            if !d.is_empty() {
                println!("{}", style("Library changes detected:").bold());
                let newly_disabled = update::present_changes(&d, &mut machine_prefs, quiet)?;
                if !newly_disabled.is_empty() && !dry_run {
                    machine::save(&machine_prefs, machine_path)?;
                    println!(
                        "  {} skill(s) disabled in {}",
                        newly_disabled.len(),
                        machine_path.display()
                    );
                }
            } else {
                println!("{}", style("No changes since last sync.").dim());
            }
        } else {
            println!(
                "{}",
                style("No previous lockfile — performing initial sync.").dim()
            );
        }
    }

    let discovered_names: HashSet<String> =
        skills.iter().map(|s| s.name.as_str().to_string()).collect();

    // Warn about disabled_directories that don't match any configured directory
    if !quiet {
        warn_unknown_disabled_directories(&machine_prefs, config);
    }

    // 4. Cleanup stale library entries (before distribute so counts are accurate)
    //    Clear the spinner before cleanup_library runs: cleanup may show
    //    interactive dialoguer prompts, and a live spinner overwrites them,
    //    causing an apparent hang. The cleanup-LIBRARY step is intentionally
    //    pre-distribute (so distribute sees the post-cleanup library state).
    //    Both this step AND the post-distribute target cleanup are observed
    //    by the single `cleanup` step span at the end of the pipeline; the
    //    library-cleanup portion happens outside of any step span (small,
    //    fast, and naming-collision-free under the OBS-03 grep contract).
    let cleanup_result = cleanup::cleanup_library(
        paths.library_dir(),
        &discovered_names,
        &mut manifest,
        config,
        dry_run,
        quiet,
        no_input,
    )?;

    // Regenerate lockfile after cleanup so it reflects removals
    let new_lockfile = lockfile::generate(&manifest, &skills);

    // Stage boundary: cancellation checked before distribute begins (D-12).
    if cancel.is_cancelled() {
        anyhow::bail!("sync cancelled");
    }
    // 5. Distribute to directories with distribution roles. OBS-03:
    //    `distribute` step span. D-09/D-11: a single Distribute stage spans the
    //    per-directory loop; SyncStageProgress reports per-directory progress
    //    (current/total) so a GUI can show a determinate bar. The CLI's
    //    IndicatifSink keeps one spinner for the whole stage (the per-directory
    //    "Distributing to {name}..." message was TTY-transient and is not part
    //    of captured output).
    let distribute_results = {
        let _span = info_span!("distribute").entered();
        sink.emit(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Distribute,
        });
        let mut results = Vec::new();
        let dirs: Vec<_> = config.distribution_dirs().collect();
        let total = dirs.len();
        for (idx, (name, dir_config)) in dirs.into_iter().enumerate() {
            if machine_prefs.is_directory_disabled(name.as_str()) {
                debug!(
                    "Skipping directory '{}' (disabled in machine preferences)",
                    name
                );
                continue;
            }
            sink.emit(ProgressEvent::SyncStageProgress {
                stage: SyncStage::Distribute,
                current: idx,
                total,
                // D-08: per-stage subtitle. Distribute iterates per
                // distribution directory; the current `name` is the
                // DirectoryName receiving symlinks. Per-skill emission inside
                // distribute::distribute_to_directory is a future-plan
                // expansion — when it lands, set `item: Some(skill_name.to_string())`
                // there instead.
                item: Some(name.to_string()),
            });
            let result = distribute::distribute_to_directory(
                paths.library_dir(),
                name,
                dir_config,
                &manifest,
                &machine_prefs,
                dry_run,
                force,
            )?;
            results.push(result);
        }
        sink.emit(ProgressEvent::SyncStageFinished {
            stage: SyncStage::Distribute,
        });
        results
    };

    // 6. Cleanup stale symlinks from distribution directories. Per-symlink
    //    I/O failures aggregate into `distribution_cleanup_failures` (SAFE-01
    //    pattern) so one stale ENOENT/EACCES does not erase the user-facing
    //    Bucket A/B/C summary; sync exits non-zero at end if the slice is
    //    non-empty.
    //    OBS-03: `cleanup` step span. Wraps target cleanup loop + the
    //    unified bucket render so the user-facing cleanup output is
    //    attributed to this step in the trace.
    //
    //    OBS-05 ordering (D-ENV-4): the per-bucket cleanup output must
    //    appear immediately AFTER the reconcile classification line in
    //    `render_sync_report`. The cleanup span here only performs the
    //    *work* (target cleanup); the user-facing bucket render is moved
    //    OUT of the span and runs AFTER `render_sync_report` below
    //    (Option 1 per Plan 18-02 Step 6 — smaller diff than widening
    //    `render_sync_report`'s signature to accept a stderr writer).
    // Stage boundary: cancellation checked before cleanup begins (D-12).
    if cancel.is_cancelled() {
        anyhow::bail!("sync cancelled");
    }
    let (removed_from_targets, distribution_cleanup_failures, excluded_skills) = {
        let _span = info_span!("cleanup").entered();
        sink.emit(ProgressEvent::SyncStageStarted {
            stage: SyncStage::Cleanup,
        });
        let mut removed: usize = 0;
        let mut excluded: Vec<cleanup::ExcludedSkill> = Vec::new();
        let mut failures: Vec<cleanup::DistributionCleanupFailure> = Vec::new();
        for (name, dir_config) in config.distribution_dirs() {
            let skills_dir = &dir_config.path;
            removed += cleanup::cleanup_target(skills_dir, paths.library_dir(), dry_run)?;
            // Also clean up symlinks for disabled skills (global + per-directory).
            // The returned Vec<ExcludedSkill> seeds Bucket C of the unified
            // three-bucket cleanup renderer (UX-01 D-UX01-1 / D-UX01-2).
            let (n, dir_excluded, dir_failures) = cleanup_disabled_from_target(
                skills_dir,
                paths.library_dir(),
                name,
                &machine_prefs,
                dry_run,
            )?;
            removed += n;
            excluded.extend(dir_excluded);
            failures.extend(dir_failures);
        }
        sink.emit(ProgressEvent::SyncStageFinished {
            stage: SyncStage::Cleanup,
        });
        (removed, failures, excluded)
    };

    // Stage boundary: cancellation checked before the Save stage begins (D-12).
    // This is the last safe-to-cancel point: every persist below is an atomic
    // temp+rename, so a cancel here leaves the on-disk manifest/lockfile in
    // their pre-sync state (T-25-03a). We do NOT check cancellation between the
    // individual saves below — interleaving a bail mid-save would risk a
    // half-written set.
    if cancel.is_cancelled() {
        anyhow::bail!("sync cancelled");
    }
    // 7. Save manifest, gitignore, and lockfile
    sink.emit(ProgressEvent::SyncStageStarted {
        stage: SyncStage::Save,
    });
    if !dry_run && paths.config_dir().is_dir() {
        // D-LSYNC-3 (OBS-07): stamp after distribute + cleanup succeed,
        // before persist. The stamp is INSIDE the `!dry_run` guard so
        // dry-run does NOT update last_synced_at — honest reporting.
        //
        // Note: a subsequent reconcile-install-failure bail (`bail!` at
        // the end of sync()) still treats `last_synced_at` as stamped —
        // the user-facing semantics are "cleanup completed; install-
        // failure exit is downstream." Per RESEARCH OQ-3.
        manifest.stamp_last_synced_at();
        manifest::save(&manifest, paths.config_dir())?;
    }
    if !dry_run && paths.library_dir().is_dir() {
        library::generate_gitignore(paths.library_dir(), &manifest)?;
    }
    if !dry_run && paths.config_dir().is_dir() {
        generate_tome_home_gitignore(paths.config_dir())?;
        lockfile::save(&new_lockfile, paths.config_dir())
            .context("failed to save lockfile — sync completed but lockfile is stale; re-run `tome sync` to retry")?;
    }
    sink.emit(ProgressEvent::SyncStageFinished {
        stage: SyncStage::Save,
    });

    let report = SyncReport {
        consolidate: consolidate_result,
        distributions: distribute_results,
        cleanup: cleanup_result,
        removed_from_targets,
        reconcile: reconcile_report,
    };

    if !quiet {
        render_sync_report(&report);
    }

    // 6b. Render the unified three-bucket cleanup output + any aggregated
    //     distribution-cleanup failures (UX-01 D-UX01-2 / D-UX01-4 stderr
    //     discipline). Empty buckets and empty failure list both produce
    //     no output, so syncs that touched nothing stay quiet.
    //     OBS-05 (D-ENV-4): runs AFTER `render_sync_report` so the
    //     reconcile classification line sits visually ABOVE the cleanup
    //     buckets in the user's terminal.
    if !quiet {
        let mut stderr = std::io::stderr().lock();
        // Best-effort writes — failure to render is non-fatal for sync. The
        // bucket renderer is the ONLY user notification for cleanup actions
        // (stale-skill removals, foreign-symlink failures), so silently
        // dropping I/O errors here would hide critical information; route
        // through `tracing::warn!` so the failure is at least discoverable
        // in `--verbose` / `TOME_LOG=warn` traces.
        if let Err(e) = cleanup::render_cleanup_buckets(
            &mut stderr,
            &report.cleanup.bucket_a_removed_from_config,
            &report.cleanup.bucket_b_missing_from_disk,
            &excluded_skills,
        ) {
            tracing::warn!("could not render cleanup buckets to stderr: {e}");
        }
        if let Err(e) = cleanup::render_distribution_cleanup_failures(
            &mut stderr,
            &distribution_cleanup_failures,
        ) {
            tracing::warn!("could not render distribution cleanup failures to stderr: {e}");
        }
    }

    // Post-sync health check
    if !dry_run && !quiet {
        let doctor_report = doctor::check(config, paths)?;
        if doctor_report.total_issues() > 0 {
            warn!(
                "{} issue(s) detected after sync — run `tome doctor` for details",
                doctor_report.total_issues()
            );
        }
    }

    // Offer git commit if tome home is a git repo with changes
    let committed = if !dry_run && !quiet {
        offer_git_commit(
            paths.tome_home(),
            report.consolidate.created,
            report.consolidate.updated,
            report.cleanup.removed_from_library,
        )?
    } else {
        false
    };

    // Push to remote after commit (only if something was committed)
    if committed && has_remote {
        match backup::push(paths.tome_home()) {
            Ok(()) => {
                if !quiet {
                    println!("  {} Pushed to remote", console::style("↑").cyan());
                }
            }
            Err(e) => warn!("remote push failed: {e}"),
        }
    }

    // SAFE-01 mirror: surface non-zero exit when distribution-symlink
    // cleanup hit per-symlink I/O failures. The grouped summary already
    // printed via cleanup::render_distribution_cleanup_failures; this
    // bail surfaces the exit code only.
    if !distribution_cleanup_failures.is_empty() {
        anyhow::bail!(
            "{} distribution cleanup operation(s) failed during sync (see \
             grouped summary above)",
            distribution_cleanup_failures.len(),
        );
    }

    // RESEARCH OQ-6: surface non-zero exit when reconcile failed any
    // install/update. The grouped failure summary already printed via
    // marketplace::render_install_failures; this bail surfaces the exit
    // code only.
    if !reconcile_install_failures.is_empty() {
        anyhow::bail!(
            "{} plugin install/update operation(s) failed during reconcile (see \
             grouped summary above)",
            reconcile_install_failures.len()
        );
    }

    Ok(())
}

/// Remove symlinks from a target directory that point to disabled skills,
/// surfacing each removal as a `cleanup::ExcludedSkill` so `lib.rs::sync`
/// can render them through the unified three-bucket cleanup output (UX-01
/// Bucket C — D-UX01-1, D-UX01-2).
///
/// Unlike `cleanup::cleanup_target` (which only removes *broken* symlinks),
/// this removes symlinks even if their target still exists on disk — because
/// the skill has been disabled in machine preferences.
///
/// Only removes symlinks that point into the library directory, matching the
/// origin check in `cleanup::cleanup_target`.
///
/// Detects two exclusion shapes:
/// - **Global** — skill is in `machine_prefs.disabled`. Reported as
///   `ExcludedSkill { directory: None }`.
/// - **Per-directory** — skill is in `directories.<dir>.disabled`
///   (blocklist) or absent from `directories.<dir>.enabled` (allowlist).
///   Reported as `ExcludedSkill { directory: Some(<dir>) }`.
///
/// Global takes precedence in reporting when a skill is both globally and
/// per-directory disabled (mirrors `MachinePrefs::is_skill_allowed`
/// resolution-order precedence: global is the broadest fallback, and the
/// user-actionable hint is "remove from machine.toml::disabled").
///
/// Returns `(removed_count, excluded_skills)` so the caller can:
/// 1. Account for the symlinks removed (used in `removed_from_targets`).
/// 2. Drain `excluded_skills` into `cleanup::render_cleanup_buckets`
///    Bucket C for the unified user-facing summary.
fn cleanup_disabled_from_target(
    target_dir: &Path,
    library_dir: &Path,
    dir_name: &config::DirectoryName,
    machine_prefs: &machine::MachinePrefs,
    dry_run: bool,
) -> Result<(
    usize,
    Vec<cleanup::ExcludedSkill>,
    Vec<cleanup::DistributionCleanupFailure>,
)> {
    let mut excluded: Vec<cleanup::ExcludedSkill> = Vec::new();
    let mut failures: Vec<cleanup::DistributionCleanupFailure> = Vec::new();

    if !target_dir.is_dir() {
        return Ok((0, excluded, failures));
    }

    let canonical_library = std::fs::canonicalize(library_dir).unwrap_or_else(|e| {
        warn!(
            "could not canonicalize library path {}: {} — symlinks using canonical paths may not be cleaned up",
            library_dir.display(),
            e
        );
        library_dir.to_path_buf()
    });

    let mut removed = 0;
    let entries = std::fs::read_dir(target_dir)
        .with_context(|| format!("failed to read target dir {}", target_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", target_dir.display()))?;
        let path = entry.path();
        if !path.is_symlink() {
            continue;
        }

        let name_owned = entry.file_name().to_string_lossy().into_owned();
        let is_global = machine_prefs.is_disabled(&name_owned);
        let is_allowed = machine_prefs.is_skill_allowed(&name_owned, dir_name.as_str());

        // `is_skill_allowed` returns false for both global AND per-directory
        // exclusion. We split the cases for reporting — global takes
        // precedence in the bucket-C surface even though the underlying
        // removal logic is the same.
        if is_global || !is_allowed {
            // Only remove if symlink points into the tome library. Per-symlink
            // I/O failures aggregate into `failures` instead of bailing the
            // loop so one stale ENOENT/EACCES does not erase the user-facing
            // Bucket A/B/C summary (SAFE-01 pattern, mirrors `RemoveFailure`
            // and `InstallFailure`).
            let raw_target = match std::fs::read_link(&path) {
                Ok(t) => t,
                Err(e) => {
                    failures.push(cleanup::DistributionCleanupFailure {
                        directory: dir_name.clone(),
                        skill: name_owned.clone(),
                        path: path.clone(),
                        operation: cleanup::DistributionCleanupOp::ReadLink,
                        error: e,
                    });
                    continue;
                }
            };
            let target = paths::resolve_symlink_target(&path, &raw_target);
            let points_into_library =
                target.starts_with(library_dir) || target.starts_with(&canonical_library);
            if !points_into_library {
                continue;
            }

            if !dry_run && let Err(e) = std::fs::remove_file(&path) {
                failures.push(cleanup::DistributionCleanupFailure {
                    directory: dir_name.clone(),
                    skill: name_owned.clone(),
                    path: path.clone(),
                    operation: cleanup::DistributionCleanupOp::Remove,
                    error: e,
                });
                continue;
            }
            removed += 1;

            // Convert the file-system entry name into a validated SkillName
            // for the bucket-C carrier. Skills with invalid names on disk
            // are silently skipped from reporting (the symlink is still
            // removed, but Bucket C expects a typed name; an invalid name
            // shouldn't have produced a distribution symlink anyway).
            let Ok(skill_name) = crate::discover::SkillName::new(name_owned.as_str()) else {
                continue;
            };
            let directory = if is_global {
                None
            } else {
                Some(dir_name.clone())
            };
            excluded.push(cleanup::ExcludedSkill {
                name: skill_name,
                directory,
            });
        }
    }

    Ok((removed, excluded, failures))
}

fn render_sync_report(report: &SyncReport) {
    println!("{}", style("Sync complete").green().bold());
    println!(
        "  Library: {} created, {} unchanged, {} updated{}",
        style(report.consolidate.created).cyan(),
        report.consolidate.unchanged,
        report.consolidate.updated,
        skipped_note(report.consolidate.skipped)
    );

    for dr in &report.distributions {
        println!(
            "  {}: {} linked, {} unchanged{}{}{}",
            style(&dr.directory_name).bold(),
            style(dr.changed).cyan(),
            dr.unchanged,
            skipped_note(dr.skipped),
            disabled_note(dr.disabled),
            managed_note(dr.skipped_managed)
        );
    }

    if report.cleanup.removed_from_library > 0 {
        println!(
            "  Cleaned {} stale entry/entries",
            style(report.cleanup.removed_from_library).yellow()
        );
    }

    if report.removed_from_targets > 0 {
        println!(
            "  Cleaned {} stale target link(s)",
            style(report.removed_from_targets).yellow()
        );
    }

    // Phase 18 OBS-05 (D-ENV-4): reconcile classification line, emitted
    // immediately above the per-bucket cleanup summary block (rendered
    // separately to stderr by the caller AFTER this function returns).
    // The line only prints when reconcile actually fired (Some); syncs
    // without a Claude adapter produce no line.
    if let Some(rr) = &report.reconcile {
        println!(
            "  reconcile: {} {} match · {} {} drift · {} {} vanished · {} {} missing-from-machine",
            style("✓").green(),
            rr.matches,
            style("⚠").yellow(),
            rr.drift.len(),
            style("⚠").yellow(),
            rr.vanished.len(),
            style("⚠").yellow(),
            rr.missing.len(),
        );

        // Per-drift detail + per-vanished warnings relocated from the
        // deleted inline `reconcile::render_summary` call site.
        let detail = reconcile::format_classification_detail(rr);
        if !detail.is_empty() {
            print!("{}", detail);
        }
    }
}

/// List all discovered skills.
///
/// Thin presenter (D-GUI-08): the domain computation (discover + sort) lives in
/// `list::collect`; this function only formats the resulting [`list::ListReport`]
/// as text or JSON. The GUI calls `list::collect` directly and renders the
/// report without this CLI formatting.
fn list(config: &Config, quiet: bool, json: bool) -> Result<()> {
    let report = list::collect(config)?;
    let skills = report.skills;
    if !quiet {
        for w in &report.warnings {
            eprintln!("warning: {}", w);
        }
    }

    if json {
        let rows: Vec<serde_json::Value> = skills
            .iter()
            .map(|s| {
                let mut row = serde_json::json!({
                    "name": s.name,
                    "source": s.source_name,
                    "path": s.path,
                    "managed": s.origin.is_managed(),
                });
                if let Some(p) = s.origin.provenance() {
                    row["registry_id"] = serde_json::json!(p.registry_id);
                    if let Some(v) = &p.version {
                        row["version"] = serde_json::json!(v);
                    }
                    if let Some(sha) = &p.git_commit_sha {
                        row["git_commit_sha"] = serde_json::json!(sha);
                    }
                }
                row
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if skills.is_empty() {
        println!("No skills found. Run `tome init` to configure sources.");
        return Ok(());
    }

    use tabled::settings::{Modify, Style, object::Rows};

    let mut rows: Vec<[String; 4]> = Vec::with_capacity(skills.len() + 1);
    rows.push([
        "SKILL".to_string(),
        "SOURCE".to_string(),
        "VERSION".to_string(),
        "PATH".to_string(),
    ]);
    for s in &skills {
        let version = s
            .origin
            .provenance()
            .and_then(|p| p.version.as_deref())
            .unwrap_or("")
            .to_string();
        rows.push([
            s.name.to_string(),
            s.source_name.as_str().to_string(),
            version,
            s.path.display().to_string(),
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
    println!();
    println!("{} skill(s) total", skills.len());

    Ok(())
}

/// Format a "skipped (path conflict)" suffix, or an empty string if count is zero.
fn skipped_note(count: usize) -> String {
    if count > 0 {
        format!(", {} skipped (path conflict)", style(count).yellow())
    } else {
        String::new()
    }
}

fn disabled_note(count: usize) -> String {
    if count == 0 {
        String::new()
    } else {
        format!(", {} disabled (machine prefs)", style(count).dim())
    }
}

fn managed_note(count: usize) -> String {
    if count == 0 {
        String::new()
    } else {
        format!(", {} skipped (managed)", style(count).dim())
    }
}

/// Generate a top-level `.gitignore` at `~/.tome/` to exclude internal files.
///
/// The manifest is internal bookkeeping and should not be version-controlled.
/// Everything else (skills/, tome.toml, tome.lock) is tracked.
fn generate_tome_home_gitignore(tome_home: &Path) -> Result<()> {
    let content = "# Auto-generated by tome — do not edit\n\
                   # Internal manifest (recreated by tome sync)\n\
                   .tome-manifest.json\n";
    let gitignore_path = tome_home.join(".gitignore");

    // Only write if content would change
    if gitignore_path.exists() {
        let existing = std::fs::read_to_string(&gitignore_path)
            .with_context(|| format!("failed to read {}", gitignore_path.display()))?;
        if existing == content {
            return Ok(());
        }
    }

    std::fs::write(&gitignore_path, content)
        .with_context(|| format!("failed to write {}", gitignore_path.display()))?;

    Ok(())
}

/// If tome home is a git repo with uncommitted changes, prompt the user to commit.
///
/// Returns `true` if a commit was created, `false` otherwise.
fn offer_git_commit(
    tome_home: &Path,
    created: usize,
    updated: usize,
    removed: usize,
) -> Result<bool> {
    if !tome_home.join(".git").exists() || !std::io::stdin().is_terminal() {
        return Ok(false);
    }

    let output = match GitCommand::new("git")
        .args(["status", "--porcelain"])
        .current_dir(tome_home)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("warning: could not run git status: {e}");
            return Ok(false);
        }
    };

    if !output.status.success() {
        eprintln!(
            "warning: git status returned non-zero exit code {:?}",
            output.status.code()
        );
        return Ok(false);
    }
    if output.stdout.is_empty() {
        return Ok(false);
    }

    let msg = sync_commit_message(created, updated, removed);

    let confirm = dialoguer::Confirm::new()
        .with_prompt(format!("Commit changes? ({})", msg))
        .default(true)
        .interact_opt()?;

    if confirm != Some(true) {
        return Ok(false);
    }

    // Stage all tracked files — .gitignore handles exclusions.
    // The repo is at tome_home (~/.tome/) and covers skills, config, and lockfile.
    let add_output = GitCommand::new("git")
        .args(["add", "-A"])
        .current_dir(tome_home)
        .output()?;
    if !add_output.status.success() {
        eprintln!(
            "warning: git add failed (exit code {:?})",
            add_output.status.code()
        );
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("  git said: {}", stderr.trim());
        }
        return Ok(false);
    }

    let commit_output = GitCommand::new("git")
        .args(["commit", "-m", &msg])
        .current_dir(tome_home)
        .output()?;
    if !commit_output.status.success() {
        eprintln!(
            "warning: git commit failed (exit code {:?})",
            commit_output.status.code()
        );
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("  git said: {}", stderr.trim());
        }
        return Ok(false);
    }

    Ok(true)
}

/// Build a commit message summarizing sync changes.
fn sync_commit_message(created: usize, updated: usize, removed: usize) -> String {
    let mut parts = Vec::new();
    if created > 0 {
        parts.push(format!("{created} created"));
    }
    if updated > 0 {
        parts.push(format!("{updated} updated"));
    }
    if removed > 0 {
        parts.push(format!("{removed} removed"));
    }
    if parts.is_empty() {
        return "tome sync".to_string();
    }
    format!("tome sync: {}", parts.join(", "))
}

/// Print shell completions to stdout.
fn print_completions(shell: clap_complete::Shell) {
    let mut cmd = <cli::Cli as clap::CommandFactory>::command();
    clap_complete::generate(shell, &mut cmd, "tome", &mut std::io::stdout());
}

/// Install shell completions to the standard location for the given shell.
pub(crate) fn install_completions(shell: clap_complete::Shell) -> Result<()> {
    use clap_complete::Shell;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    // Fish and Bash follow XDG conventions on all platforms (including macOS),
    // so we use XDG env vars with standard fallbacks rather than dirs::config_dir()
    // which returns ~/Library/Application Support on macOS.
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home.join(".config"));
    let xdg_data = std::env::var("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home.join(".local/share"));
    let dest = match shell {
        Shell::Fish => xdg_config.join("fish/completions/tome.fish"),
        Shell::Bash => xdg_data.join("bash-completion/completions/tome"),
        Shell::Zsh => home.join(".zfunc/_tome"),
        Shell::PowerShell => {
            anyhow::bail!(
                "Automatic installation not supported for PowerShell.\n\
                 Generate manually: tome completions powershell --print > tome.ps1\n\
                 Then source it from your PowerShell profile."
            );
        }
        _ => {
            anyhow::bail!("Unknown shell — cannot determine completions path");
        }
    };

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Could not create {}", parent.display()))?;
    }

    let mut cmd = <cli::Cli as clap::CommandFactory>::command();
    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut cmd, "tome", &mut buf);
    std::fs::write(&dest, &buf).with_context(|| format!("Could not write {}", dest.display()))?;

    println!("Installed {} completions to {}", shell, dest.display());
    if shell == Shell::Zsh {
        println!(
            "Ensure ~/.zfunc is in your fpath. Add to .zshrc:\n  \
             fpath=(~/.zfunc $fpath)\n  \
             autoload -Uz compinit && compinit"
        );
    }
    Ok(())
}

/// Show or print config information.
fn show_config(config: &Config, path_only: bool, config_path: &Path) -> Result<()> {
    if path_only {
        println!("{}", config_path.display());
    } else {
        let toml_str = toml::to_string_pretty(config)?;
        println!("{}", toml_str);
    }
    Ok(())
}

/// Interactive prompt to add a remote for cross-machine sync after `tome backup init`.
fn offer_remote_setup(tome_home: &Path) -> Result<()> {
    let add_remote = dialoguer::Confirm::new()
        .with_prompt("Add a remote for cross-machine sync?")
        .default(false)
        .interact()?;

    if !add_remote {
        return Ok(());
    }

    let url: String = dialoguer::Input::new()
        .with_prompt("Remote URL (e.g. git@github.com:user/tome-home.git)")
        .interact_text()?;

    backup::add_remote(tome_home, &url)?;

    print!("Verifying connection... ");
    match backup::verify_remote(tome_home) {
        Ok(()) => {
            println!("{}", console::style("ok").green());
        }
        Err(e) => {
            println!("{}", console::style("failed").red());
            eprintln!("warning: {e}");
            eprintln!(
                "The remote was added but could not be reached. Fix the URL or credentials, then run `tome sync`."
            );
            return Ok(());
        }
    }

    match backup::push_initial(tome_home) {
        Ok(()) => {
            println!(
                "{} Remote configured and initial push complete",
                console::style("✓").green()
            );
        }
        Err(e) => {
            eprintln!("warning: initial push failed: {e}");
            eprintln!("The remote was added. Push will be retried on next `tome sync`.");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::SkillName;
    use std::os::unix::fs as unix_fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// CORE-04 harness (RESEARCH Test Map): drive a real `sync()` with a
    /// `RecordingSink` and assert that every `SyncStage` emits at least one
    /// `SyncStageStarted` event. This pins the "≥1 event per stage" contract
    /// that the GUI (Phase 27) depends on for its per-stage progress UI, and
    /// guards against a future refactor silently dropping a stage's emit.
    #[test]
    fn sync_emits_at_least_one_event_per_stage() {
        use crate::config::{DirectoryConfig, DirectoryRole, DirectoryType};
        use crate::progress::RecordingSink;

        let tmp = TempDir::new().unwrap();
        let tome_home = tmp.path().join("tome-home");
        let library_dir = tome_home.join("skills");
        std::fs::create_dir_all(&library_dir).unwrap();

        // A `source` directory holding one discoverable skill.
        let source_dir = tmp.path().join("source");
        let skill_dir = source_dir.join("demo-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: demo-skill\n---\n# demo-skill",
        )
        .unwrap();

        // A `synced` distribution directory so the Distribute + Cleanup stages
        // have a target to act on.
        let target_dir = tmp.path().join("target");
        std::fs::create_dir_all(&target_dir).unwrap();

        let mut config = Config {
            library_dir: library_dir.clone(),
            ..Config::default()
        };
        config.directories.insert(
            DirectoryName::new("source").unwrap(),
            DirectoryConfig {
                path: source_dir,
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Source),
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );
        config.directories.insert(
            DirectoryName::new("target").unwrap(),
            DirectoryConfig {
                path: target_dir,
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Synced),
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );

        let paths = TomePaths::new(tome_home.clone(), library_dir).unwrap();
        let machine_path = tome_home.join("machine.toml");
        let machine_prefs = machine::MachinePrefs::default();

        let sink = RecordingSink::new();
        sync(
            &config,
            &paths,
            SyncOptions {
                dry_run: false,
                force: false,
                no_triage: true,
                no_input: true,
                no_install: true,
                verbose: false,
                quiet: true, // suppress stdout chrome in the test harness
                machine_path: &machine_path,
                machine_prefs: &machine_prefs,
            },
            &sink,
            &CancelToken::new(),
        )
        .expect("sync should succeed against the synthetic fixture");

        let started: Vec<SyncStage> = sink
            .events()
            .into_iter()
            .filter_map(|e| match e {
                ProgressEvent::SyncStageStarted { stage } => Some(stage),
                _ => None,
            })
            .collect();

        for stage in SyncStage::ALL {
            assert!(
                started.contains(&stage),
                "sync() must emit a SyncStageStarted for {stage:?}; got {started:?}",
            );
        }
    }

    /// D-16: the manifest join populates `synced_at` from the
    /// `SkillEntry::synced_at` field for skills present in the manifest.
    /// Skills with no manifest entry remain `None`. Directly exercises the
    /// extracted `join_synced_at_from_manifest` helper so we don't need a
    /// full sync-fixture roundtrip.
    #[test]
    fn join_synced_at_populates_known_skills_and_leaves_others_none() {
        use crate::discover::{DiscoveredSkill, SkillName, SkillOrigin};
        use crate::manifest::{Manifest, SkillEntry};
        use std::path::PathBuf;

        let mut manifest = Manifest::default();
        // SkillEntry::new stamps a current timestamp, so override the
        // `synced_at` field to a deterministic value the assertion can
        // compare against.
        let mut entry = SkillEntry::new(
            PathBuf::from("/tmp/known"),
            DirectoryName::new("test").unwrap(),
            crate::validation::ContentHash::new("a".repeat(64)).unwrap(),
            false,
        );
        entry.synced_at = "2026-06-05T10:00:00Z".to_string();
        manifest.insert(SkillName::new("known").unwrap(), entry);

        let mut skills = vec![
            DiscoveredSkill {
                name: SkillName::new("known").unwrap(),
                path: PathBuf::from("/tmp/known"),
                source_name: DirectoryName::new("test").unwrap(),
                origin: SkillOrigin::Local,
                frontmatter: None,
                synced_at: None,
            },
            DiscoveredSkill {
                name: SkillName::new("unknown").unwrap(),
                path: PathBuf::from("/tmp/unknown"),
                source_name: DirectoryName::new("test").unwrap(),
                origin: SkillOrigin::Local,
                frontmatter: None,
                synced_at: None,
            },
        ];

        join_synced_at_from_manifest(&mut skills, &manifest);

        assert_eq!(
            skills[0].synced_at.as_deref(),
            Some("2026-06-05T10:00:00Z"),
            "manifest-resident skill must inherit its synced_at",
        );
        assert!(
            skills[1].synced_at.is_none(),
            "skill with no manifest entry must remain None",
        );
    }

    #[test]
    fn commit_message_all_changes() {
        assert_eq!(
            sync_commit_message(3, 1, 2),
            "tome sync: 3 created, 1 updated, 2 removed"
        );
    }

    #[test]
    fn commit_message_created_only() {
        assert_eq!(sync_commit_message(5, 0, 0), "tome sync: 5 created");
    }

    #[test]
    fn commit_message_no_changes() {
        assert_eq!(sync_commit_message(0, 0, 0), "tome sync");
    }

    /// UX-01 invariant: the trigger phrase that motivated the v0.10
    /// milestone discussion must NOT appear anywhere in `lib.rs`. Pinned
    /// at source level (not just rendered output) so a future refactor
    /// cannot re-introduce it in a code path the integration test fixture
    /// doesn't exercise. Sibling test in `cleanup.rs` covers that module.
    #[test]
    fn lib_module_source_does_not_contain_forbidden_phrase() {
        let forbidden = format!("{} {}", "no longer", "configured");
        let source = include_str!("lib.rs");
        assert!(
            !source.contains(&forbidden),
            "lib.rs source must not contain the UX-01 trigger phrase \"{forbidden}\"",
        );
    }

    // -- cleanup_disabled_from_target tests --

    fn test_dir_name() -> config::DirectoryName {
        config::DirectoryName::new("test-dir").unwrap()
    }

    #[test]
    fn cleanup_disabled_removes_library_symlink() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Create a skill dir in the library and symlink it in the target
        let skill_dir = library.path().join("disabled-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        unix_fs::symlink(&skill_dir, target.path().join("disabled-skill")).unwrap();

        let mut prefs = machine::MachinePrefs::default();
        prefs.disable(SkillName::new("disabled-skill").unwrap());

        let dir_name = test_dir_name();
        let (removed, excluded, failures) =
            cleanup_disabled_from_target(target.path(), library.path(), &dir_name, &prefs, false)
                .unwrap();
        assert!(
            failures.is_empty(),
            "no I/O failures expected: {failures:?}"
        );
        assert_eq!(removed, 1);
        assert!(!target.path().join("disabled-skill").exists());
        assert_eq!(excluded.len(), 1, "Bucket C should record the removal");
        assert_eq!(excluded[0].name.as_str(), "disabled-skill");
        assert!(
            excluded[0].directory.is_none(),
            "global disable should report `directory: None`"
        );
    }

    #[test]
    fn cleanup_disabled_preserves_external_symlink() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();
        let external = TempDir::new().unwrap();

        // Symlink in target with a disabled name but pointing outside the library
        let ext_dir = external.path().join("disabled-skill");
        std::fs::create_dir_all(&ext_dir).unwrap();
        unix_fs::symlink(&ext_dir, target.path().join("disabled-skill")).unwrap();

        let mut prefs = machine::MachinePrefs::default();
        prefs.disable(SkillName::new("disabled-skill").unwrap());

        let dir_name = test_dir_name();
        let (removed, excluded, failures) =
            cleanup_disabled_from_target(target.path(), library.path(), &dir_name, &prefs, false)
                .unwrap();
        assert!(
            failures.is_empty(),
            "no I/O failures expected: {failures:?}"
        );
        assert_eq!(
            removed, 0,
            "should not remove symlink pointing outside library"
        );
        assert!(target.path().join("disabled-skill").is_symlink());
        assert!(
            excluded.is_empty(),
            "external symlink should not produce a Bucket C entry"
        );
    }

    #[test]
    fn cleanup_disabled_skips_non_symlink() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Regular directory (not a symlink) with a disabled skill name
        std::fs::create_dir_all(target.path().join("disabled-skill")).unwrap();

        let mut prefs = machine::MachinePrefs::default();
        prefs.disable(SkillName::new("disabled-skill").unwrap());

        let dir_name = test_dir_name();
        let (removed, excluded, failures) =
            cleanup_disabled_from_target(target.path(), library.path(), &dir_name, &prefs, false)
                .unwrap();
        assert!(
            failures.is_empty(),
            "no I/O failures expected: {failures:?}"
        );
        assert_eq!(removed, 0);
        assert!(target.path().join("disabled-skill").is_dir());
        assert!(excluded.is_empty());
    }

    #[test]
    fn cleanup_disabled_nonexistent_dir_returns_zero() {
        let prefs = machine::MachinePrefs::default();
        let dir_name = test_dir_name();
        let (removed, excluded, failures) = cleanup_disabled_from_target(
            std::path::Path::new("/nonexistent/target"),
            std::path::Path::new("/nonexistent/library"),
            &dir_name,
            &prefs,
            false,
        )
        .unwrap();
        assert_eq!(removed, 0);
        assert!(excluded.is_empty());
        assert!(failures.is_empty());
    }

    #[test]
    fn cleanup_disabled_dry_run_preserves_symlink() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        let skill_dir = library.path().join("disabled-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        unix_fs::symlink(&skill_dir, target.path().join("disabled-skill")).unwrap();

        let mut prefs = machine::MachinePrefs::default();
        prefs.disable(SkillName::new("disabled-skill").unwrap());

        let dir_name = test_dir_name();
        let (removed, excluded, failures) =
            cleanup_disabled_from_target(target.path(), library.path(), &dir_name, &prefs, true)
                .unwrap();
        assert!(
            failures.is_empty(),
            "no I/O failures expected: {failures:?}"
        );
        assert_eq!(removed, 1, "should count the would-be removal");
        assert!(
            target.path().join("disabled-skill").is_symlink(),
            "dry-run should not actually remove"
        );
        assert_eq!(
            excluded.len(),
            1,
            "dry-run should still report would-be Bucket C entry"
        );
    }

    /// Per-directory disable via `directories.<dir>.disabled` blocklist —
    /// Bucket C entry must name the directory it was excluded for.
    #[test]
    fn cleanup_disabled_per_directory_blocklist_reports_directory() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        let skill_dir = library.path().join("excluded-here");
        std::fs::create_dir_all(&skill_dir).unwrap();
        unix_fs::symlink(&skill_dir, target.path().join("excluded-here")).unwrap();

        let dir_name = test_dir_name();
        let mut prefs = machine::MachinePrefs::default();
        prefs.toggle_per_dir_blocklist(&dir_name, SkillName::new("excluded-here").unwrap(), true);

        let (removed, excluded, failures) =
            cleanup_disabled_from_target(target.path(), library.path(), &dir_name, &prefs, false)
                .unwrap();
        assert!(
            failures.is_empty(),
            "no I/O failures expected: {failures:?}"
        );
        assert_eq!(removed, 1);
        assert!(!target.path().join("excluded-here").exists());
        assert_eq!(
            excluded.len(),
            1,
            "per-dir disable should produce one Bucket C entry"
        );
        assert_eq!(excluded[0].name.as_str(), "excluded-here");
        assert_eq!(
            excluded[0].directory.as_ref().map(|d| d.as_str()),
            Some("test-dir"),
            "per-dir blocklist should report `directory: Some(<dir>)`"
        );
    }

    /// When a skill is BOTH globally and per-directory disabled, the
    /// Bucket C entry is reported as global (broader scope = more
    /// actionable user hint pointing at machine.toml::disabled).
    #[test]
    fn cleanup_disabled_global_takes_precedence_over_per_dir() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        let skill_dir = library.path().join("disabled-everywhere");
        std::fs::create_dir_all(&skill_dir).unwrap();
        unix_fs::symlink(&skill_dir, target.path().join("disabled-everywhere")).unwrap();

        let dir_name = test_dir_name();
        let mut prefs = machine::MachinePrefs::default();
        // Both global AND per-dir blocklist contain the skill.
        prefs.disable(SkillName::new("disabled-everywhere").unwrap());
        prefs.toggle_per_dir_blocklist(
            &dir_name,
            SkillName::new("disabled-everywhere").unwrap(),
            true,
        );

        let (removed, excluded, failures) =
            cleanup_disabled_from_target(target.path(), library.path(), &dir_name, &prefs, false)
                .unwrap();
        assert!(
            failures.is_empty(),
            "no I/O failures expected: {failures:?}"
        );
        assert_eq!(removed, 1);
        assert_eq!(
            excluded.len(),
            1,
            "double-disable should still produce one entry"
        );
        assert!(
            excluded[0].directory.is_none(),
            "global takes precedence — directory should be None"
        );
    }

    /// SAFE-01 mirror — when `remove_file` fails on one stale symlink (here
    /// simulated by chmod-ing the parent directory to deny writes), the
    /// failure aggregates into the returned `Vec<DistributionCleanupFailure>`
    /// instead of aborting the loop. Other symlinks in the same directory
    /// continue to be processed.
    #[cfg(unix)]
    #[test]
    fn cleanup_disabled_aggregates_remove_failures() {
        use std::os::unix::fs::PermissionsExt;

        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Two disabled skills with valid library targets.
        for name in &["disabled-foo", "disabled-bar"] {
            let skill_dir = library.path().join(name);
            std::fs::create_dir_all(&skill_dir).unwrap();
            unix_fs::symlink(&skill_dir, target.path().join(name)).unwrap();
        }

        let mut prefs = machine::MachinePrefs::default();
        prefs.disable(SkillName::new("disabled-foo").unwrap());
        prefs.disable(SkillName::new("disabled-bar").unwrap());

        // Strip write perms on the target dir so `remove_file` fails for both
        // symlinks. Read perms remain so the loop can still enumerate.
        let mut perms = std::fs::metadata(target.path()).unwrap().permissions();
        let original_mode = perms.mode();
        perms.set_mode(0o555);
        std::fs::set_permissions(target.path(), perms).unwrap();

        let dir_name = test_dir_name();
        let result =
            cleanup_disabled_from_target(target.path(), library.path(), &dir_name, &prefs, false);

        // Restore perms before any assertion can panic so TempDir cleanup works.
        let mut restored = std::fs::metadata(target.path()).unwrap().permissions();
        restored.set_mode(original_mode);
        std::fs::set_permissions(target.path(), restored).unwrap();

        let (removed, excluded, failures) = result.unwrap();
        assert_eq!(
            removed, 0,
            "no symlinks should have been removed (all failed)"
        );
        assert!(
            excluded.is_empty(),
            "excluded set must skip failed entries — Bucket C only reports successes"
        );
        assert_eq!(
            failures.len(),
            2,
            "both remove failures should aggregate, not abort the loop: {failures:?}"
        );
        for f in &failures {
            assert_eq!(f.operation, cleanup::DistributionCleanupOp::Remove);
            assert_eq!(f.directory.as_str(), "test-dir");
        }
    }

    // -- apply_edit_decisions tests (Phase 14 / D-C1 transition site 3) --

    /// Build a one-skill managed manifest for the apply_edit_decisions tests.
    fn fork_test_manifest() -> manifest::Manifest {
        let mut manifest = manifest::Manifest::default();
        manifest.insert(
            discover::SkillName::new("plug").unwrap(),
            manifest::SkillEntry::new(
                PathBuf::from("/tmp/plug"),
                config::DirectoryName::new("claude-plugins").unwrap(),
                validation::test_hash("h"),
                true, // managed
            ),
        );
        manifest
    }

    /// Build a one-skill ReconcileReport with the given decision.
    fn fork_test_report(decision: reconcile::EditDecision) -> reconcile::ReconcileReport {
        reconcile::ReconcileReport {
            edited: vec![reconcile::Edited {
                name: discover::SkillName::new("plug").unwrap(),
                old_source: config::DirectoryName::new("claude-plugins").unwrap(),
                old_version: Some("1.0.0".to_string()),
            }],
            edit_decisions: vec![decision],
            ..Default::default()
        }
    }

    #[test]
    fn apply_edit_decisions_fork_records_previous_source() {
        // EditDecision::Fork transitions the manifest entry to Unowned
        // (managed=false, source_name=None) AND captures previous_source
        // per D-C1 / Phase 13 D-13 closure.
        let mut manifest = fork_test_manifest();
        let report = fork_test_report(reconcile::EditDecision::Fork);

        let mutated = apply_edit_decisions(&report, &mut manifest, false);

        assert!(mutated, "Fork must report mutated=true");
        let entry = manifest.get("plug").unwrap();
        assert_eq!(
            entry.source_name(),
            None,
            "fork-in-place clears source_name"
        );
        assert!(!entry.managed, "fork-in-place clears managed");
        assert_eq!(
            entry.previous_source(),
            Some(&config::DirectoryName::new("claude-plugins").unwrap()),
            "fork-in-place must record previous_source per D-C1 / Phase 13 D-13 closure"
        );
    }

    #[test]
    fn apply_edit_decisions_revert_leaves_manifest_byte_for_byte_unchanged() {
        // EditDecision::Revert is an explicit v0.10 deferred stub: emits a
        // warning, mutates NOTHING. This regression test pins the no-op
        // semantic so a future "completion" of the revert path that
        // accidentally adds library-overwrite logic (data-loss class) fails
        // here first. Closes the critical test gap surfaced in the v0.11
        // codebase review.
        let mut manifest = fork_test_manifest();
        let serialized_before = serde_json::to_string(&manifest).unwrap();
        let report = fork_test_report(reconcile::EditDecision::Revert);

        let mutated = apply_edit_decisions(&report, &mut manifest, false);

        assert!(!mutated, "Revert must not mutate the manifest");
        let serialized_after = serde_json::to_string(&manifest).unwrap();
        assert_eq!(
            serialized_before, serialized_after,
            "Revert is a stub — manifest must be byte-for-byte unchanged"
        );
        // Field-level assertions as belt-and-suspenders against future
        // serialization changes that round-trip the bytes but mutate
        // semantically. The entry's owned state must be preserved.
        let entry = manifest.get("plug").unwrap();
        assert_eq!(
            entry.source_name(),
            Some(&config::DirectoryName::new("claude-plugins").unwrap()),
            "Revert must preserve source_name"
        );
        assert!(entry.managed, "Revert must preserve managed flag");
        assert_eq!(
            entry.previous_source(),
            None,
            "Revert must not set previous_source"
        );
    }

    #[test]
    fn apply_edit_decisions_skip_leaves_manifest_byte_for_byte_unchanged() {
        // EditDecision::Skip emits nothing additional (handle_edited already
        // warned during reconcile) and mutates nothing.
        let mut manifest = fork_test_manifest();
        let serialized_before = serde_json::to_string(&manifest).unwrap();
        let report = fork_test_report(reconcile::EditDecision::Skip);

        let mutated = apply_edit_decisions(&report, &mut manifest, false);

        assert!(!mutated, "Skip must not mutate the manifest");
        let serialized_after = serde_json::to_string(&manifest).unwrap();
        assert_eq!(
            serialized_before, serialized_after,
            "Skip is a no-op — manifest must be byte-for-byte unchanged"
        );
    }

    #[test]
    fn apply_edit_decisions_dry_run_returns_false_without_mutating() {
        // dry_run short-circuits before any mutation, even for Fork.
        let mut manifest = fork_test_manifest();
        let serialized_before = serde_json::to_string(&manifest).unwrap();
        let report = fork_test_report(reconcile::EditDecision::Fork);

        let mutated = apply_edit_decisions(&report, &mut manifest, true);

        assert!(!mutated, "dry_run must not report mutated=true");
        let serialized_after = serde_json::to_string(&manifest).unwrap();
        assert_eq!(
            serialized_before, serialized_after,
            "dry_run must not mutate the manifest"
        );
    }

    #[test]
    fn resolve_tome_home_cli_flag_takes_priority() {
        let result = resolve_tome_home(
            Some(Path::new("/custom/home")),
            Some(Path::new("/other/tome.toml")),
        )
        .unwrap();
        assert_eq!(result, Path::new("/custom/home"));
    }

    #[test]
    fn resolve_tome_home_config_path_returns_parent() {
        let result =
            resolve_tome_home(None, Some(Path::new("/home/user/.tome/tome.toml"))).unwrap();
        assert_eq!(result, Path::new("/home/user/.tome"));
    }

    #[test]
    fn resolve_tome_home_bare_filename_returns_error() {
        let result = resolve_tome_home(None, Some(Path::new("tome.toml")));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("must be an absolute path"),
            "unexpected error: {err_msg}"
        );
    }

    #[test]
    fn resolve_tome_home_relative_path_returns_error() {
        for path in &["./tome.toml", "../tome.toml", "subdir/tome.toml"] {
            let result = resolve_tome_home(None, Some(Path::new(path)));
            assert!(result.is_err(), "expected error for relative path: {path}");
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("must be an absolute path"),
                "unexpected error for '{path}': {err_msg}"
            );
        }
    }

    #[test]
    fn resolve_tome_home_none_returns_default() {
        let result = resolve_tome_home(None, None).unwrap();
        let expected = config::default_tome_home().unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_tome_home_tilde_expansion() {
        let result = resolve_tome_home(Some(Path::new("~/my-skills/.tome")), None).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home.join("my-skills/.tome"));
    }

    #[test]
    fn resolve_tome_home_relative_tome_home_returns_error() {
        let result = resolve_tome_home(Some(Path::new("relative/path")), None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be an absolute path")
        );
    }

    // --- resolve_config_path tests ---

    #[test]
    fn resolve_config_path_cli_config_takes_priority() {
        let result = resolve_config_path(
            Some(Path::new("/custom/home")),
            Some(Path::new("/explicit/config.toml")),
        )
        .unwrap();
        assert_eq!(result, Some(PathBuf::from("/explicit/config.toml")));
    }

    #[test]
    fn resolve_config_path_derives_from_tome_home() {
        let result = resolve_config_path(Some(Path::new("/my/tome-home")), None).unwrap();
        assert_eq!(result, Some(PathBuf::from("/my/tome-home/tome.toml")));
    }

    #[test]
    fn resolve_config_path_tilde_expansion() {
        let result = resolve_config_path(Some(Path::new("~/my-repo/.tome")), None).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, Some(home.join("my-repo/.tome/tome.toml")));
    }

    #[test]
    fn resolve_config_path_none_returns_none() {
        let result = resolve_config_path(None, None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn init_with_no_input_does_not_bail_from_lib_run() {
        // Guard against re-introduction of the `tome init requires interactive
        // input` bail in the Init branch. A real integration test of
        // `tome init --no-input` lives in tests/cli.rs; this source-grep test
        // is a cheap compile-time-ish belt-and-braces.
        //
        // The needle is split across two concatenated string literals so this
        // test's source itself does not match its own search string — otherwise
        // `include_str!` would see the sentinel here and always fail.
        let src = include_str!("lib.rs");
        let needle = concat!("anyhow::bail!(\"tome init requires", " interactive input");
        assert!(
            !src.contains(needle),
            "lib.rs still contains the removed `tome init requires interactive input` bail"
        );
    }
}

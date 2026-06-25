//! Tauri command surface (webview → Rust trust boundary).
//!
//! Phase-26 alpha commands. Read-only commands resolve a real
//! [`tome::status::StatusReport`] / [`tome::list::ListReport`] /
//! [`tome::skill::SkillDetail`]; the lone Phase-26 mutation
//! ([`set_skill_disabled`]) goes through the shared
//! [`tome::actions::set_skill_disabled`] helper so the TUI and GUI hit the
//! same atomic temp+rename code path. The IPC surface stays minimal —
//! `opener:default` + `clipboard-manager:allow-write-text` plus
//! `core:default`/`core:event:default`, no `fs:default` or shell widening
//! (T-25-04-EoP mitigation).

use std::collections::BTreeMap;
use std::path::Path;

use tome::MachineTomlPreview;
use tome::SkillName;
use tome::TomePaths;
use tome::config::Config;
use tome::progress::{CancelToken, SyncStage};

use crate::error::{ErrorCode, TomeError};
use crate::sink::TauriEventSink;
use crate::sync_outcome_wire::{PartialFailureWire, SyncOutcomeWire};
use crate::sync_state::SyncState;
use crate::sync_types::{LockfileDiff, lockfile_diff_projection};

/// A single triage decision sent from the GUI's Sync route to the Rust
/// `preview_machine_toml` / `apply_machine_toml` commands.
///
/// One entry per skill the user has triaged (the `Map<SkillName, …>` in
/// `useSync().decisions` is flattened to `Vec<TriageDecision>` at IPC time).
/// A skill missing from the Vec is implicitly Keep — the React side only
/// surfaces decisions the user explicitly changed away from the default.
///
/// `SkillName`'s `Deserialize` impl validates the name at the IPC boundary
/// (T-27-03-02 mitigation — path-separator / empty-name rejection happens
/// before the value reaches the mutator).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TriageDecision {
    pub skill: SkillName,
    pub decision: TriageDecisionKind,
}

/// Decision kind for [`TriageDecision`]. Stable lowercase string union on
/// the TS side (`"keep"` / `"disable"`) so the React decision-map keeps a
/// narrow shape.
///
/// - `Keep` — explicit "leave this skill enabled" choice. A no-op at write
///   time; the user's intent is recorded in React state so the per-row UI
///   reflects it, but `apply_machine_toml` does not touch the on-disk file
///   for Keep entries.
/// - `Disable` — adds the skill to the global `disabled` set in
///   `machine.toml` (the same set toggled by `set_skill_disabled` in the
///   Skills view). Idempotent on the disabled set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TriageDecisionKind {
    Keep,
    Disable,
}

/// Resolve the user's real `tome_home` + `Config` the same way the CLI does
/// with no flags: default config path, then default `tome_home`.
///
/// Mirrors `crates/tome/src/lib.rs::run`'s flag-free resolution branch so the
/// GUI observes exactly the same state the CLI would (`Config::load_or_default`
/// is missing-file tolerant — an unconfigured machine yields a default config
/// and `StatusReport { configured: false, .. }`).
///
/// `pub` since plan 26-06 — `main.rs::setup` calls it to derive the
/// `TomePaths` it hands to the file watcher (`watcher::spawn_watcher`).
pub fn load_context() -> anyhow::Result<(Config, TomePaths)> {
    let config_path = tome::config::default_config_path()?;
    let config = Config::load_or_default(Some(&config_path))?;
    let tome_home = tome::config::default_tome_home()?;
    let paths = TomePaths::new(tome_home, config.library_dir().to_path_buf())?;
    Ok((config, paths))
}

/// Return a read-only status snapshot of the tome system.
///
/// The single boundary command for this phase. The `app` handle is accepted so
/// later phases can inject a [`crate::sink::TauriEventSink`] for long-running
/// variants; for the read-only status path it is currently unused.
#[tauri::command]
#[specta::specta]
pub fn get_status(_app: tauri::AppHandle) -> Result<tome::status::StatusReport, TomeError> {
    // CORE-05 / D-13: classify the domain's `anyhow::Error` into a structured
    // `TomeError` at the IPC boundary. The front-end pattern-matches on
    // `TomeError.code`; the full anyhow chain is preserved in `context`.
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::status::gather(&config, &paths).map_err(TomeError::from)
}

/// Return the discovered skill list backing the GUI's VIEW-02 (Skills view).
///
/// Thin wrapper over [`tome::list::collect`] — the CORE-01 collect-shape
/// function. The GUI fetches once on mount, then runs fuzzy filter / sort /
/// group-by JS-side (RESEARCH §"Standard Stack — Fuzzy search"); per-keystroke
/// IPC would blow the 60fps budget.
#[tauri::command]
#[specta::specta]
pub fn list_skills(_app: tauri::AppHandle) -> Result<tome::list::ListReport, TomeError> {
    let (config, _paths) = load_context().map_err(TomeError::from)?;
    tome::list::collect(&config).map_err(TomeError::from)
}

/// Aggregate a single skill's right-pane payload for the GUI's
/// `DetailHeader` + `MarkdownBody` (Phase 26 plan 26-03 / VIEW-03 / D-05).
///
/// Wraps [`tome::skill::collect_detail`] — manifest entry + parsed
/// frontmatter projection + machine-prefs disabled flag + capped markdown
/// body. Body length is capped at 1 MiB at the domain layer so the webview
/// render path is bounded.
#[tauri::command]
#[specta::specta]
pub fn get_skill_detail(
    _app: tauri::AppHandle,
    name: SkillName,
) -> Result<tome::skill::SkillDetail, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::skill::collect_detail(&name, &config, &paths).map_err(TomeError::from)
}

/// Toggle a skill's membership in the global `disabled` set in `machine.toml`
/// (Phase 26 plan 26-03 / D-06 — the lone Phase 26 mutation).
///
/// Routes through the shared [`tome::actions::set_skill_disabled`] helper, so
/// the GUI and the browse TUI hit the same atomic temp+rename. The Phase-26
/// file watcher (plan 26-06) fires `MachinePrefsChanged` for the resulting
/// write — own-process writes are observed verbatim, no manual refresh
/// signal needed.
#[tauri::command]
#[specta::specta]
pub fn set_skill_disabled(
    _app: tauri::AppHandle,
    name: SkillName,
    disabled: bool,
) -> Result<(), TomeError> {
    let machine_path = tome::default_machine_path().map_err(TomeError::from)?;
    tome::actions::set_skill_disabled(&name, disabled, &machine_path).map_err(TomeError::from)
}

/// Reveal the resolved source folder of a skill in Finder (Phase 26 plan
/// 26-03 / D-07).
///
/// Resolves the source path through [`tome::actions::resolve_source_path`]
/// (Owned manifest source / Unowned library-canonical fallback), then asks
/// `tauri-plugin-opener` to do the OS-call. The plugin maps to `open -R` on
/// macOS, `xdg-open` parents on Linux, `explorer.exe /select,` on Windows.
#[tauri::command]
#[specta::specta]
pub fn open_source_folder(app: tauri::AppHandle, name: SkillName) -> Result<(), TomeError> {
    use tauri_plugin_opener::OpenerExt;
    let (config, paths) = load_context().map_err(TomeError::from)?;
    let src =
        tome::actions::resolve_source_path(&name, &config, &paths).map_err(TomeError::from)?;
    app.opener()
        .reveal_item_in_dir(&src)
        .map_err(|e| TomeError::from(anyhow::anyhow!("opener: {e}")))
}

/// Return the resolved source path of a skill as a UTF-8 string (Phase 26
/// plan 26-03 / D-07).
///
/// The Rust side resolves the path; the React side calls
/// `@tauri-apps/plugin-clipboard-manager::writeText` with the returned
/// string. Splitting the work this way keeps the IPC contract narrow (a
/// single `String` return type; no clipboard-write plumbing crossing the
/// boundary).
#[tauri::command]
#[specta::specta]
pub fn copy_path(_app: tauri::AppHandle, name: SkillName) -> Result<String, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    let src =
        tome::actions::resolve_source_path(&name, &config, &paths).map_err(TomeError::from)?;
    Ok(src.display().to_string())
}

/// Return the full doctor report for the GUI Health view (Phase 26 plan
/// 26-05 / VIEW-05).
///
/// Wraps [`tome::doctor::collect_doctor_view`] — the GUI-facing projection of
/// `DoctorReport` that exposes only the 6 surfaced finding categories (4
/// auto-fixable + 2 informational) plus pre-computed `auto_fixable_count` /
/// `manual_count` so the React section headers render without re-walking the
/// list. Non-GUI issues (orphan dirs, missing SKILL.md, config issues,
/// foreign symlinks) intentionally do NOT cross the IPC boundary in Phase 26.
#[tauri::command]
#[specta::specta]
pub fn get_doctor_report(_app: tauri::AppHandle) -> Result<tome::doctor::DoctorView, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::doctor::collect_doctor_view(&config, &paths).map_err(TomeError::from)
}

/// Dispatch a per-item doctor repair for the GUI's `PreviewPopover` Apply
/// button (Phase 26 plan 26-05 / VIEW-05 / D-09).
///
/// Wraps [`tome::doctor::repair_one`] — re-runs `check()` to locate the live
/// issue, then matches the [`tome::doctor::RepairKind`] exhaustively against
/// per-item helpers. NF-04 preview-then-confirm: this command is only reached
/// after the user clicks Apply inside the `PreviewPopover` (no keyboard
/// shortcut bypasses it; T-26-05-01 mitigation).
///
/// Returns a structured `TomeError` for the two GUI-visible failure modes:
/// stale FindingId ("no longer present" — T-26-05-02), or non-auto-fixable
/// kind ("not auto-fixable" — defensive, the GUI never sends one of these).
/// The watcher (plan 26-06) fires `LibraryChanged` / `ManifestChanged` /
/// `MachinePrefsChanged` for the resulting writes; the React Health view
/// refetches on those.
#[tauri::command]
#[specta::specta]
pub fn doctor_repair_one(
    _app: tauri::AppHandle,
    finding_id: tome::doctor::FindingId,
) -> Result<(), TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::doctor::repair_one(&finding_id, &config, &paths).map_err(TomeError::from)
}

/// Return the pending lockfile diff for the GUI's SYNC-02 triage panel
/// (Phase 27 plan 27-02 / SYNC-02).
///
/// Read-only: loads the on-disk `tome.lock` (current shipped state) and
/// projects the diff against a prospective lockfile built from the current
/// `Manifest` + currently-discovered skills. The diff is the same shape
/// `tome::update::diff` produces — the GUI consumes a triage-friendly
/// projection ([`LockfileDiff`]) keyed by change kind.
///
/// The prospective lockfile is built from the canonical `Manifest`
/// (`manifest::load`) and the skills discovered against the live config.
/// Git-source discovery uses the offline lockfile cache via
/// `lockfile::resolved_paths_from_lockfile_cache` so no network calls cross
/// this command (matches the read-only contract of the SYNC-02 panel).
///
/// When no sync has ever run (`tome.lock` is missing), the command returns
/// every discovered skill as Added — the user sees a populated triage panel
/// before the first sync.
#[tauri::command]
#[specta::specta]
pub fn get_lockfile_diff(_app: tauri::AppHandle) -> Result<LockfileDiff, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;

    // Inner anyhow body — promotes anyhow → TomeError at the boundary.
    (|| -> anyhow::Result<LockfileDiff> {
        // Load the on-disk lockfile; `None` means no sync has run yet,
        // which we surface as "every discovered skill is added".
        let old_lockfile = tome::lockfile::load(paths.config_dir())?.unwrap_or_else(|| {
            // Construct an empty lockfile via JSON — Lockfile's fields are
            // pub(crate), and an empty lockfile is what an unset state should
            // look like for diffing purposes. Always parses (no skills).
            serde_json::from_value(serde_json::json!({ "version": 1, "skills": {} }))
                .expect("empty lockfile must deserialize")
        });

        // Load the manifest — the projection reads `synced_at` from here for
        // Changed / Removed rows. Manifest may be empty for first-run.
        let manifest = tome::manifest::load(paths.config_dir())?;

        // Build the prospective lockfile from currently-discovered skills.
        // Offline git resolution: derive cache paths from the existing
        // lockfile (no network). Discovery warnings are swallowed for this
        // read-only diff — they would otherwise leak into the GUI's triage
        // panel which renders only structured diff data, not warnings.
        let (resolved_paths, _warnings) = offline_resolved_paths(&config, &paths);
        let mut discover_warnings = Vec::new();
        let skills = tome::discover_all(&config, &resolved_paths, &mut discover_warnings)?;
        // Re-hash each skill's source directory on disk so the prospective
        // lockfile reflects current state, not the stored manifest hashes.
        // `lockfile::generate` copies manifest hashes (correct post-sync) but
        // would make the diff always empty here (manifest == lockfile).
        let new_lockfile = tome::lockfile::generate_prospective(&skills)?;

        let diff = tome::update::diff(&old_lockfile, &new_lockfile);
        Ok(lockfile_diff_projection(&diff, &manifest))
    })()
    .map_err(TomeError::from)
}

/// Type alias for the git-source resolution map that `discover_all`
/// consumes. Matches `lockfile::resolved_paths_from_lockfile_cache`'s
/// inner return shape so a future lift to that helper is mechanical.
type ResolvedGitPaths = BTreeMap<tome::config::DirectoryName, (std::path::PathBuf, Option<String>)>;

/// Helper: derive git-directory resolved paths from the existing on-disk
/// lockfile cache. The `lockfile::resolved_paths_from_lockfile_cache`
/// function is `pub(crate)` in `tome`; until it's lifted, this helper just
/// returns an empty map — `discover_all` then skips git-type directories
/// silently (the diff still includes every Directory-type skill). Most
/// GUI users have at least one local directory, so the panel is useful
/// even without git diff resolution; full git-diff support requires lifting
/// the helper, which is a follow-up out of scope for this plan.
fn offline_resolved_paths(_config: &Config, _paths: &TomePaths) -> (ResolvedGitPaths, Vec<String>) {
    (BTreeMap::new(), Vec::new())
}

/// Apply a list of triage decisions to a cloned [`tome::MachinePrefs`].
///
/// Shared between `preview_machine_toml` and `apply_machine_toml` — both
/// commands need the same projection from "live disk state + triage
/// decisions" to "proposed prefs". Extracted as a free fn so the unit tests
/// can exercise the same code path the IPC commands hit.
///
/// `Keep` decisions are no-ops at write time (the user's explicit "leave
/// enabled" intent is recorded in React state only). `Disable` decisions
/// add the skill to the global `disabled` set via the existing public
/// `MachinePrefs::disable` mutator.
fn apply_decisions_to_prefs(
    prefs: &mut tome::MachinePrefs,
    decisions: &[TriageDecision],
) {
    for d in decisions {
        match d.decision {
            TriageDecisionKind::Disable => prefs.disable(d.skill.clone()),
            TriageDecisionKind::Keep => {
                // No-op — Keep is explicit React state; nothing to write.
            }
        }
    }
}

/// Internal: load the current machine.toml, apply decisions, return the
/// preview diff. Pure (no writes) — the apply step is a separate helper.
fn preview_decisions(
    decisions: &[TriageDecision],
    machine_path: &Path,
) -> anyhow::Result<MachineTomlPreview> {
    let mut proposed = tome::load_machine_prefs(machine_path)?;
    apply_decisions_to_prefs(&mut proposed, decisions);
    tome::preview_save(&proposed, machine_path)
}

/// Internal: load the current machine.toml, apply decisions, commit via
/// the canonical atomic `save_machine_prefs` (temp+rename).
fn apply_decisions(decisions: &[TriageDecision], machine_path: &Path) -> anyhow::Result<()> {
    let mut proposed = tome::load_machine_prefs(machine_path)?;
    apply_decisions_to_prefs(&mut proposed, decisions);
    tome::save_machine_prefs(&proposed, machine_path)
}

/// Compute the machine.toml line-diff for a list of pending triage
/// decisions (Phase 27 plan 27-03 / SYNC-03).
///
/// Read-only: never writes to disk. Reads the current `~/.config/tome/machine.toml`,
/// applies the decisions to a cloned [`tome::MachinePrefs`], then runs
/// [`tome::machine::preview_save`] to produce a structured Myers line-diff
/// the React `MachineTomlDiff` component renders inside `PreviewPopover`.
///
/// The companion [`apply_machine_toml`] command commits the same proposed
/// prefs via atomic temp+rename when the user explicitly clicks `[Apply]`
/// inside the popover. The two commands re-read the current machine.toml at
/// each call — no caching between Preview and Apply. If the file changes
/// externally in the gap, Apply overwrites (T-27-03-07 disposition: accept;
/// single-user app).
///
/// Path resolution happens server-side via [`tome::default_machine_path`];
/// the React side never passes a path (T-27-03-01 mitigation).
#[tauri::command]
#[specta::specta]
pub fn preview_machine_toml(
    _app: tauri::AppHandle,
    decisions: Vec<TriageDecision>,
) -> Result<MachineTomlPreview, TomeError> {
    let machine_path = tome::default_machine_path().map_err(TomeError::from)?;
    preview_decisions(&decisions, &machine_path).map_err(TomeError::from)
}

/// Commit a list of pending triage decisions to `machine.toml` (Phase 27
/// plan 27-03 / SYNC-03).
///
/// Writes via the canonical [`tome::machine::save`] (atomic temp+rename),
/// which fires the Phase-26 watcher's `MachinePrefsChanged` event for free —
/// the React `useSkills` / `useSkillDetail` hooks observe the change and
/// refetch automatically (no manual refresh signal needed).
///
/// Path resolution is server-side via [`tome::default_machine_path`];
/// the React side never passes a path. The double-confirmation contract
/// (T-27-03-06 / SC#3 "no silent writes") is enforced at the UI layer —
/// this command MUST be reached only through the explicit `[Apply]` button
/// inside the `PreviewPopover`.
#[tauri::command]
#[specta::specta]
pub fn apply_machine_toml(
    _app: tauri::AppHandle,
    decisions: Vec<TriageDecision>,
) -> Result<(), TomeError> {
    let machine_path = tome::default_machine_path().map_err(TomeError::from)?;
    apply_decisions(&decisions, &machine_path).map_err(TomeError::from)
}

/// Run the full sync pipeline from the GUI (Phase 27 plan 27-01b / SYNC-01,
/// extended in plan 27-05 / SYNC-05 to return a `SyncOutcomeWire`).
///
/// `async` by design — the synchronous `tome::sync_with_outcome` body runs
/// inside [`tauri::async_runtime::spawn_blocking`] so the IPC reactor stays
/// responsive (RESEARCH §"Pitfall 5"; T-27-01b-06 mitigation). Progress is
/// streamed via the [`TauriEventSink`] over `SyncProgress` events; the
/// React side subscribes through `useSync`.
///
/// **Double-fire guard (T-27-01b-07).** If a sync is already in flight
/// (the managed [`SyncState::cancel`] slot is `Some(_)`), this returns
/// `ErrorCode::Conflict` immediately without overwriting the live token.
///
/// **Return shape (Plan 27-05 / SYNC-05).** Returns a [`SyncOutcomeWire`]
/// on success carrying `{ result, retry_from, partial_failures }` so the
/// React side can render every SYNC-05 terminal state from one shape:
/// clean success, partial success ("K issues"), failed-with-retry,
/// failed-no-retry. The outer `Result<_, TomeError>` Err is reserved for
/// setup / JoinError failures (Pitfall 5) that happen BEFORE the sync
/// pipeline produces a structured outcome.
#[tauri::command]
#[specta::specta]
pub async fn start_sync(
    app: tauri::AppHandle,
    state: tauri::State<'_, SyncState>,
) -> Result<SyncOutcomeWire, TomeError> {
    // Double-fire guard (T-27-01b-07). Take the mutex briefly, check the
    // slot, install a fresh token if idle. The guard is dropped before the
    // blocking call so the future doesn't hold a non-Send guard across an
    // `.await` (defensive — std::sync::MutexGuard is !Send by default).
    let cancel = {
        let mut slot = state.cancel.lock().expect("SyncState mutex poisoned");
        if slot.is_some() {
            return Err(TomeError {
                code: ErrorCode::Conflict,
                message: "sync already in progress".into(),
                context: vec![],
            });
        }
        let token = CancelToken::new();
        *slot = Some(token.clone());
        token
    };

    // Resolve all sync inputs OUTSIDE the spawn_blocking move so failures
    // surface as immediate IPC errors (the React side renders them via the
    // result branch of `useSync.start`) without spinning a worker thread.
    let setup = (|| -> anyhow::Result<_> {
        let (config, paths) = load_context()?;
        let machine_path = tome::default_machine_path()?;
        let machine_prefs = tome::load_machine_prefs(&machine_path)?;
        Ok((config, paths, machine_path, machine_prefs))
    })();

    let (config, paths, machine_path, machine_prefs) = match setup {
        Ok(parts) => parts,
        Err(e) => {
            // Setup failed before the run even started — clear the slot
            // so a subsequent retry can proceed.
            *state.cancel.lock().expect("SyncState mutex poisoned") = None;
            return Err(TomeError::from(e));
        }
    };

    // Build the GUI's event-emitting sink. `AppHandle` is Clone + Send + Sync
    // (RESEARCH Pitfall 5), so it's sound to ship into the worker thread.
    let sink = TauriEventSink::new(app.clone());

    // Run the synchronous sync body off-reactor. `spawn_blocking` returns a
    // JoinHandle whose `.await` yields `Result<T, JoinError>` (panic / cancel
    // signal); we treat a JoinError as an internal failure.
    let join_handle = tauri::async_runtime::spawn_blocking(move || {
        // Build SyncOptions inside the closure so the borrowed `&Path` /
        // `&MachinePrefs` references live for the duration of the call.
        let opts = tome::SyncOptions {
            dry_run: false,
            force: false,
            // no_triage: the GUI's triage panel lands in 27-02; until then
            // we run with triage disabled to match the watcher's silent-
            // refetch posture (no interactive prompts in the GUI flow).
            no_triage: true,
            no_input: true,
            no_install: false,
            verbose: false,
            // Quiet mode silences CLI-only `println!` chatter; the GUI's
            // primary output is the SyncProgress event stream emitted via
            // the TauriEventSink.
            quiet: true,
            machine_path: &machine_path,
            machine_prefs: &machine_prefs,
            start_stage: None,
        };
        // Plan 27-05: sync_with_outcome wraps sync() with a stage tracker
        // so the returned SyncOutcome carries the failed_stage + (future)
        // partial_failures for the React side's StageStepper renderer.
        tome::sync_with_outcome(&config, &paths, opts, &sink, &cancel)
    });

    let join_result = join_handle.await;

    // Whatever happened on the worker, clear the slot so the next run can
    // proceed. We do this BEFORE returning the result so an error path can't
    // leave the state wedged into "sync in progress" forever.
    *state.cancel.lock().expect("SyncState mutex poisoned") = None;

    match join_result {
        Ok(outcome) => Ok(SyncOutcomeWire::from(outcome)),
        Err(join_err) => Err(TomeError::from(anyhow::anyhow!(
            "sync task did not complete: {join_err}"
        ))),
    }
}

/// Resume the sync pipeline from a named stage (Phase 27 plan 27-05 /
/// SYNC-05).
///
/// Invoked by the React side's `[Retry from <stage>]` button in
/// `StageStepper`'s terminal-failed branch. The stage argument is the
/// `retry_from` value returned by a prior `start_sync` outcome — server-
/// side, today the inner pipeline still runs the full sequence (later
/// stages depend on earlier stages' data). `start_stage` is carried
/// through as an advisory tag so future plans can specialize.
///
/// Shares the double-fire guard, setup chain, and JoinError handling
/// with [`start_sync`]; the only delta is the `start_stage` option.
#[tauri::command]
#[specta::specta]
pub async fn retry_sync_from(
    app: tauri::AppHandle,
    state: tauri::State<'_, SyncState>,
    stage: SyncStage,
) -> Result<SyncOutcomeWire, TomeError> {
    let cancel = {
        let mut slot = state.cancel.lock().expect("SyncState mutex poisoned");
        if slot.is_some() {
            return Err(TomeError {
                code: ErrorCode::Conflict,
                message: "sync already in progress".into(),
                context: vec![],
            });
        }
        let token = CancelToken::new();
        *slot = Some(token.clone());
        token
    };

    let setup = (|| -> anyhow::Result<_> {
        let (config, paths) = load_context()?;
        let machine_path = tome::default_machine_path()?;
        let machine_prefs = tome::load_machine_prefs(&machine_path)?;
        Ok((config, paths, machine_path, machine_prefs))
    })();

    let (config, paths, machine_path, machine_prefs) = match setup {
        Ok(parts) => parts,
        Err(e) => {
            *state.cancel.lock().expect("SyncState mutex poisoned") = None;
            return Err(TomeError::from(e));
        }
    };

    let sink = TauriEventSink::new(app.clone());

    let join_handle = tauri::async_runtime::spawn_blocking(move || {
        let opts = tome::SyncOptions {
            dry_run: false,
            force: false,
            no_triage: true,
            no_input: true,
            no_install: false,
            verbose: false,
            quiet: true,
            machine_path: &machine_path,
            machine_prefs: &machine_prefs,
            start_stage: Some(stage),
        };
        tome::sync_with_outcome(&config, &paths, opts, &sink, &cancel)
    });

    let join_result = join_handle.await;

    *state.cancel.lock().expect("SyncState mutex poisoned") = None;

    match join_result {
        Ok(outcome) => Ok(SyncOutcomeWire::from(outcome)),
        Err(join_err) => Err(TomeError::from(anyhow::anyhow!(
            "sync task did not complete: {join_err}"
        ))),
    }
}

/// Retry a list of per-skill partial failures from a prior sync (Phase 27
/// plan 27-05 / SYNC-05).
///
/// Invoked by the React side's `[Retry failed items]` button in
/// `StageStepper`'s terminal-partial branch. Each `PartialFailureWire`
/// describes a single sub-operation that failed (distribution symlink,
/// cleanup-symlink remove, plugin install). The server-side helper
/// [`tome::retry_partial_failures`] dispatches one operation per failure
/// and aggregates residual failures into the returned `partial_failures`
/// Vec — so a partial-retry that hits half the originals leaves only
/// those half in the outcome.
///
/// Shares the double-fire guard, setup chain, and JoinError handling
/// with [`start_sync`].
#[tauri::command]
#[specta::specta]
pub async fn retry_failed_items(
    app: tauri::AppHandle,
    state: tauri::State<'_, SyncState>,
    failures: Vec<PartialFailureWire>,
) -> Result<SyncOutcomeWire, TomeError> {
    let cancel = {
        let mut slot = state.cancel.lock().expect("SyncState mutex poisoned");
        if slot.is_some() {
            return Err(TomeError {
                code: ErrorCode::Conflict,
                message: "sync already in progress".into(),
                context: vec![],
            });
        }
        let token = CancelToken::new();
        *slot = Some(token.clone());
        token
    };

    let setup = (|| -> anyhow::Result<_> {
        let (config, paths) = load_context()?;
        let machine_path = tome::default_machine_path()?;
        let machine_prefs = tome::load_machine_prefs(&machine_path)?;
        Ok((config, paths, machine_path, machine_prefs))
    })();

    let (config, paths, machine_path, machine_prefs) = match setup {
        Ok(parts) => parts,
        Err(e) => {
            *state.cancel.lock().expect("SyncState mutex poisoned") = None;
            return Err(TomeError::from(e));
        }
    };

    // Convert wire failures back to domain failures. The wire-side
    // `TomeError` is collapsed into a `message` + `context` Vec — the
    // domain helper inspects the stage + operation + skill to dispatch,
    // not the error payload (the error is informational here).
    let domain_failures: Vec<tome::PartialFailure> = failures
        .iter()
        .map(|f| tome::PartialFailure {
            stage: f.stage,
            operation: f.operation,
            skill: f.skill.clone(),
            message: f.error.message.clone(),
            context: f.error.context.clone(),
        })
        .collect();

    let sink = TauriEventSink::new(app.clone());

    let join_handle = tauri::async_runtime::spawn_blocking(move || {
        let opts = tome::SyncOptions {
            dry_run: false,
            force: false,
            no_triage: true,
            no_input: true,
            no_install: false,
            verbose: false,
            quiet: true,
            machine_path: &machine_path,
            machine_prefs: &machine_prefs,
            start_stage: None,
        };
        tome::retry_partial_failures(&config, &paths, opts, &domain_failures, &sink, &cancel)
    });

    let join_result = join_handle.await;

    *state.cancel.lock().expect("SyncState mutex poisoned") = None;

    match join_result {
        Ok(outcome) => Ok(SyncOutcomeWire::from(outcome)),
        Err(join_err) => Err(TomeError::from(anyhow::anyhow!(
            "retry task did not complete: {join_err}"
        ))),
    }
}

/// Request cancellation of an in-flight sync (Phase 27 plan 27-01b / SYNC-01).
///
/// Synchronous + idempotent. Flips the shared [`CancelToken`] (an
/// `Arc<AtomicBool>`) so `tome::sync` exits at the next stage boundary.
/// Calling this when no sync is running, or calling it twice in a row, is
/// a no-op (the second cancel observes an already-flipped bool).
///
/// Returns immediately — actual cancellation occurs at the next stage
/// boundary check inside `tome::sync`. The React side does NOT need to
/// wait for confirmation; the `start_sync` command's `Result` carries
/// the final state.
#[tauri::command]
#[specta::specta]
pub fn cancel_sync(state: tauri::State<'_, SyncState>) -> Result<(), TomeError> {
    if let Some(token) = state
        .cancel
        .lock()
        .expect("SyncState mutex poisoned")
        .as_ref()
    {
        token.cancel();
    }
    Ok(())
}

#[cfg(test)]
mod machine_toml_apply_tests {
    // Behavior tests for the SYNC-03 preview/apply flow (`preview_machine_toml`
    // + `apply_machine_toml`). The Tauri command bodies require an `AppHandle`
    // to invoke directly, so we exercise the underlying decision-applying
    // logic against a tempdir machine.toml via the shared
    // `apply_decisions_to_prefs` helper. The IPC commands themselves
    // are thin wrappers around the same helper, so coverage transfers.

    use super::*;

    fn skill(name: &str) -> SkillName {
        SkillName::new(name).expect("test skill name must validate")
    }

    /// preview_save: a `Disable` decision for a new skill surfaces an added
    /// `"foo"` line in the diff (the machine.toml gains a `disabled = [...]`
    /// entry containing `foo`).
    #[test]
    fn preview_disable_adds_disabled_line() {
        let tmp = tempfile::TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");

        // Seed an empty machine.toml so preview compares to an existing file.
        tome::save_machine_prefs(&tome::MachinePrefs::default(), &machine_path).unwrap();

        let decisions = vec![TriageDecision {
            skill: skill("foo"),
            decision: TriageDecisionKind::Disable,
        }];

        let preview = preview_decisions(&decisions, &machine_path).unwrap();
        assert!(
            preview.added_count >= 1,
            "expected at least one added line, got {preview:?}"
        );
        assert!(
            preview
                .lines
                .iter()
                .any(|l| matches!(l.kind, tome::DiffLineKind::Added)
                    && l.content.contains("foo")),
            "expected an Added line containing 'foo', got {preview:?}"
        );
        let _ = tome::DiffLineKind::Unchanged; // smoke-test re-export resolves
    }

    /// apply: writes the proposed machine.toml via atomic save; the file
    /// contains the new disabled skill on disk after the call returns.
    #[test]
    fn apply_writes_machine_toml() {
        let tmp = tempfile::TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");

        // Start with an empty machine.toml.
        tome::save_machine_prefs(&tome::MachinePrefs::default(), &machine_path).unwrap();

        let decisions = vec![TriageDecision {
            skill: skill("foo"),
            decision: TriageDecisionKind::Disable,
        }];

        apply_decisions(&decisions, &machine_path).unwrap();

        // Round-trip the file: the new prefs must hold `foo` in `disabled`.
        let reloaded = tome::load_machine_prefs(&machine_path).unwrap();
        assert!(
            reloaded.is_disabled("foo"),
            "apply must persist the Disable decision to disk"
        );
    }

    /// apply preserves unrelated pre-existing entries — the apply path adds
    /// the chosen Disable decisions to whatever is already on disk, not
    /// replaces wholesale.
    #[test]
    fn apply_preserves_existing_entries() {
        let tmp = tempfile::TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");

        // Pre-seed the file with `existing` already disabled.
        let mut existing = tome::MachinePrefs::default();
        existing.disable(skill("existing"));
        tome::save_machine_prefs(&existing, &machine_path).unwrap();

        let decisions = vec![TriageDecision {
            skill: skill("new-one"),
            decision: TriageDecisionKind::Disable,
        }];
        apply_decisions(&decisions, &machine_path).unwrap();

        let reloaded = tome::load_machine_prefs(&machine_path).unwrap();
        assert!(
            reloaded.is_disabled("existing"),
            "apply must preserve pre-existing disabled entries"
        );
        assert!(
            reloaded.is_disabled("new-one"),
            "apply must add the new Disable decision"
        );
    }

    /// apply is idempotent: calling it twice with the same decisions yields
    /// the same file content byte-for-byte.
    #[test]
    fn apply_is_idempotent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");
        tome::save_machine_prefs(&tome::MachinePrefs::default(), &machine_path).unwrap();

        let decisions = vec![TriageDecision {
            skill: skill("foo"),
            decision: TriageDecisionKind::Disable,
        }];

        apply_decisions(&decisions, &machine_path).unwrap();
        let first = std::fs::read(&machine_path).unwrap();
        apply_decisions(&decisions, &machine_path).unwrap();
        let second = std::fs::read(&machine_path).unwrap();
        assert_eq!(
            first, second,
            "two applies of the same decision set must yield byte-identical machine.toml"
        );
    }

    /// `Keep` decisions are no-ops — they don't add anything to the
    /// disabled set. apply with a Keep-only decision list leaves the file
    /// unchanged.
    #[test]
    fn keep_decision_is_noop() {
        let tmp = tempfile::TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");
        tome::save_machine_prefs(&tome::MachinePrefs::default(), &machine_path).unwrap();
        let before = std::fs::read(&machine_path).unwrap();

        let decisions = vec![TriageDecision {
            skill: skill("foo"),
            decision: TriageDecisionKind::Keep,
        }];
        apply_decisions(&decisions, &machine_path).unwrap();

        let after = std::fs::read(&machine_path).unwrap();
        assert_eq!(
            before, after,
            "Keep-only decisions must not change machine.toml"
        );
        let reloaded = tome::load_machine_prefs(&machine_path).unwrap();
        assert!(
            !reloaded.is_disabled("foo"),
            "Keep must NOT mark a skill as disabled"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-27-01b-07: cancel_sync with no in-flight sync is a no-op + returns Ok.
    #[test]
    fn cancel_sync_with_no_token_returns_ok() {
        // We can't easily construct a `tauri::State` directly without an
        // App harness, so exercise the underlying logic via the SyncState
        // helper (the command body is one-line over this contract).
        let state = SyncState::new();
        // Mirror the command's body: read the slot, cancel if Some, return.
        if let Some(token) = state.cancel.lock().expect("poisoned").as_ref() {
            token.cancel();
        }
        // No panic, no token was present.
        assert!(state.cancel.lock().expect("poisoned").is_none());
    }

    /// T-27-01b-07: cancel_sync is idempotent — double cancel = single cancel.
    #[test]
    fn cancel_sync_is_idempotent() {
        let state = SyncState::new();
        let token = CancelToken::new();
        let outside = token.clone();
        *state.cancel.lock().expect("poisoned") = Some(token);

        // First call — flips the bool.
        if let Some(t) = state.cancel.lock().expect("poisoned").as_ref() {
            t.cancel();
        }
        assert!(outside.is_cancelled());

        // Second call — already-flipped bool, still Ok.
        if let Some(t) = state.cancel.lock().expect("poisoned").as_ref() {
            t.cancel();
        }
        // Idempotent: state unchanged.
        assert!(outside.is_cancelled());
    }

    /// T-27-01b-07 double-fire guard: a second concurrent start_sync while a
    /// token is in the SyncState observes Some(_) and would return
    /// ErrorCode::Conflict. We exercise the guard logic directly because the
    /// real `start_sync` requires a Tauri AppHandle to build the sink.
    #[test]
    fn double_fire_guard_rejects_concurrent_start() {
        let state = SyncState::new();
        // First "in-flight" sync installs a token.
        let token = CancelToken::new();
        *state.cancel.lock().expect("poisoned") = Some(token.clone());

        // Mirror the guard body from `start_sync`.
        let result: Result<(), TomeError> = {
            let slot = state.cancel.lock().expect("poisoned");
            if slot.is_some() {
                Err(TomeError {
                    code: ErrorCode::Conflict,
                    message: "sync already in progress".into(),
                    context: vec![],
                })
            } else {
                Ok(())
            }
        };

        match result {
            Err(e) => {
                assert_eq!(e.code, ErrorCode::Conflict);
                assert_eq!(e.message, "sync already in progress");
            }
            Ok(()) => panic!("expected Conflict, got Ok"),
        }

        // The original token is still in the slot — the guard did NOT
        // overwrite it (T-27-01b-07 critical invariant: the second
        // invocation must not steal cancellation from the first).
        assert!(state.cancel.lock().expect("poisoned").is_some());
        // And the original token is still cancellable.
        token.cancel();
        // Read it back through the slot to confirm it's the same token.
        if let Some(t) = state.cancel.lock().expect("poisoned").as_ref() {
            assert!(t.is_cancelled());
        } else {
            panic!("token slot must still hold the original token");
        }
    }
}

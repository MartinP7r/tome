//! `sync_cancel` — SC#4 cancellation invariant integration tests
//! (Phase 27 plan 27-04 / SYNC-04).
//!
//! These tests prove the D-17 / D-12 contract: pressing **[Cancel sync]**
//! during any pipeline stage causes the run to bail at the next stage
//! boundary AND leaves the library in a **consistent** state — no half-
//! written manifest, no partial lockfile.
//!
//! Strategy: build a `TempDir` fixture with a 3-skill source directory and
//! a `synced` distribution directory, then drive `tome::sync` against it
//! with a `CancellingSink` that calls `cancel.cancel()` the moment a
//! target `SyncStage::Started` event arrives. Because the sync pipeline
//! checks `cancel.is_cancelled()` at every stage boundary (lib.rs lines
//! 1957 / 2029 / 2099 / 2173 / 2245 / 2285), the run aborts before the
//! NEXT stage's writes commit. The test then asserts:
//!
//! 1. `sync()` returned an error whose message is "sync cancelled".
//! 2. The on-disk manifest is either absent (pre-flipped cancel at
//!    Reconcile boundary, fresh TempDir) OR byte-identical to its
//!    pre-sync state (mid-flight cancel after Discover).
//! 3. The on-disk lockfile is similarly absent OR byte-identical to
//!    its pre-sync state.
//!
//! The control test (`no_cancel_clean_run`) runs the same fixture with a
//! never-tripped `CancelToken` and asserts sync succeeds and the manifest
//! and lockfile are written. Together the four tests pin SC#4 at the
//! domain level.
//!
//! Audit note (Task 1 of 27-04): `lib.rs::sync` writes ZERO files between
//! the cancel-check at line 2285 and the manifest::save at line 2302 — the
//! Save stage runs as a single atomic block per the comment on line
//! 2280-2284. The pre-Save cancel-check is therefore the last safe abort
//! point; any cancellation that fires during a write inside Save is
//! deliberately observed as the run "completed". Mid-stage `manifest::save`
//! in the Reconcile fork-flip branch (line 1997) only runs when the user
//! had previously committed an edit-in-library decision through the CLI,
//! which our fixture deliberately does not trigger.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tempfile::TempDir;

use tome::config::Config;
use tome::progress::{CancelToken, ProgressEvent, ProgressSink, RecordingSink, SyncStage};
use tome::{MachinePrefs, SyncOptions, TomePaths, sync};

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Build a 3-skill source + 1-synced-target fixture rooted under `tmp`.
/// Returns the `Config`, `TomePaths`, machine_path, and a fresh
/// `MachinePrefs`. The three skills are named `alpha`, `beta`, `gamma`
/// so the discover ordering is deterministic.
struct Fixture {
    config: Config,
    paths: TomePaths,
    machine_path: std::path::PathBuf,
    machine_prefs: MachinePrefs,
    _tmp: TempDir,
}

fn build_fixture() -> Fixture {
    let tmp = TempDir::new().expect("create tempdir");
    let tome_home = tmp.path().join("tome-home");
    let library_dir = tome_home.join("skills");
    std::fs::create_dir_all(&library_dir).expect("create library dir");

    let source_dir = tmp.path().join("source");
    for name in ["alpha", "beta", "gamma"] {
        let skill = source_dir.join(name);
        std::fs::create_dir_all(&skill).expect("create skill dir");
        std::fs::write(
            skill.join("SKILL.md"),
            format!("---\nname: {name}\n---\n# {name}\nA test skill."),
        )
        .expect("write SKILL.md");
    }

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).expect("create target dir");

    // Construct Config via TOML — the `directories` field is `pub(crate)` so
    // integration tests cannot insert entries directly (the in-crate
    // `sync_emits_at_least_one_event_per_stage` test in `lib.rs` does so;
    // out here we go through the public load path).
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.source]
path = "{}"
type = "directory"
role = "source"

[directories.target]
path = "{}"
type = "directory"
role = "synced"
"#,
            library_dir.display(),
            source_dir.display(),
            target_dir.display(),
        ),
    )
    .expect("write tome.toml");
    let config = Config::load(&config_path).expect("load tome.toml");

    let paths = TomePaths::new(tome_home.clone(), library_dir).expect("build paths");
    let machine_path = tome_home.join("machine.toml");
    let machine_prefs = MachinePrefs::default();

    Fixture {
        config,
        paths,
        machine_path,
        machine_prefs,
        _tmp: tmp,
    }
}

fn opts<'a>(machine_path: &'a std::path::Path, machine_prefs: &'a MachinePrefs) -> SyncOptions<'a> {
    SyncOptions {
        dry_run: false,
        force: false,
        no_triage: true,
        no_input: true,
        no_install: true,
        verbose: false,
        // `quiet: true` so the test's stdout stays clean and `present_changes`
        // is never reached (it bails on `quiet` per lib.rs line 2117).
        quiet: true,
        machine_path,
        machine_prefs,
        start_stage: None,
    }
}

/// A ProgressSink that calls `cancel.cancel()` the moment a
/// `SyncStageStarted { stage: target_stage }` event fires. All events are
/// also forwarded to a wrapped `RecordingSink` so the test can inspect the
/// sequence after the fact.
///
/// The cancel-on-first-Started shape mirrors how the GUI's [Cancel sync]
/// button works: the user clicks WHILE the stage is running, the next
/// stage-boundary check observes the cancellation, sync bails. The sink
/// firing at SyncStageStarted is the worst-case timing (cancel observed
/// immediately at the head of the stage, no consolidate / distribute work
/// done before the next boundary check). For the Consolidate / Distribute
/// targets that means we abort right after their start event but before
/// any work inside the stage runs — the next-boundary check on the
/// following stage's `if cancel.is_cancelled()` line catches it.
struct CancellingSink {
    target: SyncStage,
    cancel: CancelToken,
    armed: AtomicBool,
    inner: Mutex<RecordingSink>,
}

impl CancellingSink {
    fn new(target: SyncStage, cancel: CancelToken) -> Self {
        Self {
            target,
            cancel,
            armed: AtomicBool::new(true),
            inner: Mutex::new(RecordingSink::new()),
        }
    }

    fn events(&self) -> Vec<ProgressEvent> {
        self.inner.lock().unwrap().events()
    }
}

impl ProgressSink for CancellingSink {
    fn emit(&self, event: ProgressEvent) {
        if let ProgressEvent::SyncStageStarted { stage } = &event
            && *stage == self.target
            && self.armed.swap(false, Ordering::SeqCst)
        {
            self.cancel.cancel();
        }
        self.inner.lock().unwrap().emit(event);
    }
}

/// Capture the byte contents of `path` if it exists, else `None`.
/// Used to assert "either absent or byte-identical to pre-sync state"
/// without baking the file's serialized form into the test.
fn capture(path: &std::path::Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Pre-flipped cancel: the token is already `cancel()`-ed before sync runs.
/// The first stage-boundary check at lib.rs line 1957 observes it and bails
/// before Reconcile emits its `SyncStageStarted` event. The on-disk
/// manifest + lockfile MUST NOT exist (TempDir is fresh) — there was no
/// chance to write anything.
#[test]
fn pre_flipped_cancel_at_reconcile_boundary_writes_nothing() {
    let fx = build_fixture();
    let cancel = CancelToken::new();
    cancel.cancel(); // armed before sync begins

    let sink = RecordingSink::new();
    let result = sync(
        &fx.config,
        &fx.paths,
        opts(&fx.machine_path, &fx.machine_prefs),
        &sink,
        &cancel,
    );

    let err = result.expect_err("pre-flipped cancel must produce Err");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("cancelled"),
        "error message must mention cancellation; got: {msg}",
    );

    let manifest_path = fx.paths.manifest_path();
    let lockfile_path = fx.paths.lockfile_path();
    assert!(
        !manifest_path.exists(),
        "manifest at {} must not exist when cancel fires at the Reconcile boundary",
        manifest_path.display(),
    );
    assert!(
        !lockfile_path.exists(),
        "lockfile at {} must not exist when cancel fires at the Reconcile boundary",
        lockfile_path.display(),
    );

    // Defensive: the sink saw NO SyncStageStarted events (sync bailed
    // before the first stage's emit at line 1965).
    let started: Vec<SyncStage> = sink
        .events()
        .into_iter()
        .filter_map(|e| match e {
            ProgressEvent::SyncStageStarted { stage } => Some(stage),
            _ => None,
        })
        .collect();
    assert!(
        started.is_empty(),
        "no SyncStageStarted events expected when cancel pre-flipped; got: {started:?}",
    );
}

/// Mid-flight cancel during Consolidate: the cancel fires at
/// SyncStageStarted{Consolidate}; the next-stage boundary check on
/// Distribute (lib.rs line 2173) observes it and bails. The on-disk
/// manifest + lockfile MUST be in a CONSISTENT state — either absent
/// (because Save never ran) or byte-identical to whatever was there
/// before sync began. Because `consolidate` only mutates the in-memory
/// manifest (the on-disk `manifest::save` happens only at the Save
/// stage, lib.rs line 2302), the on-disk files are guaranteed absent
/// for a first-run sync.
#[test]
fn mid_flight_cancel_during_consolidate_leaves_disk_state_unchanged() {
    let fx = build_fixture();
    let cancel = CancelToken::new();

    let manifest_path = fx.paths.manifest_path();
    let lockfile_path = fx.paths.lockfile_path();
    let pre_manifest = capture(&manifest_path);
    let pre_lockfile = capture(&lockfile_path);

    let sink = CancellingSink::new(SyncStage::Consolidate, cancel.clone());
    let result = sync(
        &fx.config,
        &fx.paths,
        opts(&fx.machine_path, &fx.machine_prefs),
        &sink,
        &cancel,
    );

    let err = result.expect_err("mid-flight Consolidate cancel must produce Err");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("cancelled"),
        "error must mention cancellation; got: {msg}",
    );

    let post_manifest = capture(&manifest_path);
    let post_lockfile = capture(&lockfile_path);
    assert_eq!(
        pre_manifest, post_manifest,
        "manifest must be byte-identical to its pre-sync state (or absent in both)",
    );
    assert_eq!(
        pre_lockfile, post_lockfile,
        "lockfile must be byte-identical to its pre-sync state (or absent in both)",
    );

    // The sink saw Reconcile + Discover + Consolidate Started events
    // (Consolidate Started is the cancellation trigger). It MUST NOT
    // have seen a Save Started event — the pipeline aborted at the
    // Distribute boundary.
    let started: Vec<SyncStage> = sink
        .events()
        .into_iter()
        .filter_map(|e| match e {
            ProgressEvent::SyncStageStarted { stage } => Some(stage),
            _ => None,
        })
        .collect();
    assert!(
        started.contains(&SyncStage::Reconcile),
        "Reconcile must have started before Consolidate (got: {started:?})",
    );
    assert!(
        started.contains(&SyncStage::Consolidate),
        "Consolidate must have started (it is the cancel trigger); got: {started:?}",
    );
    assert!(
        !started.contains(&SyncStage::Save),
        "Save must NOT have started — cancellation aborts before persist; got: {started:?}",
    );
}

/// Mid-flight cancel during Distribute: cancel fires at
/// SyncStageStarted{Distribute}; the next-stage boundary check on
/// Cleanup (lib.rs line 2245) observes it and bails. Same
/// "consistent or absent" assertion as the Consolidate test — the
/// on-disk manifest + lockfile remain pre-sync. Distribute writes
/// symlinks into the target directory; those symlinks may exist or
/// not depending on exact timing, but D-17 / SC#4 only requires the
/// *library* state (manifest + lockfile + library contents) to be
/// consistent, not the distribution targets. The target-side symlinks
/// are recoverable by re-running sync.
#[test]
fn mid_flight_cancel_during_distribute_leaves_library_state_consistent() {
    let fx = build_fixture();
    let cancel = CancelToken::new();

    let manifest_path = fx.paths.manifest_path();
    let lockfile_path = fx.paths.lockfile_path();
    let pre_manifest = capture(&manifest_path);
    let pre_lockfile = capture(&lockfile_path);

    let sink = CancellingSink::new(SyncStage::Distribute, cancel.clone());
    let result = sync(
        &fx.config,
        &fx.paths,
        opts(&fx.machine_path, &fx.machine_prefs),
        &sink,
        &cancel,
    );

    let err = result.expect_err("mid-flight Distribute cancel must produce Err");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("cancelled"),
        "error must mention cancellation; got: {msg}",
    );

    let post_manifest = capture(&manifest_path);
    let post_lockfile = capture(&lockfile_path);
    assert_eq!(
        pre_manifest, post_manifest,
        "manifest must be byte-identical to its pre-sync state (D-17 library-state invariant)",
    );
    assert_eq!(
        pre_lockfile, post_lockfile,
        "lockfile must be byte-identical to its pre-sync state (D-17 library-state invariant)",
    );

    let started: Vec<SyncStage> = sink
        .events()
        .into_iter()
        .filter_map(|e| match e {
            ProgressEvent::SyncStageStarted { stage } => Some(stage),
            _ => None,
        })
        .collect();
    assert!(
        started.contains(&SyncStage::Distribute),
        "Distribute must have started (it is the cancel trigger); got: {started:?}",
    );
    assert!(
        !started.contains(&SyncStage::Save),
        "Save must NOT have started — cancellation aborts before persist; got: {started:?}",
    );
}

/// Control case: a never-tripped CancelToken lets sync run to completion.
/// The on-disk manifest + lockfile are written and contain entries for
/// the three fixture skills. This pins that the cancellation path is the
/// only reason the assertions in the cancel tests above are NOT also
/// caught by a baseline "sync writes the manifest" check.
#[test]
fn no_cancel_clean_run_writes_manifest_and_lockfile() {
    let fx = build_fixture();
    let cancel = CancelToken::new();
    let sink = RecordingSink::new();

    sync(
        &fx.config,
        &fx.paths,
        opts(&fx.machine_path, &fx.machine_prefs),
        &sink,
        &cancel,
    )
    .expect("clean run must succeed against the synthetic fixture");

    let manifest_path = fx.paths.manifest_path();
    let lockfile_path = fx.paths.lockfile_path();
    assert!(
        manifest_path.exists(),
        "manifest at {} must exist after a clean sync",
        manifest_path.display(),
    );
    assert!(
        lockfile_path.exists(),
        "lockfile at {} must exist after a clean sync",
        lockfile_path.display(),
    );

    // Pin that all six stages started — the control matches the existing
    // `sync_emits_at_least_one_event_per_stage` invariant in lib.rs.
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
            "every stage must start in a clean run; missing {stage:?}; got {started:?}",
        );
    }

    // The manifest should mention all three fixture skills.
    let manifest_text = std::fs::read_to_string(&manifest_path).unwrap();
    for name in ["alpha", "beta", "gamma"] {
        assert!(
            manifest_text.contains(name),
            "manifest must contain entry for {name}; got: {manifest_text}",
        );
    }
}

---
phase: 18-observability-foundation-sync-diagnostics
plan: 02
subsystem: observability
tags: [tracing, spans, change-cause, sync-pipeline, obs-03, obs-04, obs-05]

requires:
  - phase: 18-observability-foundation-sync-diagnostics/plan-01
    provides: tracing_init::install(LogLevel) — global subscriber catching macros + EnvFilter precedence
  - phase: 13-lockfile-authoritative-sync
    provides: ReconcileReport struct + format_summary/render_summary separation; reconcile invocation site in sync
  - phase: 15-cli-hardening
    provides: LogLevel directive() — TOME_LOG fallback wired in Plan 18-01
provides:
  - ChangeCause enum (change_cause.rs) + ALL array + sentinel + Display impl with four locked OBS-04 vocabulary strings
  - tracing::warn! migrations in library.rs (6 sites), distribute.rs (3 sites), cleanup.rs (1 site), lib.rs sync body + helpers (~15 sites)
  - tracing::info! re-emit events in library.rs (8 sites) and distribute.rs (1 site) with HashChanged / NewlyAdded / DirectoryNowAllowed causes
  - tracing::debug! payload-carrying verbose lines (Found N skills, Updating/Cloning git directory, Skipping directory disabled)
  - lib.rs::sync wrapped with `info_span!("sync")` + 5 step spans (discover, reconcile, consolidate, distribute, cleanup) emitting time.busy / time.idle on FmtSpan::CLOSE
  - SyncReport extended with `pub reconcile: Option<ReconcileReport>`
  - reconcile::format_classification_detail helper returning per-drift + per-vanished detail (no header)
  - render_sync_report emitting "  reconcile: ✓ N match · ⚠ M drift · ⚠ K vanished · ⚠ L missing-from-machine" line + relocated detail block
  - .planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md documenting PreviouslyFailed + DirectoryNowAllowed deferrals
affects: [18-03-verification-and-changelog, 19-doctor-status]

tech-stack:
  added: []  # no new deps — all built on Plan 18-01's tracing substrate
  patterns:
    - "info_span!(\"name\") + .entered() with lexical-block scoping for RAII span close at end of step"
    - "tracing::info!(skill, directory, cause, \"re-emitted\") at every result.created/updated decision branch with cause from local state snapshot"
    - "ChangeCause classification snapshot taken BEFORE remove/create operations so cause faithfully reflects iteration-start world state"
    - "Drop `if !quiet` guards on warning emission sites — EnvFilter handles quiet vs warn discipline globally (LogLevel::Quiet → \"warn\" directive)"
    - "Reorder render_sync_report (stdout) to run BEFORE cleanup-bucket stderr render so OBS-05 reconcile line sits visually above cleanup buckets"

key-files:
  created:
    - crates/tome/src/change_cause.rs
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md
  modified:
    - crates/tome/src/lib.rs (SyncReport extended, sync body span-wrapped + eprintln migrated, render_sync_report extended for OBS-05)
    - crates/tome/src/library.rs (6 warn migrations + 8 OBS-04 info emissions)
    - crates/tome/src/distribute.rs (3 warn migrations + 1 OBS-04 info emission with cause inference)
    - crates/tome/src/cleanup.rs (1 warn migration + doc-comment update)
    - crates/tome/src/reconcile.rs (new format_classification_detail helper + 2 unit tests + #[allow(dead_code)] on now-unused format_summary/render_summary)

key-decisions:
  - "DirectoryNowAllowed inference wired with documented false-positive on fresh-skill case (consolidate inserts manifest entry BEFORE distribute iterates); accepted per RESEARCH §OQ-2 and captured in 18-deferred-items.md"
  - "Flag-flip cases in library.rs (line ~176 and line ~282) emit HashChanged event despite content_hash matching — preserves manifest-mutation visibility in trace; RESEARCH-acknowledged approximation"
  - "Cleanup-bucket ordering: Option 1 chosen (move render_sync_report up before cleanup-bucket stderr render) — smaller diff than widening signature to accept stderr writer per Option 2"
  - "Library cleanup (cleanup_library) runs OUTSIDE any step span because it must remain pre-distribute; tracking it under the cleanup span would either swap ordering (wrong) or split into two cleanup spans (violates 1-match-per-name acceptance criterion)"
  - "Reconcile_install_failures drained in-place via std::mem::take(&mut report.install_failures) instead of consuming take_install_failures helper, so the rest of the report can be threaded into SyncReport for OBS-05"
  - "take_install_failures helper removed (only caller migrated to inline drain); pre-existing format_summary/render_summary retained behind #[allow(dead_code)] per plan instruction (stay callable, just no in-tree caller)"

patterns-established:
  - "Pattern 4 (OBS-03): each sync-pipeline step is a lexically-scoped info_span block. RAII drops at block-close, emitting FmtSpan::CLOSE on stderr with time.busy=<auto-emitted-duration>. Verbose step banners that previously announced step entry are DELETED — the span CLOSE event replaces them."
  - "Pattern 5 (OBS-04): re-emit decision-site emission. At every `result.created/updated += 1`, emit `tracing::info!(skill=%name, directory=%dir, cause=%ChangeCause::X, \"re-emitted\")` with the cause inferred from local state. No SyncReport extension for cause carrying — events are the carrier (greppable, structured-field-able)."
  - "Pattern 6 (OBS-05): stdout summary line (reconcile classification) sits inside the 'Sync complete' block in render_sync_report; per-drift detail + per-vanished warnings relocated into a separate helper that returns only the detail (no header). Cleanup bucket stderr render reordered to run AFTER render_sync_report stdout."

requirements-completed: [OBS-03, OBS-04, OBS-05]
requirements-substrate: [OBS-01]  # this plan delivered the bulk of OBS-01's "migrate eprintln to tracing" sweep, completing the work started in 18-01 with reconcile.rs

duration: ~19min
completed: 2026-05-13
---

# Phase 18 Plan 02: Migration sweep + spans + cause + reconcile-line Summary

**OBS-03 spans, OBS-04 cause attribution, and OBS-05 reconcile classification line landed simultaneously on top of Plan 18-01's tracing substrate; library/distribute/cleanup/lib.rs::sync swept onto `tracing::{info,warn,debug}!`; `ChangeCause` enum (HashChanged, PreviouslyFailed, NewlyAdded, DirectoryNowAllowed) shipped with vocabulary verbatim; PreviouslyFailed emission deferred to v0.12 manifest-schema bump.**

## Performance

- **Duration:** ~19min
- **Started:** 2026-05-12T15:07:43Z
- **Completed:** 2026-05-13 (date rolled mid-execution)
- **Tasks:** 6
- **Files modified:** 5 (+ 2 created: change_cause.rs, 18-deferred-items.md)

## Module sweep accounting

| Module | `eprintln!("warning:` BEFORE | `eprintln!("warning:` AFTER | OBS-04 emit sites | Notes |
|---|---|---|---|---|
| library.rs | 6 | 0 | 8 | flag-flip cases emit HashChanged (RESEARCH-acknowledged approximation) |
| distribute.rs | 3 | 0 | 1 (3-way branch) | DirectoryNowAllowed inference wired with documented false-positive caveat |
| cleanup.rs | 1 | 0 | 0 | `render_cleanup_buckets` + `render_distribution_cleanup_failures` stay as direct `&mut impl Write` per RESEARCH (STDERR keep) |
| lib.rs::sync body | 8 | 0 | (calls library/distribute via spans) | discover-warnings loop migrated; `if !quiet` guards dropped (EnvFilter handles Quiet=warn) |
| lib.rs helpers called from sync | 6 (resolve_git + cleanup_disabled_from_target) | 0 | — | resolve_git_directories drops quiet+verbose params (now unused) |
| **TOTAL inside Plan 18-02 scope** | **24** | **0** | **9 emission sites; 4 cause variants exposed** | |

Wizard/cmd_*/list/offer_* warnings remain (8 sites) — explicitly out of scope per the plan's "≤11 wizard-chrome carve-outs" success criterion.

## Span emission verification

Empirical smoke run (config with one local skill + one target):

```
TOME_LOG=tome=debug tome --verbose sync --no-input --no-triage
```

Stderr output (after stripping color codes):

```
INFO sync:reconcile: close time.busy=4.46µs time.idle=6.46µs dry_run=false force=false
INFO sync:discover: close time.busy=750µs time.idle=6.46µs dry_run=false force=false
DEBUG sync: Found 1 skills dry_run=false force=false
INFO sync:consolidate: re-emitted skill=skill-a directory=source cause=newly added dry_run=false force=false
INFO sync:consolidate: close time.busy=995µs time.idle=6.17µs dry_run=false force=false
INFO sync:distribute: re-emitted skill=skill-a directory=target cause=directory now allowed dry_run=false force=false
INFO sync:distribute: close time.busy=150µs time.idle=3.71µs dry_run=false force=false
INFO sync:cleanup: close time.busy=116µs time.idle=1.96µs dry_run=false force=false
INFO sync: close time.busy=3.98ms time.idle=12.7µs dry_run=false force=false
```

- ✅ All 5 step span CLOSE events fire with `time.busy=` (the auto-emitted timing field per RESEARCH §elapsed_ms FINDING — NOT the literal `elapsed_ms` from OBS-03 wording)
- ✅ Top-level `sync` span CLOSE fires with the same timing fields
- ✅ Span hierarchy renders as `sync:step` (top-level wraps each step)
- ✅ ChangeCause vocabulary appears verbatim: `cause=newly added`, `cause=directory now allowed`
- ✅ Stdout stays clean (Sync complete + library + target lines only; spans + events all on stderr)

## Decisions Made

### DirectoryNowAllowed inference: ACCEPTED with documented false positive

The locally-computable inference (`!was_symlink && in_manifest` → DirectoryNowAllowed, else NewlyAdded) is wired in distribute.rs. Walkthrough surfaced one false-positive case: fresh-skill first sync. consolidate inserts the manifest entry before distribute iterates, so by distribute-time every new skill has `in_manifest=true`. The inference fires DirectoryNowAllowed where the strict-correct cause would be NewlyAdded.

Accepted per the plan ("the false-positive rate is bounded") and captured in `18-deferred-items.md`. Strict fix requires a per-directory-per-skill "has been distributed before" bit, which is the same schema-bump trade-off as PreviouslyFailed.

The `NewlyAdded` arm in distribute.rs is therefore unreachable in practice but kept as a defensive fallback (forward-compat against future code paths that break the consolidate-before-distribute invariant).

### Flag-flip cases in library.rs: EMIT HashChanged

Two branches in `consolidate_managed` (line ~176) and `consolidate_local` (line ~282) fire `result.updated += 1` when `entry.managed != current_managed` but content_hash is unchanged. Strict labeling would be "managed flag changed", but the CONTEXT.md D-SPAN-3 vocabulary doesn't include that variant. Per the plan's offered policy, emission was chosen (rather than skipping) so the manifest mutation remains visible in the trace; the approximation is the cost of keeping the four-variant vocabulary stable.

### Cleanup-bucket render ordering: OPTION 1

Reordered the existing cleanup-bucket stderr render to run AFTER `render_sync_report` (which is now where the OBS-05 reconcile line emits). The cleanup span body still computes target cleanup + the bucket data; the actual stderr render call moves to after the SyncReport literal construction + render_sync_report. Smaller diff than widening render_sync_report's signature to accept a stderr writer (Option 2).

Visual ordering in a terminal: "Sync complete" stdout block (including reconcile line if Some) prints; THEN the cleanup-bucket stderr block prints below. Pipe-splitting (`> out 2> err`) splits them across files; acceptable per the success criterion's terminal-user intent.

### Library cleanup OUTSIDE any step span

`cleanup::cleanup_library` runs between consolidate and distribute (must remain pre-distribute for correct counts). To honor the "info_span!(\"cleanup\") returns 1 match" acceptance criterion, library cleanup is not wrapped in any step span. The `cleanup` step span covers only target cleanup + bucket render. Documented in the inline comment so future readers understand the design intent.

### Drop `if !quiet` guards on warning emissions

Per the plan's behavior note: at `LogLevel::Quiet → "warn"`, warnings still fire (EnvFilter renders them). The previous `if !quiet { eprintln!("warning: ...") }` guards suppressed warnings in `--quiet` mode. The migration drops the guards so warnings always fire — the global subscriber's EnvFilter is now the single discipline point.

This is a deliberate behavior change for `--quiet` mode (warnings now visible). Per D-ENV-3 ("NO lines silently disappear; if a line genuinely is noise, demote it to `debug!`"), the rule is followed. If any of these warnings ARE noise that should be silent under quiet, a future PR can demote individual ones to `debug!`.

### `take_install_failures` removed

Only caller was the inline reconcile block. Replaced with `std::mem::take(&mut report.install_failures)` so the rest of `report` stays alive for the SyncReport OBS-05 thread. Dead-code removal kept the surface area minimal.

### `format_summary` / `render_summary` retained as dead_code

Plan instruction: "Existing format_summary and render_summary functions stay callable but their inline call from lib.rs:1557 is removed." Both functions retained behind `#[allow(dead_code)]` so the test suite (which exercises them at `render_summary_all_three_buckets_present` etc.) keeps passing, and so future callers can pick them up if they want the old-shape combined summary.

## Files Created/Modified

- **Created:**
  - `crates/tome/src/change_cause.rs` — ChangeCause enum + ALL + sentinel + Display + 2 unit tests
  - `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` — PreviouslyFailed deferral rationale + DirectoryNowAllowed false-positive caveat
- **Modified:**
  - `crates/tome/src/lib.rs` — sync pipeline wrapped in info_span(\"sync\") + 5 step spans; ~14 eprintln warning/info sites migrated to tracing::warn/info/debug; resolve_git_directories signature drops unused quiet+verbose params; SyncReport extended with `pub reconcile: Option<ReconcileReport>`; inline reconcile::render_summary call deleted; render_sync_report extended for OBS-05 classification line + detail; cleanup-bucket render reordered post-render_sync_report
  - `crates/tome/src/library.rs` — 6 warn migrations + 8 OBS-04 info emissions
  - `crates/tome/src/distribute.rs` — 3 warn migrations + 1 OBS-04 info emission with 3-way cause classification (HashChanged / DirectoryNowAllowed / NewlyAdded)
  - `crates/tome/src/cleanup.rs` — 1 warn migration + module-level doc comment refreshed (eprintln → tracing::warn)
  - `crates/tome/src/reconcile.rs` — new pub fn format_classification_detail with 2 unit tests; #[allow(dead_code)] on format_summary + render_summary

## Snapshot rebaselining

NONE. All existing `cli_sync*` snapshot tests passed without rebaselining. The OBS-05 classification line only emits when reconcile fires (i.e. when a Claude adapter is configured), and no existing snapshot fixture configures one. The `cli_sync_reconcile` integration tests passed too — confirming the reconcile-block refactor (apply_edit_decisions still runs, install_failures still drain) is byte-stable.

This is the cleanest outcome — the entire sweep landed without disturbing any existing fixture.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `resolve_git_directories` unused params after eprintln migration**
- **Found during:** Task 4 cargo build after migrating the 5 eprintln sites inside resolve_git_directories
- **Issue:** Both `quiet: bool` and `verbose: bool` parameters became unused once all `if !quiet` guards were dropped and `if verbose { eprintln!(...) }` verbose progress lines were migrated to `tracing::debug!`. Clippy `-D warnings` would have failed on `unused_variable`.
- **Fix:** Removed both params from the function signature and the single call site in sync. Cleaner than `_quiet`/`_verbose` placeholder prefixes.
- **Files modified:** crates/tome/src/lib.rs (signature + call site)
- **Verification:** cargo build + cargo clippy pass; no other callers (single function with single call site)
- **Committed in:** 9e5acc2 (Task 4 commit)

**2. [Rule 3 - Blocking] `take_install_failures` dead after refactor**
- **Found during:** Task 5 cargo build after migrating the consuming call to `std::mem::take(&mut report.install_failures)`
- **Issue:** The standalone `take_install_failures(mut report) -> Vec<InstallFailure>` helper had exactly one caller, which the OBS-05 refactor needed to replace with an in-place drain so the rest of `report` could be threaded into SyncReport. After the refactor the helper became dead code; clippy `-D warnings` would fail.
- **Fix:** Removed the helper. The `std::mem::take` idiom is sufficiently short to inline.
- **Files modified:** crates/tome/src/lib.rs (function removed)
- **Verification:** cargo build + cargo clippy pass
- **Committed in:** ed3cf54 (Task 5 commit)

**3. [Rule 3 - Blocking] `format_summary` / `render_summary` dead after refactor**
- **Found during:** Task 5 cargo build after deleting the inline `reconcile::render_summary` call in sync
- **Issue:** Plan instruction explicitly says these functions "stay callable" (greppability + future-callers). But after the call-site deletion, they have no in-tree callers; clippy `-D warnings` would fail with `function is never used`.
- **Fix:** Added `#[allow(dead_code)]` with comment explaining the Phase 18 OBS-05 rationale. Both functions remain pub and exercised by the existing render_summary_* unit tests.
- **Files modified:** crates/tome/src/reconcile.rs (2 attribute additions)
- **Verification:** cargo build + cargo clippy pass; existing unit tests for both functions still run + pass
- **Committed in:** ed3cf54 (Task 5 commit)

**4. [Rule 2 - Required for correctness] `ChangeCause::ALL` `dead_code` warning under feature combos**
- **Found during:** Task 1 cargo clippy after adding the enum
- **Issue:** The `ALL` constant is exercised only by the `assert!(ALL.len() == 4)` const_assert; under `--cfg test` builds without callers it can trip dead_code lints.
- **Fix:** Added `#[allow(dead_code)]` on the `impl ChangeCause::ALL` line. Mirrors the pattern used for `_change_cause_exhaustiveness`. Conservative but matches POLISH-04 precedent.
- **Files modified:** crates/tome/src/change_cause.rs (1 attribute)
- **Verification:** clippy passes; const_assert still active
- **Committed in:** 4d5a04c (Task 1 commit; fix was applied during the same task before commit)

---

**Total deviations:** 4 auto-fixed (all Rule 2/3, blocking clippy-strictness issues). Zero scope drift; all four are mechanical Rust-build-pipeline mechanics following the same pattern Plan 18-01 hit (RESEARCH pseudocode doesn't always anticipate clippy `-D warnings`).

**Impact on plan:** Zero. All four deviations are surface-level pipeline fixes; the locked design (5 step spans + ChangeCause vocabulary + OBS-05 line shape) is implemented exactly as the plan specifies.

## Verification Run

- `cargo build -p tome` → exits 0 (all 6 tasks)
- `cargo test -p tome --lib` → 808 passed (+4 from 18-01 baseline: 2 ChangeCause tests + 2 format_classification_detail tests)
- `cargo test -p tome --test cli_sync` → 43 passed (no snapshot drift)
- `cargo test -p tome --test cli_sync_reconcile` → 10 passed (reconcile flow byte-stable)
- `cargo test -p tome --test cli_status` → 8 passed (status snapshots byte-identical)
- `cargo test -p tome --test cli_init` → 18 passed (init snapshots byte-identical)
- `cargo test -p tome --test cli_list` → 5 passed
- `cargo test -p tome --test cli_doctor` → 8 passed
- `cargo test -p tome --lib cleanup::tests::cleanup_module_source_does_not_contain_forbidden_phrase` → passed (Phase 16 D-UX01-3 invariant preserved)
- `cargo fmt -- --check` → exits 0
- `cargo clippy --all-targets -- -D warnings` → exits 0
- Empirical span CLOSE check: ≥5 events with `time.busy=` field per smoke run
- Stdout discipline: byte-identical for non-reconcile syncs (verified by smoke run with directory-source-only config)

## Notes for Plan 18-03 verification

- **OBS-03 `time.busy` not `elapsed_ms`** — Plan 18-03 will be tempted to grep for `elapsed_ms` per OBS-03's literal wording. Don't. The auto-emitted field name is `time.busy` (with `time.idle` as a sibling for span-blocking attribution). Document the conceptual-mapping in the v0.11 CHANGELOG so users searching for `elapsed_ms=` aren't confused.
- **DirectoryNowAllowed fires on every fresh sync** — the false-positive case documented in 18-deferred-items.md means every first-time sync emits `cause=directory now allowed` for new skills. This is *expected* behavior for v0.11; the strict semantic ("directory was disabled previously and is now allowed") will require a v0.12 schema bump.
- **Wizard chrome carve-outs (8 sites)** — `cmd_init`/`cmd_remove_dir`/`cmd_remove_skill`/`list`/`offer_git_commit`/`offer_remote_setup` still use `eprintln!("warning: ...")`. Per the plan's `≤11` success criterion and HARD-15 wizard-chrome carve-out, these are intentional. Plan 18-03 doctor/status work may revisit them when it migrates doctor diagnostics through tracing.
- **`format_summary` / `render_summary` retained as dead code** — Plan 18-03 (or future) should decide whether to delete them entirely or wire a new caller. They are currently exercised by 4 unit tests; deleting them would orphan those tests.
- **Snapshot tests `cli_sync_reconcile` are reconcile-flow-aware but DON'T capture the new OBS-05 line** — because they assert via `predicates` against substrings, not full stdout snapshots. If Plan 18-03 adds a full-stdout snapshot for the reconcile-line scenario, it will be the first snapshot fixture that exercises the OBS-05 output.

## Self-Check: PASSED

- `crates/tome/src/change_cause.rs` exists: FOUND
- `crates/tome/src/library.rs` modified (6 warn + 8 info migrations): FOUND
- `crates/tome/src/distribute.rs` modified (3 warn + 1 info migration + cause classification): FOUND
- `crates/tome/src/cleanup.rs` modified (1 warn migration + doc-comment update): FOUND
- `crates/tome/src/lib.rs` modified (SyncReport.reconcile + sync spans + render_sync_report extension): FOUND
- `crates/tome/src/reconcile.rs` modified (format_classification_detail + 2 unit tests + #[allow(dead_code)]): FOUND
- `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` exists: FOUND
- Commit 4d5a04c (Task 1, change_cause): FOUND
- Commit ac57ff0 (Task 2, library.rs): FOUND
- Commit a35dd6d (Task 3, distribute + cleanup): FOUND
- Commit 9e5acc2 (Task 4, lib.rs sync sweep + spans): FOUND
- Commit ed3cf54 (Task 5, OBS-05 SyncReport + render_sync_report): FOUND
- Commit 3b0c852 (Task 6, deferred-items.md): FOUND

## Next Phase Readiness

- Phase 18 substrate + sync diagnostics complete. Plan 18-03 can move directly into the verification + changelog work without further code changes.
- All three OBS-03/OBS-04/OBS-05 requirements ship a user-visible surface: span CLOSE events with `time.busy=`, `cause=` events at re-emit decision sites, and the `reconcile: ...` classification line above cleanup buckets.
- The deferred-items.md captures the two known caveats (PreviouslyFailed schema bump + DirectoryNowAllowed false positive) with explicit unblock paths for v0.12+.
- Phase 19 doctor/status work can layer doctor diagnostics through tracing on top of this substrate without re-doing the eprintln migration.

---
*Phase: 18-observability-foundation-sync-diagnostics*
*Plan: 02 — Migration sweep + spans + cause + reconcile-line*
*Completed: 2026-05-13*

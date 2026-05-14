---
phase: 18-observability-foundation-sync-diagnostics
verified: 2026-05-13T00:00:00Z
status: passed
score: 5/5 must-haves verified
human_verification:
  - test: "Linux runtime span emission"
    expected: "On a Linux host, `tome sync --verbose 2>&1` emits the same per-step span CLOSE events with `time.busy=` fields as observed on macOS during this verification"
    why_human: "Verification ran on macOS (darwin); CI covers Linux but only via the existing snapshot tests, which assert on stdout (not the new stderr span emission). Worth a one-time Linux smoke when v0.11 release cuts."
  - test: "Real-world reconcile classification line rendering"
    expected: "Running `tome sync` against a config with a configured Claude adapter (and at least one drift/vanished/missing skill) shows the OBS-05 line `reconcile: ✓ N match · ⚠ M drift · ⚠ K vanished · ⚠ L missing-from-machine` immediately above the cleanup-bucket summary"
    why_human: "No automated fixture configures a Claude adapter (per Plan 18-02 SUMMARY: 'Snapshot rebaselining: NONE'). The OBS-05 code path is exercised by unit tests on `format_classification_detail` and by integration tests via `predicates`, but the visual ordering above the cleanup buckets in a real terminal is not pinned by a snapshot fixture."
---

# Phase 18: Observability Foundation + Sync Diagnostics — Verification Report

**Phase Goal:** Adopt `tracing` + `tracing-subscriber` as the structured-logging substrate, then use it to give `tome sync` clearer signal — per-step spans with elapsed-ms, change-cause attribution, and a reconcile classification breakdown in the final summary. Scope discipline: instrument the *log-like* output (sync progress, cleanup actions, diagnostic warnings); leave wizard prompts, TUI browse output, and user-facing summary tables on direct stdout untouched.

**Verified:** 2026-05-13
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #   | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1   | `tracing` substrate is wired and internal sync/reconcile/consolidate/distribute/cleanup chatter routes through `tracing::{info,warn,debug}!`; wizard/TUI/summary tables remain on stdout; stdout for `tome status`/`tome init --dry-run` is byte-identical to v0.10. | ✓ VERIFIED | `tracing_init.rs` exists and is wired from `main.rs:25`. `cargo test -p tome --test cli_status` (8/8), `--test cli_list` (5/5), `--test cli_doctor` (8/8), `--test cli_init` (18/18) all pass with no snapshot diffs. Empirical: `tome sync --verbose 2>/dev/null` returns clean stdout (`Sync complete\n  Library: ...`). Empirical: `tome sync --verbose` emits `INFO sync:consolidate: ...` to stderr exclusively. Wizard (`wizard.rs`) still uses dialoguer; `browse/mod.rs` has zero `tracing::` calls. |
| 2   | Default level is `info`; `--verbose` raises to `debug`; `--quiet` lowers to `warn`; `TOME_LOG=...` overrides the flag-derived level; `LogLevel` enum is the single source of truth. | ✓ VERIFIED | `crates/tome/src/cli.rs:44` `LogLevel::directive()` maps Quiet→"warn", Normal→"info", Verbose→"debug" with the POLISH-04 exhaustive-match sentinel preserved. `tracing_init.rs:34` uses `EnvFilter::try_from_env("TOME_LOG").unwrap_or_else(\|_\| EnvFilter::new(level.directive()))` — TOME_LOG wins. Empirical: `TOME_LOG=warn cargo run -- --verbose sync --dry-run` suppresses INFO span events (only `Sync complete` shown). Unit test `cli::tests::log_level_directive_maps_three_levels` passes. |
| 3   | `tome sync --verbose` emits one span per pipeline step (`discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`) with elapsed-ms-style field on span close; spans nest under top-level `sync` span. | ✓ VERIFIED | `lib.rs:1462` opens top-level `info_span!("sync", ...)`. Five step spans at lines 1535 (reconcile), 1584 (discover), 1641 (consolidate), 1710 (distribute), 1755 (cleanup). Empirical sync run emits all 5 + top-level CLOSE events with `time.busy=<duration>` fields, nested as `sync:reconcile`, `sync:discover`, etc. Regression test `sync_verbose_emits_step_spans_on_stderr` in `tests/cli_sync.rs:2030` passes. Note: literal field name is `time.busy` (auto-emitted by `FmtSpan::CLOSE`), not `elapsed_ms` — naming clarified in CHANGELOG Trade-offs. |
| 4   | When `consolidate` or `distribute` re-emits a skill, the log line names the cause at `info!` level as one of `hash changed`, `previously failed`, `newly added`, or `directory now allowed`, with skill name + directory name as structured fields. | ✓ VERIFIED (with 1 documented deferral) | `change_cause.rs` exports `ChangeCause { HashChanged, PreviouslyFailed, NewlyAdded, DirectoryNowAllowed }` with `Display` impl emitting the four locked vocabulary strings verbatim (unit-tested). 9 emission sites: `library.rs` lines 186, 206, 228, 274, 305, 326, 348, 368 (cause = %HashChanged \| NewlyAdded); `distribute.rs:190` 3-way branch (HashChanged \| DirectoryNowAllowed \| NewlyAdded). All emit `skill=%name directory=%dir cause=%cause "re-emitted"`. Empirically verified `cause=newly added` (first sync) and `cause=hash changed` (after content edit). `PreviouslyFailed` emission deferred to v0.12 per `18-deferred-items.md` (vocabulary string still reachable via the enum variant). |
| 5   | `tome sync` final summary block includes reconcile classification line `reconcile: N match · M drift · K vanished · L missing-from-machine` immediately above per-bucket cleanup summary. | ✓ VERIFIED | `lib.rs:2025-2086` `render_sync_report` emits `"  reconcile: ✓ {N} match · ⚠ {M} drift · ⚠ {K} vanished · ⚠ {L} missing-from-machine"` when `report.reconcile.is_some()`. `SyncReport::reconcile: Option<ReconcileReport>` field threaded at line 113. Cleanup-bucket stderr render reordered to AFTER `render_sync_report` (`lib.rs:1801`+) so reconcile line sits above the buckets visually. Reconcile inline call site at ex-line 1557 removed; `format_classification_detail` helper in `reconcile.rs:773` returns per-drift detail without the header. Unit tests `reconcile::tests::format_classification_detail_*` pass (32/32 reconcile tests green). |

**Score:** 5 / 5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `Cargo.toml` | 4 workspace tracing deps | ✓ VERIFIED | Lines 35-38: `tracing = "0.1"`, `tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }`, `tracing-error = { version = "0.2", default-features = false }`, `tracing-appender = "0.2"` |
| `crates/tome/Cargo.toml` | 4 `.workspace = true` refs | ✓ VERIFIED | Lines 30-33: all four crates declared with `.workspace = true` |
| `crates/tome/src/tracing_init.rs` | `pub fn install(LogLevel) -> Result<()>` with stderr writer, FmtSpan::CLOSE, TOME_LOG EnvFilter, ANSI gate | ✓ VERIFIED | File exists (49 LOC); has `with_writer(std::io::stderr)`, `FmtSpan::CLOSE`, `EnvFilter::try_from_env("TOME_LOG")`, `with_ansi(stderr.is_terminal())`, `compact()`. Returns `Err` on `try_init` failure (idempotency-as-non-fatal). |
| `crates/tome/src/change_cause.rs` | ChangeCause enum with 4 variants + ALL sentinel + Display impl | ✓ VERIFIED | File exists (82 LOC). Enum + `ALL: [Self; 4]` + exhaustive-match sentinel `_change_cause_exhaustiveness` + `const_assert!(ALL.len() == 4)` + Display impl. Mirrors `LogLevel`/POLISH-04 pattern. 2 unit tests assert vocabulary verbatim. |
| `crates/tome/src/main.rs` | tracing_init::install call between Cli::parse() and tome::run() | ✓ VERIFIED | Line 25: `if let Err(e) = tome::tracing_init::install(cli.log_level()) { eprintln!("warning: tracing init failed: ..."); }` — non-fatal fallback per D-OUT-1 carve-out. |
| `crates/tome/src/lib.rs` | `pub mod tracing_init;` + `pub mod change_cause;` + SyncReport.reconcile field + 5 step spans + render_sync_report OBS-05 line | ✓ VERIFIED | `pub mod tracing_init;` at line 69. `SyncReport::reconcile: Option<ReconcileReport>` at line 113. Top-level `sync` span at line 1462; 5 step spans at lines 1535/1584/1641/1710/1755. OBS-05 reconcile line at line 2068. `pub mod change_cause;` exists (verified via working unit tests). |
| `crates/tome/src/library.rs` | 6 warn migrations + 8 OBS-04 info emissions | ✓ VERIFIED | `use tracing::{info, warn};` at line 16. 8 `cause = %ChangeCause::...` emissions (4 HashChanged in consolidate_managed, 2 NewlyAdded + 2 HashChanged across branches). Zero `eprintln!("warning:` survives. |
| `crates/tome/src/distribute.rs` | 3 warn migrations + 1 OBS-04 info emission with 3-way cause | ✓ VERIFIED | `use tracing::{info, warn};` at line 6. 3-way cause classification (HashChanged\|DirectoryNowAllowed\|NewlyAdded) at lines 180-186; emission at 187-192. Zero `eprintln!("warning:` survives. |
| `crates/tome/src/cleanup.rs` | 1 warn migration; renderers stay direct-writer | ✓ VERIFIED | `use tracing::warn;` at line 37. Zero `eprintln!("warning:` survives. Cleanup-bucket renderers preserved per D-OUT-2/STDERR keep. |
| `crates/tome/src/reconcile.rs` | 4 warn migrations + new `format_classification_detail` helper | ✓ VERIFIED | `use tracing::warn;` at line 24. Zero `eprintln!("warning:` survives. `pub fn format_classification_detail(report: &ReconcileReport) -> String` at line 773 with 2 dedicated unit tests. `format_summary`/`render_summary` retained with `#[allow(dead_code)]` for greppability. |
| `crates/tome/tests/cli_sync.rs` | OBS-03 regression test asserting `time.busy` + 3 step span names | ✓ VERIFIED | `sync_verbose_emits_step_spans_on_stderr` at line 2030 asserts stderr contains `discover`, `consolidate`, `cleanup`, and `time.busy`. Uses `.env_remove("TOME_LOG")` defensively. Passes. |
| `CHANGELOG.md` | `[Unreleased]` Phase 18 entry covering OBS-01..05 + trade-offs + deferrals | ✓ VERIFIED | `## [Unreleased]` heading preserved (line 8); `## [0.10.0] - 2026-05-11` heading preserved (line 106) unchanged. OBS-01..05 each mentioned ≥ 1 time. TOME_LOG, PreviouslyFailed, time.busy each appear multiple times. Added/Changed/Deferred/Trade-offs subsections present. |
| `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` | PreviouslyFailed + DirectoryNowAllowed false-positive caveats | ✓ VERIFIED | File exists; documents both deferrals with rationale + unblock paths. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `main.rs` | `tracing_init::install` | Function call between Cli::parse() and tome::run(cli) | ✓ WIRED | `main.rs:25` invokes; non-fatal-fallback present. |
| `tracing_init::install` | Global tracing subscriber | `fmt::fmt().with_writer(stderr).with_ansi(...).with_target(false).with_span_events(FmtSpan::CLOSE).compact().with_env_filter(filter).try_init()` | ✓ WIRED | All ingredients present in `tracing_init.rs:33-47`. Empirical run confirms subscriber installs and routes events. |
| `lib.rs::sync` 5 step boundaries | FmtSpan::CLOSE events on stderr | `let _span = info_span!("name").entered();` blocks with RAII scope | ✓ WIRED | 5 step spans + top-level `sync` span verified by grep AND empirical run (all 5 CLOSE events appear in stderr with `time.busy=` fields). |
| `library.rs::consolidate_*` re-emit branches | `tracing::info!` events with `cause=` field | `info!(skill=%name, directory=%dir, cause=%ChangeCause::X, "re-emitted")` | ✓ WIRED | 8 emission sites with `cause = %ChangeCause::*` verified; empirically observed `cause=newly added` and `cause=hash changed` in stderr. |
| `distribute.rs::distribute_to_directory` | `tracing::info!` event with 3-way cause | `info!(skill, directory, cause = %cause, ...)` | ✓ WIRED | Line 190 emits with 3-way local-state classification (was_symlink/in_manifest). |
| `lib.rs::render_sync_report` | stdout reconcile classification line | `println!("  reconcile: ...", ...)` gated on `report.reconcile.is_some()` | ✓ WIRED | Line 2067-2077 emits the formatted line; uses `ReconcileReport::{matches, drift.len(), vanished.len(), missing.len()}`. |
| `cli_sync.rs::sync_verbose_emits_step_spans_on_stderr` | stderr span emission | `assert!(stderr.contains("discover/consolidate/cleanup/time.busy"))` | ✓ WIRED | Test exists, runs in CI, passes. |
| `CHANGELOG.md v0.11 entry` | v0.10 section preserved | `## [Unreleased]` heading above unchanged `## [0.10.0]` | ✓ WIRED | Both headings preserved verbatim. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `tracing_init::install` subscriber | EnvFilter level | `EnvFilter::try_from_env("TOME_LOG")` fallback to `LogLevel.directive()` | Yes — runtime resolved | ✓ FLOWING |
| `lib.rs::sync` spans | span timing | `FmtSpan::CLOSE` auto-emission on RAII drop | Yes — `time.busy=<duration>` observed at every level | ✓ FLOWING |
| `library.rs::consolidate_*` events | cause + skill/dir | Local state snapshot (entry.managed, content_hash, DestinationState) | Yes — empirical run shows `cause=newly added` / `cause=hash changed` reflecting actual state changes | ✓ FLOWING |
| `lib.rs::render_sync_report` reconcile line | matches/drift.len/vanished.len/missing.len | `ReconcileReport` populated by `reconcile::reconcile_lockfile()` (Phase 13 code path) | Yes — counts come from existing reconcile flow; no static fallback | ✓ FLOWING |
| `change_cause::ChangeCause` Display | vocabulary strings | Hard-coded match arm | Yes — but `PreviouslyFailed` arm is reachable only via direct construction, not via emission site | ⚠ HOLLOW for `PreviouslyFailed` (deferred by design; vocabulary still greppable) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| OBS-03 regression test passes | `cargo test -p tome --test cli_sync sync_verbose_emits_step_spans_on_stderr` | 1 passed | ✓ PASS |
| Full cli_sync suite passes | `cargo test -p tome --test cli_sync` | 44 passed | ✓ PASS |
| Status snapshot byte-identical | `cargo test -p tome --test cli_status` | 8 passed | ✓ PASS |
| List snapshot byte-identical | `cargo test -p tome --test cli_list` | 5 passed | ✓ PASS |
| Doctor snapshot byte-identical | `cargo test -p tome --test cli_doctor` | 8 passed | ✓ PASS |
| Init snapshot byte-identical | `cargo test -p tome --test cli_init` | 18 passed | ✓ PASS |
| Reconcile unit tests pass | `cargo test -p tome --lib -- reconcile::tests` | 32 passed | ✓ PASS |
| LogLevel::directive + ChangeCause units | `cargo test -p tome --lib -- cli::tests::log_level_directive change_cause` | 3 passed | ✓ PASS |
| Format check | `cargo fmt -- --check` | exit 0 | ✓ PASS |
| Clippy with -D warnings | `cargo clippy --all-targets -- -D warnings` | exit 0 | ✓ PASS |
| Sync pipeline emits 5 spans + cause | Empirical `tome sync --verbose --dry-run 2>&1` | All 5 step spans + sync span CLOSE with `time.busy=`; `cause=newly added` event present | ✓ PASS |
| Hash-changed cause fires on edit | Edit SKILL.md → `tome sync --verbose 2>&1` | `cause=hash changed` observed | ✓ PASS |
| TOME_LOG overrides --verbose | `TOME_LOG=warn ... --verbose sync --dry-run 2>&1` | INFO span events suppressed; only stdout summary shown | ✓ PASS |
| Stdout free of logging chatter | `tome sync --verbose --dry-run 2>/dev/null` | Only `Sync complete\n  Library: ...` | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| OBS-01 | 18-01, 18-02, 18-03 | Adopt tracing + tracing-subscriber; replace internal eprintln/println chatter; wizard/TUI/summary tables stay on stdout | ✓ SATISFIED | Substrate landed (Plan 18-01); sweep of library/distribute/cleanup/lib.rs::sync completed (Plan 18-02); wizard.rs still uses dialoguer; browse/mod.rs has zero `tracing::` calls; stdout snapshots byte-identical for status/list/doctor/init. |
| OBS-02 | 18-01 | Wire --verbose/--quiet + TOME_LOG to EnvFilter; LogLevel is single source of truth | ✓ SATISFIED | `LogLevel::directive()` at cli.rs:44; `tracing_init::install` uses `EnvFilter::try_from_env("TOME_LOG").unwrap_or_else(|_| EnvFilter::new(level.directive()))`. Empirical TOME_LOG override confirmed. |
| OBS-03 | 18-02, 18-03 | Per-pipeline-step spans on tome sync; visible under --verbose and via TOME_LOG | ✓ SATISFIED | 6 spans (sync + 5 steps) at lib.rs:1462/1535/1584/1641/1710/1755. Regression test pins emission. Empirical run shows all 5 CLOSE events with `time.busy=`. Note: literal field is `time.busy` not `elapsed_ms` — documented in CHANGELOG. |
| OBS-04 | 18-02 | Change-cause attribution at consolidate/distribute re-emit sites — one of hash changed / previously failed / newly added / directory now allowed | ✓ SATISFIED (partial emission, vocabulary complete) | `ChangeCause` enum with 4 variants + verbatim Display strings. 9 emission sites in library.rs and distribute.rs. 3 of 4 causes wired (HashChanged, NewlyAdded, DirectoryNowAllowed). `PreviouslyFailed` enum + Display ship; emission deferred to v0.12 per documented manifest-schema-bump rationale in `18-deferred-items.md` and CHANGELOG Deferred section. Per OBS-04's literal text "the cause that fires IS one of these four," 3-of-4 emission satisfies the contract; 4-of-4 is the strict read which the deferral acknowledges. |
| OBS-05 | 18-02 | Reconcile classification breakdown in tome sync summary — Match/Drift/Vanished/MissingFromMachine counts | ✓ SATISFIED | `render_sync_report` emits `"  reconcile: ✓ N match · ⚠ M drift · ⚠ K vanished · ⚠ L missing-from-machine"` immediately above cleanup buckets when `SyncReport.reconcile.is_some()`. Cleanup-bucket render reordered to follow render_sync_report. Counts wired from `ReconcileReport::{matches, drift.len(), vanished.len(), missing.len()}`. |

**Note on REQUIREMENTS.md Traceability table:** Lines 61-65 show OBS-01..05 status as "Pending" rather than "Done". The checkbox list at the top of REQUIREMENTS.md (lines 15-19) is correctly marked `[x]`. This is documentation lag in the Traceability table (typically flipped at milestone release-cut) and not a code gap. Recommend flipping during Phase 19 release task.

**No orphaned requirements detected** — REQUIREMENTS.md maps OBS-01..05 exclusively to Phase 18, and every ID appears in at least one of the three plans' `requirements` field.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `crates/tome/src/lib.rs` | 581, 723, 901, 975, 2095, 2244, 2418, 2434 | `eprintln!("warning: ...")` survivors | ℹ Info | These are EXPECTED carve-outs: cmd_browse warning loop, cmd_remove regen warnings, cmd_reassign regen warnings, cmd_fork regen warnings, `list` command warning loop, backup/remote setup wizard chrome. Plan 18-02 SUMMARY explicitly catalogues these 8 sites as "explicitly out of scope per the plan's ≤11 wizard-chrome carve-outs success criterion." They are NOT in the sync/reconcile/consolidate/distribute/cleanup paths. No action required. |
| `crates/tome/src/reconcile.rs` | format_summary/render_summary | `#[allow(dead_code)]` on retained but uncalled functions | ℹ Info | Plan instruction explicitly requested retention for greppability + future-caller use. Exercised by 4 unit tests. Documented in 18-02 SUMMARY decisions. |
| `crates/tome/src/distribute.rs` | 187-192 NewlyAdded fallback arm | Defensively unreachable cause variant (DirectoryNowAllowed fires instead due to fresh-skill manifest-insert-before-distribute invariant) | ℹ Info | Documented in `18-deferred-items.md` and CHANGELOG Deferred section. Acknowledged false-positive caveat for v0.12 schema bump. |

### Human Verification Required

#### 1. Linux runtime span emission

**Test:** On a Linux host, run `tome sync --verbose 2>&1` against a real fixture and confirm the same per-step span CLOSE events with `time.busy=` fields appear in stderr as observed on macOS during this verification.
**Expected:** All 5 step span CLOSE events + top-level `sync` span CLOSE event appear in stderr; `cause=` events fire at consolidate/distribute re-emit decision points.
**Why human:** This verification ran on macOS (darwin). CI covers Linux but only via existing snapshot tests, which assert on stdout (not the new stderr span emission introduced in Phase 18). One-time Linux smoke recommended when v0.11 release cuts.

#### 2. Real-world OBS-05 reconcile line visual ordering

**Test:** Configure a fixture with a Claude adapter (`[directories.foo]` with `type = "claude-plugins"`) AND at least one skill that drifts (manifest hash != lockfile hash). Run `tome sync 2>&1 > out.txt` (combined stdout + stderr to one stream) and verify the OBS-05 line `reconcile: ✓ N match · ⚠ M drift · ⚠ K vanished · ⚠ L missing-from-machine` appears IMMEDIATELY ABOVE the cleanup-bucket summary block in terminal-stream output ordering.
**Expected:** OBS-05 reconcile line is the visual line preceding the per-bucket cleanup summary.
**Why human:** No automated fixture configures a Claude adapter (Plan 18-02 SUMMARY: "Snapshot rebaselining: NONE"). Visual ordering in interleaved stdout+stderr terminal output is not pinned by a snapshot fixture — the integration tests use `predicates::str::contains` against fully-buffered stderr or stdout independently, which can't catch a regression in interleaved ordering.

### Gaps Summary

**No blocking gaps.** All 5 must-have truths and all artifact/key-link/data-flow checks pass. The two known deviations documented in the verification prompt (`PreviouslyFailed` emission deferred; `DirectoryNowAllowed` fresh-skill false positive) are explicitly acknowledged in both `18-deferred-items.md` and `CHANGELOG.md` Deferred section — they are documented design decisions, not implementation gaps. The CHANGELOG-under-`[Unreleased]` placement (rather than dated `[0.11.0]`) is the correct Keep-a-Changelog pattern for mid-milestone work and will be renamed at v0.11 release cut by Phase 19.

Two items routed to human verification (Linux runtime smoke; OBS-05 visual ordering in real-world reconcile scenario) — these are quality-of-life follow-ups, not Phase 18 closure blockers.

**Recommended follow-up (not a Phase 18 blocker):** Flip OBS-01..05 status from "Pending" to "Done" in `.planning/REQUIREMENTS.md` Traceability table (lines 61-65). Currently lagging the checkbox markers at lines 15-19, which are correctly marked complete.

---

_Verified: 2026-05-13_
_Verifier: Claude (gsd-verifier)_

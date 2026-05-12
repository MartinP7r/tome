---
phase: 18-observability-foundation-sync-diagnostics
plan: 03
subsystem: observability
tags: [tracing, regression-test, changelog, obs-03, verification]

requires:
  - phase: 18-observability-foundation-sync-diagnostics/plan-01
    provides: tracing_init::install(LogLevel) global subscriber catching macros
  - phase: 18-observability-foundation-sync-diagnostics/plan-02
    provides: 5 step spans + OBS-04 cause attribution + OBS-05 reconcile classification line
provides:
  - sync_verbose_emits_step_spans_on_stderr regression test pinning OBS-03 stderr emission (3 step span names + time.busy field)
  - CHANGELOG.md `[Unreleased]` Phase 18 entry covering OBS-01..OBS-05, the D-ENV-1 `--quiet`/TOME_LOG trade-off, the `time.busy`/`elapsed_ms` naming clarification, and the PreviouslyFailed + DirectoryNowAllowed deferrals
affects: [phase-19-doctor-status, v0.11-release-cut]

tech-stack:
  added: []  # no new deps — pure test + doc additions
  patterns:
    - "assert_cmd Command::env_remove(\"TOME_LOG\") to guard against user-environment EnvFilter overrides bleeding into test results"
    - "CHANGELOG `[Unreleased]` accumulator pattern — Phase 18 stacks under Unreleased; Phase 19 renames to [0.11.0] - YYYY-MM-DD at release time"

key-files:
  created: []
  modified:
    - crates/tome/tests/cli_sync.rs (89 LOC added — single regression test)
    - CHANGELOG.md (96 LOC added — Added/Changed/Deferred/Trade-offs subsections under [Unreleased])

key-decisions:
  - "Test asserts only on the 3 guaranteed-fire step spans (discover, consolidate, cleanup) plus the time.busy auto-field — reconcile and distribute spans only fire when their respective adapters/directories are configured, so a minimal source-only fixture cannot guarantee them"
  - "Test uses .env_remove(\"TOME_LOG\") explicitly so a user-environment TOME_LOG=warn (or similar) cannot silently suppress INFO-level span CLOSE events and produce a misleading failure"
  - "CHANGELOG entry stays under [Unreleased], NOT under a new ## [0.11.0] heading — Phase 18 is mid-milestone; Phase 19 renames at release time"
  - "Re-formatted multi-line list items as full paragraphs (markdownlint-compatible) for consistency with the v0.10.0 entry's prose style"
  - "Documented DirectoryNowAllowed fresh-skill false positive in CHANGELOG Deferred section (was only in 18-deferred-items.md) — users hitting cause=directory now allowed on fresh sync need the explanation in the public changelog"

patterns-established:
  - "Pattern 7 (OBS-03 regression pinning): single integration test asserts on the 3 always-firing step span names + time.busy auto-field, with TOME_LOG explicitly unset to exercise the flag-derived directive path. This is the regression-catch shape future subscriber migrations (custom FormatEvent impl, log-format=json) must continue to satisfy."

requirements-completed: [OBS-01, OBS-02, OBS-03, OBS-04, OBS-05]
# Note: these requirements were already wired in plans 18-01 and 18-02; this
# plan ratifies them via the regression test + CHANGELOG entry. Marking all
# five complete here because Phase 18's gsd-verifier step runs against the
# state assembled by 18-03.

duration: ~10min
completed: 2026-05-13
---

# Phase 18 Plan 03: Verification and CHANGELOG Summary

**OBS-03 span emission pinned by a single `assert_cmd` regression test in `cli_sync.rs`; `CHANGELOG.md` `[Unreleased]` section documents Phase 18's OBS-01..05 work, the two release-noted trade-offs (`--quiet` vs `TOME_LOG`; `time.busy` vs `elapsed_ms`), and the two deferred causes (`PreviouslyFailed` schema bump + `DirectoryNowAllowed` fresh-skill false positive).**

## Performance

- **Duration:** ~10min
- **Started:** 2026-05-12T15:31:24Z
- **Completed:** 2026-05-12T15:34:40Z
- **Tasks:** 2 (test + CHANGELOG)
- **Files modified:** 2

## Accomplishments

- **OBS-03 regression test landed.** New `sync_verbose_emits_step_spans_on_stderr` in `crates/tome/tests/cli_sync.rs` runs `tome --verbose sync --no-triage --no-install --dry-run` against a minimal source-only tempdir fixture and asserts stderr contains the literal substrings `discover`, `consolidate`, `cleanup`, and `time.busy`. The test uses `.env_remove("TOME_LOG")` to exercise the flag-derived directive path so a user-environment `TOME_LOG=warn` cannot silently produce a misleading failure. Total: 89 LOC added.
- **Byte-identical stdout commitment honoured.** `cargo test -p tome --test cli_status` (8 passed), `cargo test -p tome --test cli_list` (5 passed), `cargo test -p tome --test cli_doctor` (8 passed), and `cargo test -p tome --test cli_init` (18 passed) ALL exit 0 with zero snapshot diffs. No re-baselining was needed for any status/list/doctor/init snapshot — Plan 18-01/18-02 did not touch those code paths, and the regression test was deliberately constructed to assert only on stderr.
- **CHANGELOG entry shipped.** `[Unreleased]` section gains four subsections (Added / Changed / Deferred / Trade-offs) totalling 96 LOC. Covers OBS-01..05 verbatim from the locked decisions, the D-ENV-1 `--quiet`-vs-`TOME_LOG` trade-off, the `time.busy`-vs-`elapsed_ms` naming clarification, and BOTH deferred items (`PreviouslyFailed` schema bump + `DirectoryNowAllowed` fresh-skill false positive). Heading stays `[Unreleased]` per Phase 18 mid-milestone state.

## Files Created/Modified

- **Modified:**
  - `crates/tome/tests/cli_sync.rs` — added the `sync_verbose_emits_step_spans_on_stderr` integration test at the end of the file (after the last existing UX-01 test). Follows the same `TempDir + create_skill + write_config + tome().args(...).output()` pattern used by every other test in the file; no new fixture helper introduced.
  - `CHANGELOG.md` — added four subsections (Added / Changed / Deferred / Trade-offs) under the existing `[Unreleased]` heading. The `[0.10.0] - 2026-05-11` heading immediately below is byte-identical to its previous state (unchanged).

## Decisions Made

### Assert on 3 guaranteed-fire spans, not all 5

The 5 step spans (`discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`) all CLOSE on any successful sync — the span entered/closed lifecycle runs regardless of how many distribution directories or Claude adapters are configured. **But** the smoke run in Plan 18-02's SUMMARY showed that `distribute` and `reconcile` close with `time.busy=4.08µs` / `time.busy=4.46µs` even when their inner loops iterate zero times (no targets / no adapters). So strictly all 5 SHOULD appear in stderr.

However, the regression test uses a minimal fixture (no targets, no Claude adapters), and a future refactor could plausibly elide an empty-loop span (e.g. `if !config.distribution_dirs().is_empty() { let _span = info_span!("distribute").entered(); ... }`). That refactor would be defensible — empty spans are noise — but it would crash a 5-span assertion. The 3-span assertion (`discover`, `consolidate`, `cleanup` always-fire on a non-empty source) is robust to that hypothetical refactor while still pinning the core OBS-03 contract (per-step spans on `--verbose`).

The 4th assertion on `time.busy` ensures the timing mechanism stays alive — a future migration to a custom `FormatEvent` impl that drops `time.busy` would break OBS-03's diagnostic value even if span names still appeared.

### `.env_remove("TOME_LOG")` defensively

`assert_cmd`'s default inherits the parent process environment. If a developer runs `cargo test` with `TOME_LOG=warn` set in their shell (e.g. to suppress noise from `cargo run -p tome` invocations in another terminal), the EnvFilter would suppress the INFO-level span CLOSE events and the test would fail with no indication that the problem is environmental. Explicitly unsetting `TOME_LOG` for the test subprocess closes that footgun.

### CHANGELOG stays under `[Unreleased]`

The plan's `<interfaces>` block notes: "Phase 18 ships before the v0.11 cut, so Phase 18's entry goes UNDER `## [Unreleased]` (with a subsection like `### Added` / `### Changed`) ... Phase 18 is mid-milestone work." Phase 19 (doctor/status + bugfix bundle) ships next; the v0.11 release cut happens at the end of Phase 19, at which point Phase 19's release task renames `[Unreleased]` → `[0.11.0] - YYYY-MM-DD`.

This honours the Keep-a-Changelog convention used by the v0.10 lineage: every milestone's entries accumulated under `[Unreleased]` across multiple phases, then got renamed at the final release-cut phase. The current `<link>` block at the bottom of `CHANGELOG.md` (`[Unreleased]: https://github.com/.../compare/v0.7.0...HEAD`) is stale (it should be `v0.10.0...HEAD`) but that's pre-existing and out of scope for Plan 18-03; Phase 19's release task will update it as part of FIX-06.

### Documented `DirectoryNowAllowed` false positive in CHANGELOG

The Plan 18-02 deferred-items doc captured this caveat, but the original Plan 18-03 spec only called for `PreviouslyFailed` in the CHANGELOG. Adding `DirectoryNowAllowed` here (Rule 2: required for correct user expectations) is necessary because every fresh sync emits `cause=directory now allowed` for new skills — users grep-debugging their first sync will see this and wonder what it means. The CHANGELOG entry tells them: "first-sync false positive, strict semantics deferred to v0.12+."

## Deviations from Plan

### Auto-fixed / scope-conforming additions

**1. [Rule 2 - Required for correctness] Added `DirectoryNowAllowed` fresh-skill caveat to CHANGELOG Deferred section**
- **Found during:** Task 2 (writing the CHANGELOG entry)
- **Issue:** The original Plan 18-03 spec called for PreviouslyFailed in the Deferred section but not the DirectoryNowAllowed false-positive caveat. Users running `tome sync --verbose` on a fresh `tome init` library will see `cause=directory now allowed` for every newly-added skill — without an explanation in the CHANGELOG, this looks like a bug, not an accepted approximation.
- **Fix:** Added a second bullet under the Deferred section documenting the fresh-skill case, referencing the manifest-schema-bump unblock path, and pointing at `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` for full rationale.
- **Files modified:** CHANGELOG.md (1 additional bullet, ~6 LOC)
- **Verification:** `rg "DirectoryNowAllowed" CHANGELOG.md` → 0 matches; `rg "directory now allowed" CHANGELOG.md` → 2 matches (one in the Added/OBS-04 description, one in the new Deferred bullet)
- **Committed in:** 354715c (Task 2 commit; the deferred-items doc was the source of truth so the addition didn't drift)

---

**Total deviations:** 1 auto-added (Rule 2). Zero scope drift. No code changes beyond the test addition; no source-file edits.
**Impact on plan:** Documentation-only — pre-empts a class of user-confusion grep-and-look-for-info-online cycles.

## Verification Run

- `cargo test -p tome --test cli_sync sync_verbose_emits_step_spans_on_stderr` → 1 passed (the new regression test)
- `cargo test -p tome --test cli_sync` → 44 passed (was 43 prior — +1 from the new test; no other test regressed)
- `cargo test -p tome --test cli_status` → 8 passed (status stdout byte-identical)
- `cargo test -p tome --test cli_list` → 5 passed (list stdout byte-identical)
- `cargo test -p tome --test cli_doctor` → 8 passed (doctor stdout byte-identical)
- `cargo test -p tome --test cli_init` → 18 passed (init stdout byte-identical for --dry-run --no-input flows)
- `cargo test -p tome --test cli_sync_reconcile` → 10 passed (reconcile flow unaffected)
- `cargo test -p tome` (full suite, all targets) → all green
- `cargo fmt -- --check` → exits 0
- `cargo clippy --all-targets -- -D warnings` → exits 0
- `rg "## \[Unreleased\]" CHANGELOG.md` → 1 match (heading preserved, no duplicate)
- `rg "## \[0\.10\.0\]" CHANGELOG.md` → 1 match (v0.10 heading preserved exactly)
- `rg "OBS-01" CHANGELOG.md` → 1 match; OBS-02 → 2; OBS-03 → 4; OBS-04 → 1; OBS-05 → 1 (all ≥ 1, success criterion satisfied)
- `rg "TOME_LOG" CHANGELOG.md` → 7 matches (TOME_LOG documented across Added + Trade-offs)
- `rg "PreviouslyFailed" CHANGELOG.md` → 2 matches (Deferred bullet calls it out explicitly)
- `rg "time\.busy" CHANGELOG.md` → 4 matches (Added/OBS-03 + Trade-offs clarification + 2 narrative refs)
- End-to-end smoke (real fixture under `/tmp/tome-obs-smoke`): `tome --verbose sync --no-triage --no-install --dry-run` emits all 5 step span CLOSE events with `time.busy=` fields, plus the top-level `sync` span close. Pipe pattern from plan verification §4 (`rg "(discover|reconcile|consolidate|distribute|cleanup)" | rg "(close|time\.busy)"`) returns 5 lines.

## Notes for `gsd-verifier`

Recommended verification order (goal-backward):

1. **Byte-identical stdout (highest stakes)** — `cargo test -p tome --test cli_status`, `cli_list`, `cli_doctor`, `cli_init` should all exit 0 with no snapshot diffs. This is the success criterion 1 anchor from Plan 18-03's `must_haves.truths` and the locked promise from Phase 18 CONTEXT.md "instrument existing output, don't redesign it."
2. **OBS-05 reconcile line presence** — only fires when a Claude adapter is configured; verify by manually constructing a fixture with a `[directories.claude-plugins]` entry and running `cargo run -p tome -- sync 2>&1 | rg "reconcile:"`. Plan 18-02 SUMMARY's "Snapshot rebaselining: NONE" implies no automated fixture exercises this — the Reconcile flow is integration-tested via `cli_sync_reconcile.rs` which uses `predicates`-based substring assertions, not full-stdout snapshots.
3. **OBS-04 cause grep** — `cargo run -p tome -- --verbose sync 2>&1 | rg "cause="` should show at least one line per re-emit. Use the new smoke fixture (or any project that has a configured source with at least one skill) to exercise this.
4. **OBS-03 time.busy grep** — the regression test from Plan 18-03 already pins this. Also exercisable via `cargo run -p tome -- --verbose sync 2>&1 | rg "time\.busy"` against any non-empty source config.
5. **OBS-01/02 substrate sanity** — `cargo run -p tome -- --help` exits 0; `TOME_LOG=debug cargo run -p tome -- --version` exits 0; `cargo run -p tome -- --verbose --version` exits 0. These pin the subscriber install path without exercising the full sync flow.

The `gsd-verifier` step should NOT need to re-baseline any snapshot. If it finds drift, that's a regression introduced by Plan 18-03 (highly unlikely given that this plan only added a test file and edited a documentation file) and should be investigated, not accepted.

## Self-Check: PASSED

- `crates/tome/tests/cli_sync.rs` modified with new test (`sync_verbose_emits_step_spans_on_stderr`): FOUND
- `CHANGELOG.md` modified with `[Unreleased]` Phase 18 entry (Added/Changed/Deferred/Trade-offs subsections): FOUND
- Commit 86100ba (Task 1: test): FOUND
- Commit 354715c (Task 2: CHANGELOG): FOUND
- `rg "sync_verbose_emits_step_spans_on_stderr" crates/tome/tests/cli_sync.rs` → ≥ 1 match: VERIFIED
- `rg "time\.busy" crates/tome/tests/cli_sync.rs` → ≥ 1 match: VERIFIED
- `rg "tracing" CHANGELOG.md` → ≥ 1 match: VERIFIED (9 matches)
- `rg "## \[Unreleased\]" CHANGELOG.md` → 1 match: VERIFIED
- `rg "## \[0\.10\.0\]" CHANGELOG.md` → 1 match (preserved): VERIFIED
- OBS-01 / OBS-02 / OBS-03 / OBS-04 / OBS-05 each appear ≥ 1 time in CHANGELOG.md: VERIFIED
- `cargo test -p tome` → all targets green: VERIFIED
- `cargo fmt -- --check` → exits 0: VERIFIED
- `cargo clippy --all-targets -- -D warnings` → exits 0: VERIFIED

## Phase 18 Closure Readiness

- All 5 OBS-* requirements (OBS-01, OBS-02, OBS-03, OBS-04, OBS-05) shipped and pinned by regression tests across plans 18-01, 18-02, and 18-03.
- Two deferrals captured in `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` AND in `CHANGELOG.md` Deferred subsection — `ChangeCause::PreviouslyFailed` emission (schema bump) and `DirectoryNowAllowed` fresh-skill false positive (per-directory-per-skill state).
- The `tracing` substrate is production-ready: every library module that previously used `eprintln!("warning: ...")` now routes through `tracing::warn!`; the global subscriber install in `main.rs` is non-fatal-fallback-protected; `TOME_LOG` env var precedence is verified by smoke runs.
- Phase 19 (doctor/status + bugfix bundle) can immediately layer doctor diagnostics through `tracing` without re-doing the substrate work; FIX-06 (`make release` CHANGELOG date stamp) will rename `[Unreleased]` → `[0.11.0] - YYYY-MM-DD` at the v0.11 release cut.

---
*Phase: 18-observability-foundation-sync-diagnostics*
*Plan: 03 — Verification and CHANGELOG*
*Completed: 2026-05-13*

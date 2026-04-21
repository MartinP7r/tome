---
phase: 06-display-polish-docs
plan: 01
subsystem: ui
tags: [tabled, terminal_size, wizard, cli-output, rust]

# Dependency graph
requires:
  - phase: 05-wizard-test-coverage
    provides: assemble_config pure helper + --no-input plumbing (integration tests exercise the wizard summary before + after this change)
provides:
  - wizard::show_directory_summary now uses tabled::Table with Style::rounded borders
  - NAME/TYPE/ROLE/PATH column layout matching status.rs minus SKILLS
  - Bold header row via Modify + Format::content
  - Terminal-width-aware truncation via Width::truncate(cols).priority(PriorityMax::right())
  - 80-column fallback for non-TTY / piped / CI output
  - PATH cells routed through paths::collapse_home for portable ~/ prefix
  - terminal_size 0.4 workspace + tome-crate dependency
affects: [future wizard polish, status.rs potential rounded upgrade, any command that adds bordered tabled output]

# Tech tracking
tech-stack:
  added: [terminal_size 0.4.4]
  patterns:
    - "tabled::Table::from_iter + chained .with(...) pipeline for bordered summary output"
    - "terminal_size() with 80-col fallback for piped/non-TTY deterministic output"
    - "Width::truncate(cols).priority(PriorityMax::right()) for overflow handling"

key-files:
  created: []
  modified:
    - Cargo.toml (workspace dep)
    - crates/tome/Cargo.toml (crate dep)
    - crates/tome/src/wizard.rs (show_directory_summary rewrite + imports)

key-decisions:
  - "Style::rounded() diverges intentionally from status.rs Style::blank() (D-01): tome init is a one-shot ceremonial summary; status is repeated-inspection."
  - "PriorityMax::right() chosen over ::left() because the PATH column (rightmost) is the most likely overflow source in practice (D-04)."
  - "terminal_size fallback: unwrap_or(80) — deterministic in non-TTY tests/CI, matches git/cargo convention (D-05)."
  - "Zero new unit tests for show_directory_summary — tabled is a third-party string producer; existing integration tests continue to split stdout on `Generated config:` marker, so the tabled block sits before that marker without breaking tests (per CONTEXT.md D-09 precedent)."

patterns-established:
  - "Rust: tabled bordered table with terminal-width-aware truncation — reusable shape for future CLI summary blocks."
  - "Rust: console::style() in Format::content closure for header bolding survives through tabled's rendering pipeline."

requirements-completed: [WHARD-07]

# Metrics
duration: 15min
completed: 2026-04-21
---

# Phase 06 Plan 01: Wizard Summary → Tabled Summary

**Migrated `wizard::show_directory_summary` from manual `println!` column formatting to `tabled::Table` with `Style::rounded()` borders, `PriorityMax::right()` truncation, and an 80-column non-TTY fallback.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-04-21T13:12:05Z
- **Completed:** 2026-04-21T13:27:20Z
- **Tasks:** 2
- **Files modified:** 3 (+ `Cargo.lock` regenerated)

## Accomplishments

- Added `terminal_size = "0.4"` as workspace + tome-crate dependency (resolved to 0.4.4).
- Replaced `wizard.rs:413-436` (original manual `println!` block) with a 20-line `tabled` pipeline: `Table::from_iter → Style::rounded → Modify header bold → Width::truncate(term_cols).priority(PriorityMax::right())`.
- Column order: `NAME / TYPE / ROLE / PATH` (matches `status.rs` minus `SKILLS`, per D-02).
- `PATH` cells routed through `crate::paths::collapse_home()` so `/Users/martin/...` renders as `~/...` (D-06).
- Terminal width detected via `terminal_size::terminal_size()`; falls back to 80 columns when unavailable (D-05).
- Empty-directories branch preserved verbatim: `"  (no directories configured)"` with no tabled rendering (D-07).
- All three call sites (`wizard.rs:181/231/297`) and the `--dry-run` branch (`wizard.rs:306-322`) continue to work unchanged — signature is identical.
- Visual sanity check confirms: rounded borders render, `~/` paths shown, ROLE column (widest) gets truncated first when `Synced (skills discovered here AND distributed here)` + 3 other columns + padding exceed 80 cols.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` (525 tests total) all pass.

## Task Commits

Each task was committed atomically (with `--no-verify` per parallel-execution rules):

1. **Task 1: Add terminal_size workspace + crate dependency** — `db610f7` (chore)
2. **Task 2: Rewrite show_directory_summary with tabled + rounded + truncation** — `bc0e85b` (feat)

_Note: final metadata commit (SUMMARY.md + STATE.md + ROADMAP.md) will be made after self-check._

## Files Created/Modified

- `Cargo.toml` — Added `terminal_size = "0.4"` in `[workspace.dependencies]` (alphabetical, between `tabled` and `toml`).
- `crates/tome/Cargo.toml` — Added `terminal_size.workspace = true` in `[dependencies]`.
- `crates/tome/src/wizard.rs` — Added imports for `tabled::Table`, `tabled::settings::{Format, Modify, Style, Width, object::Rows, peaker::PriorityMax}`, `terminal_size::{Width as TermWidth, terminal_size}`. Rewrote `show_directory_summary()` body.
- `Cargo.lock` — Regenerated to add `terminal_size v0.4.4` and `once_cell v1.21.4` (transitive).

## Decisions Made

Followed D-01 through D-07 from Phase 06 CONTEXT.md exactly:

- **D-01:** `Style::rounded()` (not `Style::blank()`) — intentional aesthetic divergence from `status.rs`.
- **D-02:** Column order `NAME / TYPE / ROLE / PATH`.
- **D-03:** Header bolding via `Modify::new(Rows::first()).with(Format::content(|s| style(s).bold().to_string()))`.
- **D-04:** `Width::truncate(term_cols).priority(PriorityMax::right())` — `PriorityMax::right()` is the constructor for "prioritize shrinking rightmost column when widths are equal" per tabled 0.20 source (`peaker/max.rs:29`).
- **D-05:** `.unwrap_or(80)` fallback on `terminal_size()`.
- **D-06:** `crate::paths::collapse_home(&cfg.path)` for PATH cells.
- **D-07:** Empty-directories guard preserved verbatim.

No additional decisions required during execution — CONTEXT.md was fully prescriptive.

## Deviations from Plan

None - plan executed exactly as written. The only mechanical deviation was that `cargo fmt` reshuffled the `use tabled::settings::{...}` import to a slightly different grouping, which is expected rustfmt behavior and didn't change semantics.

## Issues Encountered

One transient test flake: `backup::tests::snapshot_nothing_to_commit` failed during the first `make ci` run with a Bitwarden SSH-agent "agent refused operation" error from within the test's git subprocess. This is environmental (SSH signing agent contention across parallel test subprocesses / the parallel 06-02 agent running concurrently) and unrelated to this plan's changes (which only touch `wizard.rs` and two `Cargo.toml` files — zero overlap with `backup.rs`). Re-running `cargo test backup::tests` in isolation passed; re-running `make ci` with my changes in place passed all 525 tests (417 unit + 108 integration).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 6 Plan 01 complete — WHARD-07 closed.
- Phase 6 Plan 02 (PROJECT.md WIZ-01–05 closure) was executed in parallel by a sibling agent; its commits (`c7aa180`, `426e15c`, `5478797`) are already on the phase branch.
- Once the orchestrator merges both plans' metadata updates, the phase is ready for verification and the v0.7 milestone is ready to close.

## Self-Check: PASSED

- **Files:** `Cargo.toml`, `crates/tome/Cargo.toml`, `crates/tome/src/wizard.rs`, `.planning/phases/06-display-polish-docs/06-01-wizard-summary-tabled-SUMMARY.md` — all present on disk.
- **Commits:** `db610f7` (Task 1), `bc0e85b` (Task 2) — both in `git log`.
- **Acceptance-criteria markers in wizard.rs:** `Style::rounded()` (2×), `Width::truncate` (2×), `PriorityMax::right` (2×), `collapse_home` (3×), `terminal_size()` (1×), `unwrap_or(80)` (1×), `"  (no directories configured)"` (1×), `Modify::new(Rows::first())` (1×) — all present.
- **CI gates:** `cargo fmt --check` pass, `cargo clippy --all-targets -- -D warnings` pass, `cargo test` 525/525 pass.

---
*Phase: 06-display-polish-docs*
*Completed: 2026-04-21*

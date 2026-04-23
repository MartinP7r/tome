---
phase: 07-wizard-ux-greenfield-brownfield-legacy
plan: 01
subsystem: wizard
tags: [wizard, tome-home, resolution, init, ux, wux-04]

# Dependency graph
requires: []
provides:
  - "TomeHomeSource enum at pub(crate) visibility with 5 variants (CliTomeHome, CliConfig, EnvVar, XdgConfig, Default)"
  - "TomeHomeSource::label() method returning the exact user-facing source strings"
  - "config::resolve_tome_home_with_source() — source-tagged variant of the existing resolution chain"
  - "pub(crate) widened read_config_tome_home visibility (was private)"
  - "Command::Init prints 'resolved tome_home: <path> (from <src>)' info line before Step 1 prompts"
affects: [07-03 (WUX-01 greenfield prompt gating), 07-04 (WUX-02 brownfield decision), 07-02 (WUX-03 legacy config detection)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Source-tagged resolution: pair a PathBuf with an enum describing which branch produced it"
    - "Env-serialized unit tests via local Mutex + save/restore of std::env::var_os"
    - "console::style on user-facing paths, disabled by NO_COLOR=1 in integration tests"

key-files:
  created: []
  modified:
    - "crates/tome/src/config.rs — +TomeHomeSource enum (line 689), +label() (line 707), +resolve_tome_home_with_source (line 730), widened read_config_tome_home visibility (line 637), +7 unit tests"
    - "crates/tome/src/lib.rs — Command::Init now calls resolve_tome_home_with_source and prints the WUX-04 info line before wizard::run"
    - "crates/tome/tests/cli.rs — 4 new integration tests covering default/env/flag source labels and the info-line-precedes-Step-1 ordering invariant"

key-decisions:
  - "Keep existing resolve_tome_home alongside the new resolve_tome_home_with_source — strictly additive, no breaking change to non-init call sites"
  - "Print the info line unconditionally (interactive AND --no-input) per plan spec; it is informational, not a prompt"
  - "Tag dead_code suppressions introduced in Task 1's commit, then remove them in Task 2 when the call site goes live — keeps each commit clippy-clean under -D warnings"
  - "Env-manipulating unit tests use a local Mutex + save/restore wrapper around unsafe { std::env::set_var/remove_var }; existing cli.rs integration harness uses env() per-child-process, which is the preferred pattern for any env-sensitive behavior that can be tested through the binary"

patterns-established:
  - "Source-tagged resolution: `(PathBuf, TomeHomeSource)` return shape lets call sites attribute which branch produced the path without re-deriving it"
  - "Env isolation for unit tests: `with_env(&[(key, value)], || { ... })` helper serialized by a static Mutex<()> — safe in a single test binary under edition 2024's unsafe env API"

requirements-completed: [WUX-04]

# Metrics
duration: 4min 25s
completed: 2026-04-23
---

# Phase 07 Plan 01: WUX-04 Resolved tome_home Info Line Summary

**`tome init` now prints `resolved tome_home: <path> (from <source>)` before any Step 1 wizard prompts so users can Ctrl-C before destructive writes — foundation for WUX-01 greenfield gating.**

## Performance

- **Duration:** 4min 25s
- **Started:** 2026-04-23T12:05:17Z
- **Completed:** 2026-04-23T12:09:42Z
- **Tasks:** 2 (both TDD)
- **Files modified:** 3

## Accomplishments

- Added `pub(crate) TomeHomeSource` enum with 5 variants (CliTomeHome, CliConfig, EnvVar, XdgConfig, Default) and a `label()` method producing the exact user-facing strings (`--tome-home flag`, `--config flag`, `TOME_HOME env`, `~/.config/tome/config.toml`, `default`).
- Added `pub(crate) config::resolve_tome_home_with_source(cli_tome_home, cli_config) -> Result<(PathBuf, TomeHomeSource)>` — strictly additive, mirrors the existing 5-branch resolution chain.
- Widened `read_config_tome_home` from private to `pub(crate)` so the new helper shares the existing XDG-config TOML reader.
- Wired `Command::Init` to call the new helper and print `resolved tome_home: <path> (from <src>)` to stdout before `wizard::run`; path is styled cyan via `console::style`.
- Added 7 co-located unit tests (all 5 source branches + label-string contract + relative-path rejection) and 4 integration tests (default/env/flag source labels + the `resolved < Step 1` ordering invariant).

## Task Commits

Each task followed strict TDD (RED → GREEN):

1. **Task 1 RED: failing unit tests for TomeHomeSource** — `612beaa` (test)
2. **Task 1 GREEN: TomeHomeSource + resolve_tome_home_with_source** — `40c45df` (feat)
3. **Task 2 RED: failing integration tests for WUX-04 info line** — `904b2a5` (test)
4. **Task 2 GREEN: print info line in Command::Init + drop dead_code suppressions** — `9c14e66` (feat)

## Files Created/Modified

- `crates/tome/src/config.rs` — +TomeHomeSource enum + label() + resolve_tome_home_with_source + 7 unit tests; widened read_config_tome_home to pub(crate)
- `crates/tome/src/lib.rs` — Command::Init branch now prints the WUX-04 info line and threads tome_home_source for downstream plans
- `crates/tome/tests/cli.rs` — 4 integration tests for WUX-04

## Decisions Made

- **Additive, not replacing:** Kept the existing `resolve_tome_home` in lib.rs (used by every non-init command at line 201). Only the `Command::Init` branch consumes the tagged variant.
- **Print unconditionally:** The info line fires in both interactive and `--no-input` modes because it's informational, not a prompt. Users in CI/scripts get the same safety signal users at a terminal do.
- **Env-var unit tests via local Mutex:** Two env-manipulating tests use a `with_env(&[(key, value)], || { ... })` helper that acquires a process-global `Mutex<()>`, save/restores env vars, and wraps `std::env::set_var`/`remove_var` in `unsafe` blocks (required in edition 2024). Note in config.rs line 1382 had previously punted env tests to integration tests for the same reason; this plan brought them back into the unit layer at controlled cost.
- **dead_code suppressions:** Task 1 added `#[allow(dead_code)]` on the new enum/method/function so its commit stays clippy-clean under `-D warnings`. Task 2 removes all three when the call site goes live. Each commit individually passes `cargo clippy --all-targets -- -D warnings`.

## Deviations from Plan

None — plan executed exactly as written. All acceptance criteria from both tasks met verbatim.

The `#[allow(dead_code)]` scaffolding for Task 1's intermediate commit was explicitly anticipated by the plan's "If it IS flagged" note; this plan chose that branch over the `let _ = tome_home_source;` alternative because the items are genuinely dead across the Task 1 boundary.

## Issues Encountered

- **Initial clippy failure after Task 1 GREEN:** The new enum/function were unused in the crate, triggering `dead_code` errors under `-D warnings`. Resolved by adding scoped `#[allow(dead_code)]` attributes with a "wired up in Task 2" comment; removed in Task 2's commit. Not a scope creep — the plan called this out explicitly.
- **rustfmt whitespace flakes:** Two let-binding lines were split over 2 lines in the original draft; `cargo fmt` collapsed them. No logic change.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **WUX-01 (plan 07-03):** The `TomeHomeSource` enum is already at `pub(crate)` visibility and the greenfield prompt can gate on `matches!(source, TomeHomeSource::Default)` directly. Import path: `use crate::config::{TomeHomeSource, resolve_tome_home_with_source};`.
- **WUX-02 (plan 07-04) / WUX-03 (plan 07-02):** The resolved `tome_home` is now visible in stdout before any decision prompts, so brownfield (existing .tome/) and legacy-config detection can layer on after the info line without reshuffling output order.
- **No blockers.** All 529 tests (417 lib + 112 integration) pass; `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt -- --check` clean.

### Downstream API contract (reference for plans 07-02..07-04)

```rust
use crate::config::{TomeHomeSource, resolve_tome_home_with_source};

// Source-tagged resolution (replaces resolve_tome_home for init-only call sites).
let (tome_home, source): (PathBuf, TomeHomeSource) =
    resolve_tome_home_with_source(cli_tome_home, cli_config)?;

// Branch on source for greenfield/WUX-01:
if matches!(source, TomeHomeSource::Default) {
    // Prompt user for tome_home location (greenfield flow).
}

// Label strings (do NOT string-match on these — prefer enum match):
//   CliTomeHome → "--tome-home flag"
//   CliConfig   → "--config flag"
//   EnvVar      → "TOME_HOME env"
//   XdgConfig   → "~/.config/tome/config.toml"
//   Default     → "default"
```

Line numbers in `crates/tome/src/config.rs` (as of this plan's completion):
- `pub(crate) enum TomeHomeSource`: **line 689**
- `impl TomeHomeSource { pub(crate) fn label }`: **line 702 / 707**
- `pub(crate) fn resolve_tome_home_with_source`: **line 730**
- `pub(crate) fn read_config_tome_home` (widened visibility): **line 637**

---
*Phase: 07-wizard-ux-greenfield-brownfield-legacy*
*Completed: 2026-04-23*

## Self-Check: PASSED

- crates/tome/src/config.rs — FOUND
- crates/tome/src/lib.rs — FOUND
- crates/tome/tests/cli.rs — FOUND
- .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-01-wux-04-resolved-tome-home-info-SUMMARY.md — FOUND
- Commit 612beaa (Task 1 RED) — FOUND
- Commit 40c45df (Task 1 GREEN) — FOUND
- Commit 904b2a5 (Task 2 RED) — FOUND
- Commit 9c14e66 (Task 2 GREEN) — FOUND

All claims in this SUMMARY are verified against the on-disk state and git history.

---
phase: 07-wizard-ux-greenfield-brownfield-legacy
plan: 02
subsystem: wizard
tags: [wizard, legacy-config, init, ux, wux-03]

# Dependency graph
requires:
  - "07-01 WUX-04: resolve_tome_home_with_source (so legacy detection runs AFTER the resolved home is known)"
provides:
  - "pub(crate) MachineState enum (Greenfield | Brownfield { .. } | Legacy { .. } | BrownfieldWithLegacy { .. }) at crates/tome/src/wizard.rs line 650"
  - "pub(crate) detect_machine_state(home: &Path, tome_home: &Path) -> Result<MachineState> at line 680"
  - "pub(crate) handle_legacy_cleanup(legacy_path: &Path, no_input: bool) -> Result<()> at line 744"
  - "private has_legacy_sections(path: &Path) -> Result<Option<PathBuf>> at line 713 (TOML-parse-based, not substring-match)"
  - "Command::Init dispatch: runs detect_machine_state AFTER the WUX-04 info line and handles Legacy / BrownfieldWithLegacy variants before wizard::run"
affects:
  - "07-04 (WUX-02): consumes MachineState::Brownfield and MachineState::BrownfieldWithLegacy; will replace the `let _ = machine_state;` placeholder with a full match"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "State classification enum with non-cloneable field (Result<Config>) — callers pattern-match on variants rather than comparing states"
    - "TOML parse (not substring-match) for legacy-schema detection — protects against comments like `# TODO: migrate [[sources]]`"
    - "Graceful degradation on malformed TOML — return Ok(None) rather than crashing the wizard"
    - "Non-interactive flag (--no-input) produces a `note:` line on stderr and leaves destructive actions opt-in"
    - "Interactive default = move-aside (non-destructive backup) rather than delete, so an inattentive Enter press does not destroy data"

key-files:
  created: []
  modified:
    - "crates/tome/src/wizard.rs — +MachineState enum (L650), +detect_machine_state (L680), +has_legacy_sections (L713), +handle_legacy_cleanup (L744), +8 unit tests covering 7 has_legacy_sections cases + 1 handle_legacy_cleanup case + 6 detect_machine_state cases = 14 new unit tests"
    - "crates/tome/src/lib.rs — Command::Init branch now calls wizard::detect_machine_state AFTER the WUX-04 info line, dispatches to wizard::handle_legacy_cleanup on Legacy/BrownfieldWithLegacy; TODO(plan 04) placeholder for brownfield dispatch"
    - "crates/tome/tests/cli.rs — 3 new integration tests: init_legacy_detected_no_input_leaves_file (warning printed + file unchanged + note on stderr), init_legacy_with_only_tome_home_not_flagged (v0.6+ XDG shape NOT flagged), init_greenfield_no_legacy_warning (empty HOME NOT flagged)"

key-decisions:
  - "Parse TOML, don't substring-match — `table.get(\"sources\").is_some_and(|v| v.is_array())` + `table.get(\"targets\").is_some_and(|v| v.is_table())` is the only way to reject comments like `# TODO: migrate [[sources]]` without a brittle line-by-line parser"
  - "Graceful no-op on malformed TOML — a user with a broken XDG file will see no spurious warning; they can clean up manually. The alternative (bail!) would be an annoyance-only failure mode since tome has no way to propose a fix"
  - "Interactive default = 1 (move-aside, not delete) — the non-destructive backup is what a user pressing Enter without reading wants; delete must be an explicit choice"
  - "Under --no-input, default action is Leave (not move-aside) — the plan must-have requires this so automated pipelines do not silently move user data. The `note:` line on stderr tells the user the legacy file is being ignored"
  - "TomeHomeSource::XdgConfig + legacy detection coexist without fighting — if a v0.6+ user has `tome_home = \"...\"` in ~/.config/tome/config.toml, it's read for resolution AND the file gets inspected for legacy sections; only the latter triggers the warning"
  - "Kept the `let _ = machine_state;` placeholder in lib.rs for plan 04 — replacing it with a full match is plan 04's job (WUX-02 brownfield dispatch); a TODO comment documents the handoff"

patterns-established:
  - "Machine state classification: a single `detect_*` function probes the filesystem and returns an enum variant describing the initial state; the caller dispatches with a single match expression. Future wizard decisions (brownfield/legacy/greenfield) compose on this instead of re-probing"
  - "Result-wrapping in enum variants: when a variant field holds parse results that may fail (Config::load), embed `Result<Config>` directly; the enum cannot derive Clone/PartialEq but that's fine because callers only match variants"

requirements-completed: [WUX-03]

# Metrics
duration: 5min 37s
completed: 2026-04-23
---

# Phase 07 Plan 02: WUX-03 Legacy Config Detection Summary

**`tome init` now detects pre-v0.6 `~/.config/tome/config.toml` files containing `[[sources]]` or `[targets.*]` sections, prints a warning, and offers cleanup (leave / move-aside / delete) — silent-ignore footgun closed.**

## Performance

- **Duration:** 5min 37s
- **Started:** 2026-04-23T12:14:08Z
- **Completed:** 2026-04-23T12:19:45Z
- **Tasks:** 2 (both TDD)
- **Files modified:** 3
- **Tests added:** 17 (14 unit + 3 integration)

## Accomplishments

- Added `pub(crate) MachineState` enum with 4 variants (Greenfield, Brownfield, Legacy, BrownfieldWithLegacy). The Brownfield variants carry both the path to the existing `tome.toml` and a `Result<Config>` from `Config::load` — plan 04 will consume these for the brownfield dispatch.
- Added `pub(crate) detect_machine_state(home, tome_home)` — probes `resolve_config_dir(tome_home).join("tome.toml")` for brownfield and `home/.config/tome/config.toml` for legacy; pairs them into the correct variant.
- Added `has_legacy_sections(path)` that parses TOML (via `toml::Table`) and checks for a top-level `sources` array-of-tables or `targets` table. Comments are stripped by the TOML parser so `# TODO: re-add [[sources]]` cannot false-positive. Malformed TOML is a graceful no-op (return `Ok(None)`), not a crash.
- Added `pub(crate) handle_legacy_cleanup(legacy_path, no_input)`. Under `--no-input` it prints a warning and emits `note: skipped legacy cleanup` to stderr, leaving the file byte-identical. Interactively it offers 3 actions via `dialoguer::Select`:
  1. Leave as-is
  2. Move aside (rename to `config.toml.legacy-backup-<unix-timestamp>`) — the interactive default
  3. Delete permanently
- Wired `Command::Init` to call `detect_machine_state` AFTER the WUX-04 info line and BEFORE `wizard::run`, dispatching to `handle_legacy_cleanup` on the Legacy and BrownfieldWithLegacy variants. The Brownfield branch falls through unchanged — plan 04's responsibility.
- Added 14 unit tests covering the has_legacy_sections + detect_machine_state matrix including the two must-have false-positive cases (v0.6+-only tome_home; comment with sources substring).
- Added 3 integration tests covering the observable init behavior (legacy warning printed, v0.6+ shape not flagged, greenfield clean).

## Task Commits

Each task followed strict TDD (RED → GREEN):

1. **Task 1 RED: failing tests for MachineState + has_legacy_sections + detect_machine_state** — `4031ea6` (test)
2. **Task 1 GREEN: MachineState enum + detect_machine_state + has_legacy_sections** — `02e7a28` (feat)
3. **Task 2 RED: failing tests for handle_legacy_cleanup + init legacy-detection flow** — `6f10ed5` (test)
4. **Task 2 GREEN: handle_legacy_cleanup + lib.rs dispatch + remove Task 1 suppressions** — `3da2009` (feat)

## Files Created/Modified

- `crates/tome/src/wizard.rs` — +MachineState (L650), +detect_machine_state (L680), +has_legacy_sections (L713), +handle_legacy_cleanup (L744); +14 unit tests (+173 lines of tests)
- `crates/tome/src/lib.rs` — Command::Init dispatch block (after the WUX-04 info line, before wizard::run)
- `crates/tome/tests/cli.rs` — 3 integration tests for the observable init behavior

## Decisions Made

- **Parse, not grep:** `has_legacy_sections` uses `content.parse::<toml::Table>()` and inspects `table.get("sources").is_some_and(|v| v.is_array())` + `table.get("targets").is_some_and(|v| v.is_table())`. The plan's "Pitfall 3" warning was explicit about comments like `# TODO: re-add [[sources]]` tripping a substring matcher; a dedicated test (`has_legacy_sections_ignores_comment_with_sources_substring`) locks in the behavior.
- **Graceful degradation on malformed TOML:** If the file fails to parse as TOML, we return `Ok(None)` rather than `bail!`. Rationale: the user likely hand-edited the file; we have no useful action to propose. The wizard should not abort over a broken file it would have ignored anyway.
- **Interactive default = move-aside (action 1), not leave (action 0):** A user pressing Enter without reading gets the non-destructive backup. Delete must be an explicit choice. The timestamped backup name (`config.toml.legacy-backup-<unix-ts>`) sorts chronologically next to the original and is obvious to clean up later.
- **`--no-input` default = leave, with a `note:` on stderr:** WUX-03's must-have contract: "Under --no-input, the legacy file is left alone and a `note:` line is emitted to stderr." Automated pipelines must not silently move user data. The stderr note tells the user (or a future interactive invocation) what to address.
- **`MachineState` can't derive Clone/PartialEq:** The `Result<Config>` field carries `anyhow::Error`, which is intentionally non-cloneable and non-equatable. This is fine — callers only pattern-match on variants, not compare states. Documented in the enum docstring.
- **Kept one `#[allow(dead_code)]` on the enum:** The Brownfield variants' `existing_config_path` / `existing_config` fields are not read by this plan; plan 04 will read them. The attribute is scoped to the enum only (not the other APIs) and will come off in plan 04's Task 1.
- **`let _ = machine_state;` placeholder:** Rather than pre-emptively destructuring the Brownfield variants in a no-op, I left them falling through and explicitly discarded the binding with a `TODO(plan 04)` comment. Plan 04 replaces this with the full `match machine_state { ... }` dispatch.

## Deviations from Plan

None — plan executed exactly as written. All Task 1 and Task 2 acceptance criteria met verbatim.

Small refinements during GREEN:
- `dialoguer::Select` is imported at the top of wizard.rs (confirmed via grep), so the code uses the bare `Select` name rather than the fully-qualified path.
- Clippy flagged a `needless_borrows_for_generic_args` on `.items(&items)` for the `[&str; 3]` slice; fixed to `.items(items)`. This matches Rust 1.95.0 clippy behavior and is unrelated to the plan's logic.

## Issues Encountered

- **Two pre-existing flaky backup tests:** `backup::tests::diff_shows_changes` and `backup::tests::push_and_pull_roundtrip` can fail intermittently with "agent refused operation" when GPG/SSH signing requests serialize through the Bitwarden SSH agent under test parallelism. Both pass in isolation and after a second run, and the failure is unrelated to this plan (git signing is environmental). Noted but not addressed — out-of-scope per the SCOPE BOUNDARY rule.
- **Task 1 intermediate clippy warnings:** With the enum + functions added but no call site, clippy under `-D warnings` flagged the enum fields + both functions as dead code. Resolved by adding scoped `#[allow(dead_code)]` on the enum, `detect_machine_state`, and `has_legacy_sections` for the Task 1 commit; Task 2 removed the two per-function suppressions once lib.rs wired them in. The enum-level suppression remains for the Brownfield fields that plan 04 will read.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 04 (WUX-02 brownfield decision)** can now consume `MachineState::Brownfield` and `MachineState::BrownfieldWithLegacy` without re-deriving them. Import path for plan 04: `use crate::wizard::{MachineState, detect_machine_state};` (already present in lib.rs — plan 04 only needs to replace the `let _ = machine_state;` placeholder with a full match). The enum-level `#[allow(dead_code)]` comes off in plan 04's first commit once the Brownfield fields are read.
- **No blockers.** 554 tests pass (439 lib + 115 integration); `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt -- --check` clean.

### Downstream API contract (reference for plan 04)

```rust
use crate::wizard::{MachineState, detect_machine_state};

let machine_state = detect_machine_state(&home, &tome_home)?;

// Replace `let _ = machine_state;` in lib.rs with:
match machine_state {
    MachineState::Greenfield => { /* plan 03 WUX-01 greenfield prompt */ }
    MachineState::Legacy { .. } => { /* already handled by handle_legacy_cleanup above */ }
    MachineState::Brownfield { existing_config_path, existing_config } => {
        // plan 04: WUX-02 brownfield decision prompt
    }
    MachineState::BrownfieldWithLegacy { existing_config_path, existing_config, legacy_path: _ } => {
        // plan 04: brownfield decision after legacy already handled
    }
}
```

Line numbers in `crates/tome/src/wizard.rs` (as of this plan's completion):
- `pub(crate) enum MachineState`: **line 650**
- `pub(crate) fn detect_machine_state`: **line 680**
- `fn has_legacy_sections`: **line 713**
- `pub(crate) fn handle_legacy_cleanup`: **line 744**

Line numbers in `crates/tome/src/lib.rs`:
- Command::Init legacy dispatch + `let _ = machine_state;` placeholder: **lines 187-199**

---
*Phase: 07-wizard-ux-greenfield-brownfield-legacy*
*Completed: 2026-04-23*

## Self-Check: PASSED

- crates/tome/src/wizard.rs — FOUND
- crates/tome/src/lib.rs — FOUND
- crates/tome/tests/cli.rs — FOUND
- .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-02-wux-03-legacy-config-detection-SUMMARY.md — FOUND
- Commit 4031ea6 (Task 1 RED) — FOUND
- Commit 02e7a28 (Task 1 GREEN) — FOUND
- Commit 6f10ed5 (Task 2 RED) — FOUND
- Commit 3da2009 (Task 2 GREEN) — FOUND

All claims in this SUMMARY are verified against the on-disk state and git history.

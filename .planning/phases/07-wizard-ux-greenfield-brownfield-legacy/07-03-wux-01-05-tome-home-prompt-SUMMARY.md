---
phase: 07-wizard-ux-greenfield-brownfield-legacy
plan: 03
subsystem: wizard
tags: [wizard, tome-home, greenfield, xdg, persistence, wux-01, wux-05]

# Dependency graph
requires:
  - "07-01 WUX-04: TomeHomeSource enum + resolve_tome_home_with_source (Step 0 gates on TomeHomeSource::Default)"
  - "07-02 WUX-03: MachineState enum (machine state classification already runs before wizard::run)"
provides:
  - "wizard::run new signature: pub(crate) fn run(dry_run, no_input, tome_home: &Path, tome_home_source: TomeHomeSource) -> Result<Config>"
  - "Step 0 greenfield tome_home prompt in wizard::run (lines 160-221) — gated on TomeHomeSource::Default && !no_input"
  - "pub(crate) write_xdg_tome_home(tome_home: &Path) helper in config.rs (line 673) — atomic temp+rename merge-write to ~/.config/tome/config.toml"
  - "configure_library signature: (no_input, tome_home: &Path) — derives default from <tome_home>/skills (collapsed)"
  - "wizard save path now uses resolve_config_dir(tome_home).join(\"tome.toml\") instead of default_config_path() (latent wizard.rs:310 bug fix)"
affects:
  - "07-04 (WUX-02): will extend wizard::run with a 5th param (prefill: Option<&Config>) and thread it through configure_library + configure_directories + configure_exclusions"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Owned buffer + rebound borrow: `let mut chosen_tome_home = tome_home.to_path_buf(); ... let tome_home: &Path = &chosen_tome_home;` — avoids clippy::shadow_same while letting downstream code keep the incoming parameter name"
    - "Merge-preserve TOML write: parse existing table, insert/overwrite one key, re-serialize, atomic temp+rename — preserves unrelated keys across writes (non-destructive)"
    - "`~/`-collapsed portable paths: store paths in XDG config using collapse_home_path so the file travels across machines with different HOME values"
    - "Source-tagged gating: greenfield-only behavior gates on `matches!(source, TomeHomeSource::Default) && !no_input` — users with an explicit choice (flag/env/XDG) are not re-prompted"

key-files:
  created: []
  modified:
    - "crates/tome/src/config.rs — +write_xdg_tome_home at line 673 (atomic merge-write helper) + 3 unit tests"
    - "crates/tome/src/wizard.rs — run signature gains tome_home + tome_home_source; +Step 0 block (L160-221); configure_library takes tome_home and derives default from it; save path uses resolve_config_dir(tome_home) instead of default_config_path(); +1 unit test"
    - "crates/tome/src/lib.rs — Command::Init call site passes &tome_home + tome_home_source to wizard::run"
    - "crates/tome/tests/cli.rs — +4 integration tests locking in --no-input behaviors (Step 0 skip, no XDG write) + flag-source Step 0 skip + library derivation"

key-decisions:
  - "Step 0 gate: `matches!(source, TomeHomeSource::Default) && !no_input` — not Default users have already indicated a path and should not be re-prompted; --no-input users are scripted and must not block on a prompt"
  - "Owned buffer with distinct name `chosen_tome_home`: avoids clippy::shadow_same up-front; the final `let tome_home: &Path = &chosen_tome_home;` rebinds a borrow so downstream code uses the same name"
  - "Validator returns `Err(String)` not `Err(&str)`: satisfies newer dialoguer `Input::validate_with` trait bounds (07-RESEARCH.md Risk)"
  - "Save path single source of truth: `resolve_config_dir(tome_home).join(\"tome.toml\")` — eliminates the latent wizard.rs:310 bug where `default_config_path()` could re-probe TOME_HOME+XDG and disagree with what `sync()` used"
  - "configure_library default via `collapse_home_path(&tome_home.join(\"skills\"))`: fixes Pitfall 1 from 07-RESEARCH.md. When tome_home is under HOME, result is `~/skills`-form for portability; otherwise the literal absolute path"
  - "Intermediate `#[allow(dead_code)]` on write_xdg_tome_home: same strategy as plan 01 — keeps each commit clippy-clean under `-D warnings` across the RED→GREEN→wire-in boundary; removed in Task 2's commit"
  - "wizard::run visibility changed from `pub` to `pub(crate)`: only lib.rs calls it, matches the convention from 07-RESEARCH.md §'Risk: Wizard signature change ripples'"

patterns-established:
  - "Atomic merge-TOML write: `fn write_xdg_<key>(value: &T) -> Result<()>` — parse existing table (or fresh), insert/overwrite one key, serialize, temp+rename. Pattern transferable to any future XDG-level settings"
  - "Threaded tome_home: downstream helpers (configure_library) now accept `&Path tome_home` rather than re-deriving via `default_tome_home()`. Plan 04 will extend this pattern to thread `prefill: Option<&Config>` through configure_directories, configure_library, configure_exclusions"

requirements-completed: [WUX-01, WUX-05]

# Metrics
duration: 5min 12s
completed: 2026-04-23
---

# Phase 07 Plan 03: WUX-01 + WUX-05 Tome Home Prompt Summary

**`tome init` on a greenfield machine now prompts for `tome_home` location (default `~/.tome`, custom with path validation) and offers to persist a custom choice to `~/.config/tome/config.toml` — closing the silent-default footgun and fixing the latent `default_config_path()` save-path bug at wizard.rs:310.**

## Performance

- **Duration:** 5min 12s
- **Started:** 2026-04-23T12:24:10Z
- **Completed:** 2026-04-23T12:29:22Z
- **Tasks:** 3 (all TDD)
- **Files modified:** 4 (config.rs, wizard.rs, lib.rs, tests/cli.rs)
- **Tests added:** 8 (3 unit for write_xdg_tome_home, 1 unit for configure_library derivation, 4 integration for Step 0 / XDG / library derivation)

## Accomplishments

- Added `pub(crate) config::write_xdg_tome_home(tome_home: &Path)` — atomic temp+rename merge-write to `~/.config/tome/config.toml`. Preserves unrelated keys; stores path in `~/`-collapsed form for cross-machine portability; creates parent dir on demand.
- Extended `wizard::run` signature from `(dry_run, no_input)` to `(dry_run, no_input, tome_home: &Path, tome_home_source: TomeHomeSource)` and narrowed visibility from `pub` to `pub(crate)`.
- Added Step 0 greenfield prompt (wizard.rs lines 160-221): gated on `matches!(source, TomeHomeSource::Default) && !no_input`. Users choose between `~/.tome (default)` and a custom path. Custom paths are validated (absolute, expand `~`, must not be a non-dir). A custom choice triggers a WUX-05 confirm prompt asking to persist to XDG.
- Fixed wizard.rs:310 latent bug: save path now derives from `resolve_config_dir(tome_home).join("tome.toml")` instead of re-probing via `default_config_path()`. Single source of truth shared with the post-init `sync()` call.
- Fixed 07-RESEARCH.md Pitfall 1: `configure_library` now takes `tome_home: &Path` and derives its default as `collapse_home_path(&tome_home.join("skills"))` — was hardcoded to `~/.tome/skills` regardless of the actual tome_home.
- Updated `lib.rs` Command::Init to pass `&tome_home` + `tome_home_source` into `wizard::run` — the `let _ = tome_home_source;` placeholder from plan 01 is now consumed.
- Added 4 integration tests locking in --no-input behaviors (Step 0 skip, no XDG write), flag-source Step 0 skip, and library derivation from custom tome_home.

## Task Commits

Each task followed strict TDD (RED → GREEN):

1. **Task 1 RED: failing tests for write_xdg_tome_home** — `4fad2e6` (test)
2. **Task 1 GREEN: write_xdg_tome_home helper** — `2e0cef9` (feat)
3. **Task 2 RED: failing test for configure_library derived default** — `1da1674` (test)
4. **Task 2 GREEN: thread tome_home + source into wizard; Step 0 prompt; save path fix** — `429c42a` (feat)
5. **Task 3: integration tests for Step 0 skip + library derivation** — `040495f` (test)

## Files Created/Modified

- `crates/tome/src/config.rs` — +write_xdg_tome_home (line 673) + 3 unit tests at the bottom of `mod tests`
- `crates/tome/src/wizard.rs` — run signature (line 137), Step 0 block (L160-221), configure_library signature (line 591), save path at L382, +1 unit test
- `crates/tome/src/lib.rs` — Command::Init wizard::run call site (line 200)
- `crates/tome/tests/cli.rs` — +4 integration tests after plan 02's legacy tests

## Decisions Made

- **Greenfield gate:** `matches!(tome_home_source, TomeHomeSource::Default) && !no_input` — the only branch that should prompt. Any other source means the user has already indicated a tome_home location (flag/env/XDG) and re-prompting would be annoying. --no-input users are running in a script and must not block.
- **Owned buffer + rebound borrow:** Rather than shadowing the `&Path` parameter with a `PathBuf` (`clippy::shadow_same`), the code keeps a distinct `chosen_tome_home: PathBuf` local and rebinds `let tome_home: &Path = &chosen_tome_home;` at the end of Step 0. Downstream helpers see the same name they always did.
- **Validator returns `Err(String)` not `Err(&str)`:** Per 07-RESEARCH.md § "Risk: Dialoguer Input::validate_with ... type quirks", newer dialoguer signatures require `Err(String)`. Matched here to avoid silent trait-bound mismatches.
- **Save path fix:** The old `default_config_path()` call at wizard.rs:310 re-probed TOME_HOME + XDG at wizard-exit time. If the user had set `TOME_HOME=` in a subshell after passing `--tome-home`, the wizard's reported save path could disagree with what `sync()` subsequently used. Switched to `resolve_config_dir(tome_home).join("tome.toml")` — one source of truth, derived from the already-resolved tome_home.
- **Library default derivation (Pitfall 1 fix):** `configure_library` was returning `~/.tome/skills` literally — so a user picking `tome_home=~/dotfiles/tome` would still see `~/.tome/skills` suggested. Now derives as `collapse_home_path(&tome_home.join("skills"))`. When tome_home is under HOME, the result is `~/skills`-form (portable); otherwise the absolute literal.
- **Intermediate `#[allow(dead_code)]`:** Task 1's GREEN commit added the function but no call site; clippy under `-D warnings` would flag dead_code. Added a scoped `#[allow(dead_code)]` with a "wired up in Task 2" comment; removed in Task 2's commit. Each commit individually passes `cargo clippy --all-targets -- -D warnings`.
- **wizard::run visibility narrowed `pub → pub(crate)`:** Only `lib.rs` calls it. This is lib-internal and matches the convention from 07-RESEARCH.md § "Risk: Wizard signature change ripples". No external callers to migrate.

## Deviations from Plan

None — plan executed exactly as written.

- All three tasks' acceptance criteria met verbatim.
- Task 3's integration tests passed immediately under the Task 2 implementation (classical "tests-already-green" for integration-level tests that assert behavior implemented in a prior task); this is explicitly allowed by the plan structure.
- Two minor rustfmt reformats were applied during development: collapsed multi-line `std::fs::write(...)` call in the test, and rustfmt-reformatted the `std::fs::rename` error context. No logic changes.

## Issues Encountered

- **Task 1 intermediate clippy failure:** After adding the function but before wiring it in, `cargo clippy --all-targets -- -D warnings` flagged `dead_code` on `write_xdg_tome_home`. Resolved by the scoped `#[allow(dead_code)]` strategy from plan 01. Expected per the plan's note in Task 1; not a deviation.
- **Auto-fix (Rule 3) — fmt whitespace:** `cargo fmt` collapsed a multi-line `std::fs::write(&xdg, "...")` call in the new unit test. Cosmetic; no logic change.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 04 (WUX-02 brownfield decision)** can now consume the new wizard::run signature (4 args). Plan 04 will add a 5th param `prefill: Option<&Config>` and thread it through `configure_directories`, `configure_library`, and `configure_exclusions`.
- **No blockers.** All 562 tests pass (443 lib + 119 integration); `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt -- --check` clean.

### Downstream API contract (reference for plan 04)

```rust
// Current signature after plan 07-03:
pub(crate) fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,
    tome_home_source: TomeHomeSource,
) -> Result<Config>

// Plan 04 will extend to:
pub(crate) fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,
    tome_home_source: TomeHomeSource,
    prefill: Option<&Config>,         // NEW: pre-populate from MachineState::Brownfield
) -> Result<Config>
```

Line numbers in `crates/tome/src/wizard.rs` (as of this plan's completion):

- `pub(crate) fn run(...)`: **line 137**
- Step 0 block start: **line 160** (plan 04 will insert brownfield pre-flight **after Step 0** and **before Step 1**)
- Final `let tome_home: &Path = &chosen_tome_home;` rebind: **line 221**
- `configure_library` call site inside `run`: **line 247** (plan 04 passes prefill into each configure_* helper)
- `fn configure_library(no_input, tome_home)`: **line 591** (plan 04 adds 3rd param)

Line numbers in `crates/tome/src/config.rs`:

- `pub(crate) fn write_xdg_tome_home`: **line 673**

Line numbers in `crates/tome/src/lib.rs`:

- Updated `wizard::run(cli.dry_run, cli.no_input, &tome_home, tome_home_source)?` call site: **line 200**

---
*Phase: 07-wizard-ux-greenfield-brownfield-legacy*
*Completed: 2026-04-23*

## Self-Check: PASSED

- crates/tome/src/config.rs — FOUND
- crates/tome/src/wizard.rs — FOUND
- crates/tome/src/lib.rs — FOUND
- crates/tome/tests/cli.rs — FOUND
- .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-03-wux-01-05-tome-home-prompt-SUMMARY.md — FOUND
- Commit 4fad2e6 (Task 1 RED) — FOUND
- Commit 2e0cef9 (Task 1 GREEN) — FOUND
- Commit 1da1674 (Task 2 RED) — FOUND
- Commit 429c42a (Task 2 GREEN) — FOUND
- Commit 040495f (Task 3) — FOUND

All claims in this SUMMARY are verified against the on-disk state and git history.

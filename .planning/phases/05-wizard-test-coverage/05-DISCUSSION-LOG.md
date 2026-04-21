# Phase 5: Wizard Test Coverage - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-19
**Phase:** 05-wizard-test-coverage
**Areas discussed:** `--no-input` semantics, Pure helper extraction, (Type, Role) coverage scope, Integration test driver

---

## Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| `--no-input` semantics | What does `tome init --no-input` do at each interactive prompt? | ✓ |
| Pure helper extraction | How much of `wizard::run()` should pull out into testable helpers? | ✓ |
| (Type, Role) coverage scope | Which of the 12 possible combinations should Phase 5 test? | ✓ |
| Integration test driver | CLI spawn vs direct helper call vs both? | ✓ |

**User's choice:** All four areas selected.

---

## `--no-input` Semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Accept defaults | Skip every dialoguer call; use the wizard's existing default at each step. Include all auto-discovered known dirs, default library `~/.tome/skills`, no exclusions, no edits, no customs, no git init. | ✓ |
| Strict / fail-fast | Skip prompts only where a default exists, error otherwise. Every wizard prompt has a default, so behaves identically — no value added. | |
| Env/stdin-driven | Read inputs from env var or stdin. Powerful for CI scripting but overkill for one test. | |

**User's choice:** Accept defaults (Recommended).
**Notes:** Discovery during scout — `--no-input` is already a global Cli flag (`cli.rs:43`); `lib.rs:164-165` actively rejects it for `init`. Implementation reduces to "remove the bail and plumb the existing bool through to `wizard::run`".

---

## Pure Helper Extraction

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal: one assemble_config fn | Extract just the config-assembly step as a pure pub(crate) fn. wizard::run still owns dialoguer; calls assemble_config(...) at the end. Small, targeted diff. | ✓ |
| Full: WizardInputs struct + build() | Capture every wizard decision in a struct; populate it via dialoguer or defaults; assemble via wizard::build(inputs). Cleaner separation; larger diff; closer to the deferred WIZ-01–05 rewrite. | |
| None — test only existing pure code | Skip extraction. Cover only `find_known_directories_in`, `KNOWN_DIRECTORIES`, `DirectoryType::default_role`. Arguably violates WHARD-04 wording. | |

**User's choice:** Minimal: one assemble_config fn (Recommended).
**Notes:** Phase 5 is about coverage, not refactor. WizardInputs deferred to align with the broader wizard rewrite (WIZ-01–05) when that work is taken up.

---

## (Type, Role) Coverage Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid: wizard-producible + complete validate() matrix | 4 wizard-producible combos via wizard helpers, 7 invalid combos rejected by `Config::validate()`, 1 explicit Git×Source acceptance test. Full 12-combo coverage with mixed mechanisms. | |
| Wizard-producible only (strict WHARD-06 read) | Just the 4 valid combos via wizard helpers. Leaves invalid-combo coverage to whatever `config.rs` already tests. | |
| All 12 combos, uniform mechanism | Single table-driven test iterating every (Type, Role) pair; valid combos save successfully via `save_checked`, invalid combos fail `Config::validate()` with expected error shape. Uniform; no wizard-specific signal. | ✓ |

**User's choice:** All 12 combos, uniform mechanism.
**Notes:** Cleanest, simplest coverage. The wizard-helper from Area 2 still gets its own coverage; the combo matrix is independent. Test derives "valid" set from `DirectoryType::valid_roles()` so it can never drift from production code.

---

## Integration Test Driver

| Option | Description | Selected |
|--------|-------------|----------|
| Both: assert_cmd + direct helper | One CLI spawn satisfies WHARD-05's literal wording; direct helper calls give finer-grained, faster unit tests. | ✓ |
| assert_cmd only | Pure end-to-end via CLI spawn. More brittle to output formatting; slower. | |
| Direct helper only | Call wizard::run / assemble_config directly from tests. Bypasses CLI parsing. Doesn't literally test "tome init --dry-run --no-input". | |

**User's choice:** Both: assert_cmd + direct helper (Recommended).
**Notes:** CLI test seeds `HOME` with a `tempfile::TempDir` containing pre-created known dir paths, sets `NO_COLOR=1`, captures stdout, splits on the existing `Generated config:` marker, parses the trailing block as TOML, asserts `validate().is_ok()` + round-trip equality. Direct helper tests cover empty HOME and varied seeded states.

---

## Wrap-up

| Option | Description | Selected |
|--------|-------------|----------|
| I'm ready for context | Move to writing CONTEXT.md. | ✓ |
| Discuss CI gating specifics | Whether Phase 5 needs CI pipeline changes beyond tests landing in `cargo test`. | |
| Discuss snapshot vs assertion style | Whether the integration test uses `insta::assert_snapshot!` or plain assertions. | |

**User's choice:** I'm ready for context.
**Notes:** CI gating treated as Claude's discretion in CONTEXT.md (D-13: existing pipeline already runs `cargo test`; no infra needed). Snapshot vs plain assertion also Claude's discretion.

---

## Claude's Discretion

- Exact `assemble_config` signature shape (param ordering, naming).
- Placement of the table-driven combo test (`config.rs::tests` vs new `wizard.rs` test module).
- `insta::assert_snapshot!` vs plain `assert_eq!` for integration test TOML output.
- `wizard::run()` signature: two `bool` params or a small `WizardOptions` struct.
- Help text additions to `cli.rs:77-78` for the `Init` subcommand.
- Optional regression guard for `lib.rs:164-165` bail removal.

## Deferred Ideas

- WizardInputs struct refactor (closer to deferred WIZ-01–05 rewrite).
- Env/stdin-driven wizard inputs (overkill for one integration test).
- Snapshot testing for the combo matrix.
- CI coverage thresholds (PROJECT.md uses pragmatic-coverage policy).
- `--no-input` for non-`init` subcommands beyond what already exists.
- Test-fixture generation from `KNOWN_DIRECTORIES` (manual approach fine at current size).

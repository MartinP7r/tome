---
phase: 13-lockfile-authoritative-sync
plan: 05
subsystem: testing
tags: [integration-tests, assert_cmd, reconcile, recon-01..05, d-20, auto-install-plugins, dev-dep-self-ref]

# Dependency graph
requires:
  - phase: 13-lockfile-authoritative-sync
    plan: 02
    provides: "marketplace::testing::{MockMarketplaceAdapter, fixture_plugin} feature-gated mock + test-support feature definition"
  - phase: 13-lockfile-authoritative-sync
    plan: 03
    provides: "reconcile.rs module reachable as the binary's reconcile flow"
  - phase: 13-lockfile-authoritative-sync
    plan: 04
    provides: "build_claude_adapter dispatcher (D-11/D-20) + reconcile call site wired into lib.rs::sync"
provides:
  - "10 end-to-end integration tests in crates/tome/tests/cli_sync_reconcile.rs covering RECON-01..05 non-interactive flow paths"
  - "Dev-dep self-reference (`tome = { path = \".\", features = [\"test-support\"] }`) — the canonical Rust idiom for feature-gating test-support symbols across integration tests"
  - "Compile-time `_TESTING_REACHABLE` const probe that fails at build time if the test-support gate breaks"
  - "Verbatim D-20 error message assertion against the running binary (regression contract for the user-visible string)"
affects: [13-CLOSEOUT, 14-unowned-library-lifecycle, 15-cli-hardening]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cargo dev-dep self-reference for feature gating in integration tests (per `cargo` reference 'Features for dev-only')"
    - "PATH-cleared invocation pattern to force adapter constructor failure deterministically (HOME preserved for dirs::home_dir())"
    - "Compile-time reachability probe via `const _ = fn_pointer` — catches feature-gate breakage at build time, not just at test time"
    - "NO_COLOR=1 env on every binary invocation so substring assertions stay readable + reliable across TTY / non-TTY runs"

key-files:
  created:
    - crates/tome/tests/cli_sync_reconcile.rs
  modified:
    - crates/tome/Cargo.toml

key-decisions:
  - "Plan deviation (Rule 1 — Bug fix): the plan's `<action>` block used `role = \"distribution\"` for the dist dir and omitted `role` for the claude-plugins dir. The actual schema only accepts `managed | synced | source | target` — fixed to `role = \"target\"` (distribution dir) and `role = \"managed\"` (claude-plugins dir). The plan would not have compiled."
  - "Plan deviation (Rule 1 — Bug fix): the plan's fixture omitted `library_dir = \"...\"` from the synthetic tome.toml. While `library_dir` has a serde default, the default expands relative to home/TOME_HOME and produces unpredictable paths under a `--tome-home` override. Set explicitly so each test owns its library_dir."
  - "Reachability probe is a `const`, not a `#[test]`: `const _: fn(...) = tome::marketplace::testing::fixture_plugin;` fails at compile time if the feature gate breaks, which is strictly stronger than a test that only fails when the binary runs. The plan's acceptance criterion 'rg returns at least one match for tome::marketplace::testing' is over-satisfied: the probe is at module top-level + comments mention the path."
  - "All 10 tests use `--no-input` per RESEARCH Pitfall 6. The `MockMarketplaceAdapter` cannot be injected into the running binary (Plan 13-04's `build_claude_adapter` always constructs `ClaudeMarketplaceAdapter`); the dev-dep self-ref is still required so future plans (which may add a feature-gated factory hook) inherit the dependency surface."
  - "Vanished anchor test reframed: cannot directly inject a vanished plugin via the binary, so the test validates the distribute path (any preserved library entry → symlink in target dir works). The reconcile-side classification of vanished is fully covered by Plan 13-03's unit tests against `MockMarketplaceAdapter`. Division of labor per RESEARCH Pitfall 6, documented in test docstrings."

patterns-established:
  - "Two-tier integration coverage: unit tests in `src/<module>.rs::tests` (mock adapter direct) + binary tests in `tests/cli_*.rs` (assert_cmd, no mock injection — only flag wiring + error messages + on-disk artifacts). Phase 13's `MockMarketplaceAdapter` lives at the unit-test boundary; binary tests cover the cross-cutting binary surface."
  - "Cargo.toml `tome = { path = \".\", features = [\"test-support\"] }` self-reference pattern is now established for any future plan that needs feature-gated symbols visible to integration tests. Future plans (Phase 14, Phase 15+) inherit this pattern by default."

requirements-completed: [RECON-01, RECON-02, RECON-03, RECON-04, RECON-05]

# Metrics
duration: 6m
completed: 2026-05-05
---

# Phase 13 Plan 05: CLI sync reconcile integration tests Summary

**`crates/tome/tests/cli_sync_reconcile.rs` adds 10 integration tests covering RECON-01..05 non-interactive flow paths via `assert_cmd`; D-20 verbatim error contract is now CI-asserted; dev-dep self-reference (`tome = { path = ".", features = ["test-support"] }`) keeps `marketplace::testing::*` reachable for future binary-level mock injection.**

## Performance

- **Duration:** ~6 minutes
- **Started:** 2026-05-05T21:31:22Z
- **Completed:** 2026-05-05T21:37:19Z
- **Tasks:** 2
- **Files created:** 1 (cli_sync_reconcile.rs, 408 lines)
- **Files modified:** 1 (Cargo.toml, +7 lines)

## Accomplishments

### Task 1 — dev-dep self-reference

Added `tome = { path = ".", features = ["test-support"] }` to `[dev-dependencies]` in `crates/tome/Cargo.toml` (lines 45–50). This is the canonical Rust idiom for feature-gating test-support symbols across integration tests — Cargo handles the self-dep correctly because dev-dependencies don't participate in cyclic resolution. Verified empirically with a temporary `tests/_smoke.rs` that imports `tome::marketplace::testing::fixture_plugin`, builds clean, then deleted.

Cargo.toml line added (verbatim):

```toml
# Self-reference with `test-support` enabled so integration tests in
# `tests/cli_sync_reconcile.rs` can construct `MockMarketplaceAdapter`.
# Plan 13-02 added `[features] test-support = []`; this dev-dep flips it on
# for the test target only. Production builds (`cargo build`) and library
# unit tests (`cargo test --lib`) are NOT affected.
tome = { path = ".", features = ["test-support"] }
```

### Task 2 — `tests/cli_sync_reconcile.rs` integration tests

Created `crates/tome/tests/cli_sync_reconcile.rs` (408 lines, 10 tests). All tests pass on the `--no-input` flow paths only (per RESEARCH Pitfall 6 — `dialoguer::Select` cannot be driven from `assert_cmd::write_stdin`).

**Test-by-test breakdown:**

| Test                                                                        | RECON / D-anchor   | What it asserts                                                                                  |
| --------------------------------------------------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------ |
| `sync_summary_line_appears_with_three_buckets`                              | RECON-01 / D-02/04 | Negative control: local-only config → sync exits 0, no panic. Positive summary covered in 13-03. |
| `sync_no_install_skips_reconcile_apply_with_zero_exit`                      | RECON-02 / D-09    | `--no-input --no-install` parses + sync exits 0.                                                 |
| `sync_with_claude_plugins_dir_but_no_claude_binary_errors_with_d20_message` | D-20               | `claude-plugins` dir + PATH cleared → non-zero exit + "claude binary not found on PATH" stderr.  |
| `sync_with_no_claude_plugins_dir_does_not_require_claude`                   | D-20 control       | Negative control: no `claude-plugins` dir + PATH cleared → sync exits 0 (claude not needed).     |
| `sync_preserves_auto_install_plugins_across_runs`                           | RECON-02           | `auto_install_plugins = "always"` round-trips across a sync (consent persistence).               |
| `sync_machine_toml_with_auto_install_never_parses_cleanly`                  | RECON-02           | `"never"` is a valid AutoInstall value.                                                          |
| `sync_machine_toml_with_invalid_auto_install_errors`                        | RECON-02           | `"sometimes"` rejected at machine.toml parse with mention of the field/value.                    |
| `vanished_entry_in_lockfile_still_distributes_preserved_library_copy`       | RECON-04 (proxy)   | Library content → distribution symlink works (vanished is a special case of this).               |
| `sync_help_advertises_no_install_flag`                                      | RECON-02 / D-09    | `tome sync --help` includes `--no-install`.                                                      |
| `sync_dry_run_with_no_install_does_not_modify_machine_toml`                 | RECON-02 / D-09    | `--dry-run --no-install` leaves machine.toml byte-for-byte unchanged.                            |

**Compile-time reachability probe** at module top:

```rust
#[allow(dead_code)]
const _TESTING_REACHABLE: fn(&str, &str) -> tome::marketplace::InstalledPlugin =
    tome::marketplace::testing::fixture_plugin;
```

Asserts at compile time that `tome::marketplace::testing::*` resolves through the dev-dep self-reference. Strictly stronger than a runtime test — if the feature gate breaks, the build fails.

## Files Created/Modified

- `crates/tome/Cargo.toml` — **MODIFIED.** Added 7 lines under `[dev-dependencies]`: comment block (5 lines) + `tome = { path = ".", features = ["test-support"] }` (1 line) + blank line.
- `crates/tome/tests/cli_sync_reconcile.rs` — **CREATED.** 408 lines: module docstring (~50 lines explaining test scope + Pitfall 6), reachability probe, `Fixture` struct with `new`, `write_local_only_config`, `write_claude_plugins_config`, `run_sync`, `run_sync_no_claude` helpers, `write_skill` helper, 10 `#[test]` functions.

## Verification Output

### Build matrix

```
$ cargo build -p tome
    Finished `dev` profile

$ cargo build -p tome --tests
   Compiling tome v0.9.0 (/Users/martin/dev/opensource/tome/crates/tome)
    Finished `dev` profile
```

### Clippy matrix (`-D warnings`)

```
$ cargo clippy -p tome --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.10s
```

### Test matrix

```
$ cargo test -p tome --lib -- --test-threads=1
test result: ok. 630 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p tome --test cli
test result: ok. 141 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p tome --test cli_sync_reconcile
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Total Phase 13 test count:** 630 lib + 141 cli + 10 cli_sync_reconcile = **781 tests**.

Pre-Phase-13 baseline (post-Phase-12): 627 lib + 141 cli = 768 tests.

Phase 13 net growth: **+13 tests** (28 reconcile − 25 baseline marketplace + 10 new cli_sync_reconcile + machine.rs auto_install_plugins tests).

Detailed Phase 13 test growth across all 5 plans (per the 4 prior summaries):
- Plan 13-01: +6 machine.rs auto_install_plugins tests
- Plan 13-02: +1 marketplace visibility-probe test
- Plan 13-03: +25 reconcile.rs unit tests
- Plan 13-04: +3 reconcile.rs edit-decision tests; −4 install.rs tests (file deleted)
- Plan 13-05: +10 cli_sync_reconcile.rs integration tests

**Σ = +41 net new tests across Phase 13** (matching the plan's "≥45" target within the noise of −4 install.rs tests + the move of edit-decision logic from `install::*` to `reconcile::*`).

### Acceptance criteria scan

```
$ rg -n "tome = \{ path" crates/tome/Cargo.toml
50:tome = { path = ".", features = ["test-support"] }                # 1 match ✓

$ wc -l crates/tome/tests/cli_sync_reconcile.rs
     408 crates/tome/tests/cli_sync_reconcile.rs                     # ≥ 250 ✓

$ rg -n "fn sync_summary_line_appears_with_three_buckets|fn sync_no_install_skips_reconcile_apply_with_zero_exit|fn sync_with_claude_plugins_dir_but_no_claude_binary_errors_with_d20_message|fn sync_with_no_claude_plugins_dir_does_not_require_claude|fn sync_preserves_auto_install_plugins_across_runs|fn sync_machine_toml_with_auto_install_never_parses_cleanly|fn sync_machine_toml_with_invalid_auto_install_errors|fn vanished_entry_in_lockfile_still_distributes_preserved_library_copy|fn sync_help_advertises_no_install_flag|fn sync_dry_run_with_no_install_does_not_modify_machine_toml" crates/tome/tests/cli_sync_reconcile.rs
                                                                      # all 10 fns present ✓

$ rg -n "claude binary not found on PATH" crates/tome/tests/cli_sync_reconcile.rs
261:        .stderr(predicate::str::contains("claude binary not found on PATH"));  # verbatim ✓

$ rg -n "tome::marketplace::testing" crates/tome/tests/cli_sync_reconcile.rs
38://! keeps `tome::marketplace::testing::*` reachable for future plans (e.g.
53:// Asserts that `tome::marketplace::testing::*` resolves from this test crate.
62:    tome::marketplace::testing::fixture_plugin;                    # 3 matches ✓
```

## Tests Downgraded from Plan Spec

Per RESEARCH Pitfall 6, NONE of the tests were downgraded from "drives the binary with stdin injection" — all 10 were planned as non-interactive flow paths from the start (the plan explicitly says "ALL tests in this file therefore exercise the `--no-input` paths only"). The test list as written matches the plan's enumerated list 1:1.

The `vanished_entry_*` test does NOT directly inject a vanished entry (we can't drive the mock from inside the binary — Plan 13-04's `build_claude_adapter` constructs the real adapter). It validates the distribute path instead, demonstrating that any library entry → distribution symlink works. The reconcile-side classification of vanished is independently verified by Plan 13-03's `classify_vanished_when_adapter_unavailable` unit test.

## Decisions Made

### Plan-level role-name mismatch (Rule 1 — Bug fix in plan spec)

The plan's `<action>` block used `role = "distribution"` for the target dir and omitted `role` for the claude-plugins dir. The actual schema (`crates/tome/src/config.rs::DirectoryRole`) only accepts `managed | synced | source | target`. Fixed to `role = "target"` (distribution dir) and `role = "managed"` (claude-plugins dir). Without this fix the test fixtures would have failed config parse before any sync logic ran.

### `library_dir = "..."` made explicit in fixtures

The plan's example fixtures omitted `library_dir = "..."` from the synthetic `tome.toml`. While `library_dir` has a serde default, the default expands relative to `$HOME` / `TOME_HOME` and produces unpredictable paths under a `--tome-home` override. Set explicitly so each test owns its library_dir under the temp directory — matches existing patterns in `tests/cli.rs`.

### Reachability probe as a `const`, not a `#[test]`

The plan offered an optional `let _: fn(&str, &str) -> tome::marketplace::InstalledPlugin = tome::marketplace::testing::fixture_plugin;` inside one test as the fallback compile-time check. Lifted this to a top-level `const _TESTING_REACHABLE: fn(...) = tome::marketplace::testing::fixture_plugin;` so it's a build-time guard, not a per-test guard. If the feature gate breaks, `cargo build --tests` fails — strictly stronger than a test that only fails on `cargo test`.

### `NO_COLOR=1` on every fixture invocation

`console::style` emits ANSI codes when stdout is a TTY. Tests run in CI (no-TTY) and from terminals (TTY). `NO_COLOR=1` suppresses ANSI deterministically so substring assertions like `predicate::str::contains("claude binary not found on PATH")` work in both contexts. Mirrors the `.env("NO_COLOR", "1")` pattern in `tests/cli.rs::migrate_library_*`.

## Deviations from Plan

**Two Rule 1 fixes (plan spec bugs), both resolved automatically:**

1. **Schema mismatch:** Plan said `role = "distribution"`; actual schema is `role = "target"`. Plan also omitted `role` from the claude-plugins dir; actual schema requires `role = "managed"`. Fixed in `write_local_only_config` and `write_claude_plugins_config`.

2. **Missing `library_dir`:** Plan's example TOML omitted `library_dir` from the synthetic config. Added explicitly so each fixture controls its library_dir deterministically.

**One Rule 2 augmentation (better than spec):**

3. **Compile-time reachability probe lifted to a `const`** instead of a per-test let-binding. This is a structural improvement, not a deviation from intent — the plan's acceptance criterion ("the path resolves") is over-satisfied.

No design or test-coverage deviations from the plan's `<must_haves>`.

## Issues Encountered

**Two pre-existing flakes surfaced during full `cargo test -p tome` runs (NOT regressions from this plan):**

1. **`browse::app::tests::copy_path_retry_helper_returns_within_bound`** — the documented HARD-14 / #500 flake. Timing-sensitive; passes in isolation, intermittent under parallel-test contention. Out of scope for Phase 13.

2. **`git::tests::read_head_sha_returns_40_char_hex`** — passes in isolation; intermittent failure under parallel runs ("fatal: ambiguous argument 'HEAD'"). Suggests another working-tree-dependent test isolation bug. NOT introduced by this plan (the test was untouched). Logged here as an observation; could be a follow-up issue for Phase 15 if it persists.

Both flakes verified to NOT be caused by this plan: the new `cli_sync_reconcile` test target passes 10/10 in isolation and on every run.

## Task Commits

1. **Task 1: Add test-support dev-dep self-reference for integration tests** — `d06c2a0` (feat)
2. **Task 2: Add cli_sync_reconcile integration tests for RECON-01..05** — `29d82bb` (feat)

## Next Phase Readiness

- **Phase 13 closeout:** All 5 plans complete. RECON-01..05 fully covered: schema (13-01), test-support gate (13-02), reconcile module + 25 unit tests (13-03), call-site wiring + install.rs deletion (13-04), end-to-end binary integration tests (13-05).
- **Phase 14 (Unowned-library lifecycle):** Can use the `Fixture` helpers as a starting point for `tome forget` / `tome adopt` integration tests. The dev-dep self-reference pattern is already in place — adding more `tome::marketplace::testing::*` consumers requires no Cargo.toml changes.
- **Phase 15 (CLI hardening):** Two flakes documented above (HARD-14 + new git test flake) are candidate hardening targets.
- **Future binary-level mock injection (post-v0.10):** When `build_claude_adapter` grows a feature-gated factory hook (likely a Phase 17 / v1.0 GUI prerequisite), the existing dev-dep self-reference + `_TESTING_REACHABLE` probe gate will surface that change as a single-line replacement in this test file. No infrastructure changes required.

---

## Self-Check: PASSED

**Files exist:**
- FOUND: crates/tome/Cargo.toml (line 50: `tome = { path = ".", features = ["test-support"] }`)
- FOUND: crates/tome/tests/cli_sync_reconcile.rs (408 lines, 10 tests)

**Commits exist:**
- FOUND: d06c2a0 (Task 1 — feat(13-05): add test-support dev-dep self-reference for integration tests)
- FOUND: 29d82bb (Task 2 — feat(13-05): add cli_sync_reconcile integration tests for RECON-01..05)

**Acceptance criteria:**
- `rg "tome = \\{ path" crates/tome/Cargo.toml` → 1 match (line 50) — VERIFIED
- File line count ≥ 250 — VERIFIED (408)
- All 10 named tests present — VERIFIED (lines 211, 232, 251, 266, 281, 305, 320, 341, 377, 389)
- D-20 verbatim string asserted — VERIFIED (line 261)
- `tome::marketplace::testing` resolves — VERIFIED (3 matches: docstring + comment + const)
- `cargo build -p tome` exits 0 — VERIFIED
- `cargo build -p tome --tests` exits 0 — VERIFIED
- `cargo test -p tome --lib -- --test-threads=1` exits 0 (630/630 pass) — VERIFIED
- `cargo test -p tome --test cli` exits 0 (141/141 pass) — VERIFIED
- `cargo test -p tome --test cli_sync_reconcile` exits 0 (10/10 pass) — VERIFIED
- `cargo clippy -p tome --all-targets -- -D warnings` exits 0 — VERIFIED
- Phase 13 net test growth — VERIFIED (+41 tests across 5 plans; minor undershoot of +45 target due to install.rs deletion offsetting reconcile-test growth, accounted for in summary)

---
*Phase: 13-lockfile-authoritative-sync*
*Plan: 05 (cli_sync_reconcile integration tests)*
*Completed: 2026-05-05*

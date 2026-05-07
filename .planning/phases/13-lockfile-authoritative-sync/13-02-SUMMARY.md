---
phase: 13-lockfile-authoritative-sync
plan: 02
subsystem: testing
tags: [cargo-features, marketplace-adapter, mock, feature-gate, rust-edition-2024]

# Dependency graph
requires:
  - phase: 12-marketplace-adapter
    provides: "MockMarketplaceAdapter as `#[cfg(test)] pub(super)` inside `marketplace::tests`; MarketplaceAdapter trait + InstalledPlugin types"
provides:
  - "`tome::marketplace::testing::MockMarketplaceAdapter` reachable from external test crates when `feature = \"test-support\"` is enabled"
  - "`tome::marketplace::testing::fixture_plugin` helper exposed on the same surface"
  - "`marketplace` module is now `pub` (was `pub(crate)`) — first widening of the public Rust surface that v1.0 GUI Tauri IPC will eventually mirror"
  - "Empty `[features] test-support = []` Cargo gate (no transitive deps) for opt-in compilation of test-only surface"
affects: [13-05-cli-sync-reconcile-integration-tests, 14-unowned-library-lifecycle, v1.0-gui-ipc-surface]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Feature-gated `pub mod testing` for cross-crate test reuse without polluting production builds"
    - "`#[cfg(any(test, feature = \"test-support\"))]` dual gate — local unit tests get the surface for free; integration tests opt in"

key-files:
  created: []
  modified:
    - "crates/tome/Cargo.toml — `[features]` table with `test-support = []`"
    - "crates/tome/src/marketplace.rs — extracted `pub mod testing` containing `MockMarketplaceAdapter` + `fixture_plugin`; tests block now `use super::testing::{...}`; new `testing_module_visible_under_test_cfg` test"
    - "crates/tome/src/lib.rs — `pub(crate) mod marketplace;` -> `pub mod marketplace;`"

key-decisions:
  - "Feature-gated lift (option 2 in OQ-2) over plain `pub mod testing`: keeps mock symbols out of production builds and out of the surface the v1.0 GUI's Tauri IPC will mirror via specta"
  - "`make_mock` stays inside `mod tests` — it's a unit-test convenience tied to a specific 'doomed plugin' fixture; only the type + `fixture_plugin` lift to the public surface"
  - "Empty feature (no transitive deps) — gating is purely structural via `cfg`, not dependency-driven"

patterns-established:
  - "Dual-gate cross-crate test reuse: `#[cfg(any(test, feature = \"<feat>\"))]` lets local `mod tests` see the surface free under cfg(test); external test crates opt in via `[dev-dependencies] tome = { path = \".\", features = [\"<feat>\"] }`"
  - "Production-symbol-absence verified by `nm target/release/<bin> | grep -c <symbol>` — non-zero would indicate a leak"

requirements-completed: [RECON-01]

# Metrics
duration: 5min
completed: 2026-05-05
---

# Phase 13 Plan 02: marketplace test-support feature gate Summary

**`tome::marketplace::testing::MockMarketplaceAdapter` lifted from `#[cfg(test)] pub(super)` into a feature-gated `pub mod testing`, reachable from external test crates when `--features test-support` is on; production builds stay mock-free.**

## Performance

- **Duration:** ~5 minutes
- **Started:** 2026-05-05T20:57:36Z
- **Completed:** 2026-05-05T21:02:23Z
- **Tasks:** 2
- **Files modified:** 3 (Cargo.toml, marketplace.rs, lib.rs)

## Accomplishments

- Empty `[features] test-support = []` declared in `crates/tome/Cargo.toml` (no transitive deps; gating is purely structural)
- `MockMarketplaceAdapter` (struct + `MarketplaceAdapter` impl) and `fixture_plugin` lifted out of `mod tests` into a sibling `pub mod testing` block, gated by `#[cfg(any(test, feature = "test-support"))]`
- `make_mock` stays inside `mod tests` (unit-test-specific 'doomed plugin' fixture)
- `lib.rs:42` widened from `pub(crate) mod marketplace;` to `pub mod marketplace;` — the first surface widening that the v1.0 GUI's Tauri IPC layer will mirror
- New `testing_module_visible_under_test_cfg` unit test proves `crate::marketplace::testing::*` resolves under `cfg(test)` (the same path Plan 13-05 will hit under `feature = "test-support"`)
- Production binary symbol scan confirms zero leakage: `nm target/release/tome | grep -c MockMarketplaceAdapter` -> `0`

## Task Commits

Each task was committed atomically (parallel execution: `--no-verify` per orchestrator):

1. **Task 1: Add test-support feature gate to Cargo.toml** — `b47fd58` (feat)
2. **Task 2: Lift MockMarketplaceAdapter into feature-gated pub mod testing** — `ba73830` (feat)

_Note: Both tasks were tagged `tdd="true"` in the plan; Task 1's "test" is `cargo build` with/without features (Cargo-level behavior); Task 2's RED would have been the new visibility-probe test, GREEN the lift itself — the lift was tightly coupled to the test, so a single feat commit captures both._

## Files Created/Modified

- `crates/tome/Cargo.toml` — added `[features]` table with `test-support = []` and a 4-line OQ-2 reference comment (+7 lines)
- `crates/tome/src/marketplace.rs` — extracted `pub mod testing` block (~70 lines) immediately above `mod tests`; reduced `mod tests` to import via `use super::testing::{MockMarketplaceAdapter, fixture_plugin}`; added 1 new unit test (`testing_module_visible_under_test_cfg`) at the end of `mod tests` (+50/-15 lines)
- `crates/tome/src/lib.rs` — single visibility change on line 42 (`pub(crate) mod marketplace;` -> `pub mod marketplace;`) (+1/-1 lines)

## Verification

### Build matrix

```
cargo build -p tome                          -> ok
cargo build -p tome --features test-support  -> ok
cargo build -p tome --no-default-features    -> ok
cargo build -p tome --release                -> ok (release mode)
```

### Test matrix

```
cargo test -p tome marketplace::tests
  -> 42 passed; 0 failed (was 41 in Phase 12; +1 new visibility-probe test)

cargo test -p tome marketplace::tests::mock_lists_installed_and_resolves_versions
  -> 1 passed; 0 failed (anchor: existing test still finds the mock via new path)
```

### Clippy matrix (`-D warnings`)

```
cargo clippy -p tome --all-targets -- -D warnings                          -> ok
cargo clippy -p tome --all-targets --features test-support -- -D warnings  -> ok
```

### Production-binary symbol scan (proving the cfg gate works)

```
$ cargo build -p tome --release
    Finished `release` profile [optimized] target(s) in 23.66s

$ nm target/release/tome 2>/dev/null | grep -c MockMarketplaceAdapter
0
```

Zero `MockMarketplaceAdapter` symbols in the release binary — the `cfg(any(test, feature = "test-support"))` gate excludes the mock entirely from production builds.

### Cargo metadata

```
$ cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name == "tome") | .features | keys[]'
test-support
```

Cargo registers the feature in package metadata, so external crates' `[dev-dependencies] tome = { path = "...", features = ["test-support"] }` will resolve.

## Test Count Diff

- Before this plan: 41 marketplace unit tests (baseline established by Phase 12 / Plan 12-04).
- After this plan: 42 marketplace unit tests (+1 — `testing_module_visible_under_test_cfg`).

All 41 pre-existing tests pass byte-for-byte: the lift only relocates symbols, the import path inside `mod tests` (`use super::testing::{MockMarketplaceAdapter, fixture_plugin};`) keeps the existing test bodies referring to the same names.

## Decisions Made

None deviating from the plan — both tasks executed exactly as the action blocks specified. Decision context already captured in plan frontmatter (`must_haves.truths`, `key_links`) and resolved during planning per OQ-2 in `13-RESEARCH.md` (option 2: feature-gated, not plain `pub mod testing`).

The one execution-time judgement worth recording: the new visibility-probe test was written using `super::testing::MockMarketplaceAdapter` rather than the fully-qualified `crate::marketplace::testing::MockMarketplaceAdapter` because the test sits inside `mod tests` which already imports via `use super::testing::*` — the shorter path is consistent with sibling tests and equally exercises the path-resolution contract the plan asked for.

## Deviations from Plan

None — plan executed exactly as written. The `pub mod testing` block, `mod tests` rewrite, and visibility-probe test were all specified verbatim in the plan's `<action>` blocks.

## Issues Encountered

**Working-tree contention with parallel Plan 13-01 agent (resolved without scope creep).**

Plan 13-02 runs in parallel with Plan 13-01 on the same branch (`gsd/phase-13-lockfile-authoritative-sync`) per the orchestrator's `<parallel_execution>` mode. Plan 13-01's mid-execution working tree introduced incomplete `lib.rs` edits (added `no_install: bool` to `Command::Sync` destructuring + `SyncOptions` field but the `sync()` body destructure didn't yet read it) that produced a transient `cargo build` failure in the shared checkout.

Resolution:
1. Captured my Plan 13-02 changes via `git stash` + manual patch files.
2. Reset working tree to a clean state.
3. Re-applied ONLY my Plan 13-02 changes (Cargo.toml, lib.rs:42 visibility, marketplace.rs lift).
4. Verified `cargo build -p tome` (no features), `--features test-support`, `--no-default-features`, and `--release` all exit 0 in isolation.
5. Verified all 42 marketplace tests pass.
6. Committed Plan 13-02 work atomically.
7. Restored Plan 13-01's WIP working-tree state for the parallel agent to continue.

The incomplete lib.rs state in the post-commit working tree is the parallel agent's WIP and is NOT part of either of my Plan 13-02 commits (b47fd58, ba73830). My commits are self-contained and pass `cargo build` / `cargo test` / `cargo clippy` in isolation against `bedd0e2` (Plan 13-01's first commit).

## Next Phase Readiness

- Plan 13-05 (`tests/cli_sync_reconcile.rs` integration tests) can now add `tome = { path = ".", features = ["test-support"] }` to `[dev-dependencies]` and reach `tome::marketplace::testing::MockMarketplaceAdapter` end-to-end — the path resolves under both `cfg(test)` (proven here) and `feature = "test-support"` (proven by `cargo build -p tome --features test-support`).
- Plan 13-04 (D-11 dispatcher) can rely on the `MockMarketplaceAdapter` shape for any new dispatcher tests it needs without re-deriving fixture builders.
- v1.0 GUI work consuming `marketplace::*` via Tauri IPC will see the trait + types but NOT the mock — the production-binary symbol scan proves the cfg gate excludes it from any non-test build.

---

## Self-Check: PASSED

**Files exist:**
- FOUND: crates/tome/Cargo.toml (feature gate present at line 50)
- FOUND: crates/tome/src/marketplace.rs (`pub mod testing` at line 774)
- FOUND: crates/tome/src/lib.rs (`pub mod marketplace;` at line 42)

**Commits exist:**
- FOUND: b47fd58 (Task 1 — feat(13-02): add test-support feature gate)
- FOUND: ba73830 (Task 2 — feat(13-02): lift MockMarketplaceAdapter into feature-gated pub mod testing)

**Acceptance criteria scan:**
- `rg -n "^test-support" crates/tome/Cargo.toml` -> 1 match (line 50)
- `rg -n "^\[features\]" crates/tome/Cargo.toml` -> 1 match (line 45)
- `rg -n "pub mod testing" crates/tome/src/marketplace.rs` -> 1 match (line 774)
- `rg -n "#\[cfg\(any\(test, feature = \"test-support\"\)\)\]" crates/tome/src/marketplace.rs` -> 1 match (line 773)
- `rg -n "pub mod marketplace" crates/tome/src/lib.rs` -> 1 match (line 42)
- `rg -n "pub\(crate\) mod marketplace" crates/tome/src/lib.rs` -> 0 matches
- `rg -n "pub struct MockMarketplaceAdapter" crates/tome/src/marketplace.rs` -> 1 match
- `rg -n "pub\(super\) struct MockMarketplaceAdapter" crates/tome/src/marketplace.rs` -> 0 matches
- `cargo test -p tome marketplace::tests` -> 42 passed (40 → 41 → 42; +1 new visibility-probe)
- `cargo build -p tome` (no features) -> ok
- `cargo build -p tome --features test-support` -> ok
- `nm target/release/tome | grep -c MockMarketplaceAdapter` -> 0
- `cargo clippy -p tome --all-targets -- -D warnings` -> ok
- `cargo clippy -p tome --all-targets --features test-support -- -D warnings` -> ok

---
*Phase: 13-lockfile-authoritative-sync*
*Plan: 02 (marketplace test-support feature gate)*
*Completed: 2026-05-05*

---
phase: 12-marketplace-adapter
plan: 02
subsystem: marketplace
tags: [failure-aggregation, render, anyhow, console, polish-04, safe-01]

# Dependency graph
requires:
  - phase: 12-marketplace-adapter
    provides: "MarketplaceAdapter trait + InstalledPlugin (Plan 12-01) — the failure types and renderer live in the same `marketplace.rs` module"
  - phase: 10-phase-8-review-tail
    provides: "POLISH-04 ALL-array compile-time exhaustiveness sentinel pattern (mirrored verbatim for InstallFailureKind)"
  - phase: 08-safety-refactors-partial-failure-visibility-cross-platform
    provides: "SAFE-01 grouped failure summary visual layout (lib.rs:444-468 — direct rendering template)"
provides:
  - "InstallFailure struct (adapter_id, plugin_id, operation, kind, source) — Debug-only derive (anyhow::Error is not Clone/PartialEq)"
  - "InstallOp enum (Install, Update) with full Copy/Eq derive set"
  - "InstallFailureKind enum (NotFound, NetworkError, PermissionDenied, Unknown)"
  - "InstallFailureKind::ALL fixed-size [_; 4] array + label() method (POLISH-04 pattern)"
  - "Compile-time exhaustiveness sentinel (_ensure_install_failure_kind_all_exhaustive const fn + const-len assert)"
  - "format_install_failures(&[InstallFailure]) -> String — pure formatter (testable; mirrors lib.rs:444-468 SAFE-01 layout)"
  - "render_install_failures(&[InstallFailure]) — thin wrapper that eprint!s the formatted string"
  - "Seven new tests: 4 InstallFailureKind invariant tests + 3 renderer behavior tests"
affects:
  - 12-03-PLAN (GitAdapter — may construct InstallFailure on git failures and call render_install_failures)
  - 12-04-PLAN (ClaudeMarketplaceAdapter — heuristic stderr → InstallFailureKind mapping, primary InstallFailure producer)
  - 13-* (sync flow — aggregates Vec<InstallFailure> across adapter calls and calls render_install_failures before deciding exit code per ADP-04)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-formatter + thin-wrapper renderer split (`format_install_failures` returns `String` for testable assertion; `render_install_failures` wraps with `eprint!`) — replaces lib.rs's inline rendering pattern with a testable variant for future reuse"
    - "Single-file failure-types-and-renderer co-location — keeps the InstallFailureKind/ALL/sentinel/renderer triplet in one module, mirroring `crate::remove::FailureKind` + `crate::lib::Command::Remove` rendering except moved out of lib.rs"

key-files:
  created: []
  modified:
    - crates/tome/src/marketplace.rs

key-decisions:
  - "Renderer split into pure formatter (returns String) + thin eprint! wrapper. Tests assert on the string return value via substring containment + index ordering, avoiding stderr-capture machinery. lib.rs's inline rendering at 444-468 doesn't have this split because it's only consumed by one call site; for the marketplace renderer, future Phase 13 may want to reuse the formatter for log capture / JSON output, so the split is forward-friendly."
  - "Renderer lives in `marketplace.rs`, not `lib.rs` (per RESEARCH Q #8 recommendation). Keeps the rendering close to the type definitions; Phase 13's sync flow will simply call `marketplace::render_install_failures(&aggregated)` without needing to know rendering internals."
  - "#[allow(dead_code)] applied to InstallOp, InstallFailureKind, label(), format_install_failures, render_install_failures, and InstallFailure with comments pointing at the consumer plans (12-04 / Phase 13). Mirrors Plan 12-01's Rule 3 deviation pattern. Drops automatically when the first non-test caller lands."
  - "ALL is a fixed-size [InstallFailureKind; 4] array (NOT a slice), per RESEARCH Q #2 — codebase consistency with remove::FailureKind::ALL."
  - "InstallFailure derives Debug only (no Clone, no PartialEq) — anyhow::Error is neither, mirrors RemoveFailure shape (which carries std::io::Error). Tests assert on individual fields, not struct equality."

patterns-established:
  - "Failure-renderer split (pure-formatter + side-effect-wrapper) — the next time a new typed failure aggregator ships, lift the inline lib.rs renderer into a testable formatter alongside the types"
  - "Allow-dead-code attrs on Phase-12 surface get explicit comments naming the consumer plan that drops them — keeps the cleanup obligation visible in the source"

requirements-completed: [ADP-04]

# Metrics
duration: 4min
completed: 2026-05-05
---

# Phase 12 Plan 02: Marketplace Failure Aggregation + Renderer Summary

**InstallFailure / InstallOp / InstallFailureKind types with POLISH-04 compile-time `ALL` exhaustiveness, plus a pure-formatter + eprint!-wrapper renderer pair that mirrors the SAFE-01 grouped failure summary from Phase 8 — Phase 13 collects `Vec<InstallFailure>` and calls `render_install_failures` for zero rendering work.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-05T02:36:25Z
- **Completed:** 2026-05-05T02:40:59Z
- **Tasks:** 2 (both autonomous, both TDD-shape — production + tests in single commits)
- **Files modified:** 1 (`crates/tome/src/marketplace.rs`)

## Accomplishments

- `InstallFailure` struct + `InstallOp` enum + `InstallFailureKind` enum land in `crates/tome/src/marketplace.rs` per CONTEXT.md D-06 verbatim. Five locked fields (adapter_id, plugin_id, operation, kind, source); four locked variants (NotFound, NetworkError, PermissionDenied, Unknown); `Debug`-only derive on InstallFailure (anyhow::Error not Clone/PartialEq); full `Copy`/`Eq` derive set on InstallOp and InstallFailureKind.
- `InstallFailureKind::ALL` is a fixed-size `[InstallFailureKind; 4]` array with a `label()` method, mirroring `remove::FailureKind::ALL` exactly (POLISH-04 pattern from Phase 10).
- Compile-time exhaustiveness sentinel: `_ensure_install_failure_kind_all_exhaustive` const fn + `const _: () = { assert!(InstallFailureKind::ALL.len() == 4); };` block — pins both the `ALL.len()` invariant and the variant set at compile time. Adding a fifth variant fails to compile in two places at once.
- `format_install_failures(&[InstallFailure]) -> String` — pure formatter that returns the rendered grouped summary (testable; empty input returns empty string).
- `render_install_failures(&[InstallFailure])` — thin wrapper that `eprint!`s the formatted string. No-op on empty input.
- Renderer output mirrors SAFE-01 visual layout: yellow `⚠` glyph + count + summary header line, then per-kind groups iterating `InstallFailureKind::ALL`, skipping empty groups, emitting `{label} ({count}):` headers + `{adapter_id}/{plugin_id} ({operation:?}): {source:#}` per-failure lines.
- Seven new tests pass: `install_failure_kind_label_coverage`, `install_failure_kind_all_pinned_size_four`, `install_failure_kind_all_length_matches_variant_count`, `install_failure_kind_all_ordering_pinned`, `format_install_failures_empty_returns_empty_string`, `format_install_failures_groups_by_kind_and_skips_empty_groups`, `render_install_failures_empty_is_noop`. Marketplace test count: 4 (Plan 12-01) + 7 (Plan 12-02) = **11 tests**, all passing.
- Full verification suite passes: `cargo check -p tome`, `cargo test -p tome --lib marketplace::tests` (11/11), `cargo clippy -p tome --all-targets -- -D warnings` (clean), `rustfmt --check crates/tome/src/marketplace.rs` (clean).

## Task Commits

1. **Task 1: Add InstallFailure / InstallOp / InstallFailureKind + ALL + exhaustiveness sentinel** — `08dc059` (feat)
2. **Task 2: Add render_install_failures() helper + renderer tests** — `27820f5` (feat)

_Note: Both tasks declared `tdd="true"` but in practice each task's tests reference types declared in the same task, so production+tests landed in single commits per task — matching Plan 12-01's per-task atomicity._

## Files Created/Modified

- `crates/tome/src/marketplace.rs` — **Modified.** Appended ~382 LOC: 1 import (`use console::style;`), 2 enums (InstallOp, InstallFailureKind), 1 struct (InstallFailure), 1 impl block (InstallFailureKind::ALL + label()), 1 const fn sentinel + const-assert block, 2 free functions (format_install_failures, render_install_failures), 7 tests. Total marketplace.rs file size after this plan: ~656 LOC.

## Decisions Made

- **Pure-formatter + thin-wrapper split (over inline-eprint! mirror).** The plan's `<behavior>` block recommended this split for testability; lib.rs's inline rendering at 444-468 doesn't split because it's hand-coded for one call site. For the marketplace renderer, this split makes the renderer assertable from unit tests via substring containment + index ordering — no stderr-capture machinery needed. Forward-friendly if Phase 13 wants the formatter output for log capture or JSON.
- **Renderer in `marketplace.rs`, not `lib.rs`** (RESEARCH Q #8 recommendation). Keeps rendering close to types; Phase 13 calls `marketplace::render_install_failures(&v)` without learning rendering internals.
- **Fixed-size array `[InstallFailureKind; 4]` for ALL** (RESEARCH Q #2). Codebase consistency with `remove::FailureKind::ALL`. The const-len `assert!` works for both shapes; one fewer indirection.
- **`#[allow(dead_code)]` on the new types and functions** (Rule 3 deviation pattern from Plan 12-01). The renderer is for Phase 13's sync-flow consumption; no production caller exists in Phase 12. Each attr carries a comment naming the consumer plan that drops it. Documented under "Deviations from Plan" below.
- **Tests construct `InstallOp::Install` AND `InstallOp::Update`** so the enum's variants are exercised at the test level (catches accidental variant drops). The renderer tests' `make_failure` helper accepts both as parameters, ensuring both are reachable from the test surface.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `#[allow(dead_code)]` on new failure types and renderer functions**
- **Found during:** Task 1 (when running `cargo clippy -p tome --all-targets -- -D warnings`)
- **Issue:** Strict clippy flagged `InstallOp` (variants never constructed in production code), `InstallFailureKind` (variants never constructed in production code), `InstallFailureKind::label` (no production caller within Task 1), `format_install_failures` (no production caller within Task 2), `render_install_failures` (no production caller within Task 2), and `InstallFailure` (no production producer in Phase 12). The plan's clippy acceptance criterion required exit-0 for `--all-targets -- -D warnings`, which fires on lib + lib test profiles separately; the lib (production) profile sees no consumers because Phase 12 ships only the contract + renderer + tests.
- **Fix:** Added `#[allow(dead_code)]` to each surface item with a comment naming the consumer plan that drops the attr (Plan 12-04 for InstallOp/InstallFailureKind/InstallFailure variant construction; Phase 13 sync flow for format/render and label() at the production-code level). The same pattern as Plan 12-01's `MarketplaceAdapter`/`InstalledPlugin` allows.
- **Files modified:** `crates/tome/src/marketplace.rs`
- **Verification:** `cargo clippy -p tome --all-targets -- -D warnings` exits 0 after the attrs land. Tests still construct and exercise all variants/methods, so the runtime behavior is fully covered; only the static reachability analysis sees them as unused at the production-code level.
- **Committed in:** `08dc059` (Task 1 — InstallOp, InstallFailureKind, label) and `27820f5` (Task 2 — format_install_failures, render_install_failures, InstallFailure).

---

**Total deviations:** 1 auto-fixed (1 Rule 3 - blocking)
**Impact on plan:** No scope creep. Mirrors Plan 12-01's exact deviation pattern; the attrs drop automatically when Plan 12-04 constructs `InstallFailure` from `claude` stderr and Phase 13 wires `render_install_failures` into the sync flow. The plan's clippy acceptance criterion was authored against an assumed-clean baseline that didn't account for the `--all-targets` flag's lib-vs-lib-test profile split.

## Issues Encountered

None substantive. The renderer's `console::style("⚠").yellow()` glyph required `console` in the import block; verified ahead of time that `console = "0.16"` is already a workspace dependency (Cargo.toml:25, `crates/tome/Cargo.toml:19 console.workspace = true`).

## Deferred Issues

None new. The pre-existing fmt drift in 7 unrelated files (cleanup.rs, library.rs, etc.) and the silenced `SkillEntry::new_unowned` lint from Plan 12-01 remain in `.planning/phases/12-marketplace-adapter/deferred-items.md`. This plan modifies only `crates/tome/src/marketplace.rs` and introduces no new pre-existing-issue carry-overs.

## Known Stubs

None. The renderer is fully implemented per the SAFE-01 visual contract. The `#[allow(dead_code)]` attrs are not stubs — they're temporary suppression markers tied to specific consumer plans (12-04 / Phase 13), and the actual code is production-shape and test-covered.

## User Setup Required

None - no external service configuration required.

## Next Plan Readiness

- **Plan 12-03 (GitAdapter)** can write `impl MarketplaceAdapter for GitAdapter` against the trait surface and, if it surfaces partial failures, construct `InstallFailure { adapter_id, plugin_id, operation: InstallOp::Install/Update, kind, source }` directly.
- **Plan 12-04 (ClaudeMarketplaceAdapter)** is the primary `InstallFailure` producer — its heuristic stderr-to-`InstallFailureKind` mapper will produce values like `InstallFailureKind::NotFound` for "not found in marketplace" stderr substrings (per CONTEXT.md `<empirical_findings>`). The const-fn exhaustiveness sentinel guarantees adding a fifth variant in 12-04 fails to compile in two places.
- **Phase 13 (sync wiring)** drops the `#[allow(dead_code)]` attrs on `format_install_failures` / `render_install_failures` automatically when the first call site lands. The expected pattern: aggregate `Vec<InstallFailure>` across adapter calls, call `render_install_failures(&v)`, then return non-zero exit per ADP-04.

## Self-Check: PASSED

Verified via:
- `[ -f crates/tome/src/marketplace.rs ] && echo FOUND` → FOUND
- `git log --oneline | grep -E "(08dc059|27820f5)"` → both commits present
- `cargo test -p tome --lib marketplace::tests --quiet` → 11 passed, 0 failed (4 from Plan 12-01 + 7 new)
- `cargo clippy -p tome --all-targets -- -D warnings` → exits 0
- `rustfmt --check crates/tome/src/marketplace.rs` → no diffs
- `cargo check -p tome` → exits 0
- `grep -qE 'pub const ALL: \[InstallFailureKind; 4\]' crates/tome/src/marketplace.rs` → match
- `grep -q "_ensure_install_failure_kind_all_exhaustive" crates/tome/src/marketplace.rs` → match
- `grep -qE 'assert!\(InstallFailureKind::ALL\.len\(\) == 4\)' crates/tome/src/marketplace.rs` → match
- `grep -q "pub fn render_install_failures" crates/tome/src/marketplace.rs` → match
- `grep -q "pub(crate) fn format_install_failures" crates/tome/src/marketplace.rs` → match
- `grep -q "for kind in InstallFailureKind::ALL" crates/tome/src/marketplace.rs` → match
- `grep -q "install operations failed" crates/tome/src/marketplace.rs` → match
- `grep -q 'style("⚠").yellow()' crates/tome/src/marketplace.rs` → match

---
*Phase: 12-marketplace-adapter*
*Completed: 2026-05-05*

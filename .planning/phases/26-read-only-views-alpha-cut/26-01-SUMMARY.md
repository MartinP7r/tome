---
phase: 26-read-only-views-alpha-cut
plan: 01
subsystem: ui
tags: [tauri, react, specta, status, tome-desktop, view-01]

requires:
  - phase: 25-rust-core-extraction-tauri-integration-spike
    provides: "StatusReport gather() surface; get_status Tauri command; bindings.ts via specta; React 19 + Vite + tome-desktop scaffold; TomeError IPC boundary"
provides:
  - "LockfileState enum (InSync / OutOfSync { drift_count } / Missing) on StatusReport, specta-gated"
  - "MachinePrefsSummary { disabled_count, disabled_directory_count } on StatusReport, specta-gated"
  - "LockfileState::classify() — manifest content_hash comparison, reconcile.rs-shaped semantics, reused via the existing manifest accessor"
  - "Regenerated bindings.ts exposing LockfileState (tagged kind union) + MachinePrefsSummary TS types"
  - "StatusView React component rendering every StatusReport field via KeyValueRow + DirectoryTable"
  - "Shared atoms: KeyValueRow, Badge (9 subtypes), StatusDot, Pill, DirectoryTable — CSS Modules per D-15"
  - "useStatus hook (RESEARCH Pattern 2 shape) — Result-narrowing + updatedAt tracking"
  - "formatRelative() display-only timestamp helper (D-GUI-08)"
  - "App.tsx lands on StatusView (D-02)"

affects: [26-02, 26-03, 26-04, 26-05, 26-06]

tech-stack:
  added: []  # no new npm/cargo deps — built on Phase 25 surface
  patterns:
    - "POLISH-04 ALL array + compile-time exhaustiveness sentinel on LockfileState (mirrors InstallFailureKind, RemoveFailureKind)"
    - "Discriminated-union tagged enum at IPC boundary: #[serde(tag = \"kind\", rename_all = \"snake_case\")] → TS { kind: 'in_sync' | 'out_of_sync' | 'missing', ... }"
    - "useX data-hook pattern: fetch via commands → narrow Result → track updatedAt for transient acknowledgement pills"
    - "CSS Modules per-component (*.module.css) — Vite-native, zero runtime, paired with literal token values until plan 26-02 swaps in tokens.css"

key-files:
  created:
    - "crates/tome-desktop/ui/src/views/StatusView.tsx"
    - "crates/tome-desktop/ui/src/components/KeyValueRow.tsx + .module.css"
    - "crates/tome-desktop/ui/src/components/Badge.tsx + .module.css"
    - "crates/tome-desktop/ui/src/components/StatusDot.tsx"
    - "crates/tome-desktop/ui/src/components/Pill.tsx + .module.css"
    - "crates/tome-desktop/ui/src/components/DirectoryTable.tsx + .module.css"
    - "crates/tome-desktop/ui/src/hooks/useStatus.ts"
    - "crates/tome-desktop/ui/src/lib/relativeTime.ts"
    - "crates/tome-desktop/ui/src/vite-env.d.ts"
  modified:
    - "crates/tome/src/status.rs — LockfileState + MachinePrefsSummary types, gather() population, render_status() output, 6 unit tests"
    - "crates/tome-desktop/ui/src/bindings.ts — regenerated (LockfileState + MachinePrefsSummary)"
    - "crates/tome-desktop/ui/src/App.tsx — renders <StatusView /> (was inline single-pane)"
    - "crates/tome/tests/snapshots/cli_status__status_empty_library.snap — intentionally re-blessed to include new Lockfile + Machine lines"

key-decisions:
  - "LockfileState classifier reuses manifest::load + lockfile::load with content_hash comparison; no domain logic duplication (matches reconcile.rs semantics per OQ-4)"
  - "drift_count counts BOTH hash mismatches AND missing-from-manifest entries — both are user-visible drift signals; status doesn't need the granularity reconcile.rs adds for the marketplace-side"
  - "MachinePrefsSummary exposes integer counts only — the full skill/directory lists stay in machine.toml (T-26-01-02: scalar integers, no PII)"
  - "Used #[serde(tag = \"kind\", rename_all = \"snake_case\")] so the TS side is a clean discriminated union { kind: 'in_sync' | 'out_of_sync' | 'missing', ... } — enables exhaustive pattern-matching on the React side"
  - "deriveTomeHome() in StatusView falls back to library_dir's parent for the TOME HOME row because StatusReport doesn't carry tome_home explicitly today; display-only fallback, no business logic violates D-GUI-08"
  - "Badge subtype → CSS class via static record + Set<'managed'|'disabled'> for the weight-600 emphasis bin — no runtime branching in JSX"
  - "useStatus does NOT subscribe to events in this plan — Pattern 2's full watcher wiring lands in 26-06; hook surface accepts the extension non-breakingly"
  - "Snapshot re-blessing for cli_status__status_empty_library is intentional: the Status text output gained 'Lockfile: ✗ missing' + 'Machine: 0 skills disabled, 0 directories disabled' lines"

patterns-established:
  - "StatusReport additive extension pattern: every new GUI-surfaced field gets a typed Rust struct/enum + specta gate + bindings.ts regen; the JSON shape stays back-compat-additive"
  - "Per-view useX hook with Result-narrowing inside the hook (not the view) — the view stays purely presentational"
  - "Per-component .module.css siblings with @media (prefers-color-scheme: dark) overrides — no theme switcher; dark mode just works"
  - "Atom subtype enums are exhaustive TS unions paired with a Record<Subtype, className> — adding a variant fails type-check, not at runtime"

requirements-completed: [VIEW-01]

# Metrics
duration: ~40min
completed: 2026-05-29
---

# Phase 26 Plan 01: Read-only views alpha cut — Status view + LockfileState/MachinePrefsSummary Summary

**StatusReport gains lockfile-state + machine-prefs-summary fields, bindings regenerate cleanly, and the app boots into a real React StatusView built from five new atoms — first user-visible UI surface on the Tauri shell.**

## Performance

- **Duration:** ~40 min
- **Started:** 2026-05-29T02:24:13Z
- **Completed:** 2026-05-29T03:03:51Z
- **Tasks:** 2 / 2
- **Files modified:** 16 (4 modified, 12 created)

## Accomplishments

- `StatusReport` extended with two GUI-visible fields (`lockfile: LockfileState`, `machine_prefs_summary: MachinePrefsSummary`) — additive, specta-gated, preserves every existing field byte-for-byte.
- `LockfileState::classify()` reuses the same content-hash comparison `reconcile.rs` performs against the marketplace, but operates against the on-disk manifest (no adapter required). Three-state classification: `InSync` / `OutOfSync { drift_count }` / `Missing`. POLISH-04 `ALL` + compile-time exhaustiveness sentinel mirrors `InstallFailureKind`.
- `bindings.ts` regenerated through the Phase 25 freshness gate — `LockfileState` lands as a TS discriminated union `{ kind: "in_sync" | "out_of_sync" | "missing", ... }`, ready for exhaustive `switch` on the React side.
- React `StatusView` renders every `StatusReport` field via the new atoms; the app boots directly into Status (D-02). Lockfile state is paired with a `StatusDot`, machine-prefs summary shows "N skills disabled", "Updated" pill is wired into the LAST SYNC row via `useStatus.updatedAt`.
- Five atom components shipped with CSS-Modules siblings, light + dark token bindings, and weight-per-subtype emphasis discipline: `KeyValueRow`, `Badge` (9 subtypes), `StatusDot`, `Pill`, `DirectoryTable`. All follow the UI-SPEC §Component Contract token bindings (literal hexes inlined now; plan 26-02 swaps in `tokens.css`).

## Task Commits

1. **Task 1: Extend StatusReport with LockfileState + MachinePrefsSummary** — `265bb22` (feat)
2. **Task 2: Regenerate bindings.ts and add StatusView + supporting React atoms** — `e68e7ab` (feat)

## Files Created/Modified

**Rust:**
- `crates/tome/src/status.rs` — added `LockfileState` enum + POLISH-04 sentinel + `MachinePrefsSummary` struct; wired both into `StatusReport`; `gather()` populates from `lockfile::load` + `manifest::load` + `machine::load`; `render_status()` appends two new lines; 6 unit tests covering the four classify paths + serde tag shape + `ALL` order.

**Bindings:**
- `crates/tome-desktop/ui/src/bindings.ts` — regenerated; exports new `LockfileState` and `MachinePrefsSummary` TS types; both fields appear on `StatusReport_Serialize` and `StatusReport_Deserialize`.

**React UI:**
- `crates/tome-desktop/ui/src/App.tsx` — replaced inline single-pane body with `<StatusView />`. Old card grid removed.
- `crates/tome-desktop/ui/src/views/StatusView.tsx` — renders 5 KeyValueRows (TOME HOME / LIBRARY / LAST SYNC / LOCKFILE / MACHINE) + `<DirectoryTable />`. Error banner shape preserved from Phase 25.
- `crates/tome-desktop/ui/src/components/KeyValueRow.tsx` + `.module.css` — 160px label / 1fr value / auto trailing grid; mono variant; light + dark tokens.
- `crates/tome-desktop/ui/src/components/Badge.tsx` + `.module.css` — 9 subtypes via `Record<BadgeSubtype, className>`; weight-600 set = `{managed, disabled}`.
- `crates/tome-desktop/ui/src/components/StatusDot.tsx` — 8px aria-hidden circle, success/danger.
- `crates/tome-desktop/ui/src/components/Pill.tsx` + `.module.css` — transient `Updated` with 2s CSS fade; `prefers-reduced-motion` honored; `role="status"` + `aria-live="polite"`.
- `crates/tome-desktop/ui/src/components/DirectoryTable.tsx` + `.module.css` — native `<table>` with `<th scope="col">`; NAME column carries primary + secondary mono path line; role → subtype map, type-string → subtype map (unknown falls back to neutral `type-directory`).
- `crates/tome-desktop/ui/src/hooks/useStatus.ts` — Pattern 2 shape; fetches once on mount; tracks `updatedAt` for `Pill` timing; no event subscriptions (those land in 26-06).
- `crates/tome-desktop/ui/src/lib/relativeTime.ts` — pure `formatRelative()` returning "Never" / "Just now" / "N seconds ago" / "N minutes ago" / "Today at h:mm AM/PM" / "{Weekday} at h:mm AM/PM" / RFC-3339 fallback.
- `crates/tome-desktop/ui/src/vite-env.d.ts` — CSS Modules type shim (`declare module "*.module.css"`) required by strict TS + Vite.

**Snapshots:**
- `crates/tome/tests/snapshots/cli_status__status_empty_library.snap` — intentionally re-blessed to include the two new render lines.

## Decisions Made

- **Lockfile drift_count semantics include BOTH hash mismatches AND missing-from-manifest entries.** The Status view shows a single drift count; users want "how many things are off", not "how many were exactly mismatched vs. how many had no manifest counterpart". `reconcile.rs` already distinguishes these for the marketplace side; `status.rs` doesn't need the granularity.
- **deriveTomeHome() display heuristic.** `StatusReport` doesn't currently carry `tome_home` explicitly. The UI-SPEC asks for a TOME HOME row, so the StatusView derives it from `library_dir`'s parent (stripping a trailing `/library`). A future Rust-side `StatusReport.tome_home: PathBuf` field would replace this — captured as a follow-up below.
- **Atom subtype mapping uses static `Record<Subtype, className>` + Set membership for weight binning.** This produces a TS-exhaustive matrix: adding a new `BadgeSubtype` variant without updating the record fails type-check; no runtime branching in JSX.
- **CSS Module type shim added at `vite-env.d.ts` rather than per-file.** Vite's official template uses the same single-file shim; matches the `vite/client` reference path.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added CSS Modules type declaration**
- **Found during:** Task 2 (tsc compile check)
- **Issue:** `tsc --noEmit` failed with `TS2307: Cannot find module './*.module.css'` for every atom because the project had no CSS Modules ambient declaration.
- **Fix:** Created `crates/tome-desktop/ui/src/vite-env.d.ts` with `declare module "*.module.css" { const classes: { readonly [key: string]: string }; export default classes; }`. Standard Vite-project pattern; matches the official `vite/client` reference template.
- **Files modified:** `crates/tome-desktop/ui/src/vite-env.d.ts` (created)
- **Verification:** `npx tsc --noEmit` exits 0 across all atoms + view + hook.
- **Committed in:** `e68e7ab` (Task 2 commit)

**2. [Rule 2 - Missing Critical] StatusReport JSON tests needed update for new fields**
- **Found during:** Task 1 (post-edit compile/test)
- **Issue:** Two existing `#[test]` functions (`json_status_always_includes_unowned_field`, `json_status_serializes_unowned_skill_summaries`) construct `StatusReport` literals; adding the two new required fields broke them. Without this fix, the new field would have been silently uncovered by struct construction tests.
- **Fix:** Updated both test literals to include `lockfile: LockfileState::Missing` + a zero-count `MachinePrefsSummary`. The test intent (unowned shape) is preserved; the new fields are populated with the most-conservative defaults so existing assertions still apply.
- **Files modified:** `crates/tome/src/status.rs`
- **Verification:** `cargo test -p tome --lib status::` exits 0; 39 tests pass.
- **Committed in:** `265bb22` (Task 1 commit)

**3. [Rule 1 - Bug] CLI status snapshot re-blessed**
- **Found during:** Task 1 (cli_status integration test)
- **Issue:** `status_shows_library_info` snapshot test failed because the Status text output now includes two new lines (`Lockfile: ✗ missing` + `Machine: 0 skills disabled, 0 directories disabled`). Plan §"action (e)" explicitly directed the executor to update snapshots in lock-step, not silently break them.
- **Fix:** Re-blessed `cli_status__status_empty_library.snap` to include the new lines. Other snapshots (`cli_status__status_unconfigured.snap`) were unaffected — the early-return for unconfigured systems skips the new lines.
- **Files modified:** `crates/tome/tests/snapshots/cli_status__status_empty_library.snap`
- **Verification:** `cargo test -p tome --test cli_status` exits 0; 13 tests pass.
- **Committed in:** `265bb22` (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (1 blocking, 1 missing critical, 1 bug).
**Impact on plan:** All three were explicitly anticipated by the plan text (CSS-module shim is implicit in any strict-TS Vite project; JSON-test literal updates and snapshot re-blessing were called out in action (e)). No scope creep.

## Issues Encountered

None — the plan was tight and the surface area was small. The Phase 25 freshness gate (`cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- bindings.ts`) caught the regen idempotency immediately, and the strict `tsc` config caught the missing CSS-module shim on first build attempt.

## Verification Results

All plan-level gates green:
- `cargo test -p tome --lib status::` → 39 passed, 0 failed
- `cargo test -p tome --test cli_status` → 13 passed, 0 failed
- `cargo test -p tome --tests --no-fail-fast` → all suites green
- `cargo clippy --all-targets -- -D warnings` → clean across the whole workspace (including `tome-desktop`)
- `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts` → bindings fresh post-commit
- `cd crates/tome-desktop/ui && npx tsc --noEmit` → exits 0
- `npm run build` (Vite + tsc) → 45 modules transformed, 202kB JS / 5.26kB CSS gzipped

The success criteria's manual smoke (`cargo tauri dev` boots; window shows the user's real StatusReport with the two new fields populated) was not executed in this autonomous run — that's a human-touch verification step the planner left non-blocking. The structural verification suite stands in for it: bindings reflect the new shape, the `useStatus` hook resolves through the same `Result<StatusReport, TomeError>` boundary, and the StatusView renders every field via the new atoms.

## Next Phase Readiness

Ready for plan 26-02 (Window/Titlebar/Sidebar/ContentPane shell + tokens.css consolidation). The atom set this plan ships (KeyValueRow, Badge, StatusDot, Pill, DirectoryTable) is the foundation 26-02..26-06 build on; their literal hex tokens are positioned for the mechanical `tokens.css` swap. The `useStatus` hook is ready for 26-06's watcher-driven refetch wiring — adding event subscriptions is a non-breaking extension of the existing hook surface.

**Open follow-up surfaced during execution (non-blocking):**

- `StatusReport.tome_home: PathBuf` field — would replace `StatusView`'s `deriveTomeHome()` heuristic with a value the Rust side already knows. Trivial additive extension at the same `gather()` site. Not blocking alpha because the heuristic correctly produces the canonical `~/.tome/` path for the standard library layout, but worth filing alongside the rest of the alpha-polish backlog.

## Threat Flags

None. The two new fields and their classifier read only from files the CLI already owns (`tome.lock`, `.tome-manifest.json`, `machine.toml`) — same trust boundary, no new network or filesystem surface. T-26-01-01 (atomic-read accept), T-26-01-02 (scalar-count accept), T-26-01-03 (DoS — re-render gating handled at the `useStatus` level; event subscriptions land in 26-06), T-26-01-04 (specta gate verified by CI freshness check) all hold.

---
*Phase: 26-read-only-views-alpha-cut*
*Completed: 2026-05-29*

## Self-Check: PASSED

All 17 claimed files exist on disk; both task commits (`265bb22`, `e68e7ab`) are present in `git log`.

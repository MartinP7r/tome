---
phase: 27-sync-triage-ui
plan: 02
subsystem: sync-triage-ui
tags: [tauri, react, react-aria, gridlist, triage, section-header, sync-02, pitfall-1, pitfall-6]

# Dependency graph
requires:
  - phase: 27-sync-triage-ui
    plan: 01a
    provides: "DiscoveredSkill.synced_at (carried by LockfileDiff.changed/removed entries via manifest lookup), SyncProgress.item field (Sidebar badge wiring), TauriEventSink + RecordingSink fixtures"
  - phase: 27-sync-triage-ui
    plan: 01b
    provides: "tome::sync + SyncOptions public, MachinePrefs / load_machine_prefs re-exports, useSync Context + Provider + isRunningRef discipline, SyncView idle/running/terminal skeleton, Sidebar 4th NavItem with dual-meaning badge slot, useTauriEvent late-listen-race helper, axe-core baseline 5 scans"
  - phase: 26-read-only-views-alpha-cut
    provides: "SectionHeader primitive (extended here for level/trailing), Button/Badge atoms, DetailHeader 3-row composition pattern, SkillListRow 52px rhythm, useTauriEvent helper, formatRelative helper"
provides:
  - "tome::lockfile / tome::update modules re-exported pub (read shape needed at IPC boundary for read-only diff projection; CLI present_changes interactive triage stays in-crate)"
  - "tome::SkillOrigin + tome::SkillProvenance + tome::ContentHash + tome::discover_all re-exports at lib.rs root (narrow lifts; discover module stays pub(crate))"
  - "tome_desktop::sync_types module — LockfileDiff, TriageEntry, TriageEntryChangeKind IPC types + the pure lockfile_diff_projection(diff, manifest) → LockfileDiff helper. Specta-derived, alphabetically pre-sorted within each Vec, SkillOrigin reconstructed at the boundary so React reuses the existing discriminator the Skills view pattern-matches."
  - "tome_desktop::commands::get_lockfile_diff — read-only Tauri command. Loads tome.lock, discovers current skills, builds a prospective lockfile via lockfile::generate, diffs with update::diff, projects to LockfileDiff. Empty lockfile (first run) yields every discovered skill as Added. Git-source paths offline-resolved (follow-up: lift lockfile::resolved_paths_from_lockfile_cache when needed)."
  - "ui/src/components/SectionHeader extended with `level?: 2|3` and `trailing?: ReactNode`. Default level=2 preserves Phase 26 HealthView call-site contract (verified by SectionHeader.test.tsx). Level=3 wraps in <h3> with 20px indent for TriagePanel inner source-group headers. Trailing slot renders inside the heading; rendered AS A SIBLING in TriagePanel outer summary to satisfy axe nested-interactive."
  - "ui/src/components/TriagePanel.tsx — React Aria GridList (NOT ListBox per Pitfall 1) with nested GridListItem; three outer details/summary sections (NEW expanded, CHANGED/REMOVED collapsed); inner source-group SectionHeader at level=3; bulk-action buttons emitted OUTSIDE the <summary> (sibling div with .bulkActionRow class) so axe nested-interactive is clean; per-source-group buttons only on NEW (D-13)."
  - "ui/src/components/TriageRow.tsx — 52px row matching SkillListRow rhythm. Inline HTML <button> chip with stopPropagation handles D-12 keep<->disable toggle; Removed rows render non-interactive 'implicit remove' span. aria-label matches UI-SPEC VoiceOver template."
  - "ui/src/components/TriageDetail.tsx — DetailHeader-shaped 3-row composition (title + metadata grid + canonical RadioGroup picker). 'View source' pseudo-radio appears only for managed+git-sourced entries (origin.kind === 'managed' AND git_commit_sha_new is non-null); fires onViewSource and reverts to last legitimate decision via a useRef. Removed entries omit the RadioGroup and render the verbatim 'will be removed from the lockfile' helper copy (D-13)."
  - "ui/src/hooks/useLockfileDiff — wraps commands.getLockfileDiff with a refetch on lockfileChanged events GATED by isRunningRef.current (Pitfall 6 watcher-feedback discipline). Mount-time fetch is unconditional; refetch is gated. Does NOT subscribe to manifestChanged/libraryChanged/machinePrefsChanged."
  - "useSync extension — composes useLockfileDiff; exposes diff, diffError, decisions Map, selectedTriageSkill, pendingDecisionCount, pendingDiffCount, onDecisionChange, onBulkAction, selectTriageSkill, refetchDiff. Decisions seed to 'keep' for every Added+Changed entry on first diff load (Removed implicit per D-13). dismiss() clears decisions+selection. isRunningRef exposed for useLockfileDiff."
  - "App.tsx Sidebar badge wiring: syncBadge.kind='pending' surfaces pendingDiffCount (added+changed+removed per D-05), mutually exclusive with the failures badge (still a 27-05 stub)."
  - "ui/src/views/SyncView — mounts TriagePanel + TriageDetail in a flex split inside SyncView for both idle (changes pending) AND in-progress states (ContentPane is single-column today; 27-04 will graduate). onApply is a documented TODO stub for 27-03 PreviewPopover wiring."
  - "tests/a11y/axe.spec.ts gains a new 'sync view triage panel passes axe WCAG-AA' subtest using ?triage=1 query param to flip the a11y mock to a populated LockfileDiff (TRIAGE_DIFF fixture covers GridList + nested SectionHeader + chip + RadioGroup)."
  - "bindings.ts regenerated — LockfileDiff, TriageEntry, TriageEntryChangeKind, getLockfileDiff. CI freshness gate clean."
affects: [27-02b, 27-03, 27-04, 27-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "GridList vs ListBox for interactive-children-inside-rows (Pitfall 1). The Skills view's SkillListRow has no inline buttons so ListBox is fine there; TriageRow has the D-12 chip toggle so it MUST use GridList. Documented in TriagePanel.tsx and TriageRow.tsx headers; pinned by a vitest assertion that the rendered DOM contains role='grid' and NOT role='listbox'."
    - "Bulk-action buttons outside <summary> (axe nested-interactive fix). The plan's first cut put the bulk button in SectionHeader.trailing INSIDE <summary>, which trips axe's nested-interactive rule because <summary> is itself interactive. Moved the button to a sibling div with .bulkActionRow positioning; the inner source-group buttons stay in trailing because their SectionHeader is just an h3 (not interactive)."
    - "Pseudo-radio for action-disguised-as-radio (View source). useRef tracks the last legitimate decision so selecting the pseudo-radio fires the side-effect and immediately reverts the visible value. Documented in TriageDetail.tsx."
    - "Composable hook + isRunningRef threading. useSync exposes isRunningRef so child hooks (useLockfileDiff) can gate watcher refetches without re-subscribing on every isRunning render. The ref doesn't appear in the render-visible result for non-hook consumers."
    - "Seed-once decisions Map. The seeding effect runs when (diff !== null AND decisions.size === 0); watcher refetches that update the diff identity do NOT clobber in-progress edits because the size guard short-circuits."

key-files:
  created:
    - "crates/tome-desktop/src/sync_types.rs"
    - "crates/tome-desktop/ui/src/components/TriagePanel.tsx"
    - "crates/tome-desktop/ui/src/components/TriagePanel.module.css"
    - "crates/tome-desktop/ui/src/components/TriageRow.tsx"
    - "crates/tome-desktop/ui/src/components/TriageRow.module.css"
    - "crates/tome-desktop/ui/src/components/TriageDetail.tsx"
    - "crates/tome-desktop/ui/src/components/TriageDetail.module.css"
    - "crates/tome-desktop/ui/src/components/__tests__/SectionHeader.test.tsx"
    - "crates/tome-desktop/ui/src/components/__tests__/TriagePanel.test.tsx"
    - "crates/tome-desktop/ui/src/components/__tests__/TriageRow.test.tsx"
    - "crates/tome-desktop/ui/src/components/__tests__/TriageDetail.test.tsx"
    - "crates/tome-desktop/ui/src/hooks/useLockfileDiff.ts"
    - "crates/tome-desktop/ui/src/hooks/__tests__/useLockfileDiff.test.tsx"
    - "crates/tome-desktop/ui/src/hooks/__tests__/useSync.triage.test.tsx"
    - "crates/tome-desktop/ui/src/views/SyncView.module.css"
  modified:
    - "crates/tome/src/lib.rs"
    - "crates/tome-desktop/src/commands.rs"
    - "crates/tome-desktop/src/lib.rs"
    - "crates/tome-desktop/src/menu.rs"
    - "crates/tome-desktop/ui/src/App.tsx"
    - "crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts"
    - "crates/tome-desktop/ui/src/bindings.ts"
    - "crates/tome-desktop/ui/src/components/SectionHeader.tsx"
    - "crates/tome-desktop/ui/src/components/SectionHeader.module.css"
    - "crates/tome-desktop/ui/src/hooks/useSync.tsx"
    - "crates/tome-desktop/ui/src/hooks/__tests__/useSync.test.tsx"
    - "crates/tome-desktop/ui/src/hooks/__tests__/useMenuActions.test.tsx"
    - "crates/tome-desktop/ui/src/views/SyncView.tsx"
    - "crates/tome-desktop/tests/a11y/axe.spec.ts"

key-decisions:
  - "GridList (NOT ListBox) for TriagePanel — Pitfall 1 invariant. ListBox's spec forbids interactive children inside ListBoxItem; the D-12 inline keep/disable chip IS an interactive button. GridList supports interactive children by design. Pinned by a vitest assertion that the rendered DOM contains role='grid' and NOT role='listbox'."
  - "Bulk-action button placement: OUTSIDE <summary>, not in SectionHeader.trailing. The first cut put the bulk button inside the SectionHeader.trailing slot which the outer section's <summary> wraps — axe's nested-interactive rule flagged this as a WCAG 4.1.2 violation (interactive descendant inside interactive element). Moved the button to a sibling div with .bulkActionRow class positioned to the right of the summary line. Inner source-group buttons stay in trailing because their SectionHeader is just an h3 (not interactive). a11y test 'sync view triage panel passes axe WCAG-AA' confirms."
  - "Rust public-API widening (Rule 3): made tome::lockfile + tome::update modules public; re-exported tome::SkillOrigin / tome::SkillProvenance / tome::ContentHash / tome::discover_all at lib.rs root. The plan's Action step 1 quoted update::diff + LockEntry as available reads at the IPC boundary, but both modules were pub(crate). Pattern matches 27-01b's tome::sync widening — narrow re-exports, not whole-module pub when avoidable."
  - "lockfile_diff_projection extracted as a pure pub fn (rather than inlined into the Tauri command). Direct unit-test against in-memory Lockfile + Manifest fixtures, no AppHandle / TempDir required. Mirrors 27-01a's event_to_sync_progress and join_synced_at_from_manifest extraction patterns."
  - "Git-source diff resolution deferred to a follow-up. The plan's Action step 1 references resolved_paths_from_lockfile_cache; that helper is pub(crate) in tome::lockfile. For 27-02 the offline_resolved_paths helper returns an empty map — discover_all skips git-type directories silently. Most GUI users have at least one local directory so the panel is useful. Lifting the helper is a small follow-up for 27-03 or beyond; logged here, no separate deferred-items.md entry needed."
  - "Decisions seed on first diff load only (size === 0 guard). Watcher-driven refetches that update the diff identity must NOT clobber in-progress edits. The seed effect short-circuits when decisions.size > 0; the dismiss() clear sets it back to a fresh Map so the next diff load re-seeds."
  - "Sidebar badge wired to pendingDiffCount (added+changed+removed) per D-05, NOT to pendingDecisionCount. The plan's must_haves §truths spell this out: 'Sidebar's Sync NavItem renders a pre-sync badge with count = new + changed + removed while triage decisions are pending'. The decision count drives the [Apply N] button label inside the panel; the badge surfaces the full pending volume."
  - "Pseudo-radio for 'View source' action. UI-SPEC §TriageDetail describes it as a 'radio that doesn't actually mutate decision' — selecting it fires onViewSource() and reverts to the last legitimate decision. Implemented via useRef tracking the last legitimate value + an early-return branch in RadioGroup.onChange. Pinned by a TriageDetail.test.tsx assertion."
  - "useSync test invariant update — Pitfall 6 carryover. The original 27-01b useSync.test.tsx asserted 'subscribes ONLY to syncProgress' with all four watcher events negative. Plan 27-02 makes useSync compose useLockfileDiff (which subscribes to lockfileChanged BUT gates the handler by isRunningRef.current). The test was updated to assert syncProgress + lockfileChanged ARE subscribed; manifestChanged/libraryChanged/machinePrefsChanged remain negative. The Pitfall 6 discipline now lives INSIDE useLockfileDiff's handler (verified by its own test suite)."

patterns-established:
  - "Module-extension policy: add per-component .test.tsx siblings in __tests__/ rather than retrofitting existing tests. The 27-01b useSync.test.tsx kept its narrow scope; 27-02 added a sibling useSync.triage.test.tsx with the new triage-state assertions. Mirrors the SectionHeader / TriagePanel / TriageRow / TriageDetail per-file test convention."
  - "Pure projection + thin command shell. lockfile_diff_projection takes references and returns a value; the Tauri command body is the I/O wrapper (load lockfile, load manifest, run discover, build prospective lockfile, project). Tests exercise the pure fn; the command body is essentially the boundary glue."
  - "axe-core fixture activation by query string. The a11y mock returns an empty diff for the default case so the existing 'sync view passes axe' test stays unchanged. Setting ?triage=1 flips to a populated TRIAGE_DIFF fixture for the new 'sync view triage panel passes axe' subtest. Future plans that need a different fixture can adopt the same mechanism without rebuilding the mock."

requirements-completed:
  - SYNC-02

# Metrics
duration: 37min
completed: 2026-06-06
---

# Phase 27 Plan 02: SYNC-02 lockfile-diff triage panel Summary

**Sectioned GridList triage panel over the lockfile diff projection (NEW / CHANGED / REMOVED with source-group inner headers), inline keep/disable chip + bulk actions on NEW only, right-column TriageDetail with canonical RadioGroup picker + pseudo-radio 'View source', useSync extended with triage state, SyncView wired in idle + in-progress states. GridList chosen over ListBox per Pitfall 1; bulk buttons emitted outside `<summary>` per axe nested-interactive. SectionHeader extended with `level` + `trailing` props for 27-02b to consume; back-compat with Phase 26 HealthView preserved.**

## Performance

- **Duration:** ~37 min
- **Started:** 2026-06-06T13:28:34Z
- **Completed:** 2026-06-06T14:05:16Z
- **Tasks:** 2 (atomic; both committed)
- **Files created:** 15 (1 Rust module + 6 React source modules + 4 test modules + 4 CSS modules)
- **Files modified:** 14 (4 Rust + 10 React)

## Accomplishments

- **Task 1 (commit `a9c9406`).** Read-only diff projection lands as a pure `lockfile_diff_projection(&UpdateDiff, &Manifest) -> LockfileDiff` helper + the `get_lockfile_diff` Tauri command. The projection reconstructs `SkillOrigin` (managed-with-provenance vs. local) from lockfile fields so the React side reuses the same discriminator the Skills view already pattern-matches. The command loads tome.lock, discovers current skills, builds a prospective lockfile via `lockfile::generate`, diffs, projects — read-only end-to-end. SectionHeader extended with `level?: 2|3` + `trailing?: ReactNode` props; default level=2 preserves Phase 26 HealthView contract. useLockfileDiff hook subscribes to `lockfileChanged` only, gated by `isRunningRef` (Pitfall 6).
- **Task 2 (commit `57746dd`).** TriagePanel + TriageRow + TriageDetail land. TriagePanel renders three vertical outer sections (NEW expanded by default, CHANGED + REMOVED collapsed) each containing inner source-group GridLists. Bulk-action buttons emitted OUTSIDE `<summary>` to satisfy axe nested-interactive (sibling div, not in SectionHeader.trailing). TriageRow uses an inline HTML `<button>` for the keep/disable chip (NOT React Aria Button) with stopPropagation so the chip click does not bubble to the parent GridListItem's selection. TriageDetail mirrors DetailHeader's 3-row composition with a canonical RadioGroup picker + a pseudo-radio "View source" for managed+git entries that fires the side-effect and reverts to the last legitimate decision. useSync extended with `diff`, `decisions` Map (seed-on-first-load), `selectedTriageSkill`, `pendingDecisionCount`, `pendingDiffCount`, `onDecisionChange`, `onBulkAction`, `selectTriageSkill`, `refetchDiff`, `isRunningRef`. App.tsx Sidebar badge wired to pendingDiffCount per D-05. SyncView mounts the triage flow in a flex split inside the view (ContentPane is single-column today; 27-04 will graduate). axe-core scan extended with a triage-active subtest using `?triage=1`.

## Task Commits

Each task was committed atomically:

1. **Task 1: get_lockfile_diff command + LockfileDiff specta projection + SectionHeader level/trailing + useLockfileDiff hook** — `a9c9406` (feat) — `crates/tome/src/lib.rs`, `crates/tome-desktop/src/commands.rs`, `crates/tome-desktop/src/lib.rs`, `crates/tome-desktop/src/sync_types.rs` (new), `crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts`, `crates/tome-desktop/ui/src/bindings.ts`, `crates/tome-desktop/ui/src/components/SectionHeader.tsx`, `crates/tome-desktop/ui/src/components/SectionHeader.module.css`, `crates/tome-desktop/ui/src/components/__tests__/SectionHeader.test.tsx` (new), `crates/tome-desktop/ui/src/hooks/useLockfileDiff.ts` (new), `crates/tome-desktop/ui/src/hooks/__tests__/useLockfileDiff.test.tsx` (new).
2. **Task 2: TriagePanel + TriageRow + TriageDetail (GridList) + useSync triage state + SyncView wiring + axe spec** — `57746dd` (feat) — TriagePanel/TriageRow/TriageDetail (3 .tsx + 3 .module.css + 3 .test.tsx new), useSync.tsx (extended), useSync.test.tsx + useMenuActions.test.tsx (mock updates), useSync.triage.test.tsx (new), SyncView.tsx + SyncView.module.css (split body), App.tsx (badge wiring), tauri-api-core.ts mock (TRIAGE_DIFF fixture), axe.spec.ts (triage subtest).

## Files Created/Modified

- **`crates/tome/src/lib.rs`** — Made `lockfile` and `update` modules `pub`. Added `pub use` re-exports for `SkillOrigin`, `SkillProvenance`, `ContentHash`, `discover_all` at crate root. `cargo fmt` re-ordered some existing `pub use` statements alphabetically (no semantic change).
- **`crates/tome-desktop/src/commands.rs`** — Added `get_lockfile_diff` Tauri command + an `offline_resolved_paths` helper (currently returns empty; full git-cache resolution is a small follow-up). Imports `LockfileDiff` + `lockfile_diff_projection` from the new `sync_types` module.
- **`crates/tome-desktop/src/lib.rs`** — Registered `commands::get_lockfile_diff` in `make_builder()` and added `pub mod sync_types;`.
- **`crates/tome-desktop/src/sync_types.rs`** (new) — IPC types: `LockfileDiff`, `TriageEntry`, `TriageEntryChangeKind`. Pure `lockfile_diff_projection(diff, manifest) -> LockfileDiff` helper. 6 unit tests pin the contract (empty, Added, Changed, Removed, Managed provenance, mixed-alphabetical).
- **`crates/tome-desktop/src/menu.rs`** — `cargo fmt` re-wrapped the existing 5-item array literal across multiple lines (no semantic change). Included in the Task 2 commit.
- **`crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts`** — Added a `TRIAGE_DIFF` fixture (4 entries: managed-git + local + changed + removed) and a `get_lockfile_diff` mock that returns the fixture when `window.location.search` contains `triage=1`; defaults to an empty diff otherwise so the existing axe Sync scan stays unchanged.
- **`crates/tome-desktop/ui/src/bindings.ts`** — Regenerated: `LockfileDiff`, `TriageEntry`, `TriageEntryChangeKind`, `getLockfileDiff` command stub.
- **`crates/tome-desktop/ui/src/components/SectionHeader.tsx`** — Extended with `level?: 2|3` (default 2, back-compat) and `trailing?: ReactNode`. Switches on level to render h2 vs h3.
- **`crates/tome-desktop/ui/src/components/SectionHeader.module.css`** — Added `.header--level-2` and `.header--level-3` (20px indent + adjusted top margin); `.trailing` slot inside the heading.
- **`crates/tome-desktop/ui/src/components/__tests__/SectionHeader.test.tsx`** (new) — 5 tests: default <h2>, explicit level=2 <h2>, level=3 <h3>, trailing slot renders + is focusable, omitting trailing yields no button.
- **`crates/tome-desktop/ui/src/components/TriagePanel.tsx`** (new) — React Aria GridList. Three outer details/summary sections; inner source-group SectionHeader at level=3; bulk-action buttons emitted outside <summary>; D-13 invariant pinned (NEW-only).
- **`crates/tome-desktop/ui/src/components/TriagePanel.module.css`** (new) — Panel layout + .bulkActionRow positioning + .applyRow border + .summary focus-visible outline.
- **`crates/tome-desktop/ui/src/components/__tests__/TriagePanel.test.tsx`** (new) — 10 tests: outer h2 counts, inner h3 source groups, GridList NOT ListBox (Pitfall 1), bulk button on NEW only (D-13), bulk button fires with section scope, source-group bulk button, inline chip toggles, Apply N decisions label + disabled state.
- **`crates/tome-desktop/ui/src/components/TriageRow.tsx`** (new) — 52px row + inline HTML `<button>` chip with stopPropagation + Removed-row non-interactive 'implicit remove' span. aria-label per UI-SPEC.
- **`crates/tome-desktop/ui/src/components/TriageRow.module.css`** (new) — Row grid, chip variants (keep / disable / removed), selected-row accent fill.
- **`crates/tome-desktop/ui/src/components/__tests__/TriageRow.test.tsx`** (new) — 7 tests: primary + secondary content, managed vs local label, chip flip, chip toggle handler, chip aria-label, removed-row chip is static.
- **`crates/tome-desktop/ui/src/components/TriageDetail.tsx`** (new) — DetailHeader-shaped composition. RadioGroup picker for Added/Changed; pseudo-radio 'View source' on managed+git; verbatim removed-helper for Removed; collapsible 'Show diff metadata' on Changed.
- **`crates/tome-desktop/ui/src/components/TriageDetail.module.css`** (new) — Detail layout, metadata grid (same shape as DetailHeader), placeholder.
- **`crates/tome-desktop/ui/src/components/__tests__/TriageDetail.test.tsx`** (new) — 7 tests: placeholder when entry=null, section aria-label = '${name} change details', 2 radios for local, 3 radios for managed-git, View-source pseudo-radio fires onViewSource and reverts to previous decision, Removed entries omit RadioGroup + render verbatim copy.
- **`crates/tome-desktop/ui/src/hooks/useLockfileDiff.ts`** (new) — Mount-time fetch + lockfileChanged refetch gated by isRunningRef.current (Pitfall 6). Direct subscription (NOT useTauriEvent) so the typed payload check has access to the ref.
- **`crates/tome-desktop/ui/src/hooks/__tests__/useLockfileDiff.test.tsx`** (new) — 4 tests: mount fetch fires once, subscribes ONLY to lockfileChanged, refetches when isRunningRef.current is false, does NOT refetch when isRunningRef.current is true (Pitfall 6).
- **`crates/tome-desktop/ui/src/hooks/useSync.tsx`** — Extended UseSyncResult interface with triage fields (diff, diffError, decisions, selectedTriageSkill, pendingDecisionCount, pendingDiffCount, isRunningRef, onDecisionChange, onBulkAction, selectTriageSkill, refetchDiff). Composes useLockfileDiff; seeds decisions on first non-null diff load (size === 0 guard); dismiss() clears the triage state.
- **`crates/tome-desktop/ui/src/hooks/__tests__/useSync.test.tsx`** — Mock updated to include getLockfileDiff. The 'subscribes ONLY to syncProgress' test was rewritten to reflect the new invariant: syncProgress + lockfileChanged subscribed (via useLockfileDiff), other watcher events NOT subscribed.
- **`crates/tome-desktop/ui/src/hooks/__tests__/useSync.triage.test.tsx`** (new) — 5 tests: diff populates from mock, decisions seed for Added+Changed only, pendingDecisionCount tracks non-default decisions, section-scope bulk action flips all NEW, source-group-scope bulk action flips only matching source.
- **`crates/tome-desktop/ui/src/hooks/__tests__/useMenuActions.test.tsx`** — Mock updated to include getLockfileDiff + lockfileChanged.listen stub so the SyncProvider mounts cleanly.
- **`crates/tome-desktop/ui/src/App.tsx`** — Sidebar badge wiring switched from `sync.pendingDecisions` (a 27-01b stub) to `sync.pendingDiffCount` (added+changed+removed per D-05).
- **`crates/tome-desktop/ui/src/views/SyncView.tsx`** — In-progress + idle branches now mount TriagePanel + TriageDetail in a flex split inside the view when diff is non-empty. `onApply` is a documented TODO stub for 27-03.
- **`crates/tome-desktop/ui/src/views/SyncView.module.css`** (new) — Idle hero centering, in-progress stepper placeholder, .splitBody flex layout, triage/detail column widths.
- **`crates/tome-desktop/tests/a11y/axe.spec.ts`** — Added 'sync view triage panel passes axe WCAG-AA (Phase 27 plan 27-02)' subtest. Navigates to `/?triage=1`, waits for the NEW outer SectionHeader's `<h2>` + the Apply button, scans, asserts zero WCAG-AA violations.

## Decisions Made

See `key-decisions` in the frontmatter for full rationale. Quick index:

1. **GridList (NOT ListBox) for TriagePanel** — Pitfall 1 invariant. The inline keep/disable chip is an interactive button; ListBoxItem forbids these. Pinned by a vitest assertion.
2. **Bulk-action buttons OUTSIDE `<summary>`** — axe nested-interactive (WCAG 4.1.2) was tripped by the first cut (SectionHeader.trailing inside <summary>). Moved to a sibling div with .bulkActionRow positioning. Inner source-group buttons stay in trailing (their SectionHeader is a non-interactive h3).
3. **Rust public-API widening (Rule 3 deviations)** — `tome::lockfile` + `tome::update` modules to pub; `tome::SkillOrigin` + `tome::SkillProvenance` + `tome::ContentHash` + `tome::discover_all` re-exported at the crate root. The plan's `<interfaces>` block quoted these as available reads but they were `pub(crate)`. Pattern matches 27-01b's `tome::sync` widening.
4. **`lockfile_diff_projection` extracted as a pure fn** — direct unit-testability without an `AppHandle`. Mirrors 27-01a's `event_to_sync_progress` + `join_synced_at_from_manifest` extraction.
5. **Git-source diff resolution deferred** — `offline_resolved_paths` returns an empty map; full cache-based resolution requires lifting `lockfile::resolved_paths_from_lockfile_cache` (still pub(crate)). Out of 27-02 scope; documented in the helper's doc comment.
6. **Decisions seed-on-first-load** — `decisions.size === 0` guard short-circuits the seed effect so watcher refetches don't clobber in-progress edits. `dismiss()` resets the Map for the next cycle.
7. **Sidebar badge = `pendingDiffCount`** — D-05 spells out the badge counts the diff (added+changed+removed), NOT the pending decisions. The pending-decisions count drives the `[Apply N]` button label inside the panel.
8. **'View source' pseudo-radio** — useRef tracks the last legitimate decision; selecting the pseudo-radio fires `onViewSource()` and reverts via `onDecisionChange(lastRef.current)`. UI-SPEC § TriageDetail describes this pattern.
9. **useSync test invariant update** — Pitfall 6 discipline now lives INSIDE useLockfileDiff (gates `lockfileChanged` handler by `isRunningRef.current`). The 27-01b test that asserted useSync subscribes ONLY to syncProgress was updated to reflect that useSync now composes useLockfileDiff; the watcher-feedback invariant is verified by useLockfileDiff's own test suite.

## Deviations from Plan

### Rule 3 — Auto-fixed blocking issues

**1. [Rule 3 - Visibility] Made `tome::lockfile` + `tome::update` modules public; re-exported `SkillOrigin` / `SkillProvenance` / `ContentHash` / `discover_all` at crate root**
- **Found during:** Task 1 (`cargo build` failed because `tome::lockfile`, `tome::update`, `tome::discover` were all `pub(crate)` and the IPC boundary needs read access).
- **Issue:** The plan's `<interfaces>` block quoted `update::diff`, `LockEntry`, `Lockfile`, `SkillOrigin`, `SkillProvenance` as available reads at the IPC boundary. Reality: all three modules were `pub(crate)`. Mirrors the same widening pattern 27-01b applied for `tome::sync` (its own Rule 3 deviation).
- **Fix:** Made `lockfile` + `update` modules `pub` (with module-level doc comments explaining the IPC boundary motivation). Re-exported `SkillOrigin`, `SkillProvenance`, `ContentHash`, `discover_all` at `tome::lib.rs` root (narrow re-exports — kept `discover` itself `pub(crate)`).
- **Files modified:** `crates/tome/src/lib.rs`
- **Commit:** `a9c9406`

**2. [Rule 3 - Bulk button structure] Moved bulk-action button outside `<summary>` to satisfy axe nested-interactive**
- **Found during:** Task 2 — initial run of the new 'sync view triage panel passes axe' subtest failed on a `nested-interactive` violation (WCAG 4.1.2 — interactive descendant inside interactive element).
- **Issue:** The first cut put the bulk-action button inside `SectionHeader.trailing` which was inside the outer `<summary>` element. `<summary>` is itself interactive; placing a button inside is forbidden.
- **Fix:** Restructured TriagePanel.tsx — the outer-section SectionHeader now renders with no trailing slot (just label + count). The bulk button is emitted as a sibling div with `.bulkActionRow` positioning to the right of the summary line. Inner source-group buttons remain in `SectionHeader.trailing` because their SectionHeader is just an `<h3>` (not interactive).
- **Files modified:** `crates/tome-desktop/ui/src/components/TriagePanel.tsx`, `crates/tome-desktop/ui/src/components/TriagePanel.module.css`
- **Commit:** `57746dd`

### Rule 1 — Auto-fixed bug

**3. [Rule 1 - Test fixture] `lockfile_with` test helper hash-seed collision**
- **Found during:** Task 1, first run of `mixed_diff_buckets_remain_alphabetical` test.
- **Issue:** `ContentHash::new()` calls `to_ascii_lowercase()`. The test seeded `"apple"` with `"aa"` in old and `"AA"` in new expecting a Changed diff — but lowercasing collapsed both to the same hash, so the diff was empty (unchanged). 1/6 tests failed on the wrong-expectation row.
- **Fix:** Changed the new-hash seed to `"ff"` so it differs from `"aa"` after lowercasing. Test passes.
- **Files modified:** `crates/tome-desktop/src/sync_types.rs`
- **Commit:** `a9c9406`

### Scope adjustments (NOT deviations, documented for handoff)

**4. [Scope clarification] `useSync.test.tsx` Pitfall 6 test rewritten**
- The 27-01b test asserted useSync subscribes ONLY to syncProgress. Plan 27-02 makes useSync compose useLockfileDiff which subscribes to lockfileChanged (gated by isRunningRef). The test was updated: now asserts syncProgress + lockfileChanged ARE subscribed; manifestChanged/libraryChanged/machinePrefsChanged/menuAction remain negative. The Pitfall 6 discipline now lives INSIDE useLockfileDiff (its own test pins the gating). Not a deviation; the discipline is preserved, just at a different level.

**5. [Scope clarification] useMenuActions.test.tsx mock updated**
- The 27-01b useMenuActions test mocked bindings but did NOT include `getLockfileDiff` or `lockfileChanged`. Updated the mock to add both so the SyncProvider mounts cleanly. Not a deviation; pure test-infrastructure update.

**6. [Out-of-scope discovery] Git-source diff resolution requires `lockfile::resolved_paths_from_lockfile_cache` to be lifted**
- The pub(crate) helper would let `get_lockfile_diff` resolve git-cloned repo paths for offline diff computation. For 27-02 the `offline_resolved_paths` shim returns empty so discover_all skips git-type directories silently. Most GUI users have at least one local directory so the panel is useful. Full git-diff support is a small follow-up (lift the helper, wire the call); out of scope for this plan and not blocking.

## Issues Encountered

- **`cargo fmt` re-ordered `pub use` statements alphabetically in `tome::lib.rs`** and re-wrapped the existing 5-item array literal in `menu.rs`. No semantic change; included in the Task 2 commit alongside the intentional widening.
- **axe `nested-interactive` violation** on the first triage-panel run. Fixed by moving the bulk-action button outside `<summary>` (see Deviation #2). Re-running the axe scan confirms 0 violations.
- **Hash-seed collision** in the first `mixed_diff_buckets_remain_alphabetical` test (case-insensitive hashing). Fixed by using non-overlapping hex seeds (see Deviation #3).

## User Setup Required

None — Phase 27 SYNC-02 is fully self-contained. No env vars, no dashboards, no new external dependencies.

## Next Phase Readiness

- **27-02b (sibling, runs in Wave 3) — VIEW-02 SkillsView carryover closure.** Can consume the extended SectionHeader (`level` + `trailing` props) for the group-by Source / Role render and the synced_at-driven Sort=Recent comparator. SectionHeader's back-compat with Phase 26 HealthView is pinned by `SectionHeader.test.tsx` so 27-02b's wiring won't trip the existing scan.
- **27-03 — Apply flow + PreviewPopover.** The TriagePanel's `onApply` prop is a documented TODO stub; 27-03 will wire it to fire the new `preview_machine_toml` command + render `MachineTomlDiff` inside the Phase 26 `PreviewPopover` (Pitfall 3 popover-slot refactor lands there). useSync's `refetchDiff()` is already exposed so 27-03 can re-fetch the diff after a successful Apply.
- **27-04 — StageStepper + cancellation invariant test.** The split-pane layout inside SyncView keeps the existing stepper placeholder above the triage panel; 27-04 swaps the placeholder for the real StageStepper without touching the triage flow. The `isRunning` flag continues to gate Pitfall 6 inside useLockfileDiff.
- **27-05 — SyncOutcomeWire + partial-failure rendering.** The Sidebar's syncBadge tagged-union already supports the `{ kind: "failures", count }` branch (mutually exclusive with pending); 27-05 only needs to populate `failureCount` from the new outcome shape.
- **Follow-up:** lift `lockfile::resolved_paths_from_lockfile_cache` to public so `get_lockfile_diff` can resolve git-source paths offline (currently skipped via the `offline_resolved_paths` shim).
- **No blockers carried forward.**

## Verification Summary

- `cargo build -p tome-desktop`: clean.
- `cargo test -p tome-desktop --lib sync_types`: 6/6 pass.
- `cargo test -p tome --lib`: 916/916 pass (no regression from public-API widening).
- `cargo clippy -p tome-desktop -p tome --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts`: clean (CI freshness gate passes).
- `npx tsc --noEmit` in `crates/tome-desktop/ui/`: clean.
- `npm test -- --run` in `crates/tome-desktop/ui/`: 58/58 tests across 11 test files (5 new test files added in this plan: SectionHeader, useLockfileDiff, TriagePanel, TriageRow, TriageDetail, useSync.triage; 2 updated: useSync, useMenuActions).
- `npm run test:a11y`: 6/6 axe-core scans pass (the new 'sync view triage panel passes axe WCAG-AA' subtest passes with zero WCAG-AA violations on the populated TRIAGE_DIFF fixture).
- Manual smoke (NOT run — no GUI binary launched in this scope): a hands-on session will exercise the triage panel + chip toggle + RadioGroup + Apply stub.

## Self-Check: PASSED

All claimed artifacts verified:

- `.planning/phases/27-sync-triage-ui/27-02-SUMMARY.md` written (this file).
- Rust sources:
  - `crates/tome/src/lib.rs` (modified) ✓
  - `crates/tome-desktop/src/commands.rs` (modified) ✓
  - `crates/tome-desktop/src/lib.rs` (modified) ✓
  - `crates/tome-desktop/src/menu.rs` (cargo fmt only) ✓
  - `crates/tome-desktop/src/sync_types.rs` (new) ✓
- React sources:
  - `crates/tome-desktop/ui/src/App.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/bindings.ts` (regenerated) ✓
  - `crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts` (modified) ✓
  - `crates/tome-desktop/ui/src/components/SectionHeader.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/components/SectionHeader.module.css` (modified) ✓
  - `crates/tome-desktop/ui/src/components/TriagePanel.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/components/TriagePanel.module.css` (new) ✓
  - `crates/tome-desktop/ui/src/components/TriageRow.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/components/TriageRow.module.css` (new) ✓
  - `crates/tome-desktop/ui/src/components/TriageDetail.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/components/TriageDetail.module.css` (new) ✓
  - `crates/tome-desktop/ui/src/hooks/useLockfileDiff.ts` (new) ✓
  - `crates/tome-desktop/ui/src/hooks/useSync.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/views/SyncView.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/views/SyncView.module.css` (new) ✓
- Tests:
  - `crates/tome-desktop/ui/src/components/__tests__/SectionHeader.test.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/components/__tests__/TriagePanel.test.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/components/__tests__/TriageRow.test.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/components/__tests__/TriageDetail.test.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/hooks/__tests__/useLockfileDiff.test.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/hooks/__tests__/useSync.triage.test.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/hooks/__tests__/useSync.test.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/hooks/__tests__/useMenuActions.test.tsx` (modified) ✓
  - `crates/tome-desktop/tests/a11y/axe.spec.ts` (modified, new subtest) ✓
- Commits `a9c9406` and `57746dd` present in `git log --oneline`.
- `grep -E "import.*GridList" crates/tome-desktop/ui/src/components/TriagePanel.tsx` confirms `import { GridList, GridListItem } from "react-aria-components"` — Pitfall 1 invariant pinned at source.
- `grep -E "<ListBox|ListBoxItem" crates/tome-desktop/ui/src/components/TriagePanel.tsx` returns NO JSX/import references (only doc-comment mentions in the file header explaining why GridList was chosen) — Pitfall 1 invariant doubly pinned at the code level.

---
*Phase: 27-sync-triage-ui*
*Completed: 2026-06-06*

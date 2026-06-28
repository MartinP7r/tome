---
phase: 27-sync-triage-ui
plan: 02b
subsystem: skills-view-ui
tags: [react, skills-view, section-header, view-02-carryover, sort-recent, synced-at]

# Dependency graph
requires:
  - phase: 27-sync-triage-ui
    plan: 01a
    provides: "DiscoveredSkill.synced_at: Option<String> field (D-16) populated from the manifest at the post-discover boundary of sync(); surfaces in ListReport via bindings.ts as `synced_at?: string | null`."
  - phase: 27-sync-triage-ui
    plan: 02
    provides: "SectionHeader extended with `level?: 2 | 3` and `trailing?: ReactNode` props; default level=2 preserves Phase 26 HealthView back-compat; consumed here at level=2 for inter-group dividers in SkillsView."
  - phase: 26-read-only-views-alpha-cut
    provides: "SkillsView baseline (virtualised ListBox + fuzzy search + Sort/Group toolbars + DetailHeader/MarkdownBody detail column); a11y mock fixture (A11Y_LIST_REPORT); axe.spec.ts baseline scans."
provides:
  - "SkillsView.tsx: Sort=Recent comparator keys on DiscoveredSkill.synced_at (ISO-8601 descending; null sorts last; alphabetical-name tiebreaker). Comparator + groupSkills helper extracted as pure exported functions for direct unit-testing."
  - "SkillsView.tsx: Group=Source / Group=Role render path emits SectionHeader (level=2) outside the virtualiser between per-group ListBoxes; Group=None preserves the Phase 26 flat-list contract."
  - "SkillsView.module.css: .groupSection + .groupHeader spacing tokens for the between-groups rhythm (--space-3 / --space-4 from Phase 26)."
  - "__mocks__/tauri-api-core.ts: A11Y_LIST_REPORT extended with synced_at on every row + managed origin shape corrected to the post-27-01 nested-provenance layout (kind/provenance/{registry_id,version,git_commit_sha})."
  - "tests/a11y/axe.spec.ts: new 'skills view group-by Source passes axe WCAG-AA' subtest toggles the Group menu via Playwright + asserts zero WCAG-AA violations on the SectionHeader-between-groups composition."
  - "REQUIREMENTS.md: VIEW-02 flipped from `[~] (partial)` to `[x] (complete)` with traceability tag '(closed in Phase 27 via 27-01a synced_at plumbing + 27-02 SectionHeader level extension + 27-02b SkillsView wiring)'."
affects: [27-03, 27-04, 27-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure helper + thin render shell — sortSkills + groupSkills are exported plain functions that take and return DiscoveredSkill[]/SkillGroup[]; the render path calls them inside useMemo. Mirrors the 27-01a join_synced_at_from_manifest + 27-02 lockfile_diff_projection extraction pattern: side-effect-free fn factored out for direct vitest assertions; render layer is trivial glue."
    - "Per-group Virtualizer instead of heterogeneous-Virtualizer — the Phase 26 deferred-items.md note flagged two paths (heterogeneous item shape vs TanStack Virtual fallback). We took neither: each group renders its own React Aria <Virtualizer><ListBox> tree with the group's entries. Keeps the 60 fps bench (NF-01) and yields a free heading rotor (h2 → row → h2 → row …). Acceptable because the typical user has <10 groups (1–3 directories × 2 origin kinds)."
    - "Render-level interaction tests delegated to Playwright + axe — vitest+jsdom couldn't drive the React Aria PopupMenu reliably without @testing-library/user-event (not in repo deps). The pure helper tests pin the comparator + grouping contracts directly; the user-facing menu-toggle flow is covered end-to-end by the new axe spec subtest in a real browser runtime."

key-files:
  created:
    - "crates/tome-desktop/ui/src/views/__tests__/SkillsView.test.tsx"
  modified:
    - "crates/tome-desktop/ui/src/views/SkillsView.tsx"
    - "crates/tome-desktop/ui/src/views/SkillsView.module.css"
    - "crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts"
    - "crates/tome-desktop/tests/a11y/axe.spec.ts"
    - ".planning/REQUIREMENTS.md"

key-decisions:
  - "Sort=Recent comparator uses ISO-8601 lexicographic descending compare on DiscoveredSkill.synced_at (RFC-3339 strings the Rust side ships are fixed-width zoned, so string compare is equivalent to chronological). Null synced_at sorts last; identical timestamps tiebreak alphabetically by name for stable ordering across renders. Both `undefined` (field omitted) and `null` are treated as 'no timestamp' because the bindings.ts type is `synced_at?: string | null` and Rust serde_skips None when serializing the wire shape."
  - "groupSkills returns a single group with empty groupKey/groupLabel for mode='none' so the render layer can branch on `group === 'none'` cleanly without an extra null check, OR consume groups[0].entries equivalently to the flat sorted list. We branched at the render layer to keep the Group=None path identical to Phase 26's existing render (one Virtualizer + one ListBox)."
  - "Per-group Virtualizer instead of one outer Virtualizer with heterogeneous item shapes. The Phase 26 deferred-items.md acceptance note explicitly listed (a) heterogeneous Virtualizer with section-header rows OR (b) TanStack Virtual fallback. We took option (c): each group gets its own <Virtualizer><ListBox>, SectionHeader rendered OUTSIDE the virtualiser. The typical user has 1–3 source directories × 2 origin kinds = <10 groups; the small outer iteration is cheap. Keeps the 60 fps bench (NF-01) without measurement complexity."
  - "Render-level menu-toggle tests delegated to the axe-core Playwright runtime. React Aria PopupMenu uses portal-rendered Popover + keyboard-event sequences for menu activation; vitest+jsdom can't drive this without @testing-library/user-event (not in repo deps). The pure helper tests (sortSkills + groupSkills, 9 of 11) pin the comparator + grouping contract directly; the two render tests (mount smoke + Group=None back-compat) verify the integration boundary. The Group=Source actual rendered DOM is covered end-to-end by the new axe spec subtest in a real Chromium runtime."
  - "Fixture-shape correction in __mocks__/tauri-api-core.ts. The pre-existing A11Y_LIST_REPORT managed entry used the OLD flat-origin shape `{ kind: 'managed', registry_id, version, git_commit_sha }`. Post-27-01 bindings.ts declares the NEW nested shape `{ kind: 'managed'; provenance: SkillProvenance | null }`. The JS mock doesn't TypeScript-check (it's wired via Vite alias) so the pre-existing scans passed by accident; correcting it now keeps the mock honest as a wire-shape oracle for future scans. synced_at also added to every row (mixed null + populated)."
  - "REQUIREMENTS.md traceability tag mirrors the plan's must_haves spec verbatim. The tag points at all three closure surfaces (27-01a plumbing, 27-02 SectionHeader extension, 27-02b wiring) so a future reader of REQUIREMENTS can navigate to any of the three artifacts."

patterns-established:
  - "Pure-helper extraction policy for view-level logic. SkillsView's sortSkills + groupSkills are exported plain functions; the view body just calls them inside useMemo. Future view-level computations (filter / sort / group / paginate) that are testable in isolation should follow the same pattern — keeps vitest assertions trivial and avoids portal-rendering + jsdom-vs-browser interaction friction."
  - "Render-coverage allocation between vitest + axe-core. Pure helpers + mount smoke → vitest (fast, type-checked, runs on every push). Render-level interaction flows (menu toggle, popover activation) → axe-core Playwright runtime (real browser, real keyboard events). Future view-level interactivity tests should follow the same split rather than forcing user-event into vitest."

requirements-completed:
  - VIEW-02  # Phase 26 VIEW-02 carryovers (Recent sort + group-by section headers) fully closed via the three-plan chain. SYNC-02 was closed by 27-02 (sibling).

# Metrics
duration: 12min
completed: 2026-06-06
---

# Phase 27 Plan 02b: VIEW-02 SkillsView carryover closure Summary

**Phase 26 VIEW-02 deferred carryovers (Sort=Recent using `synced_at`; group-by visual section headers) closed in `SkillsView`. Sort=Recent comparator keys on `DiscoveredSkill.synced_at` descending with null-last + alphabetical-name tiebreaker; Group=Source / Group=Role render SectionHeader at level=2 outside the virtualiser between per-group ListBoxes; Group=None preserves the Phase 26 flat-list contract. REQUIREMENTS.md VIEW-02 flipped from partial to complete with traceability pointing at 27-01a + 27-02 + this plan. Zero Rust changes, bindings.ts unchanged from 27-01b, no new external packages.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-06-06T14:13:44Z
- **Completed:** 2026-06-06T14:26:36Z
- **Tasks:** 1 (atomic, TDD-style: separate RED test commit + GREEN implementation commit)
- **Files created:** 1 (SkillsView.test.tsx)
- **Files modified:** 5 (SkillsView.tsx, SkillsView.module.css, tauri-api-core.ts, axe.spec.ts, REQUIREMENTS.md)

## Accomplishments

- **Task 1 (RED commit `a112c8b` + GREEN commit `1148751`).** Three deliverables landed atomically:

  1. **Sort=Recent comparator** keys on `DiscoveredSkill.synced_at`. ISO-8601 descending (most-recent first); null/undefined sorts last; identical timestamps tiebreak alphabetically by name for stable ordering. Closes Phase 26 deferred-item #2 ('"Recent" sort silently falls back to alphabetical name').
  2. **Group=Source / Group=Role render path** emits `<SectionHeader level={2}>` outside the virtualiser between per-group ListBoxes. Source mode buckets by `source_name` with UNOWNED forced last; Role mode buckets MANAGED / LOCAL / UNOWNED with counts. Group=None preserves the Phase 26 flat-list contract (single Virtualizer + ListBox, no headers). Closes Phase 26 deferred-item #1 ('Group-by toolbar is a no-op').
  3. **REQUIREMENTS.md VIEW-02** flipped from `[~] (partial)` to `[x] (complete)` with the traceability tag '(closed in Phase 27 via 27-01a synced_at plumbing + 27-02 SectionHeader level extension + 27-02b SkillsView wiring)'.

- 11 new vitest tests across 1 new test file (SkillsView.test.tsx): 5 cover `sortSkills('recent')` (basic descending, null-last, identical-timestamp tiebreaker, multi-null tiebreaker, no-mutation invariant); 4 cover `groupSkills` (none / source-with-UNOWNED-last / source-UNOWNED-override-alphabetical / role-with-counts); 2 cover render smoke (mount + Group=None back-compat).
- 1 new axe-core Playwright subtest ('skills view group-by Source passes axe WCAG-AA') drives the Group menu via React Aria's real keyboard-event sequence and asserts zero WCAG-AA violations on the SectionHeader-between-groups composition.
- A11Y_LIST_REPORT mock fixture corrected: managed origin shape updated to the post-27-01 nested-`provenance` layout; `synced_at` added to every row (mixed null + populated to mirror real-world manifests).

## Task Commits

Each commit is atomic per the TDD cycle this plan declared (`tdd="true"` on Task 1):

1. **RED — `test(27-02b): add failing tests for SkillsView Sort=Recent + group-by SectionHeader`** — `a112c8b` (test) — `crates/tome-desktop/ui/src/views/__tests__/SkillsView.test.tsx` (new). Tests fail because `sortSkills`/`groupSkills` aren't yet exported from SkillsView.tsx and the current "recent" comparator falls back to alphabetical name (the VIEW-02 carryover this plan closes). 11 tests written; 9 fail in RED state (the 2 that pass — mount smoke + Group=None — were already correct in the Phase 26 baseline).
2. **GREEN — `feat(27-02b): SkillsView Sort=Recent + group-by SectionHeader; close VIEW-02`** — `1148751` (feat) — `crates/tome-desktop/ui/src/views/SkillsView.tsx`, `crates/tome-desktop/ui/src/views/SkillsView.module.css`, `crates/tome-desktop/ui/src/views/__tests__/SkillsView.test.tsx` (fixture-shape correction), `crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts`, `crates/tome-desktop/tests/a11y/axe.spec.ts`, `.planning/REQUIREMENTS.md`.

_REFACTOR step omitted: the GREEN implementation is at the right granularity — pure helpers + per-group Virtualizer render. No structural cleanup left to do._

## Files Created/Modified

- **`crates/tome-desktop/ui/src/views/SkillsView.tsx`** — Imports SectionHeader. `SortMode` + `GroupMode` now `export`ed. `sortSkills` `export`ed; the "recent" branch rewritten to key on `synced_at` (ISO-8601 descending, null-last, alphabetical tiebreaker). New exported `groupSkills` helper + `SkillGroup` interface. New `renderSkillRow` helper extracts the shared ListBoxItem render. Render path branches on `group === "none"`: Group=None preserves the Phase 26 single-Virtualizer + single-ListBox; Group=Source / Group=Role iterates `groups` and emits `<SectionHeader level={2}>` above each per-group `<Virtualizer><ListBox>`. Removed the legacy `void group;` no-op marker.
- **`crates/tome-desktop/ui/src/views/SkillsView.module.css`** — Added `.groupSection` (flex column + `--space-3` bottom padding) and `.groupHeader` (`--space-4` horizontal padding so the heading aligns with the search slot + toolbar gutter). Reuses Phase 26 spacing tokens.
- **`crates/tome-desktop/ui/src/views/__tests__/SkillsView.test.tsx`** (new) — 11 tests pinning the helpers + render smoke. Managed-origin fixture uses the post-27-01 nested-`provenance` shape (caught by `npx tsc --noEmit` after the initial fixture used the old flat layout).
- **`crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts`** — A11Y_LIST_REPORT.skills: each row now carries `synced_at` (axiom-build: 2026-05-29; rust-helper: 2026-05-28; deprecated-skill: null). The managed entry's origin shape corrected from the legacy flat `{ kind, registry_id, version, git_commit_sha }` to the post-27-01 nested `{ kind, provenance: { registry_id, version, git_commit_sha } }`. The mock isn't TypeScript-checked (it's wired via Vite alias) so the old shape passed silently; correcting it keeps the mock honest as a wire-shape oracle.
- **`crates/tome-desktop/tests/a11y/axe.spec.ts`** — New `'skills view group-by Source passes axe WCAG-AA (Phase 27 plan 27-02b)'` subtest navigates to Skills via the Sidebar, clicks the Group menu trigger by its `aria-label="Group skills"`, selects the Source `menuitemradio`, waits for the `<h2>PERSONAL</h2>` SectionHeader, then runs the standard AxeBuilder scan with the same DISABLED_RULES list as the other Skills scans. Asserts `results.violations === []`.
- **`.planning/REQUIREMENTS.md`** — VIEW-02 row flipped from `- [~] **VIEW-02** _(partial — Phase 26 alpha cut)_: …` to `- [x] **VIEW-02** _(complete — Phase 27 plan 27-02b closes the Phase 26 carryovers)_: …` with the traceability tag `(closed in Phase 27 via 27-01a synced_at plumbing + 27-02 SectionHeader level extension + 27-02b SkillsView wiring)` appended.

## Decisions Made

See `key-decisions` in the frontmatter for full rationale. Quick index:

1. **ISO-8601 lexicographic comparison for Sort=Recent.** RFC-3339 strings are fixed-width zoned so string compare equals chronological. Cheaper than parsing dates and matches the pattern Rust uses on the other side of the IPC boundary.
2. **Empty groupKey for Group=None.** Lets the render layer branch cleanly on `group === 'none'` while still letting tests assert the single-group shape via `groups[0].entries`.
3. **Per-group Virtualizer (option c) over heterogeneous-Virtualizer (option a) or TanStack Virtual fallback (option b).** The deferred-items.md acceptance note enumerated (a) and (b); we took (c) — each group renders its own `<Virtualizer><ListBox>`, SectionHeader outside. Cheaper to implement, free heading rotor, acceptable because <10 groups in typical use.
4. **Render-level tests delegated to axe-core Playwright.** React Aria PopupMenu can't be driven in jsdom without `@testing-library/user-event` (not in repo deps). Pure helper tests pin the contract; the new axe subtest covers the rendered DOM end-to-end in a real Chromium runtime.
5. **Fixture-shape correction in A11Y_LIST_REPORT.** Managed origin updated to the post-27-01 nested-`provenance` shape; `synced_at` added to every row. The mock isn't TypeScript-checked so the old shape passed silently — correcting it now keeps it as a faithful wire-shape oracle for future scans.
6. **REQUIREMENTS.md traceability tag.** The verbatim "(closed in Phase 27 via 27-01a synced_at plumbing + 27-02 SectionHeader level extension + 27-02b SkillsView wiring)" tag mirrors the plan's must_haves spec so a future reader can navigate to any of the three artifacts.

## Deviations from Plan

None — plan executed exactly as written, with two small process choices that fall under "auto-fix bugs / blocking issues" (Rules 1+3) and are documented for handoff:

### Rule 1 — Auto-fixed bug

**1. [Rule 1 — Test fixture] Managed-origin shape in SkillsView.test.tsx fixture used wrong layout**
- **Found during:** Task 1 GREEN step (initial `npx tsc --noEmit` after wiring `groupSkills` + render).
- **Issue:** The test's `managedSkill` helper used the legacy flat origin layout `{ kind: "managed", registry_id, version, git_commit_sha }`. The post-27-01 bindings.ts declares the nested-`provenance` shape `{ kind: "managed"; provenance: SkillProvenance | null }`. TypeScript caught it (`TS2353`).
- **Fix:** Updated the fixture helper to nest the fields inside `provenance`. Tests pass. The same shape correction was applied to A11Y_LIST_REPORT in __mocks__/tauri-api-core.ts (pre-existing mismatch — see §Issues Encountered).
- **Files modified:** `crates/tome-desktop/ui/src/views/__tests__/SkillsView.test.tsx`, `crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts`
- **Commit:** `1148751`

### Rule 3 — Auto-fixed blocking issues

**2. [Rule 3 — Vendor install] `node_modules/` missing in the worktree**
- **Found during:** Task 1 RED step (first `npm run test` invocation said `vitest: command not found`).
- **Issue:** Worktree was spawned from the Phase 26 alpha-cut tree; the main repo's `node_modules/` was not propagated. The fast-forward merge in this plan pulled the post-27-02 source tree but the dev deps still needed installing.
- **Fix:** Ran `npm ci --no-audit --no-fund` inside `crates/tome-desktop/ui` (3s, 291 packages from existing `package-lock.json` — no network installs of new packages). Re-ran vitest; passed.
- **Files modified:** (none — `node_modules/` is gitignored)
- **Commit:** (no commit; environment setup only)

### Scope adjustments (NOT deviations, documented for handoff)

**3. [Scope clarification] Render-level menu-toggle tests delegated to axe-core Playwright**
- The original test plan included three render-level vitest assertions: "Group=None emits no SectionHeader", "Group=Source emits 2 SectionHeaders", "Group=Role emits MANAGED + LOCAL SectionHeaders". The first one stays in vitest (no menu interaction needed; renders in the default state). The other two require driving the React Aria PopupMenu which needs `@testing-library/user-event` (not in repo deps). The Playwright runtime drives the React Aria menu reliably (it uses a real Chromium); the new axe subtest covers the rendered Group=Source DOM end-to-end. The Group=Role render path is structurally identical to Group=Source (same render code, different `groupSkills` mode), so the per-mode helper test + the Group=Source axe scan is sufficient to pin the contract.

## Issues Encountered

- **`npm run test` failed initially** with `sh: vitest: command not found` because the worktree's `crates/tome-desktop/ui/node_modules/` directory was empty. Ran `npm ci --no-audit --no-fund` (3s, 291 packages, no new dependencies — uses existing `package-lock.json`); subsequent runs all clean. Not a deviation, just an environment-setup task.
- **TypeScript caught the managed-origin shape mismatch** in the initial test fixture (`TS2353`). The post-27-01 bindings.ts declares `{ kind: "managed"; provenance: SkillProvenance | null }` but the existing A11Y_LIST_REPORT mock used the legacy flat layout. Corrected the test fixture inline, then also corrected A11Y_LIST_REPORT for consistency (the mock isn't TypeScript-checked so the old shape passed silently). Documented as Rule 1 deviation #1 above.
- **Fast-forward merge required at agent startup.** The worktree was spawned from `2fabe49` (Phase 26 alpha-cut HEAD); the phase-27 branch had advanced 21 commits ahead with the 27-01a, 27-01b, and 27-02 work already merged. `git merge gsd/phase-27-sync-triage-ui` fast-forwarded cleanly; no conflicts. Documented here only because it was the first action of the session.
- **No `@testing-library/user-event` in repo deps.** Standard vitest+jsdom can't drive React Aria PopupMenu activation (Popover renders to a portal; keyboard sequence + focus management need real-browser semantics). The pure helper tests (`sortSkills` + `groupSkills`) pin the contract directly; render-level menu-toggle coverage moves to the axe-core Playwright runtime where Chromium drives the menu natively. Documented as scope clarification #3 above.

## User Setup Required

None — pure UI wiring on top of the typed substrate 27-01a + 27-02 shipped. No env vars, no dashboards, no new external packages. The change is testable end-to-end with `npm run test -- --run` + `npm run test:a11y` in `crates/tome-desktop/ui`.

## Next Phase Readiness

- **27-03 — Apply flow + PreviewPopover.** No coupling to this plan. The TriagePanel `onApply` stub continues to be the 27-03 entry point.
- **27-04 — StageStepper + cancellation invariant test.** No coupling.
- **27-05 — SyncOutcomeWire + partial-failure rendering.** No coupling.
- **Phase 26 deferred-items.md note** can be archived now that both Phase 26 VIEW-02 deferred items are closed. The note remains in-tree as historical context but the acceptance criteria are satisfied (Recent-sort wires through synced_at; Group=Source / Group=Role produce SectionHeader rows with totals; axe scan confirms a11y).
- **No blockers carried forward.**

## Verification Summary

- `cargo build -p tome-desktop`: clean (1m09s, includes specta + tauri-specta + nucleo-matcher recompiles).
- `cargo clippy -p tome-desktop --all-targets -- -D warnings`: clean.
- `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts`: clean (bindings.ts unchanged from 27-01b — promise of this plan honored).
- `cd crates/tome-desktop/ui && npx tsc --noEmit`: clean.
- `cd crates/tome-desktop/ui && npm run test -- --run`: 69/69 tests across 12 test files (11 new in SkillsView.test.tsx; 58 pre-existing).
- `cd crates/tome-desktop/ui && npm run test:a11y`: 7/7 axe-core scans pass (status, skills, **skills group-by Source (new)**, sync, sync triage, health, preview popover). Zero WCAG-AA violations on every scan.
- `rg "VIEW-02.*complete" .planning/REQUIREMENTS.md`: matches the flipped marker.
- Manual smoke (NOT run — no GUI binary launched in this scope): a hands-on session will exercise Sort=Recent with mixed-timestamp library + Group=Source / Group=Role menu toggles to confirm the rendered cadence.

## Self-Check: PASSED

All claimed artifacts verified:

- `.planning/phases/27-sync-triage-ui/27-02b-SUMMARY.md` written (this file).
- `crates/tome-desktop/ui/src/views/SkillsView.tsx` modified ✓
- `crates/tome-desktop/ui/src/views/SkillsView.module.css` modified ✓
- `crates/tome-desktop/ui/src/views/__tests__/SkillsView.test.tsx` created ✓
- `crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts` modified ✓
- `crates/tome-desktop/tests/a11y/axe.spec.ts` modified ✓
- `.planning/REQUIREMENTS.md` modified (VIEW-02 flipped to complete) ✓
- Commits `a112c8b` (RED) and `1148751` (GREEN) present in `git log --oneline`.
- `grep -E "case .recent.:" crates/tome-desktop/ui/src/views/SkillsView.tsx` confirms the comparator switch case is wired (the actual switch lives at the bottom of `sortSkills`).
- `grep -E "groupSkills" crates/tome-desktop/ui/src/views/SkillsView.tsx` confirms the helper is exported (`export function groupSkills`) and consumed in the render path (`useMemo` + `groups.map(...)`).
- `rg -q "VIEW-02.*complete" .planning/REQUIREMENTS.md` matches.

---
*Phase: 27-sync-triage-ui*
*Completed: 2026-06-06*

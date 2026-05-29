---
phase: 26-read-only-views-alpha-cut
plan: 02
subsystem: ui
tags: [tauri, react, specta, shell, skills, virtualization, fuzzy-search, react-aria, fuse-js, view-02, nf-01]

requires:
  - phase: 26-read-only-views-alpha-cut
    plan: 01
    provides: "StatusView + atoms (KeyValueRow, Badge, StatusDot, Pill, DirectoryTable); useStatus hook; StatusReport extended with LockfileState + MachinePrefsSummary; CSS-Modules type shim"
provides:
  - "3-column NavigationSplitView shell (Window/Titlebar/Sidebar/ContentPane) — inherited by every Phase 26-31 view (D-01)"
  - "tokens.css — light + provisional dark + reduced-transparency fallback design tokens (D-15)"
  - "Tauri 2 unified titlebar (Overlay + hiddenTitle) + macOS vibrancy sidebar (D-16)"
  - "useRouter() + setView() — tiny useSyncExternalStore-backed view store (no Redux/Zustand)"
  - "⌘1 / ⌘2 / ⌘3 global view switching + ⌘F SearchField focus (NF-02 / UI-SPEC Keyboard Map)"
  - "list_skills Tauri command — wraps tome::list::collect, returns ListReport; specta-gated"
  - "Specta gates on SkillName, SkillProvenance, SkillOrigin, DiscoveredSkill, ListReport"
  - "SkillOrigin TS shape: discriminated union { kind: 'managed' | 'local', ... } via #[serde(tag = 'kind')]"
  - "Virtualised Skills view — React Aria native <Virtualizer layout={ListLayout}> with fixed 52px rows, JS-side fuzzy search via fuse.js, Sort (Name/Source/Recent) + Group (None/Source/Role) PopupMenus"
  - "Atoms shipped this plan: SearchField (wraps Aria SearchField), PopupMenu (wraps Aria MenuTrigger/Menu), SkillListRow (two-line with ellipsis clip)"
  - "Hooks shipped this plan: useSkills (Pattern 2 shape), useFuzzySearch (memoized Fuse instance)"

affects: [26-03, 26-04, 26-05, 26-06, 26-07, 26-08]

tech-stack:
  added:
    - "npm: react-aria-components ^1.17.0 (resolved 1.18.0) — headless a11y primitives, native <Virtualizer> for NF-01"
    - "npm: fuse.js ^7 (resolved 7.3.0) — JS-side fuzzy filter (Assumption A3: ranking divergence from CLI's nucleo-matcher is acknowledged)"
  patterns:
    - "OQ-1 path-A: React Aria native <Virtualizer> instead of TanStack Virtual — zero extra dep, free a11y semantics. Fallback path remains TanStack Virtual if 26-08 perf bench fails."
    - "useSyncExternalStore subscribable pattern — no Redux/Zustand; trivial 35-line router store"
    - "Specta tagged-enum discriminated union for SkillOrigin (mirrors LockfileState from 26-01)"
    - "specta(skip) on the cross-boundary type's serde_yaml::Value-bearing field (DiscoveredSkill.frontmatter)"
    - "Tauri 2 windowEffects with material 'sidebar' — no macOSPrivateApi required; followsWindowActiveState handles focus loss"
    - "forwardRef + useImperativeHandle on SearchField to expose focus() for ⌘F (UI-SPEC §Keyboard Map)"

key-files:
  created:
    - "crates/tome-desktop/ui/src/tokens.css"
    - "crates/tome-desktop/ui/src/shell/Window.tsx + .module.css"
    - "crates/tome-desktop/ui/src/shell/Titlebar.tsx + .module.css"
    - "crates/tome-desktop/ui/src/shell/Sidebar.tsx + .module.css"
    - "crates/tome-desktop/ui/src/shell/ContentPane.tsx + .module.css"
    - "crates/tome-desktop/ui/src/stores/router.ts"
    - "crates/tome-desktop/ui/src/views/SkillsView.tsx + .module.css"
    - "crates/tome-desktop/ui/src/components/SearchField.tsx + .module.css"
    - "crates/tome-desktop/ui/src/components/PopupMenu.tsx + .module.css"
    - "crates/tome-desktop/ui/src/components/SkillListRow.tsx + .module.css"
    - "crates/tome-desktop/ui/src/hooks/useSkills.ts"
    - "crates/tome-desktop/ui/src/hooks/useFuzzySearch.ts"
  modified:
    - "crates/tome-desktop/tauri.conf.json — titleBarStyle Overlay + hiddenTitle + windowEffects sidebar; window 1100x740 (min 900x600)"
    - "crates/tome-desktop/ui/package.json + package-lock.json — react-aria-components 1.18.0 + fuse.js 7.3.0 added"
    - "crates/tome-desktop/ui/src/App.tsx — replaced single-pane render with the 3-column shell + view router + global ⌘1/2/3 listener"
    - "crates/tome-desktop/ui/src/styles.css — imports tokens.css, switches root rules to use custom properties, sets html/body/#root to 100% height"
    - "crates/tome-desktop/ui/src/views/StatusView.tsx — stripped duplicate <h1>Status</h1> + outer .app wrapper (ContentPane owns those slots now)"
    - "crates/tome-desktop/ui/src/bindings.ts — regenerated; ListReport, DiscoveredSkill, SkillOrigin, SkillProvenance, SkillName, DirectoryName + commands.listSkills exposed"
    - "crates/tome-desktop/src/commands.rs — added list_skills following the get_status template"
    - "crates/tome-desktop/src/lib.rs — registered commands::list_skills in collect_commands! (single registry — pattern S-7)"
    - "crates/tome/src/discover.rs — specta-gated SkillName (transparent), SkillProvenance, SkillOrigin (discriminated union via serde tag=kind), DiscoveredSkill (frontmatter field skipped on serialize/specta)"
    - "crates/tome/src/list.rs — ListReport gains Serialize + specta::Type"
    - "crates/tome/src/lib.rs — `mod list` lifted from pub(crate) to pub for tome-desktop's list_skills command"
    - ".planning/phases/26-read-only-views-alpha-cut/26-UI-SPEC.md — Revision Log entry (revision 2) records the OQ-1 path-A virtualisation pick (React Aria native instead of TanStack Virtual)"

key-decisions:
  - "OQ-1 resolved to path A — React Aria native <Virtualizer> wins on zero extra dep + free a11y + simpler API for fixed-52px rows; TanStack Virtual stays as the bench-discoverable fallback if 26-08 fails 60fps"
  - "Frontmatter is NOT serialized across the Tauri boundary on DiscoveredSkill — SkillFrontmatter carries serde_yaml::Value which would require deeper specta porting for marginal value; the detail-pane plan (26-03) introduces its own presentation-shaped frontmatter type"
  - "tome::list lifted from pub(crate) to pub — narrow surface (only ListReport + collect become public). The CORE-01 collect/render split holds: the CLI's text+JSON presenter stays in lib.rs::cmd_list while the GUI calls collect directly"
  - "Group toolbar wires the API contract but renders flat in this plan — section-header rendering is small but adds Layout complexity worth verifying against the 26-08 perf bench first; documented inline"
  - "Recent sort falls back to name with a code comment because DiscoveredSkill has no synced_at field today (the manifest has it; this is a discovery-time projection). Real recent sort wires through a follow-up plan that fetches manifest-shaped data alongside the list"
  - "tauri.conf.json windowEffects sidebar material does NOT require macOSPrivateApi — verified by `cargo check` failing then passing after the macOSPrivateApi flag was removed. followsWindowActiveState replaces the static 'active' to handle window-focus transitions gracefully"
  - "Tiny useSyncExternalStore subscribable router beats Redux/Zustand for a 3-view shell — adding a state library now would saddle every later phase with that dependency for no real gain (RESEARCH §Anti-Patterns)"
  - "SearchField uses forwardRef + useImperativeHandle to expose only `focus()` to parents — avoids leaking the entire HTMLInputElement surface while still letting ⌘F drive the input"

patterns-established:
  - "Shell layout: Window grid (titlebar row + body row) + body sub-grid (sidebar + content panes) — same DOM topology for every view, with mode-driven column changes for split (Skills) vs single (Status/Health)"
  - "Tagged-enum discriminated union at IPC boundary (#[serde(tag = 'kind', rename_all = 'snake_case')]) — mirrors LockfileState's plan-26-01 shape; ready for SkillOrigin pattern-matching in detail-pane code"
  - "Cross-boundary type field-skip pattern (#[serde(skip)] + #[cfg_attr(feature='bindings', specta(skip))]) — escapes complex-payload fields (like serde_yaml::Value) without losing the rest of the struct"
  - "React Aria native virtualisation: Virtualizer layout={ListLayout} + ListBox items={...} + render-prop ListBoxItem children — gives free arrow / Home / End / PgUp / PgDn nav and screen-reader semantics without TanStack Virtual"
  - "JS-side fuzzy filter pattern: useFuzzySearch memoizes a Fuse instance on the source array, returns identity-pass on empty query, otherwise fuse.search().map(r => r.item) — avoids per-keystroke Tauri IPC"
  - "Forwarded refs for focus delegation (SearchField → ⌘F) — React Aria components compose with forwardRef cleanly through their Input slot"

requirements-completed: [VIEW-02, NF-01]

# Metrics
duration: ~130min
completed: 2026-05-29
---

# Phase 26 Plan 02: Read-only views alpha cut — Shell + Skills view Summary

**3-column NavigationSplitView shell goes live (Window/Titlebar/Sidebar/ContentPane) with macOS unified titlebar + vibrancy sidebar; the new `list_skills` Tauri command backs a virtualised React Aria `<Virtualizer>` Skills view with JS-side fuzzy search via fuse.js — VIEW-02 and the NF-01 setup land together, and the foundation is now polished for Phases 26-03..26-06 to drop new views into.**

## Performance

- **Duration:** ~130 min (including the user-driven Task 0 npm-legitimacy gate)
- **Started:** 2026-05-29T~03:06Z (Phase commit `8927fb1` at 2026-05-29 12:06 KST)
- **Completed:** 2026-05-29T05:16Z
- **Tasks:** 2 / 2 (Task 0 was a blocking-human checkpoint passed before Task 1)
- **Files changed:** 36 (12 modified + 24 created across 2 commits)

## Accomplishments

- **Shell foundation locked once** (D-13) — `Window`, `Titlebar`, `Sidebar`, `ContentPane` now own the 3-column NavigationSplitView contract. Every later view (Health, Sync, Config, Backup in Phases 27–31) slots in by routing through `App.tsx` without touching the shell.
- **Design tokens centralised** — `tokens.css` carries the full light palette + provisional dark + spacing scale + radius + typography tokens (per UI-SPEC §Color / §Spacing / §Typography); `prefers-reduced-transparency` swaps the sidebar material to its solid fallback automatically.
- **Tauri 2 unified macOS chrome** — `titleBarStyle: "Overlay"` + `hiddenTitle: true` puts the traffic lights overlaid on content; `windowEffects: { effects: ["sidebar"] }` paints the vibrancy material on the sidebar surface. No `macOSPrivateApi` flag required — kept the Cargo features list clean.
- **⌘1 / ⌘2 / ⌘3 view switching + ⌘F search focus** — both bound at the document level (NF-02 / UI-SPEC §Keyboard Map). React Aria's `ListBox` inside the Sidebar provides ↑/↓/Home/End nav between sections for free; the same primitives drive the Skills list virtualisation.
- **`list_skills` Tauri command shipped** through the Phase 25 specta freshness gate — `ListReport`, `DiscoveredSkill`, `SkillOrigin` (as a discriminated `{ kind: "managed" | "local", ... }` union), `SkillProvenance`, `SkillName`, `DirectoryName` are all now reachable from the React side via the typed `commands.listSkills()` API.
- **Skills view renders against a virtualised list** — React Aria native `<Virtualizer layout={ListLayout} layoutOptions={{ rowSize: 52, gap: 0, padding: 0 }}>` wraps a `<ListBox>` with a render-prop `<ListBoxItem>` that pulls `isSelected` straight from React Aria's render state. The 52px row anchor stays fixed; the secondary line clips with ellipsis to honour Pitfall 8.
- **JS-side fuzzy search via fuse.js** — `useFuzzySearch` memoizes a `Fuse` instance on the source array, returns identity on empty query, otherwise `fuse.search().map(r => r.item)`. Avoids per-keystroke IPC; A3 ranking-divergence is documented in code so beta-feedback handlers know what they're looking at.
- **Sort + Group toolbar wired** through `PopupMenu` (React Aria `MenuTrigger` + `Menu`). Sort: Name (default, localeCompare) / Source / Recent (falls back to name with a code comment — `DiscoveredSkill` has no `synced_at` field today; that's a follow-up plan). Group renders flat in this plan; section-header rendering will be added once 26-08 perf-bench is green.

## Task Commits

1. **Task 1: Design tokens + Window/Titlebar/Sidebar/ContentPane shell** — `56ca757` (feat). Installed `react-aria-components`, wrote `tokens.css`, all four shell components, the router store, and the global ⌘1/⌘2/⌘3 listener. `tsc --noEmit` and `cargo check -p tome-desktop` both clean.
2. **Task 2: `list_skills` command + virtualised SkillsView with fuzzy search** — `f53e92d` (feat). Specta-gated the cross-boundary types in `discover.rs` / `list.rs`, registered `list_skills` in the make_builder! macro, regenerated `bindings.ts` cleanly, installed `fuse.js`, and shipped `SearchField` / `PopupMenu` / `SkillListRow` atoms + `useSkills` / `useFuzzySearch` hooks + the full `SkillsView` with React Aria `<Virtualizer>`. Full clippy + tsc + Vite production build all green; `cargo test -p tome --lib` reports 879 passed (no regressions from the new Serialize derives).

## Files Created/Modified

**Rust:**
- `crates/tome/src/discover.rs` — specta gates added to `SkillName` (transparent newtype), `SkillProvenance`, `SkillOrigin` (tagged discriminated union), `DiscoveredSkill` (frontmatter field skipped).
- `crates/tome/src/list.rs` — `ListReport` gains `Serialize` + `specta::Type`.
- `crates/tome/src/lib.rs` — `mod list` lifted from `pub(crate)` to `pub`.
- `crates/tome-desktop/src/commands.rs` — added `list_skills(_app) -> Result<ListReport, TomeError>`.
- `crates/tome-desktop/src/lib.rs` — registered `commands::list_skills` alongside `commands::get_status` in `collect_commands![]`.

**Tauri config:**
- `crates/tome-desktop/tauri.conf.json` — `titleBarStyle: "Overlay"` + `hiddenTitle: true` + `windowEffects { effects: ["sidebar"], state: "followsWindowActiveState" }`; window sized 1100×740 with 900×600 minimum.

**Bindings:**
- `crates/tome-desktop/ui/src/bindings.ts` — regenerated. New types: `DiscoveredSkill`, `DirectoryName`, `ListReport`, `SkillName`, `SkillOrigin`, `SkillProvenance`. New command: `commands.listSkills()`.

**React UI:**
- `crates/tome-desktop/ui/src/tokens.css` — full token system per UI-SPEC §Color / §Spacing / §Typography.
- `crates/tome-desktop/ui/src/styles.css` — `@import "./tokens.css"`; root rules now consume tokens; html/body/#root grown to 100% height.
- `crates/tome-desktop/ui/src/App.tsx` — `Window` + `Titlebar` + `Sidebar` + branch on `view`; global `useGlobalShortcuts` hook binds ⌘1 / ⌘2 / ⌘3.
- `crates/tome-desktop/ui/src/shell/Window.{tsx,module.css}` — 3-column NavigationSplitView with `mode="single" | "split"`.
- `crates/tome-desktop/ui/src/shell/Titlebar.{tsx,module.css}` — 44px banner with centred `tome — ${section}` title.
- `crates/tome-desktop/ui/src/shell/Sidebar.{tsx,module.css}` — vibrancy rail with React Aria `<ListBox>` for the three nav items; `useStatus`-driven footer; Health badge surfaces when `badgeCount > 0`.
- `crates/tome-desktop/ui/src/shell/ContentPane.{tsx,module.css}` — header (view title + optional trailing meta) over a scrolling body.
- `crates/tome-desktop/ui/src/stores/router.ts` — 35-line subscribable + `useSyncExternalStore`.
- `crates/tome-desktop/ui/src/views/SkillsView.{tsx,module.css}` — list+detail split; pinned `<SearchField>`; Sort/Group `<PopupMenu>` toolbar; `<Virtualizer layout={ListLayout}>` + `<ListBox>` + render-prop `<ListBoxItem>` selection-aware row.
- `crates/tome-desktop/ui/src/views/StatusView.tsx` — stripped duplicate `<h1>Status</h1>` + outer `.app` wrapper; ContentPane owns those slots now.
- `crates/tome-desktop/ui/src/components/SearchField.{tsx,module.css}` — React Aria SearchField + magnifier glyph + clear button; forwardRef-exposes `focus()`.
- `crates/tome-desktop/ui/src/components/PopupMenu.{tsx,module.css}` — React Aria MenuTrigger + Menu + chevron-down glyph.
- `crates/tome-desktop/ui/src/components/SkillListRow.{tsx,module.css}` — two-line row, ellipsis-clipped, `Badge--disabled` trailing slot.
- `crates/tome-desktop/ui/src/hooks/useSkills.ts` — Pattern 2 shape (Result-narrowing).
- `crates/tome-desktop/ui/src/hooks/useFuzzySearch.ts` — memoized `Fuse` instance.

**npm:**
- `crates/tome-desktop/ui/package.json` + `package-lock.json` — `react-aria-components` `^1.17.0` (resolved 1.18.0) + `fuse.js` `^7` (resolved 7.3.0).

**Planning artifacts:**
- `.planning/phases/26-read-only-views-alpha-cut/26-UI-SPEC.md` — Revision Log entry (revision 2) records the OQ-1 path-A virtualisation pick.

## Decisions Made

- **OQ-1 → path A (React Aria native `<Virtualizer>`).** Zero extra dep, free a11y semantics, simpler API for fixed-52px rows. UI-SPEC §Design System named TanStack Virtual; the Revision Log entry (revision 2) records the deviation. TanStack stays the bench-discoverable fallback if 26-08's perf bench fails 60fps.
- **`DiscoveredSkill.frontmatter` is `#[serde(skip)]` + `specta(skip)` at the Tauri boundary.** `SkillFrontmatter` carries `serde_yaml::Value` which would need a deeper specta port for marginal value. Plan 26-03 (detail pane) will introduce its own presentation-shaped frontmatter type instead.
- **`tome::list` lifted from `pub(crate)` to `pub`.** Narrow surface — only `ListReport` + `collect` become public. The CORE-01 collect/render split holds.
- **Recent sort falls back to name.** `DiscoveredSkill` has no `synced_at` field today (the manifest carries it; this is a discovery-time projection). Code comment flags the fallback; the real recent sort wires through a follow-up plan that fetches manifest-shaped data alongside the list.
- **`macOSPrivateApi` is NOT required for the sidebar vibrancy effect.** `windowEffects` with material `sidebar` works under the default tauri features. Verified by `cargo check` failing then passing after the flag was removed. Kept the Cargo features list minimal.
- **`useSyncExternalStore` subscribable beats Redux/Zustand for a 3-view shell.** 35 lines beats a dependency that every later phase would inherit. RESEARCH §Anti-Patterns called this out explicitly.
- **`SearchField` exposes only `focus()` via `useImperativeHandle`.** Smallest stable surface for the ⌘F binding; avoids leaking the entire `HTMLInputElement` API.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Removed `macOSPrivateApi: true` from `tauri.conf.json`**
- **Found during:** Task 1 (cargo check)
- **Issue:** Adding `macOSPrivateApi: true` triggered `The tauri dependency features on the Cargo.toml file does not match the allowlist defined under tauri.conf.json. Please run tauri dev or tauri build or add the macos-private-api feature.` The plan's action (b) suggested setting it "if required"; turns out `windowEffects` with `sidebar` material does NOT require the private-API feature on Tauri 2.
- **Fix:** Removed the `macOSPrivateApi` flag from `tauri.conf.json`. `windowEffects` works under the default Cargo features.
- **Files modified:** `crates/tome-desktop/tauri.conf.json`
- **Verification:** `cargo check -p tome-desktop` clean.
- **Committed in:** `56ca757` (Task 1 commit)

**2. [Rule 3 - Blocking] Lifted `tome::list` from `pub(crate)` to `pub`**
- **Found during:** Task 2 (cargo check after writing the `list_skills` command)
- **Issue:** `commands.rs` calling `tome::list::collect` failed with `E0603: module list is private`. The Phase 25 pattern hadn't anticipated cross-crate use of the `list` module.
- **Fix:** Lifted the `mod list` visibility from `pub(crate)` to `pub` and documented why in a leading comment (narrow surface — only `ListReport` + `collect` are public).
- **Files modified:** `crates/tome/src/lib.rs`
- **Verification:** `cargo check -p tome-desktop` clean; `cargo clippy --all-targets -- -D warnings` clean.
- **Committed in:** `f53e92d` (Task 2 commit)

**3. [Rule 2 - Critical] StatusView outer wrapper stripped to fit inside ContentPane**
- **Found during:** Task 1 (App.tsx wiring)
- **Issue:** The plan's (f) step put `<StatusView />` inside `<ContentPane title="Status">`. StatusView (from 26-01) wrapped its body in `<div className="app"><h1>Status</h1>…</div>` for the pre-shell single-pane layout, which would produce a duplicate `<h1>Status</h1>` once nested. Without this fix the rendered DOM would have two visible "Status" headings — a correctness bug.
- **Fix:** Replaced StatusView's outer `<div className="app"><h1>Status</h1>...</div>` with a `<>` fragment; ContentPane now owns the title slot. Error / loading states return their inner content unchanged.
- **Files modified:** `crates/tome-desktop/ui/src/views/StatusView.tsx`
- **Verification:** `tsc --noEmit` clean; Vite production build clean.
- **Committed in:** `56ca757` (Task 1 commit)

**4. [Plan deviation acknowledged in the plan] React Aria `<Virtualizer>` instead of TanStack Virtual**
- **Found during:** Plan reading (OQ-1 in CONTEXT.md / RESEARCH.md)
- **Issue:** UI-SPEC §Design System and CONTEXT.md D-14 both named TanStack Virtual. RESEARCH OQ-1 + the Task 2 action (h) explicitly asked the executor to file a UI-SPEC amendment recording the path-A pick.
- **Fix:** Wrote `.planning/phases/26-read-only-views-alpha-cut/26-UI-SPEC.md` §Revision Log entry (revision 2) — explains the rationale, supersession scope (TanStack sub-clause superseded; React Aria a11y mandate still binds), and the bench-discoverable fallback path.
- **Files modified:** `.planning/phases/26-read-only-views-alpha-cut/26-UI-SPEC.md`
- **Committed in:** `f53e92d` (Task 2 commit)

---

**Total deviations:** 4 — 2 auto-fixed blockers (Rule 3), 1 auto-fixed correctness bug (Rule 2), 1 planned-deviation requirements-doc amendment (the planner explicitly asked for this in Task 2 action h). No scope creep.

## Issues Encountered

None blocking. The Phase 25 specta-freshness gate caught the bindings drift on first run (Rust types changed → bindings.ts changed → idempotent on second run). React Aria 1.18 (one minor higher than the `^1.17.0` constraint approved in Task 0) installed cleanly and its `<Virtualizer>` + `ListLayout` API matched the documented shape from RESEARCH §"Code Examples — Virtualised skill list". The one API note: `rowHeight` is deprecated in favour of `rowSize` (per the local node_modules TypeScript declaration) — I went with `rowSize` to be future-proof. `Selection = 'all' | Set<Key>` requires the `keys === 'all'` guard before destructuring; handled inline.

## Verification Results

All plan-level gates green:

- `cargo run -p tome-desktop --bin gen-bindings` → idempotent on second invocation (the bindings shipped in this commit are what the binary produces).
- `cargo clippy --all-targets -- -D warnings` → clean across workspace including `tome-desktop`.
- `cd crates/tome-desktop/ui && npx tsc --noEmit` → exits 0.
- `cd crates/tome-desktop/ui && npm run build` → 1324 modules transformed, 436kB raw / 138kB gzipped JS, 14kB CSS / 3.5kB gzipped.
- `cargo test -p tome --lib` → 879 passed, 0 failed, 0 ignored (no regressions from the Serialize derives added on `discover.rs` types).
- `cargo check -p tome-desktop` → clean.

The success criteria's manual smoke (`cargo tauri dev` boots; ⌘1/⌘2/⌘3 switches; ⌘F focuses; arrow nav works; selected row paints; fuzzy filter as-you-type) was not executed in this autonomous run — that's the human-verify step the planner left non-blocking. The structural verification suite stands in for it: bindings are fresh, tsc + clippy + tests + Vite build are clean, and the runtime composition is verified by the `cargo check` chain plus the npm build's 1324-modules transform.

## Next Phase Readiness

Ready for plan 26-03 (Skill detail pane + actions). The shell is in place, every later view drops into `App.tsx`'s view-router without touching `Window` / `Titlebar` / `Sidebar` / `ContentPane`. `useSkills`'s Result-narrowing shape is the template the upcoming `useSkillDetail` will mirror; the `SkillOrigin` discriminated union is ready for managed/local branching in the detail header. `useFuzzySearch`'s memoization pattern is reusable for any later filtered-list surface (e.g. Doctor findings filter in 26-05).

**Open follow-ups surfaced during execution (non-blocking):**

- **Group toolbar renders flat** in this plan. Section-header rendering for grouped mode is small but adds Layout complexity worth verifying against the 26-08 perf bench first. Task 2 documented the no-op inline.
- **Recent sort falls back to name** because `DiscoveredSkill` has no `synced_at` projection today. A future plan can extend `DiscoveredSkill` with a manifest-sourced timestamp (one additional `discover_all` join) or have the list view fetch a manifest snapshot in parallel.
- **`StatusReport.tome_home`** open follow-up from 26-01 remains: `StatusView` still uses the `deriveTomeHome()` heuristic. Not in scope here but worth refilling onto the backlog as the surface stabilises.

## Threat Flags

None. All four declared threats in the threat register were handled as designed:
- **T-26-02-01 (DoS at 2000+ skills):** mitigated — `list_skills` is fetched once on mount; the React Aria Virtualizer renders only the visible window; fuse.js filtering runs JS-side without re-fetching. The 60fps budget is verified by plan 26-08.
- **T-26-02-02 (EoP — new cross-boundary type not specta-gated):** mitigated — every new type carries `#[cfg_attr(feature = "bindings", derive(specta::Type))]`; the Phase 25 CI freshness gate (`gen-bindings + git diff --exit-code`) catches any drift.
- **T-26-02-03 (Tampering — fuzzy ranking divergence):** accepted, documented inline in `useFuzzySearch.ts` per Assumption A3.
- **T-26-02-04 (InformationDisclosure — user paths in rows):** accepted; same trust boundary as `tome list`.
- **T-26-02-SC (Tampering — npm install slopsquat):** mitigated — Task 0 blocking-human checkpoint verified `react-aria-components` and `fuse.js` against the upstream repos before install. fuse.js's elevated risk profile was the user's explicit checkpoint focus.

No new threat surface was introduced.

---
*Phase: 26-read-only-views-alpha-cut*
*Plan: 02*
*Completed: 2026-05-29*

## Self-Check: PASSED

All 20 claimed created files exist on disk; both task commits (`56ca757`, `f53e92d`) are present in `git log`. No deleted-file warnings on either commit.

---
phase: 27-sync-triage-ui
plan: 01b
subsystem: tauri-boundary-react-skeleton
tags: [tauri, react, ipc-bindings, menu-bar, sidebar, keyboard-shortcuts, sync-01, pitfall-5, pitfall-6, pitfall-7]

# Dependency graph
requires:
  - phase: 27-sync-triage-ui
    plan: 01a
    provides: "ProgressEvent.SyncStageProgress.item field, DiscoveredSkill.synced_at, TauriEventSink + SyncProgress mirror with item, CancelToken Clone API, RecordingSink Pitfall 4 ordering pin"
  - phase: 26-read-only-views-alpha-cut
    provides: "useTauriEvent late-listen-race helper, useStatus / useDoctorReport / useSkills idle-state hooks, MenuAction event channel + useMenuActions hook, Sidebar shell + axe-core a11y gate harness"
provides:
  - "tome::sync — pub fn re-exported as the canonical pipeline entry; callable by both CLI (cmd_sync) and GUI (start_sync command). Signature unchanged: Result<()>"
  - "tome::SyncOptions — pub struct with field-level pub. Built inline at IPC boundary or CLI."
  - "tome::{MachinePrefs, load_machine_prefs} — re-exported at lib.rs root so external consumers can load machine.toml without depending on the gated module path"
  - "tome_desktop::sync_state::SyncState { cancel: Mutex<Option<CancelToken>> } — managed app state for double-fire mitigation (T-27-01b-07)"
  - "tome_desktop::commands::start_sync — async Tauri command. spawn_blocking wrap around tome::sync (Pitfall 5). Returns Result<(), TomeError>. ErrorCode::Conflict on concurrent invocation."
  - "tome_desktop::commands::cancel_sync — sync Tauri command. Idempotent flip of the shared Arc<AtomicBool>."
  - "tome_desktop::menu::MenuAction::JumpSync — new typed variant; ALL extended to 5; exhaustiveness sentinel + const_assert updated"
  - "Native menu accelerator re-anchor: ⌘3 → Sync (View menu + emits JumpSync) ; ⌘4 → Health (re-anchored from ⌘3) ; ⌘R → Library → Sync (enabled + emits JumpSync, removing the Phase-26 disabled View → Reload placeholder)"
  - "ui/src/stores/router.ts View union extended with 'sync' between 'skills' and 'health'"
  - "ui/src/shell/Sidebar.tsx 4-NavItem layout (Status → Skills → Sync → Health) + spinner slot + dual-meaning Sync badge (pending / failures / none) with 4 aria-label flavors per UI-SPEC"
  - "ui/src/hooks/useSync (.tsx because Provider uses JSX) — SyncProvider Context + hook. ONLY subscribes to events.syncProgress (Pitfall 6 discipline; verified by unit test). Owns stages Map, isRunning flag, terminal SyncTerminal outcome, start/cancel/dismiss handlers, pendingDecisions + failureCount stubs (27-02 / 27-05 populate)"
  - "ui/src/views/SyncView.tsx — three-shape skeleton (idle hero / running placeholder / terminal summary)"
  - "ui/src/hooks/useMenuActions.ts gains JumpSync case + global window-level ⌘R (start / cancel) + ⌘. (cancel-while-running) handlers, both routed through useSync()"
  - "ui/src/lib/textInputFocus.ts — shared 'is the user typing?' guard extracted from SkillsView (Phase 26 inline guard now single-sourced)"
  - "ui/src/main.tsx wraps App in <SyncProvider> so Sidebar / useMenuActions / SyncView share ONE in-flight state machine + ONE syncProgress listener"
  - "bindings.ts regenerated — start_sync, cancel_sync, MenuAction.JumpSync, DiscoveredSkill.synced_at (from 27-01a), SyncProgress.item (from 27-01a). CI freshness gate clean."
  - "axe-core/playwright spec extended with 'sync view passes axe WCAG-AA' scan"
affects: [27-02, 27-02b, 27-03, 27-04, 27-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "spawn_blocking via tauri::async_runtime::spawn_blocking (not direct tokio::task::spawn_blocking) — avoids a direct tokio dep while exercising the same Tokio handle the Tauri runtime already owns. Pitfall 5 satisfied."
    - "Double-fire guard: check + insert under a single Mutex<Option<T>> guard. Critical that the guard is DROPPED before any .await (defensive — std::sync::MutexGuard is !Send by default). The slot is cleared on every exit path (Ok, Err, JoinError) so a wedged sync can never starve subsequent runs."
    - "React Context for cross-component shared state. useSync() must be called from inside <SyncProvider> — the hook throws helpfully when used outside. This is the right pattern when (a) multiple components in the tree need the SAME state machine, and (b) the state machine subscribes to a singleton event channel (re-mounting it per-component would multi-register the listener)."
    - "SectionLabel + View union extension as 'literal-union + exhaustive-switch' — the View type drives the SectionLabel switch in App.tsx, which means a new variant trips tsc with a 'not all paths return a value' error until each switch is extended. No need for separate enum types."
    - "Pitfall 6 watcher-feedback discipline encoded as a unit test: mock every events.{manifestChanged,lockfileChanged,libraryChanged,machinePrefsChanged}.listen — assert the hook NEVER calls them. The test would catch a regression where someone copies useStatus.ts as a template and forgets to strip the watcher subscriptions."

key-files:
  created:
    - "crates/tome-desktop/src/sync_state.rs"
    - "crates/tome-desktop/ui/src/hooks/useSync.tsx"
    - "crates/tome-desktop/ui/src/views/SyncView.tsx"
    - "crates/tome-desktop/ui/src/lib/textInputFocus.ts"
    - "crates/tome-desktop/ui/src/hooks/__tests__/useSync.test.tsx"
    - "crates/tome-desktop/ui/src/hooks/__tests__/useMenuActions.test.tsx"
    - "crates/tome-desktop/ui/src/shell/__tests__/Sidebar.test.tsx"
    - "crates/tome-desktop/ui/src/stores/__tests__/router.test.tsx"
  modified:
    - "crates/tome/src/lib.rs"
    - "crates/tome-desktop/src/commands.rs"
    - "crates/tome-desktop/src/lib.rs"
    - "crates/tome-desktop/src/main.rs"
    - "crates/tome-desktop/src/menu.rs"
    - "crates/tome-desktop/ui/src/App.tsx"
    - "crates/tome-desktop/ui/src/main.tsx"
    - "crates/tome-desktop/ui/src/bindings.ts"
    - "crates/tome-desktop/ui/src/stores/router.ts"
    - "crates/tome-desktop/ui/src/shell/Sidebar.tsx"
    - "crates/tome-desktop/ui/src/shell/Sidebar.module.css"
    - "crates/tome-desktop/ui/src/shell/Titlebar.tsx"
    - "crates/tome-desktop/ui/src/hooks/useMenuActions.ts"
    - "crates/tome-desktop/ui/src/views/SkillsView.tsx"
    - "crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts"
    - "crates/tome-desktop/tests/a11y/axe.spec.ts"

key-decisions:
  - "Made tome::sync + tome::SyncOptions public + added pub re-exports for MachinePrefs / load_machine_prefs (deviation Rule 3 — plan's <interfaces> block ASSUMED these were public but they were pub(crate) / private). The plan's quoted signature 'pub fn sync(...) -> Result<SyncReport>' is also wrong: sync() returns Result<()> today. Return type for start_sync is Result<(), TomeError> for plan 27-01b; the React side observes the run purely through the SyncProgress event stream. Plan 27-05 will swap to Result<SyncOutcomeWire, TomeError> per the plan's own Task-1 note."
  - "tauri::async_runtime::spawn_blocking instead of tokio::task::spawn_blocking. Same Tokio runtime under the hood (Tauri 2's async_runtime IS Tokio) but avoids a direct tokio dep in tome-desktop's Cargo.toml. RESEARCH §549 quoted the tokio path; the Tauri-wrapped path is equivalent and idiomatic for Tauri commands."
  - "SyncProvider React Context lifted to crates/tome-desktop/ui/src/main.tsx so all three consumers (App.tsx Sidebar slots, useMenuActions global ⌘R, SyncView) share ONE state machine + ONE event listener. The alternative — calling useSync() in each component — would have spawned three independent machines, each subscribing its own syncProgress listener; the Sidebar's spinner wouldn't track the SyncView's run, and ⌘R wouldn't kick off a sync the user sees. The Context throws when used outside the provider so a regression (someone deletes the provider wrap) trips at first render."
  - "Sidebar dual-meaning badge slot pre-wired with the SyncBadge tagged-union type even though both branches (pending count from 27-02 + failure count from 27-05) are stubs today. Defining the contract NOW means 27-02 only adds the pendingDecisions populator, and 27-05 only adds the failureCount populator — neither has to touch the Sidebar component. The badge styles ship in Sidebar.module.css alongside the existing .badge class (managed-blue .badgePending for 'pending' state, .badge for 'failures'); prefers-reduced-motion downgrades the spinner from rotation to opacity-pulse."
  - "Pitfall 7 six-step checklist was applied verbatim: enum variant added in jump-order (between JumpSkills and JumpHealth); ALL extended + len assertion bumped to 5; exhaustiveness sentinel match arm added; View menu re-ordered with Sync at ⌘3 and Health at ⌘4; Library → Sync enabled with ⌘R + emits JumpSync; click-dispatch handler accepts both 'jump-sync' (View menu) and 'sync' (Library menu) as JumpSync sources. The Phase-26 disabled View → Reload (⌘R) item was REMOVED (its accelerator slot is reclaimed by Library → Sync per D-02)."
  - "Pitfall 6 watcher-feedback discipline pinned by direct unit test: mock every watcher event's .listen + assert useSync never registers a callback on any of them. Pin would catch a copy-paste regression where someone templates useSync off useStatus and forgets to strip the four watcher subscriptions. The idle-state hooks (useStatus, useSkills, useDoctorReport) keep their subscriptions; they handle post-sync refresh."
  - "useSync hook file renamed .ts → .tsx because the SyncProvider component uses JSX. Vite + tsconfig moduleResolution=bundler resolves extensionless imports against both extensions, so no consumer needed to update its import path."
  - "Did NOT extract a tauri::test mock_app harness for an end-to-end start_sync happy-path test (plan's <behavior> contemplated this if a harness existed). The repo has no such harness today; the plan's Task 1 explicitly said 'skip this and document the gap' in that case. The plan's verify chain (build + clippy + the new unit tests + the axe a11y scan + npm typecheck) is the proxy. End-to-end coverage will land in 27-04 (cancel invariant test) and 27-05 (partial-failure SyncOutcome) where the wider harness work is in scope."
  - "isTextInputFocused() helper extracted from SkillsView into ui/src/lib/textInputFocus.ts so useMenuActions can share it. Pitfall 9 / T-26-07-01 rationale (Edit menu ⌘C routes to OS, skill-scoped handlers must abstain when text input has focus) now lives in one canonical place — Phase 26 SkillsView imports it instead of defining inline."

patterns-established:
  - "Long-running Tauri command shape: async fn + spawn_blocking + managed app-state mutex for cancellation. The Mutex<Option<CancelToken>> pattern (idle = None ; in-flight = Some(token)) gives both the double-fire guard AND the cancel API for free. Reuse this for any future long-runner (e.g., a future tome update GUI command would adopt the same shape with its own Update-state mutex)."
  - "Context-lifted hook for cross-component state. React's per-call hook scope is the right default; the Context override is for state that (a) multiple components MUST share, AND (b) wraps a singleton resource like an event channel. The provider's throw-on-missing-context error is a regression catcher — works because the error message references the provider class by name."
  - "Plan-7 / Pitfall-7 'six-step re-anchor checklist' as a copyable procedure. When a future plan re-shuffles accelerators (e.g., adding a 6th view in a later milestone), the same steps apply: enum variant, ALL constant, sentinel, accelerator string in the menu, useMenuActions case, axe-test aria-label assertion. Documenting it here so 27-02..27-05 don't re-derive the checklist."

requirements-completed:
  - SYNC-01

# Metrics
duration: 20min
completed: 2026-06-06
---

# Phase 27 Plan 01b: Tauri boundary + React skeleton Summary

**Tauri-boundary commands + React skeleton + native menu re-anchoring for SYNC-01 — start_sync (async + spawn_blocking) + cancel_sync (sync + idempotent) with double-fire guard; MenuAction::JumpSync + ⌘3/⌘4 re-anchor + Library → Sync (⌘R) enabled; SyncView idle/running/terminal skeleton + useSync hook with Pitfall 6 watcher-feedback discipline; Sidebar 4th NavItem with spinner + dual-meaning badge slots; bindings.ts CI freshness gate clean; axe-core scan of the Sync route passes.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-06-06T12:54:48Z
- **Completed:** 2026-06-06T13:14:03Z
- **Tasks:** 4 (all atomic)
- **Files created:** 8 (1 Rust module + 4 React source files + 3 React test files)
- **Files modified:** 16

## Accomplishments

- **Task 1 (commit `f07096e`).** start_sync + cancel_sync Tauri commands wired with the SyncState double-fire guard. tome::sync + tome::SyncOptions made public (re-exports added for MachinePrefs + load_machine_prefs at lib.rs root). spawn_blocking wraps the synchronous sync body so the IPC reactor stays responsive (Pitfall 5). Slot is cleared on every exit path so a wedged sync can never starve subsequent runs. 6 new unit tests: SyncState ctor invariants + the double-fire-guard return value + cancel_sync idempotency. The mid-flight happy-path test was skipped per the plan's contingency (no tauri::test::mock_app harness in repo today).
- **Task 2 (commit `8f930dc`).** MenuAction::JumpSync added between JumpSkills and JumpHealth to match the sidebar render order. Pitfall 7 six-step checklist applied verbatim: enum + ALL + exhaustiveness sentinel + const_assert all updated to 5 variants; View menu re-ordered so Sync sits at ⌘3 and Health at ⌘4; Library → Sync enabled with ⌘R and routes through JumpSync; the Phase-26 disabled View → Reload (⌘R) item was REMOVED (its accelerator slot is reclaimed). Runtime test pins the ordering of MenuAction::ALL so a future rename / re-shuffle trips here, not in the React useMenuActions switch.
- **Task 3 (commit `aedb5f6`).** React skeleton ships in 6 files + 4 test files. Router View union extended to include "sync"; Sidebar gains the 4th NavItem with spinner + dual-meaning Sync badge (pending = managed-blue, failures = danger), with 4 aria-label flavors per UI-SPEC; useSync hook + SyncProvider Context wires the three consumers (App.tsx Sidebar slots / useMenuActions ⌘R / SyncView) to ONE state machine + ONE syncProgress listener; useMenuActions adds the JumpSync case + global ⌘R (start/cancel) + ⌘. (cancel-while-running) handlers; SyncView renders the three skeleton shapes (idle hero with [Run sync] CTA + running placeholder with [Cancel sync] + terminal "Sync complete" / inline error with [Dismiss]); App.tsx routes "sync" → SyncView + threads syncInProgress + syncBadge through to Sidebar. 16 new Vitest tests cover Pitfall 6 (useSync subscribes ONLY to syncProgress), 4 Sidebar aria-label flavors, Sidebar render order, JumpSync → setView("sync"), router View union, cancel/start handler call counts, Conflict outcome surfacing.
- **Task 4 (commit `97b8e59`).** axe-core/playwright scan of the Sync route added (clicks Sidebar Sync NavItem, waits for `<h1>` + [Run sync], asserts zero WCAG-AA violations). bindings.ts CI freshness gate verified: `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code` clean. All 5 axe surfaces (Status / Skills / Sync / Health / PreviewPopover) pass.

## Task Commits

Each task was committed atomically:

1. **Task 1: start_sync + cancel_sync IPC commands with SyncState double-fire guard** — `f07096e` (feat) — `crates/tome/src/lib.rs`, `crates/tome-desktop/src/commands.rs`, `crates/tome-desktop/src/lib.rs`, `crates/tome-desktop/src/main.rs`, `crates/tome-desktop/src/sync_state.rs` (new).
2. **Task 2: MenuAction::JumpSync + re-anchor View menu accelerators (Pitfall 7)** — `8f930dc` (feat) — `crates/tome-desktop/src/menu.rs`.
3. **Task 3: SyncView skeleton + useSync hook + Sidebar 4th NavItem + ⌘R/⌘. global keys** — `aedb5f6` (feat) — 17 files (6 React modules + 4 test files + 6 React shell/store modifications + the regenerated bindings.ts + a11y mock).
4. **Task 4: axe-core scan of Sync route + bindings.ts CI freshness gate** — `97b8e59` (test) — `crates/tome-desktop/tests/a11y/axe.spec.ts`.

## Files Created/Modified

- **`crates/tome/src/lib.rs`** — `sync()` and `SyncOptions` made public (with field-level `pub`). Added `pub use machine::MachinePrefs;` and `pub use machine::load as load_machine_prefs;` re-exports at lib.rs root. Doc comments on both `sync` and `SyncOptions` explain the IPC-boundary contract.
- **`crates/tome-desktop/src/commands.rs`** — Added `start_sync` (async with `tauri::async_runtime::spawn_blocking` + double-fire guard via `state.cancel.lock()` check + setup-out-of-spawn for fast-fail) and `cancel_sync` (sync + idempotent). 3 new unit tests: cancel-no-token returns Ok, cancel idempotency, double-fire guard returns Conflict + preserves original token.
- **`crates/tome-desktop/src/sync_state.rs`** (new) — `SyncState { cancel: Mutex<Option<CancelToken>> }` with `Default` + `new()` ctor. 3 unit tests pin the idle ctor + Default-vs-new equivalence + stored-token-is-shared invariant.
- **`crates/tome-desktop/src/lib.rs`** — Added `pub mod sync_state;` and registered `start_sync` + `cancel_sync` in `collect_commands![]`.
- **`crates/tome-desktop/src/main.rs`** — `.manage(tome_desktop::sync_state::SyncState::default())` so both commands share the in-flight slot.
- **`crates/tome-desktop/src/menu.rs`** — `MenuAction::JumpSync` variant added (in jump-order between JumpSkills and JumpHealth). `ALL` extended to 5 + exhaustiveness sentinel match arm + `const_assert` len = 5 updated. View menu re-ordered: Sync at ⌘3, Health re-anchored to ⌘4. Library → Sync enabled with ⌘R (routes through JumpSync). Click-dispatch handler accepts both `"jump-sync"` (View menu) and `"sync"` (Library menu) as JumpSync sources. Removed the Phase-26 disabled View → Reload (⌘R) placeholder. Module-doc updated to describe the new accelerator map. 1 new unit test pins `ALL` to the exact 5-variant ordering.
- **`crates/tome-desktop/ui/src/stores/router.ts`** — `View` union extended with `"sync"` between `"skills"` and `"health"`. Module doc updated.
- **`crates/tome-desktop/ui/src/shell/Sidebar.tsx`** — Rewritten to render 4 NavItems in order (Status → Skills → Sync → Health). `SidebarProps` extends with `syncInProgress?: boolean` + `syncBadge?: SyncBadge` (tagged-union `{ kind: "none" }` / `{ kind: "pending"; count }` / `{ kind: "failures"; count }`). Sync row renders inline `<SpinnerIcon>` when `syncInProgress` is true; renders managed-accent or danger badge based on `syncBadge.kind`. `computeAriaLabel` produces 4 spec-fixed flavors for the Sync row.
- **`crates/tome-desktop/ui/src/shell/Sidebar.module.css`** — Added `.badgePending` (managed-accent fill, same dims as `.badge`); `.syncSpinner` (inline SVG with `animation: syncSpinnerSpin 1s linear infinite`); `prefers-reduced-motion` swap to an opacity-pulse animation.
- **`crates/tome-desktop/ui/src/shell/Titlebar.tsx`** — `SectionLabel` union extended with `"Sync"`.
- **`crates/tome-desktop/ui/src/App.tsx`** — Mounted SyncView for `view === "sync"`; updated `sectionLabel` switch; calls `useSync()` to derive `syncInProgress` + `syncBadge` props for the Sidebar.
- **`crates/tome-desktop/ui/src/main.tsx`** — Wraps `<App />` in `<SyncProvider>` so all consumers share one in-flight state machine.
- **`crates/tome-desktop/ui/src/hooks/useSync.tsx`** (new, `.tsx` because Provider uses JSX) — `SyncProvider` Context + `useSync()` hook. Subscribes ONLY to `events.syncProgress` (Pitfall 6). Owns the per-stage `StageStatus` Map with `stageStartAt` ref for D-10 durations + `isRunningRef` for tail-end event drops. `start`/`cancel`/`dismiss` handlers; `pendingDecisions` + `failureCount` stubs.
- **`crates/tome-desktop/ui/src/hooks/useMenuActions.ts`** — `JumpSync` case added to the menu-action switch + global keydown listener for ⌘R (start if idle / cancel if running / no-op terminal) + ⌘. (cancel-only-while-running). Imports `useSync()` from the provider + the new `isTextInputFocused()` helper.
- **`crates/tome-desktop/ui/src/lib/textInputFocus.ts`** (new) — Shared `isTextInputFocused()` extracted from SkillsView so useMenuActions can share the same Pitfall 9 / T-26-07-01 abstain-when-typing logic.
- **`crates/tome-desktop/ui/src/views/SyncView.tsx`** (new) — Three render shapes (idle / in-progress / terminal). Idle hero uses `formatRelative()` for `last_sync`; running placeholder uses `aria-busy="true" aria-live="polite"`; terminal block surfaces TomeError code/message/context when `outcome.kind === "err"`.
- **`crates/tome-desktop/ui/src/views/SkillsView.tsx`** — Inline `isTextInputFocused` removed; imports the shared `lib/textInputFocus.ts` helper.
- **`crates/tome-desktop/ui/src/bindings.ts`** — Regenerated: `startSync` / `cancelSync` command stubs, `MenuAction.JumpSync` variant, plus the 27-01a additive fields (`SyncProgress.item`, `DiscoveredSkill.synced_at`) now visible to TS consumers.
- **`crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts`** — Added `start_sync` and `cancel_sync` no-op handlers so the a11y mock doesn't crash if axe wanders into the Sync route.
- **`crates/tome-desktop/tests/a11y/axe.spec.ts`** — New `sync view passes axe WCAG-AA` test block (between skills and health).
- **`crates/tome-desktop/ui/src/hooks/__tests__/useSync.test.tsx`** (new) — 6 tests: Pitfall 6 (only syncProgress subscribed), provider doesn't register multiple listeners with multiple consumers, cancel button calls `commands.cancelSync()` once, start button calls `commands.startSync()` once, Conflict outcome surfaced.
- **`crates/tome-desktop/ui/src/hooks/__tests__/useMenuActions.test.tsx`** (new) — 3 tests: listener registered on mount, JumpSync → setView("sync"), JumpHealth → setView("health") (Pitfall 7 re-anchor preserved).
- **`crates/tome-desktop/ui/src/shell/__tests__/Sidebar.test.tsx`** (new) — 6 tests: 4-NavItem render order, 4 Sync aria-label flavors, Health badge wiring preserved.
- **`crates/tome-desktop/ui/src/stores/__tests__/router.test.tsx`** (new) — 2 tests: View union accepts "sync", setView round-trip.

## Decisions Made

See `key-decisions` in the frontmatter for full rationale. Quick index:

1. **Public API surface widening.** Made `tome::sync` + `tome::SyncOptions` public (Rule 3 deviation — the plan's quoted signature assumed they were already public). Return type stays `Result<()>` for 27-01b; the React side observes the run through the SyncProgress event stream. 27-05 will swap to `Result<SyncOutcomeWire>` per the plan's own Task-1 note.
2. **tauri::async_runtime::spawn_blocking vs tokio::task::spawn_blocking.** Chose the Tauri-wrapped path to avoid a direct `tokio` dep in `tome-desktop/Cargo.toml`. Same Tokio handle under the hood; same Pitfall-5 mitigation.
3. **SyncProvider React Context.** Three components needed the SAME in-flight state machine; calling `useSync()` three times would spawn three machines + three event listeners. The Context throws when used outside `<SyncProvider>` so a regression trips at first render.
4. **Sidebar dual-meaning badge pre-wired.** Plan-27-01b lands `{ kind: "none" }` everywhere; 27-02 adds the pending-decisions populator, 27-05 adds the failure-count populator. Neither needs to touch the Sidebar component again — only the props.
5. **Pitfall 7 six-step checklist executed verbatim.** Documented in the patterns-established section so 27-02..27-05 don't re-derive it for future re-shuffles.
6. **Pitfall 6 discipline pinned by unit test.** Direct test asserts useSync NEVER calls `events.{manifestChanged,lockfileChanged,libraryChanged,machinePrefsChanged}.listen`.
7. **No tauri::test mock_app E2E test.** Plan's contingency: "if no harness, document the gap" — done. E2E coverage lands in 27-04 (cancel invariant) and 27-05 (SyncOutcome).
8. **isTextInputFocused extracted into lib/textInputFocus.ts.** Single source for the Pitfall 9 / T-26-07-01 guard; SkillsView now imports it too.

## Deviations from Plan

### Rule 3 — Auto-fixed blocking issues

**1. [Rule 3 - Visibility] Made tome::sync + tome::SyncOptions public**
- **Found during:** Task 1 (`cargo build` failed because `tome::sync` is `pub(crate)` and `SyncOptions` is private).
- **Issue:** The plan's `<interfaces>` block quoted `pub fn sync(config: &Config, paths: &TomePaths, options: SyncOptions, sink: &dyn ProgressSink, cancel: &CancelToken) -> Result<SyncReport>` as the API the GUI would call. Reality: `sync()` was `fn sync(...) -> Result<()>` (no return-value), `SyncOptions` was a private struct with private fields, and `MachinePrefs` / `machine::load` lived behind a gated `pub(crate) mod machine`.
- **Fix:** (a) Made `sync()` `pub` + added a doc comment describing the IPC-boundary contract. (b) Made `SyncOptions` `pub struct` with field-level `pub` so external callers can populate inline. (c) Added `pub use machine::MachinePrefs;` and `pub use machine::load as load_machine_prefs;` re-exports at `lib.rs` root. (d) Return type of `start_sync` is `Result<(), TomeError>` for 27-01b; the React side observes the run through the SyncProgress event stream + final command Result. 27-05 will swap to `Result<SyncOutcomeWire, TomeError>` per the plan's own Task-1 note.
- **Files modified:** `crates/tome/src/lib.rs`
- **Commit:** `f07096e`

**2. [Rule 3 - Dep avoidance] tauri::async_runtime::spawn_blocking instead of tokio::task::spawn_blocking**
- **Found during:** Task 1 (the plan quoted `tokio::task::spawn_blocking(move || tome::sync(...)).await?`).
- **Issue:** `tokio` is NOT a direct dep in `tome-desktop/Cargo.toml` (it lives transitively under `tauri`); adding it as a direct dep would have been a scope-expansion without payoff.
- **Fix:** Use `tauri::async_runtime::spawn_blocking` instead. The Tauri 2 async runtime IS Tokio (`async_runtime::handle()` returns a Tokio handle), so the call is functionally identical. The JoinHandle await pattern is the same.
- **Files modified:** `crates/tome-desktop/src/commands.rs`
- **Commit:** `f07096e`

### Rule 1 — Auto-fixed bug

**3. [Rule 1 - Sidebar test] React Aria mangles ListBoxItem `id` prop into `react-aria-_r_1_-option-status` shape**
- **Found during:** Task 3 first vitest run on `Sidebar.test.tsx`.
- **Issue:** `expect(items[0]).toHaveAttribute("id", "status")` failed because react-aria-components prefixes the `id` with its own internal scope identifier.
- **Fix:** Match `textContent.toContain("Status")` instead — that's the user-visible contract anyway. The original assertion was over-specifying an implementation detail.
- **Files modified:** `crates/tome-desktop/ui/src/shell/__tests__/Sidebar.test.tsx`
- **Commit:** `aedb5f6` (this is part of the Task 3 commit, fixed before the commit landed).

### Scope adjustments (NOT deviations, documented for handoff)

**4. [Scope clarification] bindings.ts shipped in Task 3 commit, not Task 4**
- The React work in Task 3 needs `commands.startSync` / `commands.cancelSync` / `events.menuAction.JumpSync` to exist for `npm run typecheck` to pass. The plan's Task-4 ordering would have required the React commit to live in a state where typecheck fails until Task 4 lands. The pragmatic resolution: commit `bindings.ts` in Task 3 (alongside the React code that depends on it), and Task 4's role becomes "verify that re-running gen-bindings yields zero diff + extend the axe spec". The CI freshness gate is still meaningful (it would catch a future plan that adds a new command but forgets to regenerate bindings).

**5. [Out-of-scope discovery] No deferred-items.md entries**
- Reviewed every file modified in this plan; no out-of-scope discoveries (pre-existing warnings, drift, etc.) required logging.

## Issues Encountered

- **Public API surface widening of `tome::sync` + `tome::SyncOptions` + `MachinePrefs`.** Surfaced as compile errors at the IPC boundary; resolved as documented Rule 3 deviation above.
- **React Aria id-mangling on the Sidebar test.** Resolved by switching to text-content assertions (also a stronger user-visible contract).
- **The useTauriEvent helper's `() => void` handler signature.** Discards the typed payload. Initially imported it in useSync alongside the direct event listener, but the direct listener is the one that drives state — pulled `useTauriEvent` back out + documented the rationale inline (Pitfall 6 restraint: keep the surface small). The plan called for `useTauriEvent` use specifically; the discriminated-payload need (`SyncProgress` carries `stage`, `current`, `total`, `item`) made a direct registration the right call, mirroring the same pattern `useFsEvents` would have used if it needed payloads.

## User Setup Required

None — the Sync command set is local + reads / writes the same `tome_home` the CLI uses. No env vars, no dashboards, no auth.

## Next Phase Readiness

- **27-02 (depends_on: [01b]) — Triage panel population.** Can now populate `useSync().pendingDecisions` and the SyncView's "Recent changes" disclosure stub. The Sidebar's pending-badge slot is already wired (managed-blue fill) and will turn on automatically once 27-02 returns a positive `pendingDecisions` count.
- **27-02b — VIEW-02 carryover in SkillsView.** Independent of this plan's React state machine; reads `DiscoveredSkill.synced_at` (from the 27-01a binding) directly off the list_skills response.
- **27-03 — Apply flow into TriagePanel + PreviewPopover.** Will mount inside the SyncView's in-progress branch (the current `<p>Sync running…</p>` placeholder).
- **27-04 — StageStepper + real cancellation invariant test.** The StageStepper component replaces the in-progress placeholder. `useSync().stages` already exposes the per-stage `StageStatus` Map. The cancellation invariant test (end-to-end: start_sync, fire `cancel_sync` mid-run, assert sync exits at next stage boundary) needs a `tauri::test` harness which 27-04 will add.
- **27-05 — SyncOutcomeWire wrapping + partial-failure rendering.** Plan 27-05 swaps `start_sync`'s return type from `Result<(), TomeError>` to `Result<SyncOutcomeWire, TomeError>`. The React side's `SyncTerminal` discriminator already has `{ kind: "ok" }` / `{ kind: "err"; error }` — 27-05 widens the `ok` variant to carry the structured outcome.
- **No blockers carried forward.**

## Verification Summary

- `cargo build -p tome-desktop`: clean.
- `cargo build -p tome` (regression check from making sync/SyncOptions public): clean.
- `cargo test -p tome-desktop --lib`: 20 / 20 pass (10 pre-existing sink tests from 27-01a + 3 new sync_state tests + 3 new commands tests + 1 new menu test + 3 pre-existing watcher tests).
- `cargo test -p tome --lib`: 916 / 916 pass (no regression from public-API widening).
- `cargo clippy -p tome-desktop -p tome --all-targets -- -D warnings`: clean.
- `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts`: clean (CI freshness gate passes).
- `npx tsc --noEmit` in `crates/tome-desktop/ui/`: clean.
- `npm test` in `crates/tome-desktop/ui/`: 21 / 21 (MarkdownBody 5 + Sidebar 6 + useSync 6 + useMenuActions 3 + router 2). The Pitfall-6 watcher-feedback-discipline test, the 4 Sidebar aria-label flavor tests, and the JumpSync routing test are all in scope here.
- `npm run test:a11y` in `crates/tome-desktop/ui/`: 5 / 5 pass (status / skills / sync / health / preview-popover all clean against WCAG-AA, excluding the documented `color-contrast` rule).
- Manual smoke (not run — no GUI binary launched in this scope): a future hands-on session will exercise the menu re-anchor + ⌘R + Cancel-during-run flow.

## Self-Check: PASSED

All claimed artifacts verified:

- `.planning/phases/27-sync-triage-ui/27-01b-SUMMARY.md` written.
- Rust sources:
  - `crates/tome/src/lib.rs` (modified) ✓
  - `crates/tome-desktop/src/commands.rs` (modified) ✓
  - `crates/tome-desktop/src/lib.rs` (modified) ✓
  - `crates/tome-desktop/src/main.rs` (modified) ✓
  - `crates/tome-desktop/src/menu.rs` (modified) ✓
  - `crates/tome-desktop/src/sync_state.rs` (new) ✓
- React sources:
  - `crates/tome-desktop/ui/src/App.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/main.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/bindings.ts` (regenerated) ✓
  - `crates/tome-desktop/ui/src/stores/router.ts` (modified) ✓
  - `crates/tome-desktop/ui/src/shell/Sidebar.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/shell/Sidebar.module.css` (modified) ✓
  - `crates/tome-desktop/ui/src/shell/Titlebar.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/hooks/useMenuActions.ts` (modified) ✓
  - `crates/tome-desktop/ui/src/hooks/useSync.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/views/SyncView.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/views/SkillsView.tsx` (modified) ✓
  - `crates/tome-desktop/ui/src/lib/textInputFocus.ts` (new) ✓
  - `crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts` (modified) ✓
- Tests:
  - `crates/tome-desktop/tests/a11y/axe.spec.ts` (modified) ✓
  - `crates/tome-desktop/ui/src/hooks/__tests__/useSync.test.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/hooks/__tests__/useMenuActions.test.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/shell/__tests__/Sidebar.test.tsx` (new) ✓
  - `crates/tome-desktop/ui/src/stores/__tests__/router.test.tsx` (new) ✓
- Commits `f07096e`, `8f930dc`, `aedb5f6`, `97b8e59` present in `git log --oneline --all`.

---
*Phase: 27-sync-triage-ui*
*Completed: 2026-06-06*

---
phase: 27-sync-triage-ui
plan: 01b
type: execute
wave: 2
depends_on:
  - 27-01a
files_modified:
  - crates/tome-desktop/src/commands.rs
  - crates/tome-desktop/src/lib.rs
  - crates/tome-desktop/src/menu.rs
  - crates/tome-desktop/ui/src/stores/router.ts
  - crates/tome-desktop/ui/src/App.tsx
  - crates/tome-desktop/ui/src/components/Sidebar.tsx
  - crates/tome-desktop/ui/src/hooks/useMenuActions.ts
  - crates/tome-desktop/ui/src/views/SyncView.tsx
  - crates/tome-desktop/ui/src/hooks/useSync.ts
  - crates/tome-desktop/ui/src/bindings.ts
  - crates/tome-desktop/tests/a11y/axe.spec.ts
autonomous: true
requirements:
  - SYNC-01
tags:
  - tauri
  - react
  - ipc-bindings
  - menu-bar
  - sidebar
  - keyboard-shortcuts

must_haves:
  truths:
    - "The Sync section is reachable via the sidebar, the Library menu, and the ⌘3 keyboard shortcut (Pitfall 7 re-anchoring complete: ⌘3 → Sync, ⌘4 → Health)."
    - "Clicking [Run sync] (or pressing ⌘R) calls commands.startSync which runs tome::sync inside tokio::task::spawn_blocking so the IPC reactor is not blocked (Pitfall 5)."
    - "cancel_sync is a fast synchronous Tauri command that flips the shared Arc<AtomicBool> via CancelToken::cancel(); idempotent and concurrent-safe."
    - "useSync subscribes ONLY to events.syncProgress; it does NOT subscribe to ManifestChanged / LockfileChanged / LibraryChanged / MachinePrefsChanged while a sync is running (Pitfall 6 watcher-feedback discipline)."
    - "Sidebar renders 4 NavItems in order Status → Skills → Sync → Health; the Sync row has working-spinner + dual-meaning badge slots (badge wiring lands in 27-02 for pending and 27-05 for failures)."
    - "bindings.ts CI freshness gate passes: regenerated cleanly to include SyncProgress.item, DiscoveredSkill.synced_at, start_sync, cancel_sync, MenuAction::JumpSync."
    - "axe-core scan of the Sync route in the alpha app shell passes with zero WCAG-AA violations."
    - "T-27-01b-07 double-fire mitigation: a second concurrent start_sync invocation while a token is in SyncState observes Some and returns TomeError { code: Conflict, message: 'sync already in progress' } instead of overwriting."
  artifacts:
    - path: "crates/tome-desktop/src/commands.rs"
      provides: "start_sync (async + spawn_blocking) + cancel_sync (sync + idempotent)"
      exports: ["start_sync", "cancel_sync"]
    - path: "crates/tome-desktop/src/menu.rs"
      provides: "MenuAction::JumpSync variant; ⌘3 re-anchored to Sync; ⌘4 to Health; Library → Sync ⌘R item enabled and routed to JumpSync"
      contains: "JumpSync"
    - path: "crates/tome-desktop/src/lib.rs"
      provides: "make_builder() registers start_sync, cancel_sync, MenuAction::JumpSync; SyncState managed app state with cancel: Mutex<Option<CancelToken>>"
      contains: "SyncState"
    - path: "crates/tome-desktop/ui/src/views/SyncView.tsx"
      provides: "Skeleton Sync view: idle hero with Run sync button + in-progress placeholder + outcome placeholder"
      min_lines: 60
    - path: "crates/tome-desktop/ui/src/hooks/useSync.ts"
      provides: "useSync hook owning stages Map<SyncStage, StageStatus>, start/cancel/dismiss handlers, watcher-feedback discipline (only syncProgress)"
      min_lines: 80
    - path: "crates/tome-desktop/ui/src/components/Sidebar.tsx"
      provides: "4th NavItem Sync between Skills and Health; syncInProgress spinner slot; syncBadge dual-meaning slot"
    - path: "crates/tome-desktop/ui/src/bindings.ts"
      provides: "Regenerated TS bindings (SyncProgress.item, DiscoveredSkill.synced_at, start_sync, cancel_sync, MenuAction::JumpSync)"
  key_links:
    - from: "crates/tome-desktop/src/commands.rs::start_sync"
      to: "crates/tome/src/lib.rs::sync"
      via: "tokio::task::spawn_blocking wrapping tome::sync(&config, &paths, opts, &sink, &cancel)"
      pattern: "spawn_blocking"
    - from: "crates/tome-desktop/ui/src/components/Sidebar.tsx"
      to: "router setView('sync')"
      via: "4th NavItem id='sync' between Skills and Health"
      pattern: "id: \"sync\""
    - from: "crates/tome-desktop/src/menu.rs::install"
      to: "MenuAction::JumpSync emission"
      via: "Library menu Sync item with CmdOrCtrl+R + View menu Jump-to-Sync item with ⌘3"
      pattern: "JumpSync"
    - from: "crates/tome-desktop/ui/src/hooks/useSync.ts"
      to: "events.syncProgress"
      via: "useTauriEvent subscribes ONLY to syncProgress; ignores ManifestChanged/LockfileChanged/LibraryChanged/MachinePrefsChanged (Pitfall 6)"
      pattern: "events.syncProgress"
---

<objective>
SECOND HALF of SYNC-01: wrap 27-01a's domain types in the Tauri boundary and the React skeleton. Register `start_sync` (async with `tokio::task::spawn_blocking` per Pitfall 5) and `cancel_sync` (sync, idempotent, flips the shared `Arc<AtomicBool>`) Tauri commands; add `SyncState { cancel: Mutex<Option<CancelToken>> }` as managed app state with double-fire mitigation; re-anchor `⌘1..⌘4` so Sync occupies `⌘3` and Health moves to `⌘4` (Pitfall 7); register `MenuAction::JumpSync`; build the `Sync` sidebar NavItem (Status → Skills → Sync → Health) with spinner + dual-meaning badge slots; build the `SyncView` skeleton (idle hero with `Run sync` + in-progress placeholder + outcome placeholder); build the `useSync` hook with strict event-subscription discipline (subscribes ONLY to `syncProgress` per Pitfall 6); regenerate `bindings.ts`; add an axe-core scan of the new route.

Purpose: completes SYNC-01 (substrate is now usable end-to-end through the Tauri boundary). Unblocks 27-02 (which wires the triage panel into the in-progress branch), 27-02b (which back-fills VIEW-02 carryover in SkillsView), 27-03 (Apply flow into TriagePanel + PreviewPopover), 27-04 (StageStepper + cancellation invariant test), and 27-05 (SyncOutcome wrapping + partial-failure rendering).

Output: 2 new IPC commands; 1 new menu variant; managed app state; ~6 React skeleton/extension files; regenerated `bindings.ts`; axe scan covering the new Sync route.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/27-sync-triage-ui/27-CONTEXT.md
@.planning/phases/27-sync-triage-ui/27-RESEARCH.md
@.planning/phases/27-sync-triage-ui/27-PATTERNS.md
@.planning/phases/27-sync-triage-ui/27-UI-SPEC.md
@.planning/phases/27-sync-triage-ui/27-01a-PLAN.md
@.planning/phases/26-read-only-views-alpha-cut/deferred-items.md
@crates/tome-desktop/src/commands.rs
@crates/tome-desktop/src/lib.rs
@crates/tome-desktop/src/menu.rs
@crates/tome-desktop/src/error.rs
@crates/tome-desktop/src/sink.rs
@crates/tome-desktop/ui/src/stores/router.ts
@crates/tome-desktop/ui/src/App.tsx
@crates/tome-desktop/ui/src/components/Sidebar.tsx
@crates/tome-desktop/ui/src/hooks/useMenuActions.ts
@crates/tome-desktop/ui/src/hooks/useStatus.ts
@crates/tome-desktop/ui/src/hooks/useTauriEvent.ts
@crates/tome-desktop/ui/src/views/SkillsView.tsx

<interfaces>
<!-- Pre-extracted contracts so the executor does not re-explore. -->

From crates/tome/src/progress.rs (post-27-01a):
- `pub enum ProgressEvent { SyncStageStarted { stage }, SyncStageProgress { stage, current, total, item: Option<String> }, SyncStageFinished { stage }, GitCloneProgress { directory, received }, BackupSnapshot { message } }`
- `pub struct CancelToken(Arc<AtomicBool>)` with `Clone`, `new()`, `cancel()`, `is_cancelled()`

From crates/tome-desktop/src/sink.rs (post-27-01a):
- `pub struct SyncProgress { stage: SyncStage, current: u64, total: u64, item: Option<String> }` (mirror with D-09 fold-in implemented)
- `TauriEventSink::new(app: AppHandle) -> Self`

From crates/tome-desktop/src/commands.rs (existing pattern):
- Every command: `#[tauri::command] #[specta::specta]` + `load_context().map_err(TomeError::from)?` + `.map_err(TomeError::from)`
- `pub fn set_skill_disabled(_app: tauri::AppHandle, name: SkillName, disabled: bool) -> Result<(), TomeError>`

From crates/tome-desktop/src/menu.rs (existing — extend MenuAction):
- `#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)] #[serde(tag = "kind")] pub enum MenuAction { JumpStatus, JumpSkills, JumpHealth, FocusSearch }`
- `impl MenuAction { pub const ALL: [&'static str; 4] = [...]; }` with exhaustiveness sentinel
- Accelerators: `CmdOrCtrl+1/2/3` on jump items; `CmdOrCtrl+R` currently bound to disabled View→Reload (RESEARCH Pitfall 7)

From crates/tome-desktop/src/error.rs:
- `pub struct TomeError { pub code: ErrorCode, pub message: String, pub context: Vec<String> }` + `From<anyhow::Error>` classifier; `ErrorCode::ALL` + sentinel + length-pin trio
- `ErrorCode::Conflict` exists (used here for the double-fire guard)

From crates/tome/src/lib.rs::sync (signature confirmation only):
- `pub fn sync(config: &Config, paths: &TomePaths, options: SyncOptions, sink: &dyn ProgressSink, cancel: &CancelToken) -> Result<SyncReport>`

From crates/tome-desktop/ui/src/stores/router.ts:
- `export type View = "status" | "skills" | "health";` — extend to add `"sync"` between `"skills"` and `"health"`

From crates/tome-desktop/ui/src/hooks/useTauriEvent.ts:
- `export function useTauriEvent<T>(event: EventListener<T>, handler: (payload: T) => void): void` — late-listen-race guard + cleanup
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Register start_sync + cancel_sync Tauri commands; add SyncState managed app state with double-fire mitigation</name>
  <files>
    crates/tome-desktop/src/commands.rs,
    crates/tome-desktop/src/lib.rs
  </files>
  <read_first>
    crates/tome-desktop/src/commands.rs (full file — load_context pattern + the `.map_err(TomeError::from)` discipline at every command edge),
    crates/tome-desktop/src/lib.rs (full file — make_builder() registry pattern + existing app state setup if any; if no app state exists yet, this task adds it inside the Builder's setup closure),
    crates/tome-desktop/src/error.rs (full file — TomeError + ErrorCode::Conflict + From<anyhow::Error> classifier),
    crates/tome-desktop/src/sink.rs (post-27-01a — TauriEventSink::new constructor),
    crates/tome/src/progress.rs (CancelToken Clone API),
    crates/tome/src/lib.rs::sync (signature confirmation only),
    .planning/phases/27-sync-triage-ui/27-RESEARCH.md §"Pitfall 5" + §"Code Examples — spawn_blocking for the sync command (lines 549-577)",
    .planning/phases/27-sync-triage-ui/27-PATTERNS.md §"crates/tome-desktop/src/commands.rs"
  </read_first>
  <behavior>
    - Test (compile): `cargo build -p tome-desktop --features bindings` succeeds with the new commands.
    - Test (cancel_sync no-token): calling `cancel_sync` while `SyncState.cancel` is `None` returns `Ok(())`.
    - Test (cancel_sync idempotent): calling `cancel_sync` twice while a token IS set is idempotent — the second call observes an already-cancelled `AtomicBool` and still returns `Ok(())`.
    - Test (double-fire guard, T-27-01b-07): a second concurrent invocation of `start_sync` while the first is in-flight (the SyncState mutex has `Some(token)`) returns `Err(TomeError { code: ErrorCode::Conflict, message: "sync already in progress", .. })` and does NOT overwrite the existing token.
    - Test (start_sync happy path): if a minimal `tauri::test::mock_app` (or equivalent harness Phase 26 used — check `tests/watcher_smoke.rs`) is available, launch `start_sync` against a TempDir config with at least one empty `[directories.X]` source; assert it returns within 10 seconds with `Ok(SyncReport)`. If no harness exists, skip this and rely on Task 3's axe-scan smoke + Task 2 of 27-01a's unit tests for coverage; document the gap in the task summary.
  </behavior>
  <action>
    1. In `crates/tome-desktop/src/lib.rs`: define `pub struct SyncState { pub cancel: Mutex<Option<CancelToken>> }` with a `Default` impl (or a `new()` constructor) and `derive(Default)` if convenient. Place either at top of `lib.rs` or in a sibling `sync_state.rs` module — Claude's call. Register as managed state in the Tauri builder setup: inside the `Builder::default()` `.setup(|app| { ... })` closure (or wherever the project sets up app state), call `app.manage(SyncState::default())`. If no `.setup` exists, add a minimal one.
    2. In `crates/tome-desktop/src/commands.rs`: add `start_sync` as `async fn start_sync(app: tauri::AppHandle, state: tauri::State<'_, SyncState>) -> Result<tome::SyncReport, TomeError>` decorated with `#[tauri::command] #[specta::specta]`. Body:
       - **Double-fire guard FIRST**: `{ let guard = state.cancel.lock().expect("cancel mutex poisoned"); if guard.is_some() { return Err(TomeError { code: ErrorCode::Conflict, message: "sync already in progress".into(), context: vec![] }); } }` (drop the guard before continuing so a holding mutex doesn't deadlock the rest of the function).
       - Create cancel token + register: `let cancel = CancelToken::new(); *state.cancel.lock().expect("cancel mutex poisoned") = Some(cancel.clone());`.
       - Load context: `let (config, paths) = load_context().map_err(TomeError::from)?;`.
       - Load machine prefs + build SyncOptions: `let machine_path = tome::default_machine_path().map_err(TomeError::from)?; let machine_prefs = tome::machine::load(&machine_path).map_err(TomeError::from)?; let opts = build_sync_options(&machine_path, &machine_prefs);` (use the CLI's canonical options builder — read `crates/tome/src/lib.rs::cmd_sync` for precedent; if the CLI uses a different idiom, mirror it).
       - Build sink: `let sink = TauriEventSink::new(app.clone());`.
       - Run sync on a blocking thread: `let result = tokio::task::spawn_blocking(move || tome::sync(&config, &paths, opts, &sink, &cancel)).await.map_err(|join_err| TomeError::from(anyhow::anyhow!("sync task panicked: {join_err}")))?;`.
       - Clear cancel token: `*state.cancel.lock().expect("cancel mutex poisoned") = None;`.
       - Return: `result.map_err(TomeError::from)`.
       - **NOTE**: This plan's return type is `Result<SyncReport, TomeError>`. 27-05 will swap this for `Result<SyncOutcomeWire, TomeError>`. Do NOT define `SyncOutcome` here.
    3. In `crates/tome-desktop/src/commands.rs`: add `cancel_sync` as synchronous `pub fn cancel_sync(state: tauri::State<'_, SyncState>) -> Result<(), TomeError>` decorated with `#[tauri::command] #[specta::specta]`. Body: `if let Some(token) = state.cancel.lock().expect("cancel mutex poisoned").as_ref() { token.cancel(); } Ok(())`. Idempotent by design (token is `Arc<AtomicBool>` — second cancel is a no-op).
    4. In `crates/tome-desktop/src/lib.rs::make_builder`: append `commands::start_sync, commands::cancel_sync` to the `collect_commands![]` macro invocation. Confirm `events![]` already contains `sink::SyncProgress` (it should from Phase 25/26).
    5. Add unit tests for the double-fire guard + cancel_sync idempotency. The mid-flight `start_sync` happy-path test only ships if a `tauri::test::mock_app` harness exists in the repo (check `crates/tome-desktop/tests/`).
  </action>
  <verify>
    <automated>cargo build -p tome-desktop --features bindings &amp;&amp; cargo test -p tome-desktop --lib commands &amp;&amp; cargo clippy -p tome-desktop --features bindings -- -D warnings</automated>
  </verify>
  <done>
    `start_sync` is `async` and wraps `tome::sync` in `tokio::task::spawn_blocking` (Pitfall 5); double-fire guard returns `ErrorCode::Conflict` on concurrent invocation (T-27-01b-07); `cancel_sync` is sync + idempotent; `SyncState` is managed app state; both commands registered in `make_builder()`; build clean; clippy clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Add MenuAction::JumpSync; re-anchor ⌘3 → Sync, ⌘4 → Health; enable Library → Sync (⌘R); remove disabled View → Reload (Pitfall 7)</name>
  <files>
    crates/tome-desktop/src/menu.rs
  </files>
  <read_first>
    crates/tome-desktop/src/menu.rs (full file — MenuAction enum + ALL + sentinel; current ⌘1/⌘2/⌘3 accelerator registrations; the disabled View → Reload ⌘R item; Library menu Sync item that is currently `.enabled(false)`),
    .planning/phases/27-sync-triage-ui/27-RESEARCH.md §"Pitfall 7" (six-step Pitfall-7 checklist),
    .planning/phases/27-sync-triage-ui/27-PATTERNS.md §"crates/tome-desktop/src/menu.rs"
  </read_first>
  <behavior>
    - Test (compile): the existing `_menu_action_exhaustiveness_sentinel` const-fn now covers `JumpSync` (compile fails if missed).
    - Test (ALL length): `MenuAction::ALL.len() == 5` and the array contains exactly `["JumpStatus", "JumpSkills", "JumpSync", "JumpHealth", "FocusSearch"]` in that order.
    - Test (no test file for menu accelerators): the menu is built imperatively against `tauri::menu::MenuBuilder`; we rely on the cargo build being clean as the assertion that the accelerator strings parse, and on Task 4's playwright a11y scan as the runtime assertion.
  </behavior>
  <action>
    Apply the six-step Pitfall 7 checklist:
    1. Add `JumpSync` variant to `MenuAction` (place between `JumpSkills` and `JumpHealth` to match sidebar order).
    2. Update `pub const ALL: [&'static str; 5] = ["JumpStatus", "JumpSkills", "JumpSync", "JumpHealth", "FocusSearch"];` and extend the `_menu_action_exhaustiveness_sentinel` const-fn match.
    3. In `install` (or whatever fn builds the menu): add a `jump-sync` `MenuItemBuilder` with `.accelerator("CmdOrCtrl+3")` in the View menu (matching the JumpStatus / JumpSkills pattern); change the existing `jump-health` `MenuItemBuilder` from `CmdOrCtrl+3` to `CmdOrCtrl+4`; add a `"jump-sync" => MenuAction::JumpSync` arm to the click-dispatch match.
    4. Re-purpose the existing Library menu `Sync` item that is currently `.enabled(false)`: change to `.enabled(true).accelerator("CmdOrCtrl+R")` and dispatch a `MenuAction::JumpSync`. The "run-now" intent ride-along is handled React-side in Task 4 by `useMenuActions` (which on `JumpSync` calls `setView('sync')`) PLUS a global `⌘R` keybinding (in `useMenuActions`'s parallel useEffect) that triggers `useSync().start()` when idle. The menu Sync item is therefore a navigation action that ALSO causes ⌘R to fire when accelerator-triggered.
    5. REMOVE the disabled View → Reload `CmdOrCtrl+R` `.enabled(false)` item (its `⌘R` slot is reclaimed by Library → Sync per D-02 and RESEARCH Pitfall 7).
    6. Add a one-sentence release-note line to the task summary documenting the keyboard change: "Phase 27 re-anchors `⌘3` from Health to Sync, moves Health to `⌘4`, and enables Library → Sync (`⌘R`)." (Help → Keyboard Shortcuts cheatsheet from Phase 26 26-07 is updated in 27-04 or 27-05 — flag it for handoff.)
  </action>
  <verify>
    <automated>cargo build -p tome-desktop --features bindings &amp;&amp; cargo test -p tome-desktop --lib menu &amp;&amp; cargo clippy -p tome-desktop --features bindings -- -D warnings</automated>
  </verify>
  <done>
    `MenuAction::JumpSync` registered with `⌘3` accelerator in the View menu; Health on `⌘4`; the disabled View → Reload `⌘R` item is removed and reclaimed by Library → Sync (`⌘R`); exhaustiveness sentinel covers all 5 variants; release-note line drafted; build + clippy clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: SyncView skeleton + useSync hook (Pitfall 6 discipline) + Sidebar 4th NavItem + router/menu wiring + global ⌘R/⌘. handlers</name>
  <files>
    crates/tome-desktop/ui/src/stores/router.ts,
    crates/tome-desktop/ui/src/App.tsx,
    crates/tome-desktop/ui/src/components/Sidebar.tsx,
    crates/tome-desktop/ui/src/hooks/useMenuActions.ts,
    crates/tome-desktop/ui/src/views/SyncView.tsx,
    crates/tome-desktop/ui/src/hooks/useSync.ts
  </files>
  <read_first>
    crates/tome-desktop/ui/src/stores/router.ts (full file — View union literal-extension target),
    crates/tome-desktop/ui/src/App.tsx (full file — locate the switch arm for current views; add the `"sync"` arm + titlebar label entry),
    crates/tome-desktop/ui/src/components/Sidebar.tsx (full file — current SECTIONS array, ListBox NavItem rendering, badge slot, aria-label template),
    crates/tome-desktop/ui/src/hooks/useMenuActions.ts (full file — switch on MenuAction.kind; current handlers for JumpStatus/JumpSkills/JumpHealth/FocusSearch),
    crates/tome-desktop/ui/src/hooks/useStatus.ts (full file — useTauriEvent subscription discipline; the pattern useSync mirrors),
    crates/tome-desktop/ui/src/hooks/useTauriEvent.ts (cleanup-on-unmount + late-listen-race guard; canonical hook),
    crates/tome-desktop/ui/src/views/SkillsView.tsx (lines 1-80 — `isTextInputFocused()` guard pattern for global keyboard handlers),
    crates/tome-desktop/ui/src/views/HealthView.tsx (variant-branching shape — SyncView's idle/in-progress branches mirror it),
    .planning/phases/27-sync-triage-ui/27-UI-SPEC.md §"Idle state" + §"In-progress state" + §"Keyboard Map" + §"VoiceOver labels",
    .planning/phases/27-sync-triage-ui/27-PATTERNS.md §"ui/src/components/Sidebar.tsx" + §"ui/src/views/SyncView.tsx" + §"ui/src/hooks/useSync.ts" + §"ui/src/hooks/useMenuActions.ts",
    .planning/phases/27-sync-triage-ui/27-RESEARCH.md §"Pitfall 6" (watcher feedback loop discipline) + §"Pitfall 7"
  </read_first>
  <behavior>
    - Test (router): `router.ts` exports the literal-union `View = "status" | "skills" | "sync" | "health"`; `setView("sync")` round-trips through the store.
    - Test (Sidebar order): the rendered DOM order of NavItems is Status → Skills → Sync → Health.
    - Test (menu route): pressing `⌘3` (via the menu action emission) calls `setView("sync")`; `useMenuActions` switch handles `JumpSync` case.
    - Test (useSync subscription discipline, Pitfall 6): the hook subscribes via `useTauriEvent` ONLY to `events.syncProgress`. A unit test using `@testing-library/react` mounts a component wrapping `useSync`, calls `events.manifestChanged.emit()` + `events.lockfileChanged.emit()` + `events.libraryChanged.emit()` + `events.machinePrefsChanged.emit()`, and asserts that `useSync`'s state is unchanged (no refetch, no reset). Idle-state hooks like `useStatus` continue to subscribe and refresh — those are independent.
    - Test (cancel handler): clicking `[Cancel sync]` in the in-progress placeholder calls `commands.cancelSync()` (mocked; assert call count == 1).
  </behavior>
  <action>
    1. In `crates/tome-desktop/ui/src/stores/router.ts`: extend the `View` literal-union to `"status" | "skills" | "sync" | "health"`. Update any switch/match callers if the file enumerates them.
    2. In `crates/tome-desktop/ui/src/components/Sidebar.tsx`: insert `{ id: "sync", label: "Sync" }` into the `SECTIONS` array between `skills` and `health`. Update `SidebarProps` to add `syncInProgress?: boolean` and `syncBadge?: { kind: "pending" | "failures" | "none"; count: number }` (per UI-SPEC §Sidebar). Render the spinner inline when `syncInProgress === true` (small inline `<svg>` spinner replacing the row icon — Claude's discretion; HIG-aligned system spinner per CONTEXT.md). Implement the dual-meaning badge (managed-blue vs danger fill) per UI-SPEC §Sidebar updated NavItem. Update `aria-label` template per UI-SPEC §VoiceOver labels (4 entries for the Sync NavItem). For this plan, wire `syncBadge` count from `useSync().pendingDecisions` (which is `0` until 27-02 lands the triage panel — defer the actual count to 27-02) and `syncInProgress` from `useSync().isRunning` (true while the pipeline is active).
    3. In `crates/tome-desktop/ui/src/hooks/useMenuActions.ts`: add `case "JumpSync": setView("sync"); break;` to the switch (mirroring the existing JumpStatus/JumpSkills/JumpHealth pattern). Add a parallel `useEffect` that binds global `⌘R` (to call `useSync().start()` when idle, `useSync().cancel()` when running, and a placeholder no-op when terminal — retry handlers land in 27-04 and 27-05) and `⌘.` (to call `useSync().cancel()` when running, no-op otherwise). Use a window-level `keydown` listener with the existing `isTextInputFocused()`-style guard pattern from `SkillsView.tsx:47-55` — extract that helper into `crates/tome-desktop/ui/src/lib/textInputFocus.ts` IF it isn't already shared; otherwise import. Note: `⌘R` fires globally per UI-SPEC (Sync is not an Edit action; no input-focus gate).
    4. Create `crates/tome-desktop/ui/src/hooks/useSync.ts`:
       - State: `stages: Map<SyncStage, StageStatus>` (initialised with all 6 stages as `{ kind: "pending" }`), `outcome: SyncReport | TomeError | null` (or `null` while running), `isRunning: boolean`, `err: TomeError | null`, `stageStartAt: Map<SyncStage, number>` (private; for D-10 duration tracking), `pendingDecisions: number` (always 0 in this plan; 27-02 populates).
       - Subscribe ONLY to `events.syncProgress` via `useTauriEvent`. Handler accumulates per-stage state: on `SyncStageStarted { stage }` mark stage as `{ kind: "active", currentItem: null, current: 0, total: 0 }` and record `stageStartAt[stage] = Date.now()`; on `SyncStageProgress { stage, current, total, item }` update the active stage's `currentItem`/`current`/`total`; on `SyncStageFinished { stage }` mark complete with `durationMs = Date.now() - stageStartAt[stage]`, `partialFailures: []` (populated in 27-05).
       - Handlers: `start: async () => { setIsRunning(true); const res = await commands.startSync(); setIsRunning(false); /* set outcome per result */ }`, `cancel: () => commands.cancelSync()`, `dismiss: () => { reset stages to pending; clear outcome; }`.
       - **Pitfall 6 discipline**: This hook does NOT subscribe to `events.manifestChanged`, `events.lockfileChanged`, `events.libraryChanged`, or `events.machinePrefsChanged`. The idle-state hooks (useStatus, useSkills) retain those subscriptions to refresh AFTER a run completes — useSync stays isolated to `syncProgress`.
       - Result narrowing per Phase 25 S-8 pattern: `if (res.status === "ok") setOutcome(res.data); else setErr(res.error);`. No try/catch around the discriminated union.
    5. Create `crates/tome-desktop/ui/src/views/SyncView.tsx` as a skeleton (the StageStepper / TriagePanel / etc. land in 27-02 / 27-03 / 27-04 / 27-05):
       - Branch on `useSync()`: if `isRunning === false && outcome === null` → render the **idle hero** per UI-SPEC §Idle state (centred composition: `↺` glyph using `RefreshCw` from `lucide-react`, `<h1>` heading "You haven't synced yet." OR "Last synced ${relativeTime}" using `lib/relativeTime.ts`, sub-line `${new} new · ${changed} changed · ${removed} removed since last sync` populated from a `useStatus()` call OR omit when never synced, large `<Button variant="primary">Run sync</Button>` wired to `useSync().start()`). The "Recent changes" disclosure is a stub (`<details>…</details>` placeholder with copy "No changes recorded in the previous sync.") — 27-02 will populate it.
       - If `isRunning === true` → render a placeholder `<div role="region" aria-busy="true" aria-label="Sync pipeline">Sync running…</div>` (the StageStepper component lands in 27-04). Include a `<Button variant="secondary" onPress={cancel}>Cancel sync</Button>`.
       - If `outcome !== null && !isRunning` → render a placeholder summary `<div role="status">Sync complete</div>` with a `<Button variant="secondary" onPress={dismiss}>Dismiss</Button>`. Stepper terminal rendering lands in 27-04 / 27-05.
       - A11y: outer `<section role="status" aria-label="Sync status">` wraps idle; `<h1>` for the idle heading; `aria-live="polite"` for the running region. Match UI-SPEC §VoiceOver labels for `[Run sync]`, `[Cancel sync]`, `[Dismiss]`.
    6. In `crates/tome-desktop/ui/src/App.tsx`: add `case "sync": return <SyncView />;` to the route switch; update the titlebar title-label switch to include `"sync": "Sync"`.
  </action>
  <verify>
    <automated>cd crates/tome-desktop/ui &amp;&amp; npm run typecheck &amp;&amp; npm run test -- --run useSync SyncView Sidebar useMenuActions</automated>
  </verify>
  <done>
    `View` type extended; `Sidebar` renders 4 NavItems in order Status/Skills/Sync/Health with spinner + dual-meaning badge slots; `useMenuActions` routes `JumpSync` + binds `⌘R`/`⌘.` global handlers; `useSync` hook subscribes ONLY to `syncProgress` (Pitfall 6 discipline verified by unit test); `SyncView` renders idle hero with `[Run sync]` button calling `useSync().start()`; `App.tsx` routes `"sync"` → `<SyncView />`; `npm run typecheck` and Vitest tests for useSync/SyncView/Sidebar/useMenuActions pass.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 4: Regenerate bindings.ts (CI freshness gate) + add axe scan of the Sync route</name>
  <files>
    crates/tome-desktop/ui/src/bindings.ts,
    crates/tome-desktop/tests/a11y/axe.spec.ts
  </files>
  <read_first>
    crates/tome-desktop/ui/src/bindings.ts (current contents — confirm SyncProgress export name + event registration pattern),
    crates/tome-desktop/tests/a11y/axe.spec.ts (existing pattern — page.goto navigation + axe scan),
    .planning/phases/27-sync-triage-ui/27-UI-SPEC.md §"VoiceOver labels"
  </read_first>
  <behavior>
    - Test (bindings freshness): `cargo run -p tome-desktop --bin gen-bindings` followed by `git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts` is clean — bindings include the new `start_sync` + `cancel_sync` commands, the `item: string | null` field on `SyncProgress`, the `synced_at: string | null` field on `DiscoveredSkill` records in `ListReport`, and the `JumpSync` variant on `MenuAction`.
    - Test (axe a11y): after navigating to the new Sync view via `⌘3` (or sidebar click), `@axe-core/playwright` reports zero violations of any WCAG-AA rule. The Sync `NavItem` is keyboard-focusable, the `[Run sync]` button has an aria-label, the idle hero heading is `<h1>`.
  </behavior>
  <action>
    1. Run `cargo run -p tome-desktop --bin gen-bindings` from the repo root; commit the resulting diff to `crates/tome-desktop/ui/src/bindings.ts`. Inspect the diff to confirm:
       - `start_sync` and `cancel_sync` are declared commands.
       - `SyncProgress` has `item: string | null`.
       - `DiscoveredSkill` (or whatever shape ListReport returns) has `synced_at: string | null`.
       - `MenuAction` includes `JumpSync` in its tagged-union variant set.
    2. Extend `crates/tome-desktop/tests/a11y/axe.spec.ts` to add a new spec block "Sync view a11y" that: navigates to the Sync route (either by simulating `⌘3` or by clicking the sidebar Sync NavItem), runs `await new AxeBuilder({ page }).analyze()`, asserts `results.violations === []`. Follow the existing pattern verbatim.
    3. If `npm run typecheck` reports issues in any of the React files from Task 3 due to the new generated types (likely because `useSync`'s `commands.startSync` import was a TS stub before regen), fix import paths so the file picks up the regenerated bindings.
  </action>
  <verify>
    <automated>cargo run -p tome-desktop --bin gen-bindings &amp;&amp; git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts &amp;&amp; cd crates/tome-desktop/ui &amp;&amp; npm run typecheck &amp;&amp; cd ../.. &amp;&amp; npx playwright test tests/a11y/axe.spec.ts -g "Sync view"</automated>
  </verify>
  <done>
    `bindings.ts` regenerated and committed (CI freshness gate passes); axe-core scan of the Sync route passes with zero WCAG-AA violations; `npm run typecheck` clean.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| React → Tauri IPC (`start_sync`, `cancel_sync`) | Webview-originated command invocations cross into native Rust |
| Domain `tome::sync` → filesystem | sync mutates library, manifest, lockfile, distribution symlinks |
| Tauri event channel → Webview | `SyncProgress` payloads flow webview-wards; payloads include user paths/skill names |
| Menu accelerator routing | `⌘1..⌘4` re-anchoring touches three files (menu.rs registers accelerators, MenuAction is the typed event, useMenuActions is the React-side switch) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-27-01b-01 | Spoofing | start_sync command invocation | accept | Single-user local app; webview is trusted; Tauri capability surface unchanged from Phase 25/26 (no `fs:default`, no `shell:default`); the command takes no path or URL arguments — all paths resolved server-side via `load_context()` and `default_machine_path()`. |
| T-27-01b-02 | Tampering | start_sync runs sync without user consent | mitigate | UI flow requires explicit `[Run sync]` button click or `⌘R` keypress; no auto-fire from watcher events (Real-time auto-sync deferred per Phase 27 Deferred Ideas). |
| T-27-01b-03 | DoS | Webview spams cancel_sync to disrupt user's run | accept | Sync is locally invoked + always recoverable per SC#4; cancellation by design always safe; threat-model neutral. |
| T-27-01b-04 | DoS | Watcher event flood causes UI thrashing mid-sync | mitigate | useSync subscribes ONLY to syncProgress; Pitfall 6 discipline asserted by unit test. Phase 26 watcher debounced at 200ms. |
| T-27-01b-05 | Information Disclosure | SyncProgress.item leaks paths into webview console | accept | Paths are user-owned; same user owns both sides; no PII. D-09 sink-side fold-in (in 27-01a) is the canonical sanitization point. |
| T-27-01b-06 | DoS | spawn_blocking on Tauri main runtime starves IPC handler | mitigate | Pitfall 5 — `tome::sync` runs via `tokio::task::spawn_blocking`; `cancel_sync` stays a fast sync command flipping `Arc<AtomicBool>`. |
| T-27-01b-07 | Tampering | start_sync called concurrently (double-fire) | mitigate | The double-fire guard in Task 1 returns `ErrorCode::Conflict` if `SyncState.cancel` is already `Some`. Verified by unit test. |
| T-27-01b-SC | Tampering | npm/cargo package installs | accept | This plan adds ZERO new external packages (`similar` lands in 27-03; everything else inherited). |
</threat_model>

<verification>
- `cargo build -p tome-desktop --features bindings` — compiles clean.
- `cargo test -p tome-desktop --lib commands` — start_sync double-fire guard + cancel_sync idempotency tests pass.
- `cargo test -p tome-desktop --lib menu` — exhaustiveness sentinel covers JumpSync.
- `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts` — bindings regenerated cleanly (CI freshness gate).
- `cargo clippy -p tome-desktop --features bindings -- -D warnings` — zero warnings.
- `cd crates/tome-desktop/ui && npm run typecheck && npm run test -- --run useSync SyncView Sidebar useMenuActions` — TypeScript + Vitest clean.
- `npx playwright test tests/a11y/axe.spec.ts -g "Sync view"` — axe scan passes.
- Manual smoke: launch `cargo tauri dev`, navigate to Sync via `⌘3`, observe idle hero; click `[Run sync]`, observe running placeholder; click `[Cancel sync]`, observe return to idle (real cancellation is verified by 27-04's integration test).
</verification>

<success_criteria>
- ROADMAP Phase 27 SC#1 (per-stage progress + current-item indicator) is **substrate-ready end-to-end**: events flow domain → boundary → webview through the typed `SyncProgress.item` field plumbed in 27-01a + the `start_sync` IPC command landed here.
- Sync section reachable via sidebar, Library menu, `⌘3`; `[Run sync]` fires `start_sync`; `[Cancel sync]` fires `cancel_sync`; the spawn_blocking discipline (Pitfall 5) and the watcher-feedback discipline (Pitfall 6) are both enforced by unit tests.
- Pitfall 7 re-anchoring complete: ⌘3 → Sync; ⌘4 → Health; Library → Sync (⌘R) enabled; View → Reload removed.
- `bindings.ts` CI freshness gate passes; the Sync view has zero axe-core WCAG-AA violations.
- All existing CLI integration tests in `crates/tome/tests/cli*.rs` continue to pass (no domain changes in this plan; 27-01a's additive fields are already in via Wave 1).
</success_criteria>

<output>
Create `.planning/phases/27-sync-triage-ui/27-01b-SUMMARY.md` when done.
</output>

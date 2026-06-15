---
phase: 26-read-only-views-alpha-cut
plan: 06
subsystem: ui+watcher
tags: [tauri, react, notify, fsevents, watcher, events, silent-refresh, view-06, nf-05, pitfall-10]

requires:
  - phase: 26-read-only-views-alpha-cut
    plan: 01
    provides: "useStatus hook (with updatedAt for Pill); Pill atom + CSS animation; StatusView gated on showUpdatedPill; LockfileState + MachinePrefsSummary on StatusReport"
  - phase: 26-read-only-views-alpha-cut
    plan: 02
    provides: "useSkills hook; SkillsView with React Aria Virtualizer + selection state; commands::list_skills"
provides:
  - "Rust-side file watcher on a dedicated std::thread using notify 8.2 + notify-debouncer-full 0.7 (200ms debounce) — watches manifest / lockfile / library / machine.toml; emits four typed tauri-specta events; OQ-3 resolved Rust-side rather than JS-side"
  - "ManifestChanged / LockfileChanged / LibraryChanged / MachinePrefsChanged — unit-struct events derived Clone+Debug+Serialize+specta::Type+tauri_specta::Event"
  - "WatcherEvent enum + WatcherPaths struct + spawn_watcher_with_sink — the testable seam isolating FSEvents+debounce from the Tauri AppHandle for the integration test"
  - "Per-hook event-subscription matrix (RESEARCH §Anti-Patterns 'no subscribe-everything sugar'): useStatus → 4 events; useSkills → 3 events (no lockfile); useSkillDetail / useDoctorReport entries land with plans 26-03 / 26-05"
  - "Generic useTauriEvent React hook with cleanup-on-unmount + late-listen race guard (cancelled flag)"
  - "SkillsView selection-preservation across watcher refresh; hidden role=status aria-live region announces 'Selected skill was removed.' when the selected skill disappears externally"
  - "Updated Pill in StatusView force-remounts on every watcher refetch via key={updatedAt} — CSS fade restarts cleanly on rapid bursts"
  - "Path-canonicalization fix inside the watcher — symlinked tome_home prefixes (TempDir, ~/.config symlink chains) no longer cause silent zero-event behaviour"
  - "Rust integration test (tests/watcher_smoke.rs, #[cfg(target_os='macos')]): own-process write + external write fire watcher events within 2000ms — Assumption A4 verified; Pitfall 10 confirmed NOT real on current macOS"
  - "load_context promoted to pub on the commands module so main.rs::setup can derive TomePaths for the watcher"
  - "tome::default_machine_path re-exported at the tome crate root (narrow re-export — single function, not the whole machine module)"

affects: [26-03, 26-05, 26-07, 26-08]

tech-stack:
  added:
    - "cargo: notify ^8.2 (resolved 8.2.0, license CC0-1.0) — FSEvents-backed file watcher"
    - "cargo: notify-debouncer-full ^0.7 (resolved 0.7.0, license MIT OR Apache-2.0) — debouncer with rename-stitching"
    - "cargo dev-dep: tempfile ^3 (matches workspace dev-dep) — TempDir for the integration test"
  patterns:
    - "Testable-seam refactor: factor a pub spawn_watcher_with_sink<F: Fn(WatcherEvent) + Send + 'static>(paths, sink) + a pub WatcherEvent enum behind the production spawn_watcher; tests record events into Arc<Mutex<Vec<WatcherEvent>>> without spinning up a Tauri runtime"
    - "FSEvents path canonicalization at watcher startup — comparisons against FSEvents-reported canonical paths use parent-dir canonicalize + parent.join(file_name) rebuild; library_dir is canonicalized directly because it's a starts_with prefix"
    - "Per-hook event subscription matrix: useStatus subscribes to ALL 4 events; useSkills subscribes to 3 (skips lockfile because list shape doesn't depend on it); the hooks that ship in 26-03 / 26-05 will subscribe to their own subsets following this template (no 'subscribe everything' sugar)"
    - "useTauriEvent late-listen race guard: cancelled flag set in the unmount cleanup is checked before each handler dispatch AND used to call the eventual unlisten if the listener promise resolves after unmount"
    - "Force-remount via key={updatedAt} on the Updated Pill — CSS animation restarts cleanly on rapid watcher bursts (would otherwise show a frozen mid-fade)"
    - "Hidden visually-hidden aria-live region for one-shot 'selected skill removed' announcement — inline-style clip rect rather than another CSS module slot"

key-files:
  created:
    - "crates/tome-desktop/src/watcher.rs"
    - "crates/tome-desktop/tests/watcher_smoke.rs"
    - "crates/tome-desktop/ui/src/hooks/useTauriEvent.ts"
  modified:
    - "crates/tome-desktop/Cargo.toml — notify + notify-debouncer-full + tempfile dev-dep"
    - "crates/tome-desktop/src/commands.rs — load_context promoted from fn to pub fn"
    - "crates/tome-desktop/src/lib.rs — pub mod watcher; 4 events registered in collect_events!"
    - "crates/tome-desktop/src/main.rs — setup closure spawns spawn_watcher after builder.mount_events; spawn errors propagate as setup errors"
    - "crates/tome-desktop/ui/src/bindings.ts — regenerated; manifestChanged / lockfileChanged / libraryChanged / machinePrefsChanged exported"
    - "crates/tome-desktop/ui/src/hooks/useStatus.ts — subscribes to all 4 events; refetch sets updatedAt = Date.now() so the Pill flashes on every silent refresh"
    - "crates/tome-desktop/ui/src/hooks/useSkills.ts — subscribes to manifest + library + machine-prefs (lockfile skipped — doesn't shift list shape)"
    - "crates/tome-desktop/ui/src/views/SkillsView.tsx — selection-preservation + aria-live announcement when the selected skill disappears externally"
    - "crates/tome-desktop/ui/src/views/StatusView.tsx — key={updatedAt} on Pill so the CSS fade restarts cleanly on every watcher refetch"
    - "crates/tome/src/lib.rs — pub use machine::default_machine_path (narrow re-export — function only, not the whole module)"
    - "Cargo.lock — notify 8.2.0 + notify-debouncer-full 0.7.0 + file-id pulled in"

key-decisions:
  - "OQ-3 resolved Rust-side (not @tauri-apps/plugin-fs::watch JS-side): typed tauri-specta events route React refetches correctly, the IPC surface stays auditable (no fs:default permission widening), and the debouncer's rename-stitching matches the atomic temp+rename pattern manifest/lockfile/machine.toml already use"
  - "Choose 'extract testable core' over 'tauri::test mock_builder' for the integration test: spawn_watcher_with_sink<Fn(WatcherEvent)> isolates FSEvents from the Tauri runtime, the production spawn_watcher becomes a thin glue around it. Cleaner test surface and avoids enabling Tauri's test feature on every cargo test"
  - "Assumption A4 holds — FSEvents DOES fire for own-process writes on current macOS APFS (verified by Test 1). No mitigation in actions::set_skill_disabled needed; the Phase-26 mutation will propagate through the watcher loop naturally"
  - "Plan 26-03 not yet landed: set_skill_disabled doesn't exist at this commit. Per the spawn note, Test 1 exercises the OS-level atomic-rename path the future action will take (machine::save uses the same write pattern). Once 26-03 lands, the test can be extended to call actions::set_skill_disabled directly"
  - "useSkillDetail (26-03) and useDoctorReport (26-05) hooks don't exist yet — the plan's event-subscription matrix lists their target subsets (manifest+library+machine-prefs / manifest+library+lockfile). The respective plans will wire those subscriptions following the useStatus/useSkills template established here"
  - "Narrow re-export tome::default_machine_path rather than lifting the whole machine module from pub(crate) to pub — keeps the GUI's dependency surface on tome small (single function), matches the existing pattern (pub use paths::TomePaths, pub use manifest::hash_directory)"
  - "200ms debounce window matches SC#1 's 200ms refresh target; the watcher fire + Tauri event + React refetch round-trip needs ~50ms headroom on top — the debounce window dominates the latency budget"
  - "key={updatedAt} on the Pill — without it, React reuses the same DOM node and the CSS animation freezes mid-fade on rapid watcher bursts. With it, every refetch remounts the Pill and the animation restarts cleanly"

patterns-established:
  - "Rust-side file watcher behind a typed tauri-specta event surface — Phase 27's longer-running sync flow can extend the same module with progress/cancel events"
  - "WatcherEvent + WatcherPaths + spawn_watcher_with_sink seam — testable without Tauri; future watchers (e.g. config-file watcher) can copy the pattern"
  - "FSEvents path-canonicalization preflight at watcher startup — anywhere the watched path may traverse a symlinked prefix (TempDir, ~/.config symlink chains, /var → /private/var) the comparison gate must use canonical paths"
  - "Generic useTauriEvent React hook — every typed event consumer pulls it in; no consumer rolls its own listen+unlisten dance"
  - "Per-hook event-subscription matrix doc inline in each hook's docstring — the 'why this subset' rationale is the contract"

requirements-completed: [VIEW-06, NF-05]

# Metrics
duration: ~70min
completed: 2026-05-29
---

# Phase 26 Plan 06: Read-only views alpha cut — File watcher + silent live refresh Summary

**The "GUI cannot drift from disk" loop closes — a Rust-side notify 8.2 + notify-debouncer-full 0.7 watcher emits four typed tauri-specta events; each Phase-26 hook subscribes to only the events it depends on; StatusView's Updated pill flashes on every refresh; SkillsView preserves selection (and announces removal via aria-live); a macOS integration test proves Pitfall 10 is not real and pins Assumption A4 down with a concrete failure mode.**

## Performance

- **Duration:** ~70 min (Task 0 human-verify checkpoint resolved upstream, then Tasks 1–4 executed)
- **Started:** 2026-05-29T04:35Z (continuation after checkpoint)
- **Completed:** 2026-05-29T05:47Z
- **Tasks:** 4 / 4 (plus Task 0 human-verify resolved before continuation spawn)
- **Files:** 3 created (`watcher.rs`, `watcher_smoke.rs`, `useTauriEvent.ts`); 10 modified

## Commits

| Task | Commit | Description |
| ---- | ------ | ----------- |
| 1 | `28727d8` | Rust watcher module + 4 typed events + `load_context` `pub` + main.rs setup wiring |
| 2 | `ee64a0c` | `useTauriEvent` hook + per-hook event subscriptions for silent refresh |
| 3 | `f8a080b` | `key={updatedAt}` Pill remount so CSS fade restarts on every watcher refetch |
| 4 | `f3495f9` | Watcher integration tests + FSEvents path-canonicalization bug fix |

## Accomplishments

- **Watcher module (`crates/tome-desktop/src/watcher.rs`):** 4 typed `tauri_specta::Event` unit structs (`ManifestChanged`, `LockfileChanged`, `LibraryChanged`, `MachinePrefsChanged`) and a `spawn_watcher` that runs `notify-debouncer-full` on a dedicated `std::thread`. Watches PARENT dirs (per Pitfall 5) and filters to exact file paths inside the debouncer callback. Recursive only for the library root.
- **Testable seam:** A `WatcherEvent` enum + `WatcherPaths` struct + `spawn_watcher_with_sink<F: Fn(WatcherEvent) + Send + 'static>(paths, sink)` are exposed `pub` so the integration test records events into an `Arc<Mutex<Vec<WatcherEvent>>>` without spinning up a Tauri runtime. The production `spawn_watcher` is now a thin glue translating each `WatcherEvent` to a typed `Event::emit(&app)` call.
- **Main.rs wiring:** the `setup` closure derives `TomePaths` via the newly-`pub` `commands::load_context()` and calls `watcher::spawn_watcher`. Spawn errors propagate as setup errors — Tauri reports them as failed app startup (clear feedback if FSEvents can't init).
- **Bindings regenerated:** `bindings.ts` now exports `events.manifestChanged`, `events.lockfileChanged`, `events.libraryChanged`, `events.machinePrefsChanged` alongside the existing `events.syncProgress`.
- **React `useTauriEvent` hook (`hooks/useTauriEvent.ts`):** generic listener with cleanup-on-unmount and a late-listen race guard (`cancelled` flag — if the component unmounts before `listen()` resolves, the eventual `unlisten` fires immediately).
- **Per-hook subscription matrix:** `useStatus` subscribes to all four events; `useSkills` subscribes to manifest + library + machine-prefs only (lockfile changes don't shift the list shape). `useStatus` sets `updatedAt = Date.now()` on every refetch so StatusView's "Updated" pill flashes for ~2s on every silent refresh.
- **SkillsView selection preservation:** when the selected skill disappears from a refetched list (renamed/removed externally), the selection clears AND a one-shot `role="status"` aria-live region announces "Selected skill was removed."
- **Pill remount on every refetch:** `<Pill key={updatedAt}>` force-remounts the Pill on each tick so the CSS fade animation restarts cleanly when watcher events arrive in rapid succession.
- **Integration test (`tests/watcher_smoke.rs`, `#[cfg(target_os = "macos")]`):**
  - `own_process_write_to_machine_toml_fires_machine_prefs_changed` — proves Pitfall 10 is NOT real on current macOS: an in-process atomic temp+rename to `machine.toml` fires `MachinePrefs` within 2000ms.
  - `external_write_to_manifest_fires_manifest_changed` — proves NF-05's concurrency promise: an "external" atomic temp+rename to `.tome-manifest.json` (the same write `tome sync` performs) fires `Manifest` within 2000ms.
- **Production bug discovered and fixed via the test (Rule 1):** the original watcher compared FSEvents-reported canonical paths against the raw user-supplied paths. Any `tome_home` that traversed a symlinked prefix (TempDir under `/var → /private/var`, user-installed symlink chains in `~/.config`) would receive zero watcher events because the path comparisons never matched. Fixed by canonicalizing parent dirs at watcher init and rebuilding file paths via `parent.join(file_name)`; canonicalizing `library_dir` directly because it's a `starts_with` prefix.
- **Domain re-export (`tome::default_machine_path`):** narrow `pub use` of the single function the watcher needs — keeps `tome::machine` `pub(crate)` to non-test callers while giving the watcher the canonical machine.toml path.

## Test Run

```
$ cargo test -p tome-desktop --test watcher_smoke
running 2 tests
test own_process_write_to_machine_toml_fires_machine_prefs_changed ... ok
test external_write_to_manifest_fires_manifest_changed ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Full workspace: `cargo test --workspace` — all 879 tome unit tests + 13 desktop unit tests + the 2 watcher_smoke tests pass.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] FSEvents path-canonicalization bug in production watcher code**
- **Found during:** Task 4 (writing the integration test — both tests failed initially despite the code looking correct)
- **Issue:** On macOS, FSEvents canonicalizes paths it reports (e.g. `/var/folders/...` → `/private/var/folders/...`). The original watcher held the un-canonicalized user-supplied paths, so the `path == &manifest_path` equality checks never matched. Any `tome_home` traversing a symlinked prefix would receive zero watcher events — silent failure with no log output.
- **Fix:** Canonicalize parent dirs at watcher startup via `std::fs::canonicalize`; rebuild file paths via `parent.join(file_name)`. Canonicalize `library_dir` directly (it's a `starts_with` prefix match). Fall back to the original path on canonicalize errors (file not created yet — comparison won't match anyway until the file exists).
- **Files modified:** `crates/tome-desktop/src/watcher.rs::run_watcher_with_sink`
- **Commit:** `f3495f9` (rolled into the same commit as the test, since the test is what exposed the bug)
- **Production impact:** Would have surfaced any time a user's `tome_home` or `~/.config` traversed a symlink — common on dotfiles-driven setups.

**2. [Rule 1 — Bug] Clippy `collapsible_if` and `manual_contains` warnings**
- **Found during:** Tasks 1 and 4 clippy gate
- **Issue:** Two minor idiomatic lints flagged by clippy `-D warnings`.
- **Fix:** Used `if X && let Err(e) = Y` collapse in watcher.rs; used `Vec::contains` instead of `.iter().any` in the test.
- **Commits:** `28727d8` (watcher fix), `f3495f9` (test fix).

### Plan-Level Adjustments (documented per spawn instructions)

**3. [Deferral] Test does NOT call `tome::actions::set_skill_disabled` — that command lands later in Wave 3 (plan 26-03)**
- **Reason:** Per the orchestrator's spawn note: "if 26-03's `set_skill_disabled` isn't yet wired at execution time, write the test to trigger a watcher event by direct `machine.toml` file mutation rather than invoking the (not-yet-existing) command, and note the deferral."
- **What was done instead:** Test 1 performs an atomic temp+rename to `machine.toml` from the test thread — the same OS-level write pattern `tome::machine::save` (and the future `actions::set_skill_disabled`) will use. This still verifies Pitfall 10 (Assumption A4): an in-process write produces a watcher event. Once 26-03 lands, the test can be extended to call `set_skill_disabled` directly without altering the contract being verified.
- **Tracking:** The test docstring + the deferral entry in this SUMMARY are the breadcrumbs.

**4. [Deferral] React `useSkillDetail` and `useDoctorReport` event subscriptions not wired in this plan**
- **Reason:** Those hooks ship with plans 26-03 (detail pane) and 26-05 (Health view), which haven't landed yet — they're Wave 3.
- **What was done:** The plan's event-subscription matrix is documented in this SUMMARY's `key-decisions` and in the watcher.rs / `useTauriEvent.ts` docstrings. The hooks shipping in 26-03/26-05 will subscribe to their own subsets (manifest+library+machine-prefs for detail; manifest+library+lockfile for doctor) following the `useStatus`/`useSkills` template established here.

**5. [Deferral] `npm test` was specified in the plan's verify gate but no Vitest is set up in `crates/tome-desktop/ui/`**
- **Reason:** Prior plans (26-01, 26-02) did not introduce a JS test framework, and adding Vitest + happy-dom + per-component tests is architecturally substantial (would touch every existing TSX file's testability assumptions). Per Rule 4 territory, but the spawn prompt's success criteria do not list JS tests as required for this plan.
- **What was done:** TypeScript check (`npx tsc --noEmit`) passes. The Rust integration test (Task 4) is the load-bearing verification for the watcher loop.
- **Tracking:** Follow-up — when Phase 27 or a later phase introduces JS testing infra, the watcher hook subscriptions become easy to cover.

## Known Stubs

None — every wired surface is backed by real data.

## Threat Flags

None — no new IPC commands, no new permissions, no new network surface. The watcher reads paths derived from existing `TomePaths`; events cross the in-process Tauri IPC the same way `SyncProgress` did in Phase 25.

## Self-Check: PASSED

All commits exist in `git log --all`:
- `28727d8` (Task 1)
- `ee64a0c` (Task 2)
- `f8a080b` (Task 3)
- `f3495f9` (Task 4)

All created files exist:
- `crates/tome-desktop/src/watcher.rs`
- `crates/tome-desktop/tests/watcher_smoke.rs`
- `crates/tome-desktop/ui/src/hooks/useTauriEvent.ts`

---
phase: 26-read-only-views-alpha-cut
plan: 03
subsystem: ui
tags:
  - tauri
  - react
  - react-aria-components
  - specta
  - tauri-plugin-opener
  - tauri-plugin-clipboard-manager
  - machine-toml

requires:
  - phase: 25-rust-core-extraction
    provides: TomeError IPC boundary; specta bindings infra; load_context() pattern
  - phase: 26-01
    provides: Status view atoms (Badge, KeyValueRow, StatusDot, Pill, DirectoryTable); tokens.css; ContentPane/Window/Sidebar shell
  - phase: 26-02
    provides: list_skills Tauri command; SkillsView 3-column layout; SkillListRow + React Aria Virtualizer; useSkills hook
  - phase: 26-06
    provides: ManifestChanged / LockfileChanged / LibraryChanged / MachinePrefsChanged typed events; useTauriEvent hook; file watcher with synchronized FSEvents registration

provides:
  - tome::actions module (resolve_source_path, set_skill_disabled) shared between TUI and GUI
  - tome::skill::SkillDetail + collect_detail aggregate for the right-pane payload
  - tome::skill::SkillFrontmatterView specta-friendly projection of SKILL.md frontmatter
  - SKILL_BODY_MAX_BYTES (1 MiB cap with truncation marker for the markdown body)
  - 4 new Tauri commands (get_skill_detail, set_skill_disabled, open_source_folder, copy_path)
  - tauri-plugin-opener + tauri-plugin-clipboard-manager wired with narrow capability grants
  - React DetailHeader + Button atoms; SkillContextMenu (right-click contract D-07); useSkillDetail + useSkillActions hooks; ariaLabels.ts pure templates
  - ⌘C / ⌘O / ⌘D keyboard shortcuts scoped to "a skill is selected"
  - The lone Phase-26 mutation surface (D-06: "Disable on this machine" via atomic machine.toml write)

affects:
  - phase: 26-04
    via: SkillDetail.body field is the input MarkdownBody renders
  - phase: 27-sync-triage-ui
    via: tome::actions pattern (cross-surface mutations through a single helper) extends to sync/remove/reassign in 27+
  - phase: 28-configuration-ui
    via: ⌘D keyboard binding HIG audit in 26-07 may move ⌘D off this surface

tech-stack:
  added:
    - tauri-plugin-opener@2.5.4 (npm + cargo)
    - tauri-plugin-clipboard-manager@2.3.2 (npm + cargo)
  patterns:
    - Cross-surface action helpers (tome::actions::*) bridging TUI and GUI without sharing presentation glue
    - Specta-friendly DTO projection (SkillFrontmatterView) when the canonical type carries types specta can't easily round-trip (serde_yaml::Value)
    - JSON-string encoding for recursive unstructured values (sidesteps specta's recursive-inline-type panic on serde_json::Value)
    - Aria-live announcement pattern for state-changing actions (D-06)
    - Shared action dispatcher hook (useSkillActions) reused by DetailHeader + context menu + keyboard shortcuts

key-files:
  created:
    - crates/tome/src/actions.rs
    - crates/tome-desktop/ui/src/components/Button.tsx
    - crates/tome-desktop/ui/src/components/Button.module.css
    - crates/tome-desktop/ui/src/components/DetailHeader.tsx
    - crates/tome-desktop/ui/src/components/DetailHeader.module.css
    - crates/tome-desktop/ui/src/components/SkillContextMenu.tsx
    - crates/tome-desktop/ui/src/components/SkillContextMenu.module.css
    - crates/tome-desktop/ui/src/hooks/useSkillDetail.ts
    - crates/tome-desktop/ui/src/hooks/useSkillActions.ts
    - crates/tome-desktop/ui/src/lib/ariaLabels.ts
  modified:
    - crates/tome/src/lib.rs (pub mod actions; pub mod skill; pub use SkillName)
    - crates/tome/src/skill.rs (added SkillDetail / SkillFrontmatterView / collect_detail; ~190 LoC)
    - crates/tome/src/browse/app.rs (apply_toggle's Global scope routes through actions::set_skill_disabled)
    - crates/tome/Cargo.toml (no specta feature changes beyond comment cleanup)
    - crates/tome-desktop/Cargo.toml (+ tauri-plugin-opener + tauri-plugin-clipboard-manager)
    - crates/tome-desktop/capabilities/main.json (+ opener:default + clipboard-manager:allow-write-text)
    - crates/tome-desktop/src/main.rs (.plugin(opener) + .plugin(clipboard_manager))
    - crates/tome-desktop/src/commands.rs (+ 4 commands)
    - crates/tome-desktop/src/lib.rs (registered 4 commands in make_builder)
    - crates/tome-desktop/ui/src/bindings.ts (regenerated)
    - crates/tome-desktop/ui/src/views/SkillsView.tsx (DetailColumn, SkillContextMenuRow, ⌘C/⌘O/⌘D)
    - crates/tome-desktop/ui/package.json + package-lock.json (+ 2 JS plugins)

key-decisions:
  - "Cross-surface tome::actions module owns path resolution + machine.toml mutation; presentation (clipboard, opener, focus) stays per-surface. TUI keeps arboard + Command::new for opener; GUI uses Tauri plugins."
  - "TUI's apply_toggle Global scope routes through actions::set_skill_disabled (then re-loads in-memory prefs from disk). PerDir blocklist / allowlist arms stay inline — they are TUI-only semantics (HARD-21 D-BROWSE-1 routing) and out of scope for the GUI's D-06 global-only mutation."
  - "SkillFrontmatterView is a specta-friendly DTO projection of SkillFrontmatter. metadata/extra fields ship JSON-encoded strings (sidesteps specta's recursive-inline-type panic on serde_json::Value; avoids dragging in the serde_yaml specta feature). 26-03's React UI doesn't render frontmatter today; the string shape is a forward-compatible carrier for 26-04+."
  - "SKILL_BODY_MAX_BYTES = 1 MiB with UTF-8-boundary-safe truncation + marker (RESEARCH Security §Markdown body size). Body always read from the library-canonical SKILL.md (v0.10 contract) — Owned managed skills' source_path may live behind package-manager visibility gates."
  - "Context menu uses neutral labels and resolves disabled state at click time (single getSkillDetail per click) rather than pre-fetching detail for every row at list mount. The DetailHeader continues to show the precise Disable / Enable label via the selected row's useSkillDetail."
  - "⌘C / ⌘O / ⌘D scoped via document-level listener inside DetailColumn (gated on `detail !== null`). HIG audit deferred to plan 26-07 — Pitfall 9 flags ⌘D for potential 'Don't Save' conflicts."
  - "set_skill_disabled does NOT emit a manual MachinePrefsChanged event. The Phase-26 watcher (plan 26-06, post-merge fix synchronizing FSEvents registration) fires the event for own-process atomic temp+rename writes naturally; useSkillDetail + useSkills both subscribe and refetch."
  - "useSkillActions also calls refetch() inline after a successful Disable click for instant UI feedback before the watcher round-trip. Both reads are idempotent so the duplicate refetch is safe."

patterns-established:
  - "Cross-surface action module (tome::actions::*): pure-Rust helpers shared by TUI + GUI, with presentation glue per surface. Extends to sync/remove/reassign in Phase 27+ per PATTERNS.md."
  - "Specta DTO projection for unstructured fields: when the canonical Rust type carries types specta can't round-trip (serde_yaml::Value) or that emit recursive inline types (serde_json::Value), wrap in a parallel specta-friendly View with String-encoded extras."
  - "Aria-live announcement pattern for D-06-style state mutations: visually-hidden `role=\"status\" aria-live=\"polite\"` region populated by a shared action-dispatcher hook, auto-cleared after 2s."
  - "Shared useSkillActions hook reused by DetailHeader, SkillContextMenu, and ⌘C/⌘O/⌘D shortcuts — single source of truth for the three actions' wiring and error handling."

requirements-completed:
  - VIEW-03

# Metrics
duration: ~70min
completed: 2026-05-29
---

# Phase 26 Plan 03: Detail Pane + 3 Actions (D-05 + D-06 + D-07) Summary

**Right-pane DetailHeader with SOURCE/HASH/SYNC metadata, three action buttons (Open / Copy / Disable), and a right-click context menu — all driving the lone Phase-26 mutation `machine.toml::disabled` via the shared `tome::actions` module reused by the browse TUI.**

## Performance

- **Duration:** ~70 min (Task 1 ~30m, Task 2 ~40m; excludes Task 0 human-verify gate)
- **Started:** 2026-05-29 ~14:35 JST (after Task 0 approval)
- **Completed:** 2026-05-29 15:49 JST
- **Tasks:** 3 (Task 0 human-verify gate + Task 1 + Task 2)
- **Files created:** 10 (1 Rust + 9 UI)
- **Files modified:** 12 (4 tome + 5 tome-desktop Rust + 3 UI)

## Accomplishments

- **`tome::actions` module landed as the cross-surface mutation pattern** — `resolve_source_path` + `set_skill_disabled` are now the single source of truth for path resolution + global `disabled` toggling. The browse TUI's Global scope arm routes through it; the GUI's `set_skill_disabled` Tauri command routes through it. Both ride the same atomic temp+rename. This is the template Phase 27+ extends for sync / remove / reassign.
- **`SkillDetail` + `collect_detail` shipped the right-pane payload contract** — manifest entry + parsed frontmatter projection + machine-prefs disabled flag + capped markdown body. Body is UTF-8-boundary-safe-truncated at 1 MiB with an explicit marker.
- **DetailHeader renders all 3 spec'd rows** — skill name + Managed/Disabled badges, SOURCE/HASH/SYNC metadata grid (mono path with middle-ellipsis + `sha256:abc12345…` truncated hash + relative time), and the action button triplet (`[Open]` `[Copy → Copied]` `[Disable ↔ Enable]`).
- **The lone Phase-26 mutation works end-to-end** — click Disable → `set_skill_disabled` writes machine.toml atomically → file watcher fires `machine-prefs-changed` → React refetches → Disabled badge appears.
- **D-07 right-click context menu shipped** — every list row has a 3-action menu (Open / Copy / Disable) wired to the same handlers the DetailHeader uses.
- **⌘C / ⌘O / ⌘D keyboard shortcuts wired** — scoped to "a skill is selected"; HIG audit deferred to 26-07 (Pitfall 9 ⌘D conflict check).
- **All gates green** — 892 tome unit tests pass, 44 cli integration tests pass, 2 watcher_smoke tests pass, `cargo clippy --workspace --all-targets -- -D warnings` clean, `tsc --noEmit` clean, bindings.ts freshness gate clean.

## Task Commits

Each task was committed atomically:

1. **Task 0: Package legitimacy gate (Tauri plugins)** — _no commit_, human-verify gate. User confirmed `@tauri-apps/plugin-opener@2.5.4` + `@tauri-apps/plugin-clipboard-manager@2.3.2` + `tauri-plugin-opener` + `tauri-plugin-clipboard-manager` all resolve to `github.com/tauri-apps/plugins-workspace`, dual-licensed Apache-2.0 OR MIT.
2. **Task 1: `tome::actions` module + `SkillDetail` shape + browse TUI refactor** — `23e0b41` (feat)
3. **Task 2: Tauri commands + plugins + capabilities + DetailHeader + context menu + ⌘ shortcuts** — `fcf3bba` (feat)

_Note: TDD wasn't a flagged mode for this plan; tests were written alongside each task._

## Files Created/Modified

### Created (Rust + UI)

- `crates/tome/src/actions.rs` — Cross-surface skill action helpers (`resolve_source_path`, `set_skill_disabled`) shared by TUI + GUI. Atomic machine.toml write via the existing `machine::save` pattern. 7 unit tests.
- `crates/tome-desktop/ui/src/components/Button.tsx` + `Button.module.css` — Primary/secondary/small-fix variants over React Aria `<Button>`. Used by DetailHeader (3 action buttons); the small-fix variant is reserved for FindingRow in plan 26-05.
- `crates/tome-desktop/ui/src/components/DetailHeader.tsx` + `DetailHeader.module.css` — 3-row layout (name + badges, SOURCE/HASH/SYNC metadata grid, 3 action buttons). Mono path with middle-ellipsis, truncated `sha256:abc12345…` hash, relative time. Copy button label flips to "Copied" for 2s; Disable label flips to "Enable on this machine" when disabled.
- `crates/tome-desktop/ui/src/components/SkillContextMenu.tsx` + `.module.css` — Right-click wrapper (D-07). React Aria `MenuTrigger` controlled via `onContextMenu`. Three menu items dispatching the same actions the DetailHeader uses.
- `crates/tome-desktop/ui/src/hooks/useSkillDetail.ts` — Fetches `commands.getSkillDetail(name)` on every name change; subscribes to `manifest-changed` / `library-changed` / `machine-prefs-changed` per plan 26-06's matrix (lockfile-changed deliberately not subscribed — NF-05 contract).
- `crates/tome-desktop/ui/src/hooks/useSkillActions.ts` — Shared action dispatcher: Open / Copy (with 2s "copied" flash) / Disable (with aria-live announcement and explicit refetch for instant feedback).
- `crates/tome-desktop/ui/src/lib/ariaLabels.ts` — Pure functions producing the verbatim aria-label templates UI-SPEC §VoiceOver labels calls out.

### Modified

- `crates/tome/src/lib.rs` — `pub mod actions; pub mod skill; pub use discover::SkillName`. Skill module widened to `pub` so tome-desktop can call `skill::collect_detail` directly; SkillName re-exported at the crate root to keep `discover`'s wider surface out of the GUI's import path.
- `crates/tome/src/skill.rs` — `SkillDetail` + `SkillFrontmatterView` + `collect_detail` + `SKILL_BODY_MAX_BYTES` (1 MiB cap with UTF-8-boundary-safe truncation marker). 6 new unit tests covering frontmatter projection, JSON-string-encoded extras, metadata-map projection, full aggregate, truncation, and missing-skill error.
- `crates/tome/src/browse/app.rs` — `apply_toggle`'s Global scope arm now calls `tome::actions::set_skill_disabled` and re-loads in-memory prefs from disk. PerDir arms stay inline (HARD-21 D-BROWSE-1 routing — not shared with the GUI's D-06 surface).
- `crates/tome-desktop/Cargo.toml` — `tauri-plugin-opener = "2"` + `tauri-plugin-clipboard-manager = "2"` (Task 0 legitimacy gate passed).
- `crates/tome-desktop/capabilities/main.json` — Added `"opener:default"` + `"clipboard-manager:allow-write-text"` (narrow grants; no fs widening — watcher stays Rust-side per OQ-3).
- `crates/tome-desktop/src/main.rs` — `.plugin(tauri_plugin_opener::init())` + `.plugin(tauri_plugin_clipboard_manager::init())` before `.invoke_handler`.
- `crates/tome-desktop/src/commands.rs` — 4 new commands: `get_skill_detail`, `set_skill_disabled`, `open_source_folder`, `copy_path`. All route through `tome::actions::*` and `.map_err(TomeError::from)` at the boundary per the PATTERNS template.
- `crates/tome-desktop/src/lib.rs` — Registered the 4 new commands in `make_builder()::collect_commands!`.
- `crates/tome-desktop/ui/src/views/SkillsView.tsx` — Added `DetailColumn` (DetailHeader + error banner + aria-live region + ⌘C/⌘O/⌘D shortcuts) and `SkillContextMenuRow` (right-click wrapper). Placeholder "Detail pane ships in 26-03" removed.
- `crates/tome-desktop/ui/src/bindings.ts` — Regenerated. Contains the 4 new commands + `SkillDetail` + `SkillFrontmatterView` types.
- `crates/tome-desktop/ui/package.json` + `package-lock.json` — `@tauri-apps/plugin-opener@^2.5.4` + `@tauri-apps/plugin-clipboard-manager@^2.3.2` added (86 transitive deps).

## Decisions Made

See frontmatter `key-decisions` for the full list. Highlights:

- **Cross-surface module owns "what", not "how".** `tome::actions` computes paths + mutates machine.toml; clipboard / opener / focus management stay per-surface. The TUI keeps its `arboard` + `Command::new("open")` glue; the GUI uses the Tauri plugins.
- **TUI scope routing was preserved.** Only the Global scope arm of `apply_toggle` routes through `actions::set_skill_disabled`. PerDir blocklist / allowlist arms stay inline because their semantics (HARD-21 D-BROWSE-1) are TUI-only and not part of the GUI's D-06 surface.
- **JSON-string DTO sidestep for specta's recursive-inline limitation.** Initial implementation used `serde_json::Value` for `SkillFrontmatterView::metadata` + `extra`, but `cargo run -p tome-desktop --bin gen-bindings` panicked with "Recursive inline types cannot be expanded" (serde_json::Value contains `Vec<Value>` and `Map<String, Value>` recursively). Fix: encode each value as a JSON string blob; JS side `JSON.parse`s on demand. 26-03 doesn't render frontmatter anyway, so the string carrier is forward-compatible.
- **Context menu uses click-time state, not pre-fetched state.** Pre-fetching `getSkillDetail` for every list row would be N + 1 IPC calls at mount; instead the menu uses a neutral "Disable on this machine" label and resolves current state via a single `getSkillDetail` on click. The DetailHeader continues to show the precise Disable / Enable label via the selected row's `useSkillDetail`.
- **No manual MachinePrefsChanged emit.** Plan 26-06's post-merge fix synchronized FSEvents watch registration in `spawn_watcher_with_sink`, and the integration test confirms own-process atomic temp+rename writes to machine.toml fire `MachinePrefsChanged`. `actions::set_skill_disabled` therefore doesn't need a manual event signal — `useSkillDetail` + `useSkills` both subscribe and refetch on their own. `useSkillActions` still calls an explicit `refetch()` post-click for instant UI feedback before the watcher round-trip; both reads are idempotent.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] specta recursive-inline-type panic on `serde_json::Value`**
- **Found during:** Task 2 (first `cargo run -p tome-desktop --bin gen-bindings` after registering `get_skill_detail`)
- **Issue:** The plan called out `SkillFrontmatterView` with `BTreeMap<String, serde_json::Value>` extras. Enabling specta's `serde_json` feature fixed the missing `Type` impl, but specta-typescript then panicked with "Recursive inline types cannot be expanded" because `serde_json::Value` contains `Vec<Value>` + `Map<String, Value>` recursively.
- **Fix:** Changed `metadata` + `extra` from `BTreeMap<String, serde_json::Value>` to `BTreeMap<String, String>` carrying JSON-encoded blobs. Tests updated to `JSON.parse` the per-value strings. The CLI build no longer needs specta's `serde_json` feature; reverted that Cargo.toml change.
- **Files modified:** `crates/tome/src/skill.rs` (struct shape + `yaml_to_json` → `yaml_to_json_string`; 2 tests rewritten), `crates/tome/Cargo.toml` (reverted feature add).
- **Verification:** `cargo run -p tome-desktop --bin gen-bindings` writes a clean bindings.ts; `cargo test -p tome --lib skill::` 17 passed.
- **Committed in:** `fcf3bba` (Task 2 commit)

**2. [Rule 3 - Blocking] `tome::discover` + `tome::skill` modules were `pub(crate)`**
- **Found during:** Task 2 (`gen-bindings` build failed on the new commands.rs references)
- **Issue:** `crates/tome-desktop/src/commands.rs` couldn't import `tome::skill::collect_detail` (module was `pub(crate)`) or `tome::discover::SkillName` (also `pub(crate)`).
- **Fix:** Widened `pub mod skill;` (the new `SkillDetail` API is the GUI contract — narrow surface, no broader leakage) and re-exported `pub use discover::SkillName;` at the crate root (keeps `discover`'s wider surface — `DiscoveredSkill`, scanners — out of the GUI's import path while making the validated newtype reachable).
- **Files modified:** `crates/tome/src/lib.rs` (`pub mod skill` + `pub use discover::SkillName`); `crates/tome-desktop/src/commands.rs` (`use tome::SkillName` instead of `tome::discover::SkillName`).
- **Verification:** workspace clippy clean; gen-bindings writes bindings.ts cleanly.
- **Committed in:** `fcf3bba` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking-fix). No Rule 1 / Rule 2 / Rule 4.
**Impact on plan:** Both fixes were direct, mechanical, and necessary for `gen-bindings` to compile. No scope creep; no architectural changes.

## Issues Encountered

- **First gen-bindings attempt panicked on `serde_json::Value` recursion.** Resolved via the DTO change above (Deviation 1). Tracked under the auto-fix log; no separate follow-up.
- **Browse TUI `apply_toggle` borrow checker required scope adjustment.** When splitting the mutate + immutable-borrow steps for the PerDir arms, had to wrap the mutate in an inner block to drop the `&mut` before the subsequent `as_ref()` immutable borrow for `machine::save`. Trivial fix; the inner-block pattern is idiomatic and matches the comment.

## User Setup Required

None — no external service configuration. The two new Tauri plugins are dependency-only adds; their permissions live in the in-repo `capabilities/main.json`.

## Known Stubs

The DetailHeader currently does NOT render any of `SkillDetail.frontmatter` (the YAML metadata fields like description, license, allowedTools). UI-SPEC §DetailHeader doesn't ask for them either — Row 1 is skill name + badges only; Row 2 is the SOURCE/HASH/SYNC grid; Row 3 is the action triplet. The `frontmatter` field is carried across the IPC boundary so plan 26-04's MarkdownBody can render the body, and so a later plan can surface the description in a tooltip / side panel. **Not a stub in the bad sense** — the field is wired end-to-end; the GUI just doesn't display it yet. Documented here for the verifier so this isn't flagged as missing.

The context menu's `disabled` flag is hard-coded to `false` (the menu label always reads "Disable on this machine" pre-click). The actual toggle uses click-time state. UI-SPEC doesn't explicitly call out a per-row Enable label requirement — the DetailHeader covers the precise label requirement for the selected row. This is a deliberate IPC-cost trade-off; if 26-07's HIG audit asks for per-row Enable labels we'd need to add a per-row disabled-state cache (e.g. ship `disabled` on `DiscoveredSkill` via `list_skills`).

## Threat Flags

No new threat surface beyond what the plan's `<threat_model>` enumerated. The two new IPC commands (`get_skill_detail`, `set_skill_disabled`) hit the same `SkillName::Deserialize` validation gate Phase 25 established; the `open_source_folder` / `copy_path` commands resolve paths through `actions::resolve_source_path` and never accept a raw path from the webview. Capability JSON grants are narrowed to `opener:default` + `clipboard-manager:allow-write-text` — no fs widening.

## Watcher Contract (carried over from plan 26-06 post-merge findings)

**Plan 26-06's post-merge testing surfaced a real production race in `spawn_watcher_with_sink`** — the function returned before FSEvents watches were registered, causing some early own-process writes to be missed by the watcher. That race was fixed (commit `28727d8` / `8c8aab7` train) by waking on event arrival via `mpsc::recv_timeout` and synchronizing registration. The integration test `own_process_write_to_machine_toml_fires_machine_prefs_changed` confirms own-process atomic temp+rename writes to `machine.toml` fire `MachinePrefsChanged`.

**Consequence for plan 26-03:** `actions::set_skill_disabled` does NOT emit a manual `MachinePrefsChanged` event. The watcher fires it naturally. `useSkillDetail` and `useSkills` both subscribe and refetch. `useSkillActions` calls an additional explicit `refetch()` post-click for instant UI feedback before the watcher round-trip; both reads are idempotent.

## Next Phase Readiness

Plan **26-04** (MarkdownBody — VIEW-04 / D-08) can lift the existing `SkillDetail.body` field straight into a `react-markdown` + `remark-gfm` renderer; the field is already in the bindings, already capped, already UTF-8-safe. The DetailHeader scrolls fixed at top while the body becomes the scrollable region — `SkillsView.module.css::detailColumn` already has `overflow: auto`; 26-04 will split it into header + body containers.

Plan **26-07** (HIG audit) should validate the ⌘C / ⌘O / ⌘D bindings against the macOS HIG (Pitfall 9 explicitly calls out ⌘D for "Don't Save" conflicts in `NSAlert` and similar). If a conflict is found, the rebind happens entirely inside `DetailColumn` (one `keydown` handler) and the DetailHeader's button labels stay verbatim.

Plan **27-*** (Sync + triage UI) inherits the `tome::actions` pattern. Future cross-surface helpers (e.g. `actions::sync_now`, `actions::reassign_skill`) will follow the same shape — pure-Rust helpers in `tome::actions::*`, presentation glue per surface.

## TDD Gate Compliance

This plan was not flagged TDD (`tdd="true"` not set). All 13 new tests (7 actions + 6 skill) were authored alongside the implementation in their respective task commits — standard `feat(...)` commits rather than separated RED → GREEN → REFACTOR.

## Self-Check: PASSED

Verified after writing this SUMMARY:

```
[ -f crates/tome/src/actions.rs ]                                                 → FOUND
[ -f crates/tome-desktop/ui/src/components/DetailHeader.tsx ]                      → FOUND
[ -f crates/tome-desktop/ui/src/components/SkillContextMenu.tsx ]                  → FOUND
[ -f crates/tome-desktop/ui/src/hooks/useSkillDetail.ts ]                          → FOUND
[ -f crates/tome-desktop/ui/src/lib/ariaLabels.ts ]                                → FOUND
git log --oneline --all | grep -q 23e0b41                                          → FOUND
git log --oneline --all | grep -q fcf3bba                                          → FOUND
```

---
*Phase: 26-read-only-views-alpha-cut*
*Completed: 2026-05-29*

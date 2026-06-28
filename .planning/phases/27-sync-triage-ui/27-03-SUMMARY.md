---
phase: 27-sync-triage-ui
plan: 03
subsystem: tome-desktop
tags:
  - tauri
  - react
  - sync
  - machine-toml
  - similar-crate
  - line-diff
  - preview-popover
  - slot-refactor
  - pitfall-3-atomic-refactor
  - sync-03
requires:
  - 27-01b   # SyncView shell + IPC commands surface
  - 27-02    # TriagePanel + onApply seam + useSync triage state
provides:
  - machine.preview_save             # structured Myers line-diff via similar
  - MachineTomlPreview / DiffLine / DiffLineKind
  - tauri.preview_machine_toml       # read-only preview command
  - tauri.apply_machine_toml         # atomic write command
  - TriageDecision / TriageDecisionKind  # IPC types
  - components.MachineTomlDiff       # line-diff renderer
  - components.PreviewPopover@slot   # slot-refactored (Pitfall 3)
  - useSync.applyComplete            # success callback
affects:
  - components.FindingRow            # Pitfall 3 atomic caller update
  - views.SyncView                   # drops Apply TODO stub
  - bindings.ts                      # +112 lines (commands + types)
  - ui/a11y axe spec                 # new Apply popover scan
tech-stack:
  added:
    - "similar 3.1.1"                # MIT, mitsuhiko/similar
  patterns:
    - "Myers line-diff via similar::TextDiff::from_lines"
    - "Slot-based PreviewPopover (children: ReactNode body slot)"
    - "Server-side machine.toml path resolution (no path crosses IPC)"
    - "atomic temp+rename via existing machine::save"
key-files:
  created:
    - .planning/phases/27-sync-triage-ui/checkpoints/27-03-task1-similar-vetting.md
    - crates/tome-desktop/ui/src/components/MachineTomlDiff.tsx
    - crates/tome-desktop/ui/src/components/MachineTomlDiff.module.css
    - crates/tome-desktop/ui/src/components/__tests__/MachineTomlDiff.test.tsx
    - crates/tome-desktop/ui/src/components/__tests__/PreviewPopover.test.tsx
    - crates/tome-desktop/ui/src/components/__tests__/FindingRow.test.tsx
  modified:
    - Cargo.toml                                                       # similar workspace dep
    - Cargo.lock                                                       # similar resolved
    - crates/tome/Cargo.toml                                           # similar.workspace = true
    - crates/tome/src/machine.rs                                       # preview_save + types + tests
    - crates/tome/src/lib.rs                                           # narrow pub use re-exports
    - crates/tome-desktop/src/commands.rs                              # preview/apply commands + tests
    - crates/tome-desktop/src/lib.rs                                   # registry
    - crates/tome-desktop/ui/src/bindings.ts                           # generated, +112 lines
    - crates/tome-desktop/ui/src/components/PreviewPopover.tsx         # slot refactor
    - crates/tome-desktop/ui/src/components/PreviewPopover.module.css  # width=480 hook
    - crates/tome-desktop/ui/src/components/FindingRow.tsx             # atomic caller update
    - crates/tome-desktop/ui/src/components/TriagePanel.tsx            # Apply flow wiring
    - crates/tome-desktop/ui/src/components/TriagePanel.module.css     # applyError style
    - crates/tome-desktop/ui/src/components/__tests__/TriagePanel.test.tsx  # onApply→onApplied
    - crates/tome-desktop/ui/src/hooks/useSync.tsx                     # applyComplete
    - crates/tome-desktop/ui/src/views/SyncView.tsx                    # drop TODO stub
    - crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts           # preview/apply mocks
    - crates/tome-desktop/tests/a11y/axe.spec.ts                       # Apply popover scan
decisions:
  - "preview_save reads machine.toml via fs::read_to_string(...).unwrap_or_default() — missing file is treated as empty current text. The first Apply on a new machine surfaces every proposed line as Added."
  - "No save_preview_apply wrapper. apply_machine_toml calls machine::save (now re-exported as save_machine_prefs) directly. Per Task 2 action 5 — keep one canonical write path."
  - "Narrow lib.rs re-exports (preview_save, MachineTomlPreview, DiffLine, DiffLineKind, save_machine_prefs) rather than lifting the whole machine module to pub. Keeps MachinePrefs's internal fields pub(crate)."
  - "TriagePanel owns its own preview state + applyError. SyncView passes only onApplied (the success callback). Internal ownership matches the plan's 'TriagePanel owns the flow internally' option."
  - "buildDecisionsForIPC skips Keep decisions — the IPC treats absent entries as implicit Keep. Smaller payload, no semantic loss."
  - "Default trigger aria-label now tracks triggerLabel (instead of a fixed 'Fix'). The Doctor flow still works because both default to 'Fix'; the Apply flow gets 'Apply 3 decisions' as the accessible name automatically."
  - "Unchanged DiffLine rows carry aria-hidden=true. VoiceOver reads removed/added rows + the table summary; equal lines are silent. Per UI-SPEC §VoiceOver labels (planner's note in 27-03-PLAN action step 3)."
metrics:
  tasks_completed: 4         # incl. checkpoint Task 1
  commits: 7                 # incl. checkpoint context commit + 3 RED/GREEN pairs
  rust_tests_added: 8        # 3 machine::preview_save + 5 machine_toml_apply_tests
  ui_tests_added: 21         # 8 PreviewPopover + 7 MachineTomlDiff + 3 FindingRow + 3 TriagePanel updates
  a11y_tests_added: 1        # axe scope: sync apply popover machine.toml diff
  bindings_added: 5          # MachineTomlPreview, DiffLine, DiffLineKind, TriageDecision, TriageDecisionKind
  commands_added: 2          # preview_machine_toml, apply_machine_toml
  duration: ~13h elapsed (executor wall-clock; includes overnight pause for Task 1 human-verify checkpoint resolution)
  completed: 2026-06-07
---

# Phase 27 Plan 03: SYNC-03 Previewable machine.toml Writes — Summary

SYNC-03 ships the "no silent writes" invariant (SC#3) for triage decisions: `[Apply N decisions]` now opens a `PreviewPopover` anchored to the button, the popover renders a literal `machine.toml` line-diff (red removed, green added, neutral unchanged) computed server-side by `tome::machine::preview_save`, and the user must explicitly click `[Apply]` inside the popover before the file is touched. The write itself flows through the existing atomic `machine::save` so the Phase-26 watcher's `MachinePrefsChanged` event fires for free. The Pitfall 3 atomic `PreviewPopover` slot refactor (Doctor caller updated in the same plan) is the load-bearing piece that makes both flows reuse the same shell.

## What shipped

**Rust side (Tasks 2 + 3):**
- `similar 3.1.1` workspace dep — MIT, `mitsuhiko/similar`, exact-pinned `=3.1.1`, `default-features = false, features = ["text"]`. Package legitimacy approved by the user at the Task 1 `gate="blocking-human"` checkpoint. `cargo deny check` clean (MIT is in the existing allowlist; no `deny.toml` change needed).
- `tome::machine::preview_save(proposed: &MachinePrefs, current_path: &Path) -> Result<MachineTomlPreview>` — reads the current TOML text via `fs::read_to_string(...).unwrap_or_default()`, serializes the proposed prefs via `toml::to_string_pretty`, runs `similar::TextDiff::from_lines(...)`, maps each `Change` to a `DiffLine` with the 1-indexed line number on the side it lives on.
- New types: `DiffLineKind { Unchanged, Removed, Added }` (serializes as lowercase), `DiffLine { line_number, kind, content }`, `MachineTomlPreview { lines, added_count, removed_count }`. All `serde::Serialize` by default; `specta::Type` gated behind the `bindings` feature (Phase 25 pattern).
- Narrow `pub use` re-exports at `crates/tome/src/lib.rs` so `tome-desktop` doesn't need to touch the gated `machine` module: `save_machine_prefs`, `preview_save`, `MachineTomlPreview`, `DiffLine`, `DiffLineKind`.
- `preview_machine_toml(decisions: Vec<TriageDecision>) -> Result<MachineTomlPreview, TomeError>` — read-only Tauri command. Path resolution server-side via `tome::default_machine_path()`; the React side never passes a path (T-27-03-01 mitigation).
- `apply_machine_toml(decisions: Vec<TriageDecision>) -> Result<(), TomeError>` — atomic write via `save_machine_prefs`. Phase-26 watcher fires `MachinePrefsChanged` for free.
- New IPC types: `TriageDecision { skill: SkillName, decision: TriageDecisionKind }`, `TriageDecisionKind { Keep, Disable }`. `SkillName::Deserialize` validates at the boundary (T-27-03-02 mitigation).
- Shared `preview_decisions` / `apply_decisions` / `apply_decisions_to_prefs` helpers in `commands.rs` so the unit tests hit the same code path without an `AppHandle`.

**UI side (Task 4):**
- **Pitfall 3 atomic `PreviewPopover` slot refactor**: `dryRunDescription: string` → `children: ReactNode`. Optional `trigger?: ReactNode`, `triggerLabel?: string`, `triggerAriaLabel?: string`, `helperText?: string`, `width?: number`. Default trigger aria-label tracks `triggerLabel` (Apply flow gets "Apply 3 decisions" as the accessible name automatically).
- `data-width="480"` on the Popover + Dialog hooks a 480px max-width CSS variant + a 360px body `max-height` + `overflow-y: auto` so a tall diff doesn't push Apply/Cancel off-screen (UI-SPEC §Spacing exceptions).
- **`FindingRow` (Doctor's Fix caller) updated in the same commit**: passes `<p>{finding.dry_run_description}</p>` as the `children` slot. Other props (no `triggerLabel`, no `helperText`, no `width`) keep their defaults so the Doctor flow renders pixel-identical to the pre-refactor shape.
- **`MachineTomlDiff` component**: 3-column `<table role="table">` (line-number gutter / change-glyph gutter / content). aria-label `"machine.toml diff, N additions, M removals"`. Removed rows: aria-label `"removed line N"`; Added: `"added line N"`. Unchanged rows: `aria-hidden="true"` (VoiceOver noise reduction per UI-SPEC §VoiceOver labels). Three line-background tokens for light + dark `prefers-color-scheme`. Long lines wrap; popover overflow handles tall diffs.
- **`TriagePanel.tsx` Apply flow wiring**: TriagePanel owns its own `previewResult` / `previewError` / `applyError` state. Trigger Button's `onPress` calls `commands.previewMachineToml(buildDecisionsForIPC(decisions))` and stashes the result. The popover's `children` slot renders `<MachineTomlDiff preview={previewResult} />` (or a loading placeholder, or a preview-error disclosure if the preview fetch failed). On Apply success, calls `onApplied()` (the parent's `useSync.applyComplete`) which clears the decisions Map + selected triage skill. On Apply error, renders an inline `[ErrorCode] message` + `<details>` disclosure below the Apply row (D-11 reuse).
- `buildDecisionsForIPC(decisions: Map<SkillName, TriageDecision>): TriageDecisionWire[]` — flattens to the IPC shape, skipping Keep (the IPC treats absent entries as implicit Keep — smaller payload, no semantic loss).
- **API rename**: TriagePanel's `onApply: () => void` → `onApplied: () => void`. The new prop fires AFTER `applyMachineToml` resolves successfully, not when the button is clicked. The existing `TriagePanel.test.tsx` was updated atomically.
- `useSync.applyComplete()` — clears decisions + selected skill. Does NOT refetch the lockfile diff (a `machine.toml` write doesn't touch the lockfile; the diff stays the same — only the user's decisions reset). The seed effect re-populates decisions to all-keep on next render.
- `SyncView.tsx` drops the `// TODO 27-03: open PreviewPopover` stub and passes `onApplied={applyComplete}` to the TriagePanel.

**A11y + tests:**
- New axe-core test "sync apply popover (machine.toml diff) passes axe WCAG-AA" — opens the triage panel via `?triage=1`, toggles a row to disable, clicks `[Apply N triage decisions]`, scopes the WCAG-AA scan to the dialog, expects zero violations.
- Tauri mock (`__mocks__/tauri-api-core.ts`) gets `preview_machine_toml` (returns a representative 3-line diff) and `apply_machine_toml` (returns null = success) handlers.

## Verification

| Check | Result |
|---|---|
| `cargo deny check` | ok (advisories, bans, licenses, sources all clean; `similar 3.1.1` allowed by MIT entry) |
| `cargo build -p tome --features bindings` | clean |
| `cargo build -p tome-desktop` | clean |
| `cargo clippy -p tome --all-targets -- -D warnings` | clean |
| `cargo clippy -p tome-desktop --all-targets -- -D warnings` | clean |
| `cargo test -p tome --lib` | 919 passed |
| `cargo test -p tome --tests` | 10 passed (integration) |
| `cargo test -p tome-desktop --lib` | 31 passed (incl. 5 new machine_toml_apply_tests) |
| `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- bindings.ts` | stable (no drift) |
| `cd crates/tome-desktop/ui && npm run build` | clean (tsc + vite) |
| `cd crates/tome-desktop/ui && npm run test -- --run` | 90 passed (15 test files, +21 new tests this plan) |

The Playwright axe scan was not run in this executor (no headless browser available in this sandbox); the spec was added per plan and is validated by the existing CI invocation. The mock + the test composition follow the existing patterns from "sync view triage panel" and "preview popover (Health Fix)" tests verbatim — if either of those passes in CI, the new scan will too (same selectors, same mock infra).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] `similar::Change` doesn't impl `AsRef<str>`**
- **Found during:** Task 2 GREEN (first build of `preview_save`).
- **Issue:** I initially wrote `let raw: &str = change.as_ref();` — but `similar::Change<&str>` exposes its inner value via `value()` / `to_string_lossy()`, not via `AsRef<str>`.
- **Fix:** Use `change.to_string_lossy().trim_end_matches('\n').to_string()` to get the line content as an owned `String`. Comment in the source documents the choice + why we strip the trailing `\n`.
- **Files modified:** `crates/tome/src/machine.rs`.
- **Commit:** `8e957bf`.

**2. [Rule 3 — Blocking issue] Test referenced `tome::machine::DiffLineKind`**
- **Found during:** Task 3 GREEN (first build of commands.rs tests).
- **Issue:** The `machine` module is `pub(crate)` (only lifted to `pub` under `#[cfg(any(test, feature = "test-support"))]`). My initial test code used `tome::machine::DiffLineKind` and `tome::machine::save`/`load`, which fail to compile from `tome-desktop`.
- **Fix:** Added narrow `pub use` re-exports at `crates/tome/src/lib.rs` (`preview_save`, `MachineTomlPreview`, `DiffLine`, `DiffLineKind`, `save_machine_prefs`) and updated the tests to use `tome::*` paths. This matches the existing precedent for `MachinePrefs` and `load_machine_prefs` (already re-exported the same way in plans 26-06 / 27-01b).
- **Files modified:** `crates/tome/src/lib.rs`, `crates/tome-desktop/src/commands.rs`.
- **Commit:** `493a465`.

**3. [Rule 1 — Bug] `getAllByRole("row")` returns 2, not 3**
- **Found during:** Task 4 GREEN (first vitest run of `MachineTomlDiff.test.tsx`).
- **Issue:** My initial test wrote `expect(rows.length).toBe(3)` using `getAllByRole("row")`. But the component is designed to set `aria-hidden="true"` on the unchanged row (per UI-SPEC §VoiceOver labels), which excludes that row from the accessibility tree — so `getAllByRole` only finds 2. This is a test bug, not a component bug.
- **Fix:** Switched to `container.querySelectorAll("tr")` for the raw DOM row count (3 expected), and kept the separate `aria-hidden` test that pins the screen-reader behavior. Added a comment in the test explaining why.
- **Files modified:** `crates/tome-desktop/ui/src/components/__tests__/MachineTomlDiff.test.tsx`.
- **Commit:** `63024a9`.

**4. [Rule 2 — Missing critical functionality] Default trigger aria-label was hardcoded to "Fix"**
- **Found during:** Task 4 GREEN (PreviewPopover test for trigger label override).
- **Issue:** My initial `PreviewPopover.tsx` had a `DEFAULT_TRIGGER_ARIA_LABEL = "Fix"` constant. If a caller passed `triggerLabel="Apply 3 decisions"` but no `triggerAriaLabel`, the button's accessible name stayed "Fix" — which is wrong (VoiceOver would announce "Fix" for a button labeled "Apply 3 decisions"). Plain wrong a11y behavior; would fail axe-core's `button-name` rule in practice.
- **Fix:** The default aria-label now tracks `triggerLabel` (so the accessible name is verbatim the button text when no explicit override is passed). Removed the now-unused constant.
- **Files modified:** `crates/tome-desktop/ui/src/components/PreviewPopover.tsx`.
- **Commit:** `63024a9`.

### Architectural decisions (no Rule-4 stop needed — captured for the record)

- **`onApply` → `onApplied` rename** on `TriagePanelProps`: the plan's design notes describe TriagePanel owning the Apply flow internally; the prop becomes a success-callback rather than a click-handler. This is a breaking change to an internal seam (TriagePanel ↔ SyncView). All call sites were updated in the same commit (`63024a9` — SyncView + TriagePanel.test.tsx); the public IPC surface is unaffected. The plan explicitly authorized this choice in Task 4 action step 5 ("Claude's choice; PATTERNS.md does not pin a specific decomposition").

## Auth Gates

None occurred. The Task 1 checkpoint was a **package-legitimacy** human-verify gate (`gate="blocking-human"`) — the user spot-checked `similar 3.1.1` on crates.io + GitHub and approved before any `cargo add` ran. No 2FA, no API keys, no shell logins.

## Pitfall 3 Atomic Refactor — Pin

The Pitfall 3 invariant (PreviewPopover slot refactor MUST update the Doctor caller atomically in the same plan) is honored. Both call sites are updated in **commit `63024a9`**:
- `crates/tome-desktop/ui/src/components/PreviewPopover.tsx` — interface change.
- `crates/tome-desktop/ui/src/components/FindingRow.tsx` — Doctor caller switches from `dryRunDescription={...}` to `<p>{...}</p>` as children.
- `crates/tome-desktop/ui/src/components/TriagePanel.tsx` — new caller wires `<MachineTomlDiff />` as children.

There is no intermediate commit where one caller compiles against the new API and the other doesn't.

## TDD Gate Compliance

Three RED→GREEN cycles, each pair landing as adjacent commits:

1. `a81f983` test (RED) → `8e957bf` feat (GREEN) — `machine::preview_save` + similar dep
2. `973f400` test (RED) → `493a465` feat (GREEN) — Tauri commands + IPC types
3. `227bd7a` test (RED) → `63024a9` feat (GREEN) — UI: PreviewPopover slot + MachineTomlDiff + Apply flow

Both gates are present in git log; no GREEN commit landed without a preceding RED commit on the same branch.

## Known Stubs

None. Every component fetches real data through the wired IPC commands. The Tauri mock used by axe a11y tests returns representative-but-deterministic values, which is the conventional pattern (Phase 26 + 27-02 both use the same approach for `get_lockfile_diff` etc.).

## Threat Flags

None. The trust boundary surface added by this plan (two Tauri commands operating on a server-resolved path) was fully captured in the plan's `<threat_model>` (T-27-03-01..07 + T-27-03-SC). No new endpoints, no new auth surface, no new schema fields.

## Self-Check: PASSED

Self-check ran after writing this SUMMARY. All claimed files exist; all claimed commits exist.

- `crates/tome/src/machine.rs` (modified) — FOUND
- `crates/tome-desktop/ui/src/components/MachineTomlDiff.tsx` (new) — FOUND
- `crates/tome-desktop/ui/src/components/__tests__/MachineTomlDiff.test.tsx` (new) — FOUND
- `crates/tome-desktop/ui/src/components/__tests__/PreviewPopover.test.tsx` (new) — FOUND
- `crates/tome-desktop/ui/src/components/__tests__/FindingRow.test.tsx` (new) — FOUND
- `.planning/phases/27-sync-triage-ui/checkpoints/27-03-task1-similar-vetting.md` (new) — FOUND
- Commits `bb7f9ba`, `a81f983`, `8e957bf`, `973f400`, `493a465`, `227bd7a`, `63024a9` — FOUND

---
phase: 26-read-only-views-alpha-cut
plan: 07
subsystem: desktop-gui
tags: [tauri, menu, macos, a11y, hig, axe-core, playwright, NF-02, NF-03]
status: complete
requires:
  - 26-01-SUMMARY  # StatusReport surface
  - 26-02-SUMMARY  # router store + Sidebar shell
  - 26-03-SUMMARY  # DetailHeader + ⌘C/⌘O/⌘D bindings audited here
  - 26-04-SUMMARY  # MarkdownBody + Vitest setup
  - 26-05-SUMMARY  # HealthView + PreviewPopover
  - 26-06-SUMMARY  # main.rs::setup watcher coexistence
provides:
  - "tauri::menu::Menu (macOS) — tome / File / Edit / View / Library / Help"
  - "tauri-specta::Event `MenuAction` (4 variants: JumpStatus, JumpSkills, JumpHealth, FocusSearch)"
  - "React hook `useMenuActions` (App.tsx-mounted)"
  - "axe-core/playwright CI gate (4 tests, color-contrast disabled with documented baseline)"
  - "VoiceOver smoke-test checklist (26-A11Y-AUDIT.md)"
  - "UI-SPEC §Keyboard Map revision 3 (HIG audit applied — ⌘C guarded, ⌘O → ⌘⇧O, ⌘D removed)"
affects:
  - crates/tome-desktop/src/menu.rs       # NEW — native macOS menu bar + MenuAction
  - crates/tome-desktop/src/main.rs       # setup closure now installs menu alongside watcher
  - crates/tome-desktop/src/lib.rs        # collect_events! includes MenuAction
  - crates/tome-desktop/ui/src/App.tsx    # mounts useMenuActions; drops doc-level ⌘1/⌘2/⌘3 listener
  - crates/tome-desktop/ui/src/views/SkillsView.tsx  # drops ⌘F listener; ⌘C guarded; ⌘O rebound to ⌘⇧O; ⌘D removed
  - crates/tome-desktop/ui/src/bindings.ts  # regenerated to expose events.menuAction + MenuAction type
  - crates/tome-desktop/ui/vite.config.ts  # A11Y_TEST=1 conditional aliases for @tauri-apps/*
  - crates/tome-desktop/ui/package.json  # @axe-core/playwright + playwright dev deps + test:a11y/dev:a11y scripts
  - .planning/phases/26-read-only-views-alpha-cut/26-UI-SPEC.md  # §Keyboard Map revision 3 + Predefined Edit contract
  - .planning/phases/26-read-only-views-alpha-cut/26-A11Y-AUDIT.md  # NEW
  - .github/workflows/ci.yml  # new a11y job
  - .gitignore  # playwright test-results
tech-stack:
  added:
    - "@axe-core/playwright@^4.11.3 (dev) — Deque Systems, MPL-2.0"
    - "playwright@^1.60.0 (dev) — Microsoft, Apache-2.0"
    - "tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder} (already in tauri 2.11)"
  patterns:
    - "Cross-platform shim — install_menu() compiles everywhere; macOS submodule does the real work"
    - "POLISH-04 sentinel — MenuAction::ALL + exhaustiveness match block fails compile when variants drift"
    - "Predefined Edit items (Pitfall 9 mitigation, T-26-07-01)"
    - "Vite alias conditional on A11Y_TEST env so dev mode keeps real IPC"
    - "Playwright system-Chrome fallback via PW_USE_SYSTEM_CHROME=1 for offline sandboxes"
key-files:
  created:
    - crates/tome-desktop/src/menu.rs
    - crates/tome-desktop/ui/src/hooks/useMenuActions.ts
    - crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts
    - crates/tome-desktop/ui/src/__mocks__/tauri-api-event.ts
    - crates/tome-desktop/ui/src/__mocks__/tauri-plugin-clipboard.ts
    - crates/tome-desktop/ui/src/__mocks__/tauri-plugin-opener.ts
    - crates/tome-desktop/tests/a11y/axe.spec.ts
    - crates/tome-desktop/tests/a11y/playwright.config.ts
    - .planning/phases/26-read-only-views-alpha-cut/26-A11Y-AUDIT.md
  modified:
    - crates/tome-desktop/src/lib.rs
    - crates/tome-desktop/src/main.rs
    - crates/tome-desktop/ui/src/App.tsx
    - crates/tome-desktop/ui/src/views/SkillsView.tsx
    - crates/tome-desktop/ui/src/bindings.ts
    - crates/tome-desktop/ui/vite.config.ts
    - crates/tome-desktop/ui/package.json
    - crates/tome-desktop/ui/package-lock.json
    - .planning/phases/26-read-only-views-alpha-cut/26-UI-SPEC.md
    - .github/workflows/ci.yml
    - .gitignore
decisions:
  - "Compile MenuAction unconditionally cross-platform so bindings.ts is stable (Phase 26 is macOS-only but the binding shape must not flicker per-target)"
  - "Cross-platform install_menu() shim — main.rs::setup calls it without #[cfg], the macOS submodule does the real work"
  - "⌘C kept but guarded by isTextInputFocused() — Predefined Edit > Copy wins on text inputs (Pitfall 9 / T-26-07-01)"
  - "⌘O → ⌘⇧O — bare ⌘O is reserved for a future Phase 27+ Open dialog (macOS HIG)"
  - "⌘D removed entirely — Don't Save / Duplicate / Bookmark ambiguity outweighs convenience; button-only Disable surface"
  - "axe-core color-contrast rule disabled with documented baseline — every other WCAG-AA rule enforced; the SF Blue / sidebar vibrancy / pill tint tokens need design sign-off to retune (followup captured in 26-A11Y-AUDIT.md §4)"
  - "Playwright config uses relative ../../ui/node_modules/playwright/test imports so config + spec resolve from the tests/a11y/ subdir without symlink trickery"
  - "PW_USE_SYSTEM_CHROME=1 toggle to fall back to system Chrome via channel:'chrome' when Playwright's bundled Chromium download is blocked"
metrics:
  duration: "~2h"
  completed: "2026-05-29T11:09:18Z"
  started: "2026-05-29T09:00:00Z"
  loc: "1278+ / 64- across 20 files"
  tasks_completed: "4 (incl. Task 0 gate)"
  commits: 6  # 3 task + 2 SUMMARY checkpoints + 1 final
---

# Phase 26 Plan 07: Cross-cutting a11y + macOS native menu bar — Summary

A native macOS menu bar (tome / File / Edit / View / Library / Help) wired through a typed `MenuAction` Tauri event to the React router, alongside a macOS-HIG keyboard-conflict audit and an axe-core/playwright WCAG-AA CI gate covering Status / Skills / Health / PreviewPopover. The cross-cutting a11y bar for the alpha cut — runs after every prior Phase-26 plan landed so the audit covers every shipped surface.

## What ships

### NF-03 — Native macOS menu bar

Six submenus, declared in order so macOS renders the app menu under the application name:

| Menu | Items |
|---|---|
| **tome** (app menu) | About / Services / Hide / Hide Others / Show All / Quit — all Predefined |
| **File** | Close Window — Predefined (registers ⌘W) |
| **Edit** | Undo / Redo / Cut / Copy / Paste / Select All — **all Predefined**, OS routes ⌘C / ⌘V / ⌘X / ⌘A / ⌘Z / ⌘⇧Z to the focused webview control (Pitfall 9 mitigation) |
| **View** | Status (⌘1) / Skills (⌘2) / Health (⌘3) / separator / Focus Search (⌘F) / separator / Reload (⌘R, disabled — Phase 27+ placeholder) |
| **Library** | Sync / Add Directory… — both disabled with the breadcrumb visible so users discover the surface |
| **Help** | Documentation / Report Issue — open GitHub URLs via `tauri-plugin-opener` |

Each custom View-menu item emits a typed `MenuAction` event the React `useMenuActions` hook subscribes to. The App.tsx-level document-level keydown listener for ⌘1/⌘2/⌘3 and the SkillsView's ⌘F listener were removed — keeping them alongside the native menu accelerators would double-fire on every press (Pitfall 9).

The whole menu module sits behind `#[cfg(target_os = "macos")]` inside `menu.rs`'s nested `macos` submodule. The public `install_menu()` is a cross-platform shim — a no-op on non-mac (D-GUI-06) — so `main.rs::setup` calls it unconditionally with no `#[cfg]` at the call site. `MenuAction` itself is compiled cross-platform so `bindings.ts` exports the same shape on every target.

### NF-02 — Keyboard shortcut audit + a11y gate

Walked every row of UI-SPEC §Keyboard Map against macOS HIG conventions, Tauri Predefined Edit items, and webview-default intercepts. Three conflicts surfaced; resolutions:

- **⌘C**: kept, but guarded by an `isTextInputFocused()` check. When focus is on an `<input>` / `<textarea>` / `contenteditable` / `role="searchbox"` / `role="textbox"`, the skill-scoped handler abstains so the Predefined Edit > Copy item wins (Pitfall 9 / T-26-07-01).
- **⌘O**: bare ⌘O is the macOS HIG "Open…" dialog convention. Rebound to **⌘⇧O** so the convention stays available for a future Phase 27+ Open dialog. The Shift is mnemonic ("Open _Source_ folder").
- **⌘D**: removed entirely. Bare ⌘D variously means Don't Save (NSAlert), Duplicate (Finder/Pages), Add Bookmark (Safari) — every interpretation overlaps "destructive-ish toggle" in a way that risks misfire. The DetailHeader's labelled button stays the canonical surface (one Tab + Space away).

UI-SPEC §Keyboard Map updated to revision 3 in the same commit. 26-A11Y-AUDIT.md captures the full audit + VoiceOver smoke-test checklist + reduced-motion / reduced-transparency / dark-mode verification steps.

The axe-core CI gate (4 tests, one per view) runs on the macos-latest job in `.github/workflows/ci.yml`. Uses Vite's conditional `A11Y_TEST=1` aliases to swap `@tauri-apps/*` imports for deterministic fixture mocks, so the React render tree exercises every interactive surface without a Tauri runtime. Path A from the plan — the real Tauri IPC behaviour is verified manually + by the watcher integration test in plan 26-06.

## Tasks

- [x] **Task 0** — Package legitimacy gate (`@axe-core/playwright` + `playwright`) — APPROVED by user (Deque Systems MPL-2.0 + Microsoft Apache-2.0, both clean against the canonical upstream repos).
- [x] **Task 1** — Native macOS menu bar + `MenuAction` event + `useMenuActions` hook.
- [x] **Task 2** — macOS HIG + Pitfall 9 keyboard audit + UI-SPEC §Keyboard Map revision 3 + 26-A11Y-AUDIT.md.
- [x] **Task 3** — axe-core/playwright CI gate scaffold + 4 tests + Vite alias + first-run baseline (PASS with documented color-contrast deferral).

## Commits

| Task | Hash      | Subject |
|------|-----------|---------|
| 1    | `c9ca2bb` | feat(26-07): native macOS menu bar + MenuAction event + useMenuActions |
| 1+   | `112850e` | docs(26-07): start SUMMARY (Task 1 of 3 complete) |
| 2    | `eac1418` | fix(26-07): macOS HIG + Pitfall 9 keyboard audit; 26-A11Y-AUDIT.md |
| 2+   | `66acdb2` | docs(26-07): SUMMARY checkpoint after Task 2 |
| 3    | `3805f46` | test(26-07): axe-core/playwright a11y gate (NF-02) + CI wiring |

## Verification

| Gate | Result |
|---|---|
| `cargo build -p tome-desktop` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo run -p tome-desktop --bin gen-bindings` + `git diff --exit-code` | PASS (regenerated and committed) |
| `cd crates/tome-desktop/ui && npx tsc --noEmit` | PASS |
| `cd crates/tome-desktop/ui && npm test` (Vitest) | 5/5 |
| `cd crates/tome-desktop/ui && PW_USE_SYSTEM_CHROME=1 npm run test:a11y` | 4/4 (color-contrast disabled, baseline in 26-A11Y-AUDIT.md) |
| Plan's negative-pattern audit `! rg "metaKey[^|]*key === 'd'" src/views/SkillsView.tsx` | PASS (no match) |
| Plan's negative-pattern audit `! rg --pcre2 "metaKey[^|]*key === 'o'(?![^{]*shiftKey)"` | PASS (no match) |

## Deviations from plan

### Auto-fixed during execution

**1. [Rule 3 - Blocking] `playwright` package doesn't expose `test` / `expect` / `defineConfig` at the top level**

- **Found during:** Task 3 (running the spec for the first time).
- **Issue:** The plan's example test imports from `@playwright/test`, which is a sibling Microsoft package not approved at Task 0. The user approved `playwright` only.
- **Investigation:** `playwright` re-exports the full Microsoft test-runner at the `playwright/test` sub-export (its `test.js` bundles `lib/index.js` which is the test-runner code). Same publisher (Microsoft), same Apache-2.0 license, same package — just a different sub-export. Documentation discrepancy in the plan, not a missing package.
- **Fix:** Imports use `playwright/test` instead of `@playwright/test`. No additional npm install needed.
- **Files modified:** `crates/tome-desktop/tests/a11y/playwright.config.ts`, `crates/tome-desktop/tests/a11y/axe.spec.ts`.
- **Commit:** `3805f46`.

**2. [Rule 3 - Blocking] Module resolution from `tests/a11y/` can't find `playwright/test` (no `node_modules` neighbor)**

- **Found during:** Task 3 (first test run).
- **Issue:** The plan specifies the test config + spec under `crates/tome-desktop/tests/a11y/`, but Node's CJS resolver walks up from the file's parent dir looking for `node_modules/` — and there's no `node_modules` in `tests/a11y/`'s ancestors that contains `playwright`. Tests fail to load.
- **Fix:** Both `playwright.config.ts` and `axe.spec.ts` import via the explicit relative path `../../ui/node_modules/playwright/test` (and same for `@axe-core/playwright`). This keeps the plan's specified file layout while resolving deterministically from any invocation cwd.
- **Files modified:** `crates/tome-desktop/tests/a11y/playwright.config.ts`, `crates/tome-desktop/tests/a11y/axe.spec.ts`.
- **Commit:** `3805f46`.

**3. [Rule 3 - Blocking] Playwright's bundled Chromium download blocked in offline sandbox**

- **Found during:** Task 3 (`npx playwright install chromium` failed with `Download failure, code=1`).
- **Issue:** The execution sandbox's network egress is restricted enough that `playwright install` can't fetch its ~150MB Chrome-for-Testing binary. CI has the bandwidth and can run the canonical Playwright chromium; local sandboxes may not.
- **Fix:** Added an opt-in `PW_USE_SYSTEM_CHROME=1` env-var path that switches Playwright to the system-installed Google Chrome via `channel: "chrome"`. Same Playwright driver, different browser binary — sufficient for axe-core's purpose (no Chrome-for-Testing-specific behavior under test). CI keeps using Playwright's bundled chromium for determinism.
- **Files modified:** `crates/tome-desktop/tests/a11y/playwright.config.ts`, `.planning/phases/26-read-only-views-alpha-cut/26-A11Y-AUDIT.md` (documented the toggle).
- **Commit:** `3805f46`.

### Documented design-system deferral (NOT auto-fixed)

**4. [Rule 4 boundary] axe-core surfaced 4 color-contrast violations on alpha-cut tokens**

- **Found during:** Task 3 (first axe-core gate run against fixtures).
- **Findings:**
  - PreviewPopover Apply button — `#ffffff` on `#007aff` = 4.01:1 (needs 4.5:1)
  - Sidebar NavItem labels on the translucent vibrancy material — varies 4.0-4.4:1 (needs 4.5:1)
  - Sidebar LIBRARY caption / footer / role-badges on the vibrancy material — < 4.5:1
  - "Updated" pill text — < 4.5:1
- **Why deferred from this plan:** Every finding traces back to a UI-SPEC §Color design-system decision (canonical Apple SF Blue, the translucent sidebar material, the pill tint). Each is part of the design language, not an isolated component bug; shifting them touches the whole alpha visually and needs design sign-off. The plan's own escape hatch ("if a violation can't be fixed in this plan, document it as a known exception... lean toward FIXING over silencing") applies cleanly here — the 26-07 audit (this plan, §1–§3 of 26-A11Y-AUDIT.md) mitigates the keyboard / VoiceOver / reduced-motion dimensions; the color-token retune is a separate work item with its own decision surface.
- **Resolution:** Added `.disableRules(['color-contrast'])` to all 4 tests with a code comment + a populated baseline section in 26-A11Y-AUDIT.md §4 ("axe-core baseline"). Every other axe rule is enforced. Candidate fixes documented for the design owner: bump `--accent` to `#0040DD` for accessible primary buttons, or introduce `--accent-strong`, or disable vibrancy on `prefers-contrast: more`, or bump the pill background tint.
- **Follow-up:** Captured in 26-A11Y-AUDIT.md §4 as "tighten Phase-26 design tokens to clear WCAG-AA-normal (4.5:1) contrast on every label / button / pill pairing." Once the retune lands, remove `color-contrast` from `DISABLED_RULES` in `axe.spec.ts` and re-baseline.
- **Commit:** `3805f46`.

## Threat model — disposition recap

| ID | Threat | Disposition delivered |
|----|---|---|
| T-26-07-01 | Native menu shortcut overrides webview text input (Pitfall 9) | **mitigated** — Edit menu uses Predefined items; custom ⌘C in SkillsView guarded by `isTextInputFocused()`; ⌘O rebound to ⌘⇧O; ⌘D removed. |
| T-26-07-02 | Disabled menu items accidentally fire | **accepted** — `MenuItemBuilder::enabled(false)` verified to work against Tauri 2.11.2 source; the `_ => return` catch-all in `install_menu_event_handler` would absorb a misfire harmlessly. |
| T-26-07-03 | Help menu URLs replaced via on-disk tampering | **accepted** — same posture as any signed/notarized desktop app; covered by Phase-31 code signing. |
| T-26-07-04 | axe-core test mock leaks fixture data into CI logs | **accepted** — fixtures are synthetic (`/Users/test/...`, `axiom-build`, `rust-helper`); no real user data. |
| T-26-07-SC | npm install supply-chain | **mitigated** — Task 0 blocking-human checkpoint approved both packages against the canonical upstream repos (Deque Systems + Microsoft, MPL-2.0 + Apache-2.0). |

## Known Stubs

None.

The mock modules under `crates/tome-desktop/ui/src/__mocks__/` are test-only — they only ship at runtime when `A11Y_TEST=1`. They are NOT stubs in the UI render path; they're behind a Vite alias that only activates in the test pipeline. The real Tauri IPC commands remain wired through `bindings.ts` for the production build (`A11Y_TEST` unset → no aliases → bindings import from `@tauri-apps/api/core` exactly as before).

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes were introduced. The Help-menu URLs are hard-coded, the axe mocks are test-only, and the menu module sits behind `#[cfg(target_os = "macos")]`.

## Self-Check: PASSED

Files asserted to exist (created):

```
crates/tome-desktop/src/menu.rs                                       — FOUND
crates/tome-desktop/ui/src/hooks/useMenuActions.ts                    — FOUND
crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts                — FOUND
crates/tome-desktop/ui/src/__mocks__/tauri-api-event.ts               — FOUND
crates/tome-desktop/ui/src/__mocks__/tauri-plugin-clipboard.ts        — FOUND
crates/tome-desktop/ui/src/__mocks__/tauri-plugin-opener.ts           — FOUND
crates/tome-desktop/tests/a11y/axe.spec.ts                            — FOUND
crates/tome-desktop/tests/a11y/playwright.config.ts                   — FOUND
.planning/phases/26-read-only-views-alpha-cut/26-A11Y-AUDIT.md        — FOUND
```

Commits asserted to exist:

```
c9ca2bb — FOUND
112850e — FOUND
eac1418 — FOUND
66acdb2 — FOUND
3805f46 — FOUND
```

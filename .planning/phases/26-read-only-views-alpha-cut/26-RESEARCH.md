# Phase 26: Read-only views — alpha cut — Research

**Researched:** 2026-05-29
**Domain:** React 19 + Tauri 2 desktop UI (read-only inspector), virtualisation, markdown, file-watching, a11y, native macOS menus
**Confidence:** HIGH on locked stack pieces (React/Tauri/specta scaffolded in Phase 25); HIGH on file-watcher choice (`notify 8.2`); HIGH on virtualisation pattern; MEDIUM on perf-bench harness shape (validated approach, exact tooling TBD); MEDIUM on detail-action handler refactor (one new pure-Rust module needed; no domain logic change).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions (do NOT relitigate)

Carried-forward from upstream (REQUIREMENTS / Phase 25 / D-GUI-*):
- **D-GUI-04** Frontend = **React 19** (irreversible from Phase 26 onward).
- **D-GUI-06** macOS only for v1.0.
- **D-GUI-07 / NF-05** App + CLI share `tome.lock` + `.tome-manifest.json`; file watcher reloads on external change. No GUI-private state.
- **D-GUI-08 / "no JS-side business logic"** Domain calls return structured types; the GUI renders results and dispatches commands only. Validation/planning/side-effects stay in Rust.

Phase 26 decisions:
- **D-01** Top-level window is a **3-column NavigationSplitView** (sidebar sections → middle list → right detail+preview), Mail/Notes/Xcode style. Replaces the scaffold's single-scroll dashboard.
- **D-02** Sidebar is **flat: Status / Skills / Health**. App **lands on Status** on launch. **Health item shows a badge count** when doctor findings exist (clears at zero findings).
- **D-03** VIEW-06 refresh = **silent live re-render** — no "refresh available" prompt. A transient "Updated" pill near the last-sync field acknowledges a watcher-driven refresh (fades ~2s). **Current selection (open skill) is preserved across refresh**.
- **D-04** VIEW-02 list controls = **always-on search field** pinned at the top of the list column (⌘F focuses it; fuzzy as-you-type matching the CLI's `nucleo` ranking) + **toolbar popup menus** for sort (name/source/recent) and group-by (none/source/role). **Defaults: sort=name, group=none.**
- **D-05** Right column = **compact metadata header + scrolling markdown body**. Header shows name, managed/local + disabled badges, source path, content hash, last sync, and the action buttons; the rendered SKILL.md body scrolls beneath. Mirrors the browse TUI's skill view.
- **D-06** **"Disable on this machine" SHIPS in Phase 26** as a live `machine.toml` write (not deferred). Rationale: single bounded write through existing machine-prefs path, low-risk, exercises the write → file-watcher → silent-refresh loop early.
- **D-07** Three actions (**open source dir / copy path / disable on this machine**) accessible from **both** the detail-pane header (primary buttons) **and** right-click context menu on list rows.
- **D-08** Markdown preview (VIEW-04) **renders in React via `react-markdown` + `remark-gfm`** at the **SC#4 subset: headings (H1–H3), lists, links, code blocks, inline bold/italic/code**. Markdown→HTML treated as presentation, not "business logic". **Supersedes VIEW-04's literal `browse/markdown.rs` wording** — the TUI renderer is hand-rolled, ratatui-only, and supports only headers + horizontal rules + inline bold-italic-code (no lists, links, or code blocks); it cannot be reused for a webview. Reconcile via REQUIREMENTS.md cleanup commit (non-blocking).
- **D-09** Confirmation model = **preview-then-confirm per fix**. Clicking "Fix" opens a small popover showing exactly what will change (reuse `doctor.rs` per-item dry-run/plan descriptions), then "Apply". Satisfies **NF-04** literally for every repair. All four `RepairKind` variants mutate the filesystem.
- **D-10 / D-11 / D-12** Per-item fixes only (no "Fix all" in alpha). Outcomes surface inline on finding rows; failures keep the row visible with the inline `TomeError` and context disclosure (SAFE-01). Non-fixable findings render with explanation + manual remediation hint and **NO Fix button**. Zero findings → explicit all-clear state; sidebar Health badge clears.
- **D-13 / D-14 / D-15 / D-16** Aesthetic bar = HIG-polished from the start. Component/a11y foundation = **React Aria (Adobe headless primitives) + custom macOS styling**, with **TanStack Virtual** for the VIEW-02 / NF-01 2000-row list — *see Open Question OQ-1 below; React Aria 1.17 ships its own native `<Virtualizer>` that may obviate TanStack Virtual.* Styling = **per-component CSS Modules (`*.module.css`) + small set of CSS custom-property design tokens** driven by `prefers-color-scheme`. Window chrome = **unified native titlebar/toolbar + traffic-light controls + vibrancy/translucent sidebar material**; follow system light/dark; respect `prefers-reduced-transparency`.

### Claude's Discretion
- SKILL.md links open in the system browser (Tauri opener); code blocks render plain (no syntax highlighting); empty-selection detail pane shows neutral placeholder.
- Status dashboard exact field layout (cards vs table grouping) — pick a HIG-aligned arrangement.
- Doctor pane flat-vs-grouped layout — UI-SPEC picks `AUTO-FIXABLE` / `NEEDS ATTENTION` grouping.
- Exact frontmatter fields shown + badge styling.
- React Aria vs Radix — D-14 locks "headless a11y primitives + custom styling"; React Aria is the chosen default. **This research confirms React Aria 1.17 is correct.**
- NF-01 perf-bench harness shape — planning detail (plan 26-08).
- Keyboard-shortcut map (NF-02) beyond the named ⌘F / ⌘R — fill out per macOS HIG (plan 26-07).

### Deferred Ideas (OUT OF SCOPE)
- Optional interim `v0.17.0` release (unreleased #542 migration + Phase 25 refactor; CLI-only since `tome-desktop` is cargo-dist-excluded).
- **Bulk "Fix all"** in the Health pane (D-10 — revisit if per-item fixing proves tedious).
- Light syntax highlighting in code blocks (optional polish).
- Sync / Config / Backup / mutating-ops UI — Phases 27–31.
- SKILL.md editing — v2 (GUI-EDIT-01).
- Stale `CLAUDE.md` "Current State" header refresh (cosmetic).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **VIEW-01** | Status dashboard — resolved `tome_home`, library dir, configured directories with role/type badges, skill count, last sync time, lockfile state, machine pref summary | `StatusReport` is already complete in `crates/tome/src/status.rs` and already wired through `get_status` (Phase 25 scaffold). Only fields missing for the visual spec: **lockfile state** (in-sync / out-of-sync) and **machine-prefs summary** ("N disabled"). See §"Standard Stack — Status dashboard". |
| **VIEW-02 + NF-01** | Virtualised skill list at 2000 skills @ 60 fps on M1 8GB during search-as-you-type, fuzzy search matching `nucleo` ranking, sort (name/source/recent), group-by (none/source/role) | TanStack Virtual + React Aria ListBox combo is feasible BUT React Aria 1.17 added a native `<Virtualizer>` wrapper that is the lower-risk path. See §"Standard Stack — Virtualisation" and OQ-1. Fuzzy match runs **JS-side via fuse.js or a thin nucleo port** to avoid per-keystroke Tauri command latency. See §"Standard Stack — Fuzzy search". |
| **VIEW-03** | Detail pane — frontmatter (parsed by existing `lint.rs`), source path, content hash, last sync, managed/local badge, disabled state. Three actions (open source / copy path / disable on this machine) wired to the same handlers as the existing browse TUI | One new pure-Rust module `tome::actions` exposes three handlers — `open_source(skill_name)`, `copy_path(skill_name) -> String` (caller copies), `set_skill_disabled(skill_name, bool)` — that both the TUI (via existing `App::execute_action`) and Tauri commands call into. Browse TUI's clipboard/opener code is currently inlined in `browse/app.rs` and would have to be refactored OR re-implemented at the Tauri edge via `tauri-plugin-opener` + `tauri-plugin-clipboard-manager`. See §"Architecture Patterns — Action handler refactor". |
| **VIEW-04** | Markdown preview renders SKILL.md body with the same Markdown subset as the CLI | **D-08 supersedes the literal wording** — the real subset is SC#4: H1–H3, lists, links, code blocks, inline bold/italic/code. Use **`react-markdown` 10.1.0 + `remark-gfm` 4.0.1** with `allowedElements`. See §"Standard Stack — Markdown". |
| **VIEW-05** | Health pane lists all `tome doctor` findings with one-click fix actions wired to the same repair handlers as interactive `tome doctor` | `doctor.rs::dispatch_repairs` is currently batch-only (loops `report.all_issues()` and applies all). Phase 26 needs a **per-finding repair API** — `doctor::repair_one(finding_id, …)` returning `Result<()>`. Finding IDs do not exist today; the `DiagnosticIssue` struct has no stable ID. Two options below. See §"Architecture Patterns — Doctor per-item fix" and OQ-2. |
| **VIEW-06** | Auto-refresh when manifest, lockfile, or library content changes externally; no stale UI after CLI sync | **`notify 8.2.0`** (stable) + **`notify-debouncer-full 0.7.0`** wired Rust-side, emitting Tauri events to React. Watch four roots: `.tome-manifest.json`, `tome.lock`, `library_dir/`, `~/.config/tome/machine.toml`. Debounce ~200 ms. See §"Architecture Patterns — File watcher". |
| **NF-01** | 60 fps perf budget verified via synthetic-skills bench in CI | Generate 2000 fake SKILL.md fixtures in a `TempDir`, point a Tauri test build at it, drive search-as-you-type via Playwright, measure FPS via `requestAnimationFrame` sampling. Bench lives at `crates/tome-desktop/tests/perf/`, runs on `macos-latest` only. See §"Architecture Patterns — Perf bench". |
| **NF-02** | All views keyboard-navigable; primary actions have keyboard shortcuts; VoiceOver labels on every interactive element | React Aria primitives (Button, ListBox, Menu, Popover, Dialog, SearchField) supply WAI-ARIA semantics + focus management out of the box. UI-SPEC §"Keyboard Map" already enumerates the shortcuts (⌘1/2/3, ⌘F, ⌘C/O/D, etc.) and the `aria-label` templates. Verify in CI with **`axe-core/playwright` 4.11.3**. See §"Standard Stack — A11y". |
| **NF-03** | Native macOS menu bar with File / Edit / View / Library / Help menus; respond to system appearance | **`tauri::menu::MenuBuilder` + `SubmenuBuilder`**, mounted in `make_builder()`/main.rs setup. Predefined items (`.cut()`/`.copy()`/`.paste()`/`.undo()`/`.redo()`) wire OS-native shortcuts free. Custom items emit menu events the React side subscribes to via `app.on_menu_event`. System appearance auto-tracks via CSS `prefers-color-scheme` (D-15/D-16). See §"Architecture Patterns — Menu bar". |
| **NF-05** | App and CLI share single `tome.lock` and `.tome-manifest.json`; concurrent CLI use while app open does not corrupt either file | Already true today: every write in `manifest.rs` / `lockfile.rs` / `machine.rs` uses atomic temp+rename. The file watcher (VIEW-06) provides the read-side reconciliation. No new concurrency primitives needed — the bug shape to watch is **read-during-rename**: re-read on `Modify(Data(Any))` / `Create(File)` events because rename targets surface as `Create`, not `Modify`. See §"Common Pitfalls — Atomic-rename read race". |
</phase_requirements>

## Summary

Phase 26 is the **first user-visible UI** for tome — a read-only inspector built on the Phase-25 React/Tauri/specta scaffold. Most of the technology choices are already locked by upstream context: React 19, React Aria, TanStack Virtual (modulo OQ-1), CSS Modules + design tokens, `react-markdown`, file-watching via Rust-side `notify`, and a native macOS menu bar via Tauri 2's built-in menu API.

The research's real value is in the **non-obvious bits**: (a) React Aria 1.17 ships a native `<Virtualizer>` that may make TanStack Virtual redundant (OQ-1); (b) the Rust domain needs **one** new module (`tome::actions`) + **one** doctor refactor (`repair_one(finding_id)`) — both small, both pure-Rust; (c) the file watcher must watch four roots and debounce ~200ms; (d) fuzzy search runs JS-side to meet the 60fps budget (avoid Tauri round-trip per keystroke); (e) markdown subset parity is policy-enforced via `react-markdown`'s `allowedElements`, not via sharing code with the TUI (the TUI renderer is too thin to share); (f) the perf-bench harness uses Playwright + `requestAnimationFrame` FPS sampling on a synthetic 2000-skill fixture.

**Primary recommendation:** Build incrementally on the existing scaffold — keep `make_builder()` as the single command/event registry, add commands in one PR per view, add the file watcher early (drives D-03's "no drift" feel), and resolve OQ-1 (React Aria native virtualizer vs TanStack Virtual) **before** plan 26-02 starts coding so the choice doesn't have to be reversed.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `StatusReport` computation (paths, counts, last-sync, unowned) | API / Rust (`tome::status::gather`) | — | Already lives there (Phase 25); GUI must not recompute. |
| Skill list collection + sorting | API / Rust (`tome::list::collect`) | — | Domain owns "what skills exist". Sort by name happens server-side; later re-sort (Source / Recent) happens client-side (display-only, no business logic). |
| Fuzzy filter ranking | Frontend / React | — | Per-keystroke at 60fps requires no IPC. Must match `nucleo` semantics — see §"Standard Stack — Fuzzy search". |
| Frontmatter parse for detail pane | API / Rust (`tome::skill::parse`) | — | Existing parser is the source of truth; surface as a Tauri command `get_skill_detail(name) -> SkillDetail`. |
| Markdown → HTML rendering | Frontend / React (`react-markdown`) | — | Pure presentation (D-08). No "business logic" violation. |
| Repair plan dry-run description (preview popover) | API / Rust (`doctor` per-finding dry-run text) | — | The wording must match the CLI's interactive prompt; centralised in Rust. |
| Repair execution (apply Fix) | API / Rust (`tome::doctor::repair_one`) | — | Filesystem mutation must go through the same code path the CLI uses. |
| "Disable on this machine" mutation | API / Rust (`tome::actions::set_skill_disabled`) | — | `MachinePrefs` write goes through `machine::save_checked`. |
| Clipboard write (copy path) | Frontend / React via `@tauri-apps/plugin-clipboard-manager` | API (fallback) | The browse TUI uses `arboard` Rust-side; the GUI can just call the JS clipboard plugin (it owns the user's intent click directly). Keep `tome::actions::copy_path` returning the *string* so both surfaces share the **path computation**. |
| Open Finder | Frontend / React via `@tauri-apps/plugin-opener` `revealItemInDir` | — | macOS-native reveal. No Rust-side opener needed (the TUI's `xdg-open`/`open` shellout is CLI-only). |
| File watcher (manifest/lockfile/library/machine.toml) | API / Rust (`notify` + `notify-debouncer-full`) | — | Must run on a background thread, emit Tauri events the webview listens for. Watching from JS via `@tauri-apps/plugin-fs::watch` is also possible — see OQ-3. |
| Menu bar (native macOS) | API / Rust (`tauri::menu::MenuBuilder`) | Frontend (event handlers) | Menus must be native (NF-03); React subscribes via `on_menu_event`. |
| Light/dark theme | Browser (`prefers-color-scheme` CSS) | — | No JS state; D-15 locked. |

## Standard Stack

### Core (already present in Phase 25 scaffold — DO NOT re-add)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| React | `19.1.0` | UI framework | D-GUI-04 locked. `[VERIFIED: package.json]` |
| React DOM | `19.1.0` | Renderer | Pairs with React 19. `[VERIFIED: package.json]` |
| TypeScript | `^5.7.2` | Type system | Phase 25 baseline. `[VERIFIED: package.json]` |
| Vite | `^6.0.7` | Bundler + dev server | Phase 25 baseline. `[VERIFIED: package.json]` |
| `@vitejs/plugin-react` | `^4.3.4` | React Vite plugin | Phase 25 baseline. `[VERIFIED: package.json]` |
| `@tauri-apps/api` | `^2` | Tauri JS bindings | Phase 25 baseline. `[VERIFIED: package.json]` |
| `tauri` (Rust) | `2.11` | Tauri 2 runtime | Phase 25 baseline. `[VERIFIED: Cargo.toml]` |
| `tauri-specta` + `specta` + `specta-typescript` | `=2.0.0-rc.25` / `=2.0.0-rc.25` / `0.0.12` | TS-binding generation | Phase 25 baseline. `[VERIFIED: Cargo.toml]` |

### Core (NEW — Phase 26 must add)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `react-aria-components` | `^1.17.0` (latest 1.17.0 published 2026-05-18) | Headless a11y primitives — `Button`, `ListBox`, `ListBoxItem`, `Menu`, `MenuTrigger`, `SearchField`, `Popover`, `Dialog`, `FocusRing`, `Virtualizer` (1.17+) | D-14 locked + this research confirms 1.17's built-in `<Virtualizer>` covers VIEW-02. `[CITED: react-aria.adobe.com/Virtualizer]` `[VERIFIED: npm view]` `[ASSUMED: React 19 fully compatible — peerDependencies includes ^19.0.0-rc.1]` |
| `@tanstack/react-virtual` | `^3.13.26` (published 2026-05-25) | List virtualisation **if** OQ-1 lands on the standalone-virtualizer path. **DO NOT ADD UNTIL OQ-1 IS RESOLVED.** | D-14 named it; this research finds React Aria's native `<Virtualizer>` is equivalent and removes one dep. `[VERIFIED: npm view]` `[CITED: tanstack.com/virtual]` |
| `react-markdown` | `^10.1.0` (published 2025-03-07) | Markdown → React renderer (VIEW-04) | Most mature React markdown lib. Safe by default (no `dangerouslySetInnerHTML`). `allowedElements` enforces SC#4 subset. `[CITED: github.com/remarkjs/react-markdown]` `[VERIFIED: npm view]` |
| `remark-gfm` | `^4.0.1` (published 2025-02-10) | GFM extras (tables, strikethrough, auto-links, task lists) — needed for fenced code blocks via `~~~` and auto-linked URLs | `react-markdown` author's recommended GFM plugin. We use it primarily for auto-linking. **Strip tables / task lists / footnotes via `allowedElements`.** `[CITED: github.com/remarkjs/react-markdown]` `[VERIFIED: npm view]` |
| `@tauri-apps/plugin-fs` | `^2.5.4` (published 2026-05-02) | Optional alternative to Rust-side file watcher (see OQ-3) | Has built-in `watch` / `watchImmediate` with `delayMs` debounce. `[CITED: v2.tauri.app/plugin/file-system]` `[VERIFIED: npm view]` |
| `@tauri-apps/plugin-opener` | `^2.5.4` (published 2026-05-02) | `revealItemInDir` for "Open source folder" action | Macos `Finder` reveal. `[CITED: v2.tauri.app/plugin/opener]` `[VERIFIED: npm view + Tauri docs search]` |
| `@tauri-apps/plugin-clipboard-manager` | `^2.3.2` (published 2026-02-02) | `writeText` for "Copy path" action | Standard Tauri clipboard plugin; no clipboard features enabled by default — must add capability + permission. `[CITED: v2.tauri.app/plugin/clipboard-manager]` `[VERIFIED: npm view]` |
| `tauri-plugin-opener` (Rust) | `2` (paired with JS plugin) | Rust-side init for the opener plugin | Required by the JS plugin. `[CITED: Tauri plugin docs]` `[ASSUMED: standard pairing pattern]` |
| `tauri-plugin-clipboard-manager` (Rust) | `2` (paired with JS plugin) | Rust-side init for the clipboard plugin | Required by the JS plugin. `[CITED: Tauri plugin docs]` `[ASSUMED: standard pairing pattern]` |
| `notify` (Rust) | `8.2.0` (stable; published 2025-08-03) | File watcher — manifest / lockfile / library / machine.toml | Industry standard for cross-platform watch; default macOS backend is FSEvents. **Avoid 9.0.0-rc.x — release candidate, not stable.** `[VERIFIED: docs.rs/notify/8.2.0, GitHub releases]` |
| `notify-debouncer-full` (Rust) | `0.7.0` (published 2026-01-23) | Debounce + rename-stitching layer over `notify` | The "ease of use" wrapper; coalesces editor-save bursts (Modify×N) into single events. `[VERIFIED: github.com/notify-rs/notify releases]` |

### Supporting (dev / testing)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| Vitest | `^4.1.7` (published 2026-05-20) | JS-side unit tests (fuzzy-match wrapper, markdown component allow-list, formatter helpers) | All Phase 26 logic with no Tauri dependency. `[VERIFIED: npm view]` |
| `@testing-library/react` | `^16.3.2` (published 2026-01-19) | Component-level tests against React 19 | Render `<DetailHeader>`, `<FindingRow>`, `<KeyValueRow>` against mock data. `[VERIFIED: npm view]` |
| Playwright | `^1.60.0` (published 2026-05-28) | End-to-end + perf bench driver against a built Tauri app | Plan 26-08 perf bench: drive search-as-you-type, sample FPS. `[VERIFIED: npm view]` |
| `@axe-core/playwright` | `^4.11.3` (MPL-2.0; published 2026-05-22) | Headless WCAG a11y check against rendered UI | Plan 26-07 a11y audit gate in CI. `[VERIFIED: npm view]` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| React Aria | Radix Primitives | D-14 names both; React Aria has stronger VoiceOver coverage on macOS (Adobe owns macOS-Safari WCAG QA). Radix is more "web-app-flavoured" (Discord, Vercel). Sticking with React Aria. |
| `react-markdown` | `marked` / `micromark` / `milkdown` | `marked` returns raw HTML strings (requires manual sanitisation). `micromark` is the parser `react-markdown` is built on (lower-level). `milkdown` is editor-focused (overkill for read-only). `react-markdown` is the safe-by-default React-native choice. |
| TanStack Virtual | React Aria's native `<Virtualizer>` | **See OQ-1.** React Aria 1.17's built-in Virtualizer wraps `ListBox` for keyboard + VoiceOver semantics free; TanStack Virtual is a more general-purpose primitive but requires hand-wiring `aria-rowindex`/focus management. Recommend the native one. |
| `notify` 9.0.0-rc.4 | `notify` 8.2.0 stable | RC line introduces breaking API changes (objc2-based macOS backend). **Stick to 8.2 stable** until 9.x ships. |
| Fuzzy match in Rust via Tauri command | JS-side fuzzy match (fuse.js or hand-port nucleo) | A Tauri command per keystroke costs ~1-5ms IPC + serialisation. At 60 fps (16ms budget) that's ~6-30% of budget gone before any rendering work. **JS-side is the safer choice.** See §"Standard Stack — Fuzzy search". |
| Rust-side file watcher | `@tauri-apps/plugin-fs` `watch` from JS | **See OQ-3.** Rust-side gives us tighter control (we can re-fetch the affected report and emit a typed `LibraryChanged` event); JS-side is one fewer plugin to glue. Recommend Rust-side. |

**Installation (additive — Phase 26 only):**

```bash
# crates/tome-desktop/ui
npm install react-aria-components react-markdown remark-gfm \
  @tauri-apps/plugin-opener @tauri-apps/plugin-clipboard-manager \
  @tauri-apps/plugin-fs
npm install --save-dev vitest @testing-library/react @testing-library/jest-dom \
  playwright @axe-core/playwright
# OPTIONAL — only if OQ-1 resolves toward TanStack:
# npm install @tanstack/react-virtual
```

```toml
# crates/tome-desktop/Cargo.toml
[dependencies]
notify = "8.2"
notify-debouncer-full = "0.7"
tauri-plugin-opener = "2"
tauri-plugin-clipboard-manager = "2"
tauri-plugin-fs = "2"  # only if OQ-3 resolves toward JS-side watcher
```

**Version verification (run before plan 26-01):**

```bash
# All packages independently verified via `npm view` / `cargo info` 2026-05-29:
npm view react-aria-components react-markdown remark-gfm \
  @tauri-apps/plugin-opener @tauri-apps/plugin-clipboard-manager \
  @tauri-apps/plugin-fs version time.modified license
cargo info notify notify-debouncer-full
```

### Standard Stack — Status dashboard (VIEW-01)

The existing `StatusReport` (`crates/tome/src/status.rs:75-93`) already carries: `configured`, `library_dir`, `library_count`, `last_sync`, `directories[]` (each with role/type/path/skill_count/warnings/override_applied), `unowned[]`, `health` (count or error).

**Two fields the UI-SPEC asks for that `StatusReport` does NOT carry yet:**

1. **Lockfile state — "In sync" vs "Out of sync".** UI-SPEC §"Per-view Design — Status" shows `LOCKFILE  In sync • ●green`. `StatusReport` currently has no lockfile field at all. **Action for plan 26-01:** Add `lockfile: LockfileState { InSync, OutOfSync { drift_count: usize }, Missing }` to `StatusReport`. The classification is `reconcile.rs`-shaped — compare lockfile content_hashes to manifest content_hashes; if they all match → `InSync`, otherwise `OutOfSync`. `[ASSUMED: this is the right semantics — confirm during plan 26-01]`
2. **Machine-prefs summary — "N skills disabled".** `MachinePrefs.disabled` is a `BTreeSet<SkillName>`; surface its `len()` plus possibly per-directory disabled counts. **Action for plan 26-01:** Add `machine_prefs_summary: MachinePrefsSummary { disabled_count: usize, disabled_directory_count: usize }` to `StatusReport`.

Both additions are **additive specta-derived structs** — regen `bindings.ts`, App.tsx renders the new KV rows. No domain logic change.

### Standard Stack — Virtualisation (VIEW-02)

**Decision pending — OQ-1.** Two paths:

**Path A (RECOMMENDED):** `react-aria-components` native `<Virtualizer>` (1.17+).

```tsx
import { ListBox, ListBoxItem, ListLayout, Virtualizer } from 'react-aria-components';

<Virtualizer layout={ListLayout} layoutOptions={{ rowHeight: 52, gap: 0, padding: 0 }}>
  <ListBox aria-label="Skills" items={filteredSkills} selectionMode="single"
           selectedKeys={[selectedName]} onSelectionChange={s => …}>
    {skill => (
      <ListBoxItem id={skill.name} textValue={skill.name}>
        <SkillListRow skill={skill} highlighted={…} />
      </ListBoxItem>
    )}
  </ListBox>
</Virtualizer>
```

Pros: ZERO additional dependency (already in `react-aria-components`), free a11y semantics (`aria-rowindex`/`aria-rowcount`/focus management), arrow-key + Home/End/PgUp/PgDn nav built-in, integrated with React Aria's selection model.
Cons: Newer API (1.17.0 published 2026-05-18 — ~10 days old at research time); fewer field examples. Variable-height rows require a `ListLayout` with `estimatedRowHeight` + measured rows (the API exists; less documented than TanStack Virtual's `measureElement`).

**Path B (FALLBACK):** TanStack Virtual + hand-wired React Aria `ListBox` semantics.

Pros: 3+ years of production use; thorough docs; `measureElement` for dynamic heights is well-trodden.
Cons: Extra ~5kB dep; have to hand-wire `aria-rowindex` + scroll-to-focused-item logic; integration friction with React Aria's `Collection` system (per GitHub `react-spectrum#5356` — known sharp edge).

**Decision criterion:** If row height stays fixed at 52px (UI-SPEC anchor), **Path A wins decisively** — no measurement complexity, free a11y. If the planner discovers wrapping/variable heights are needed (e.g., the secondary line wraps on long source names), Path A still works via `ListLayout`'s `estimatedRowHeight`, but Path B is the proven fallback.

### Standard Stack — Fuzzy search (VIEW-02 + D-04)

**Constraint:** match `nucleo` ranking from the browse TUI (`browse/fuzzy.rs`). `nucleo-matcher` is Rust-only — no JS port exists `[ASSUMED — no npm package named "nucleo" surfaced in research; verify before commit]`.

**Options:**

| Option | Cost / Latency | Match parity | Verdict |
|--------|---------------|--------------|---------|
| Tauri command per keystroke (`fuzzy_filter(query, all_names) -> Vec<usize>`) | ~1-5ms IPC + serialisation per keystroke + JSON serialisation of 2000 indices on every search | 100% parity (calls nucleo directly) | **Rejected** — eats 6-30% of 60fps budget; debouncing helps but feels laggy |
| `fuse.js` (most popular JS fuzzy lib, ~10kB, ~1M weekly DL) | ~0.5-2ms for 2000 items in pure JS | Close but not identical to nucleo (different scoring) | **Recommended** — fastest path to shipping; users won't notice ranking differences |
| Hand-port nucleo's scoring to TS | Free runtime; ~1-2 days of porting work | Exact parity | Overkill for alpha; revisit if fuse.js feels off |
| `@nucleo-search/wasm` or similar Rust→WASM build | Run nucleo in browser via WASM | Exact parity | **No published package** found `[ASSUMED — needs verification]` |

**Recommendation:** **`fuse.js`** for alpha (`^7.x` `[ASSUMED — confirm via `npm view fuse.js version` before commit]`). Document the ranking-divergence-from-CLI in plan 26-02; revisit in beta if user feedback complains. Fuzzy match is **display-only**, so this does not violate D-GUI-08.

### Standard Stack — Markdown (VIEW-04)

`react-markdown@10.1.0` + `remark-gfm@4.0.1`, configured with `allowedElements`:

```tsx
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

const allowed = [
  'h1', 'h2', 'h3',                      // headings (no h4-h6 per SC#4)
  'p', 'strong', 'em', 'code',           // inline
  'ul', 'ol', 'li',                      // lists
  'a',                                   // links
  'pre',                                 // fenced code blocks
];

<ReactMarkdown
  allowedElements={allowed}
  remarkPlugins={[remarkGfm]}
  components={{
    a: ({ href, children, ...props }) => (
      <a {...props} href={href} onClick={(e) => {
        e.preventDefault();
        if (href) openUrl(href); // tauri-plugin-opener
      }}>{children}</a>
    ),
    // h1/h2/h3, code, pre wrapped to bind UI-SPEC token classes
  }}
>{body}</ReactMarkdown>
```

**Security:** `react-markdown` is safe by default — no `dangerouslySetInnerHTML`, no raw HTML passthrough unless `rehype-raw` is added (do not add). SKILL.md content is **untrusted** (the user clones third-party git repos full of these), so the safe-by-default posture is load-bearing.

**No syntax highlighting** (per Deferred Ideas). If revisited later: `react-syntax-highlighter` + `prism-react-renderer` are the two standard picks.

### Standard Stack — A11y (NF-02)

React Aria primitives provide WCAG-AA out of the box. The two things Phase 26 must add:

1. **CI gate via `@axe-core/playwright`.** Run against a built Tauri app in headless mode, scan each of the three main views (Status / Skills / Health) + the PreviewPopover open state. Fails the build on any AA violation.
2. **Manual VoiceOver smoke test.** Plan 26-07 includes a checklist: enable VoiceOver (⌘F5), tab through every interactive element, verify each `aria-label` matches the UI-SPEC contract.

### Standard Stack — Native menu bar (NF-03)

Tauri 2's `tauri::menu` module (no plugin needed). `MenuBuilder` + `SubmenuBuilder` + `PredefinedMenuItem` cover the standard macOS menus (File / Edit / View / Window / Help). UI-SPEC keyboard map already enumerates the bindings; menu items map 1:1.

`[CITED: v2.tauri.app/learn/window-menu]`

## Package Legitimacy Audit

> slopcheck not available in this environment (`pip` not installed, slopcheck CLI not on PATH). All packages below are tagged `[ASSUMED]` and the planner must gate each install behind a `checkpoint:human-verify` task per Package Legitimacy Gate fallback.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `react-aria-components` | npm | published 2026-05-18 (1.17.0); package mature (1.0+ for years) | very high (millions/wk — Adobe-published) | github.com/adobe/react-spectrum | not run | **Approved with checkpoint** — well-known Adobe package, supply chain trusted |
| `@tanstack/react-virtual` | npm | published 2026-05-25 (3.13.26); 3+ years | very high | github.com/TanStack/virtual | not run | **Approved with checkpoint** — TanStack is industry standard (Tanner Linsley) |
| `react-markdown` | npm | published 2025-03-07 (10.1.0); 8+ years | very high (~10M/wk) | github.com/remarkjs/react-markdown | not run | **Approved with checkpoint** — remarkjs is the canonical markdown ecosystem |
| `remark-gfm` | npm | published 2025-02-10 (4.0.1) | very high | github.com/remarkjs/remark-gfm | not run | **Approved with checkpoint** — same authors as react-markdown |
| `@tauri-apps/plugin-opener` | npm | published 2026-05-02 (2.5.4) | high (official Tauri) | github.com/tauri-apps/plugins-workspace | not run | **Approved with checkpoint** — official `@tauri-apps` org |
| `@tauri-apps/plugin-clipboard-manager` | npm | published 2026-02-02 (2.3.2) | high (official Tauri) | github.com/tauri-apps/plugins-workspace | not run | **Approved with checkpoint** — official `@tauri-apps` org |
| `@tauri-apps/plugin-fs` | npm | published 2026-05-02 (2.5.4) | high (official Tauri) | github.com/tauri-apps/plugins-workspace | not run | **Approved with checkpoint** — official `@tauri-apps` org |
| `playwright` | npm | published 2026-05-28 (1.60.0) | very high | github.com/microsoft/playwright | not run | **Approved with checkpoint** — Microsoft-published |
| `@axe-core/playwright` | npm | published 2026-05-22 (4.11.3) | high (Deque) | github.com/dequelabs/axe-core-npm | not run | **Approved with checkpoint** — Deque Systems, accessibility-industry standard |
| `vitest` | npm | published 2026-05-20 (4.1.7) | very high | github.com/vitest-dev/vitest | not run | **Approved with checkpoint** — vitest-dev (Anthony Fu et al.) |
| `@testing-library/react` | npm | published 2026-01-19 (16.3.2) | very high | github.com/testing-library/react-testing-library | not run | **Approved with checkpoint** — testing-library org |
| `fuse.js` | npm | not verified yet | very high | github.com/krisk/Fuse | not run | **Verify before install** — common name = higher slopsquat risk; `npm view fuse.js repository.url` before adding |
| `notify` (Rust) | crates.io | 8.2.0 published 2025-08-03 | very high (industry std) | github.com/notify-rs/notify | not run | **Approved with checkpoint** — notify-rs org, depended on by ripgrep / cargo / etc. |
| `notify-debouncer-full` (Rust) | crates.io | 0.7.0 published 2026-01-23 | high (paired with notify) | github.com/notify-rs/notify | not run | **Approved with checkpoint** — same workspace as notify |
| `tauri-plugin-opener` (Rust) | crates.io | paired with JS plugin | high | github.com/tauri-apps/plugins-workspace | not run | **Approved with checkpoint** — official tauri-apps |
| `tauri-plugin-clipboard-manager` (Rust) | crates.io | paired with JS plugin | high | github.com/tauri-apps/plugins-workspace | not run | **Approved with checkpoint** — official tauri-apps |

**Packages removed due to slopcheck [SLOP] verdict:** none (slopcheck not run)
**Packages flagged as suspicious [SUS]:** none flagged; **`fuse.js` needs name-verification before install** (high-traffic name = elevated slopsquat target)

*Planner action: insert a `checkpoint:human-verify` task before each `npm install` / `cargo add` step so the human approves the exact `npm view <pkg> repository.url` / `cargo info <pkg>` output.*

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────── User's Mac ────────────────────────────┐
│                                                                    │
│  ┌────────────────────── tome-desktop.app ─────────────────────┐  │
│  │                                                              │  │
│  │  ┌─── WebView (React 19) ─────────────────────────────────┐ │  │
│  │  │                                                         │ │  │
│  │  │  Window (3-col NavigationSplitView)                     │ │  │
│  │  │    ├── Sidebar (NavItem ×3 + Health badge)              │ │  │
│  │  │    ├── ContentPane                                      │ │  │
│  │  │    │   ├── Status view (KeyValueRow + DirectoryTable)   │ │  │
│  │  │    │   ├── Skills view (SearchField + Virtualizer       │ │  │
│  │  │    │   │              ListBox + DetailHeader +          │ │  │
│  │  │    │   │              MarkdownBody)                     │ │  │
│  │  │    │   └── Health view (SectionHeader + FindingRow +    │ │  │
│  │  │    │                    PreviewPopover)                 │ │  │
│  │  │    └── Native menu bar (mounted by Rust)                │ │  │
│  │  │                                                         │ │  │
│  │  │  Data fetching: commands.* from bindings.ts             │ │  │
│  │  │  Event listening: events.* (sync-progress, library-     │ │  │
│  │  │                    changed, manifest-changed, lock-     │ │  │
│  │  │                    file-changed, machine-prefs-changed) │ │  │
│  │  └─────────────────────────────────────────────────────────┘ │  │
│  │                            ↑↓ Tauri IPC                        │  │
│  │  ┌─── Rust backend (tome-desktop crate) ────────────────────┐ │  │
│  │  │                                                          │ │  │
│  │  │  make_builder() — single command/event registry          │ │  │
│  │  │    Commands:                                             │ │  │
│  │  │      get_status (Phase 25 — extend with lockfile +      │ │  │
│  │  │                  machine-prefs summary)                  │ │  │
│  │  │      list_skills      ← tome::list::collect              │ │  │
│  │  │      get_skill_detail ← tome::skill::parse + manifest    │ │  │
│  │  │      get_doctor_report ← tome::doctor::check             │ │  │
│  │  │      doctor_repair_one(finding_id) ← NEW                 │ │  │
│  │  │      set_skill_disabled(name, bool) ← machine::*         │ │  │
│  │  │      open_source_folder(name) ← tome::actions            │ │  │
│  │  │      copy_path(name) -> String ← tome::actions           │ │  │
│  │  │    Events:                                               │ │  │
│  │  │      sync_progress (Phase 25, unchanged)                 │ │  │
│  │  │      library_changed                                     │ │  │
│  │  │      manifest_changed                                    │ │  │
│  │  │      lockfile_changed                                    │ │  │
│  │  │      machine_prefs_changed                               │ │  │
│  │  │      menu_action(MenuAction enum)                        │ │  │
│  │  │                                                          │ │  │
│  │  │  Background thread: file watcher                         │ │  │
│  │  │    notify::recommended_watcher → debouncer-full →        │ │  │
│  │  │    classify path → app.emit("library-changed" etc.)      │ │  │
│  │  └──────────────────────────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                            ↑↓ filesystem                              │
│  ┌────────────── shared on-disk state (NF-05) ─────────────────┐  │
│  │  ~/.tome/.tome-manifest.json   (managed by tome::manifest)   │  │
│  │  ~/.tome/tome.lock             (managed by tome::lockfile)   │  │
│  │  ~/.tome/library/<skill>/SKILL.md  (managed by tome::library)│  │
│  │  ~/.config/tome/machine.toml   (managed by tome::machine)    │  │
│  │  ~/.tome/tome.toml             (config; read-only in Phase 26)│  │
│  └──────────────────────────────────────────────────────────────┘  │
│                            ↑                                          │
│              concurrent CLI usage (NF-05)                            │
│              `tome sync`, `tome doctor`, etc.                        │
└────────────────────────────────────────────────────────────────────┘
```

**Data flow for a typical user interaction:**

1. User opens app → React calls `commands.getStatus()` → Rust resolves paths + reads manifest+config → returns `StatusReport` → React renders.
2. User types in search → React filters `skillsCache` (local state populated by `list_skills`) via `fuse.js` → re-renders `<Virtualizer>` with `filteredSkills`.
3. User clicks a skill row → React calls `commands.getSkillDetail(name)` → Rust reads `SKILL.md` + parses frontmatter + reads manifest entry → returns `SkillDetail` → React renders `<DetailHeader>` + `<MarkdownBody>`.
4. User clicks "Disable on this machine" → React calls `commands.setSkillDisabled(name, true)` → Rust writes `machine.toml` (atomic) → notify watcher fires `machine.toml` change → debouncer emits → Tauri event `machine_prefs_changed` → React re-fetches skill detail + list → silent re-render (D-03).
5. Meanwhile in another terminal, user runs `tome sync` → CLI rewrites manifest + lockfile → notify watcher fires `manifest_changed` + `lockfile_changed` → debouncer coalesces → React re-fetches `StatusReport` and `list_skills` → silent re-render + "Updated" pill (D-03).

### Recommended Project Structure

Phase 25 already scaffolds the React side. Phase 26 extends within the existing tree — **no top-level reshuffle.**

```
crates/tome-desktop/
├── src/                          # Rust IPC backend
│   ├── lib.rs                    # make_builder() — shared registry
│   ├── main.rs                   # Tauri entry; menu bar setup
│   ├── commands.rs               # Tauri command handlers (extend)
│   ├── error.rs                  # TomeError boundary (unchanged)
│   ├── sink.rs                   # TauriEventSink (unchanged)
│   ├── watcher.rs                # NEW — file watcher thread
│   ├── menu.rs                   # NEW — MenuBuilder helpers
│   └── bin/gen-bindings.rs       # unchanged
├── capabilities/main.json        # ADD opener / clipboard / fs perms
├── tauri.conf.json               # ADD window vibrancy + titlebar style
├── tests/
│   └── perf/                     # NEW — Playwright FPS bench
└── ui/                           # React frontend
    ├── package.json              # extend with new deps
    ├── vite.config.ts            # unchanged
    └── src/
        ├── main.tsx              # unchanged
        ├── App.tsx               # rewrite: 3-col shell
        ├── bindings.ts           # regenerated
        ├── styles.css            # tokens (global)
        ├── tokens.css            # NEW — design tokens per UI-SPEC
        ├── shell/                # NEW
        │   ├── Window.tsx
        │   ├── Titlebar.tsx
        │   ├── Sidebar.tsx
        │   └── ContentPane.tsx
        ├── views/                # NEW
        │   ├── StatusView.tsx
        │   ├── SkillsView.tsx
        │   ├── HealthView.tsx
        │   └── *.module.css
        ├── components/           # NEW — atoms + molecules per UI-SPEC
        │   ├── Badge.tsx
        │   ├── Button.tsx
        │   ├── KeyValueRow.tsx
        │   ├── SkillListRow.tsx
        │   ├── DetailHeader.tsx
        │   ├── MarkdownBody.tsx
        │   ├── FindingRow.tsx
        │   ├── PreviewPopover.tsx
        │   └── ... (one .tsx + .module.css per UI-SPEC component)
        ├── hooks/                # NEW
        │   ├── useStatus.ts          # commands.getStatus + watcher refresh
        │   ├── useSkills.ts          # commands.listSkills + watcher refresh
        │   ├── useSkillDetail.ts     # commands.getSkillDetail + watcher refresh
        │   ├── useDoctorReport.ts    # commands.getDoctorReport + watcher refresh
        │   ├── useFuzzySearch.ts     # fuse.js wrapper, debounced
        │   └── useTauriEvent.ts      # generic event listener
        └── lib/
            ├── relativeTime.ts       # "2 minutes ago" formatter
            └── ariaLabels.ts         # central label templates per UI-SPEC

crates/tome/src/
├── actions.rs                    # NEW — shared TUI+GUI handler module
│   pub fn open_source(...)         (computes path; doesn't shell-open)
│   pub fn copy_path(...) -> String (computes display path string)
│   pub fn set_skill_disabled(...) -> Result<()>
├── doctor.rs                     # extend — add repair_one(finding_id) API
└── status.rs                     # extend — add lockfile + machine-prefs summary
```

### Pattern 1: Tauri command — shape every new command this way

```rust
// crates/tome-desktop/src/commands.rs
#[tauri::command]
#[specta::specta]
pub fn list_skills(_app: tauri::AppHandle) -> Result<tome::list::ListReport, TomeError> {
    let (config, _paths) = load_context().map_err(TomeError::from)?;
    tome::list::collect(&config).map_err(TomeError::from)
}
```

Pattern: thin wrapper that (1) resolves context via `load_context()` (Phase 25's pattern), (2) calls a domain fn returning a structured type, (3) maps the `anyhow::Error` into `TomeError` at the boundary.

Then register in `make_builder()`:

```rust
// crates/tome-desktop/src/lib.rs
pub fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::get_status,
            commands::list_skills,                  // NEW
            commands::get_skill_detail,             // NEW
            commands::get_doctor_report,            // NEW
            commands::doctor_repair_one,            // NEW
            commands::set_skill_disabled,           // NEW
            commands::open_source_folder,           // NEW
            commands::copy_path,                    // NEW
        ])
        .events(collect_events![
            sink::SyncProgress,
            watcher::LibraryChanged,                // NEW
            watcher::ManifestChanged,               // NEW
            watcher::LockfileChanged,               // NEW
            watcher::MachinePrefsChanged,           // NEW
            menu::MenuAction,                       // NEW
        ])
        .dangerously_cast_bigints_to_number()
}
```

**Each new command/event triggers `bindings.ts` regen** — the CI gate (`git diff --exit-code`) catches drift. **All cross-boundary types must derive `#[cfg_attr(feature = "bindings", derive(specta::Type))]`.**

### Pattern 2: React side — fetching + watcher-driven refresh

```tsx
// ui/src/hooks/useStatus.ts
import { useEffect, useState } from 'react';
import { commands, events, type StatusReport_Serialize, type TomeError } from '../bindings';

export function useStatus() {
  const [status, setStatus] = useState<StatusReport_Serialize | null>(null);
  const [err, setErr] = useState<TomeError | null>(null);
  const [updatedAt, setUpdatedAt] = useState<number | null>(null);

  const refetch = async () => {
    const res = await commands.getStatus();
    if (res.status === 'ok') {
      setStatus(res.data);
      setErr(null);
      setUpdatedAt(Date.now());
    } else {
      setErr(res.error);
    }
  };

  useEffect(() => {
    refetch();
    const unlistens = [
      events.manifestChanged.listen(refetch),
      events.lockfileChanged.listen(refetch),
      events.machinePrefsChanged.listen(refetch),
      events.libraryChanged.listen(refetch),
    ];
    return () => { unlistens.forEach(u => u.then(fn => fn())); };
  }, []);

  return { status, err, updatedAt, refetch };
}
```

Pattern: every domain view has a `useX` hook that (1) fetches via a Tauri command, (2) subscribes to the relevant watcher events, (3) re-fetches on event, (4) tracks `updatedAt` for the transient "Updated" pill.

**Cancellation:** Tauri commands are short and idempotent in Phase 26 (no long-running ops). If a refetch fires while one is in flight, just accept the race — the last one wins; React's `useState` handles it. If a stronger story is needed in beta (Phase 27 sync), introduce `AbortController` + sequence numbers.

### Pattern 3: Action handler refactor (VIEW-03 + Success Criterion 3)

Success Criterion 3 says detail-pane actions must use "the same handlers as the existing browse TUI". The TUI's handlers (in `crates/tome/src/browse/app.rs`) are tightly coupled to the TUI: they mutate `App` state, format `StatusMessage`s, and inline-call `arboard` for clipboard + `Command::new("open"/"xdg-open")` for opener. They're not pure functions.

**Recommendation:** Extract the **pure parts** into a new module `crates/tome/src/actions.rs`, leave the TUI's UI-glue in place.

```rust
// crates/tome/src/actions.rs (NEW)
//! Cross-surface skill actions (TUI + GUI).
//!
//! Pure-Rust helpers shared between the browse TUI (`browse::app`) and the
//! Tauri command surface (`tome-desktop::commands`). These functions own the
//! "what" of an action — compute the path, mutate machine.toml — but not the
//! "how" of presenting the result (which is each surface's own concern).

pub fn resolve_source_path(skill_name: &SkillName, config: &Config, paths: &TomePaths)
    -> anyhow::Result<PathBuf> { … }

pub fn set_skill_disabled(skill_name: &SkillName, disabled: bool, paths: &TomePaths)
    -> anyhow::Result<()> { … }  // wraps machine::load + .disable_skill/.enable_skill + machine::save_checked
```

Then:
- **TUI** (`browse/app.rs`) — keep `App::execute_action` but have its arms call into `actions::*` instead of duplicating logic.
- **GUI** (`tome-desktop/src/commands.rs`) — `set_skill_disabled` command wraps `actions::set_skill_disabled`. `open_source_folder` command resolves the path via `actions::resolve_source_path` then calls the Tauri opener plugin (`opener::reveal_item_in_dir`) — the OS-call is now at the Tauri edge, not in `tome::actions`, because the GUI uses macOS Finder reveal (different from the TUI's `xdg-open`). `copy_path` command returns the string; the React side calls `@tauri-apps/plugin-clipboard-manager`'s `writeText` to actually copy.

**Why this split:** the browse TUI must shell out to the local opener (it runs in a terminal); the GUI uses the OS-native reveal-in-Finder. Sharing the opener code across surfaces would be wrong; sharing the **path computation** + the **machine.toml mutation** is correct.

### Pattern 4: Doctor per-item fix (VIEW-05)

Current `doctor.rs::dispatch_repairs` is batch-only: it loops `report.all_issues()` and dispatches by `RepairKind`. The GUI needs to fix **one** finding at a time.

**Recommendation:** Add a stable finding identifier and a single-item dispatcher.

```rust
// crates/tome/src/doctor.rs (extend)
#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct FindingId(String);  // newtype around a stable hash of (category, message)

impl DiagnosticIssue {
    pub fn id(&self) -> FindingId {
        // Stable across runs: hash of category + message + repair_kind
        // (anything user-visible that distinguishes findings of the same kind)
    }
}

/// Per-item repair dispatcher. Mirrors `dispatch_repairs` but operates on one
/// finding by ID. The match arms are the same; the difference is no batching
/// across like-kinds (each invocation runs exactly one repair).
pub fn repair_one(
    finding_id: &FindingId,
    config: &Config,
    paths: &TomePaths,
) -> Result<()> {
    let report = check(config, paths)?;
    let issue = report.all_issues()
        .find(|i| i.id() == *finding_id)
        .ok_or_else(|| anyhow!("finding {} no longer present (stale UI?)", finding_id))?;
    let Some(kind) = issue.repair_kind else {
        bail!("finding {} is not auto-fixable", finding_id);
    };
    match kind {
        RepairKind::RemoveStaleManifestEntry
        | RepairKind::RemoveBrokenLibrarySymlink => repair_library_one(paths, issue)?,
        RepairKind::RemoveStaleTargetSymlink => repair_target_one(config, paths, issue)?,
        RepairKind::ConsolidateTargetRealDirToSymlink => consolidate_one(config, paths, issue)?,
    }
    Ok(())
}
```

**Subtlety:** the existing batch handlers re-scan and process all matching findings in one pass (efficiency). Per-item handlers must do the same scan-and-find pattern but apply to one path only. Plan 26-05 owns the per-item helpers.

**Preview popover text:** Each `RepairKind` already has a human action label (`repair_kind_action_label(kind)`). Surface this — plus the issue's message — as the popover body. UI-SPEC `PreviewPopover` already specifies this composition.

### Pattern 5: File watcher (VIEW-06)

```rust
// crates/tome-desktop/src/watcher.rs (NEW)
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent};
use std::path::Path;
use std::time::Duration;
use tauri_specta::Event;

#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct ManifestChanged;
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct LockfileChanged;
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct LibraryChanged;
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct MachinePrefsChanged;

pub fn spawn_watcher(app: tauri::AppHandle, paths: tome::TomePaths) -> anyhow::Result<()> {
    let manifest_path = paths.config_dir().join(".tome-manifest.json");
    let lockfile_path = paths.config_dir().join("tome.lock");
    let library_dir = paths.library_dir().to_path_buf();
    let machine_path = tome::machine::default_machine_path()?;  // ~/.config/tome/machine.toml

    let app2 = app.clone();
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = new_debouncer(
            Duration::from_millis(200),  // D-03 / SC#1 200ms target
            None,                         // no tick rate override
            move |res: Result<Vec<DebouncedEvent>, _>| {
                if let Ok(events) = res {
                    let _ = tx.send(events);
                }
            },
        ).expect("debouncer init");

        // Watch all four roots. RecursiveMode::Recursive for the library dir
        // (skill content), NonRecursive for the file paths.
        for (path, mode) in [
            (manifest_path.parent().unwrap().to_path_buf(), RecursiveMode::NonRecursive),
            (lockfile_path.parent().unwrap().to_path_buf(), RecursiveMode::NonRecursive),
            (library_dir.clone(), RecursiveMode::Recursive),
            (machine_path.parent().unwrap().to_path_buf(), RecursiveMode::NonRecursive),
        ] {
            if path.exists() {
                let _ = debouncer.watch(&path, mode);
            }
        }

        while let Ok(events) = rx.recv() {
            let mut saw_manifest = false;
            let mut saw_lockfile = false;
            let mut saw_library = false;
            let mut saw_machine = false;
            for ev in events {
                for path in &ev.paths {
                    if path == &manifest_path { saw_manifest = true; }
                    if path == &lockfile_path { saw_lockfile = true; }
                    if path == &machine_path { saw_machine = true; }
                    if path.starts_with(&library_dir) { saw_library = true; }
                }
            }
            if saw_manifest { let _ = ManifestChanged.emit(&app2); }
            if saw_lockfile { let _ = LockfileChanged.emit(&app2); }
            if saw_library { let _ = LibraryChanged.emit(&app2); }
            if saw_machine { let _ = MachinePrefsChanged.emit(&app2); }
        }
    });

    Ok(())
}
```

Wire into `main.rs::setup`:

```rust
// main.rs setup closure
let app_handle = app.handle().clone();
let (_, paths) = tome_desktop::commands::load_context()?;
tome_desktop::watcher::spawn_watcher(app_handle, paths)?;
```

**Debounce target:** 200ms — fast enough for SC#1's "Refreshes within 200ms" (the watcher fire + Tauri event + React refetch round-trip needs ~50ms headroom, so the debounce window is the dominant term).

**`tome relocate` (library_dir moves):** out of Phase 26 scope (relocate is Phase 29 / OPS-03), but defensively the watcher should accept a `re_watch(new_paths)` call later. For Phase 26, watcher captures paths at startup; users who run `tome relocate` during the GUI session must restart the app (acceptable — relocate isn't in the GUI yet).

### Pattern 6: Perf bench harness (NF-01)

Plan 26-08 specifics:

```
crates/tome-desktop/tests/perf/
├── synthetic_skills.rs            # Rust helper: generate 2000 fake skills in TempDir
├── fixtures/                       # Template SKILL.md content
└── playwright/
    ├── playwright.config.ts        # Configure project: pointAtTauriBuild
    ├── 60fps-search.spec.ts        # The bench
    └── fps-sampler.ts              # requestAnimationFrame frame-time sampling
```

**Approach:**

1. **Fixture generator** (Rust, called via `cargo test --release -p tome-desktop --test perf_setup`): create a `TempDir`, write `tome.toml` pointing `tome_home` at it, generate 2000 skills with `SKILL.md` files (varying body lengths to exercise variable rendering).
2. **Launch Tauri app** in test mode pointing at the fixture's `tome_home` (set `TOME_HOME` env var that `commands.rs::load_context` honours — already does via `tome::config::default_tome_home`).
3. **Drive via Playwright:**
   - Click into Skills view, wait for list render.
   - Type `t-d-d` character-by-character (15ms inter-keystroke).
   - On each animation frame, sample `performance.now()`; compute inter-frame deltas.
4. **Assert:** 95th percentile inter-frame delta < 18ms (i.e., ~55fps p95, allowing 1-2 frame jank). Strict 60fps is unrealistic over a 1s window.

**FPS sampling code:**

```ts
// fps-sampler.ts (injected into webview via Playwright's evaluate())
window.__fpsFrames = [];
const start = performance.now();
let last = start;
function tick(t: number) {
  window.__fpsFrames.push(t - last);
  last = t;
  if (t - start < 2000) requestAnimationFrame(tick);
}
requestAnimationFrame(tick);
```

**CI integration:** macOS-only matrix job (`runs-on: macos-latest`). Skip on Linux/Windows runners. Time budget: ~30s per run; runs on main + PRs touching `ui/` or `tests/perf/`.

### Pattern 7: Native menu bar (NF-03)

```rust
// crates/tome-desktop/src/menu.rs (NEW)
use tauri::menu::{MenuBuilder, SubmenuBuilder, PredefinedMenuItem};
use tauri::{AppHandle, Manager, Wry};
use tauri_specta::Event;

#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
#[serde(tag = "kind")]
pub enum MenuAction {
    FocusSearch,
    JumpStatus,
    JumpSkills,
    JumpHealth,
    Reload,
    // Phase 27+ Library actions (disabled with tooltip in 26)
}

pub fn build_app_menu(app: &AppHandle<Wry>) -> tauri::Result<tauri::menu::Menu<Wry>> {
    // The first submenu becomes the macOS app menu — `tome`
    let app_menu = SubmenuBuilder::new(app, "tome")
        .about(Some(/* AboutMetadata */))
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .close_window()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo().redo().separator()
        .cut().copy().paste().select_all()
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .text("jump-status",  "Status").accelerator("CmdOrCtrl+1")  // pseudo-API; use Submenu builder verbatim
        // … repeated for Skills (⌘2), Health (⌘3), Reload (⌘R disabled, tooltip "Available in beta")
        .build()?;

    let library_menu = SubmenuBuilder::new(app, "Library")
        // All disabled in Phase 26 — Sync etc. come in Phase 27
        .build()?;

    let help_menu = SubmenuBuilder::new(app, "Help")
        .text("docs", "Documentation")
        .text("report-issue", "Report Issue")
        .build()?;

    MenuBuilder::new(app)
        .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &library_menu, &help_menu])
        .build()
}

pub fn install_menu_event_handler(app: &AppHandle<Wry>) {
    let app_handle = app.clone();
    app.on_menu_event(move |_app, event| {
        let action = match event.id().0.as_str() {
            "jump-status"  => MenuAction::JumpStatus,
            "jump-skills"  => MenuAction::JumpSkills,
            "jump-health"  => MenuAction::JumpHealth,
            "focus-search" => MenuAction::FocusSearch,
            "docs" | "report-issue" => {
                // Open URLs directly via opener plugin
                return;
            }
            _ => return,
        };
        let _ = action.emit(&app_handle);
    });
}
```

React subscribes:

```tsx
// hooks/useMenuActions.ts
import { events } from '../bindings';
import { useRouterStore } from '../stores/router';

useEffect(() => {
  const unl = events.menuAction.listen((evt) => {
    switch (evt.payload.kind) {
      case 'JumpStatus':  useRouterStore.setState({ view: 'status' }); break;
      case 'JumpSkills':  useRouterStore.setState({ view: 'skills' }); break;
      case 'JumpHealth':  useRouterStore.setState({ view: 'health' }); break;
      case 'FocusSearch': document.querySelector<HTMLInputElement>('[aria-label="Search skills"]')?.focus(); break;
    }
  });
  return () => { unl.then(fn => fn()); };
}, []);
```

**Disabled-menu-item state:** Items that won't work in alpha (Sync, Add) render disabled with a tooltip. The `MenuItemBuilder` has an `enabled(false)` method `[ASSUMED — confirm in Tauri 2.11 docs]`.

### Anti-Patterns to Avoid

- **Polling for state changes.** The watcher exists for a reason; do not write a `setInterval(refetch, 1000)` "just in case". It hides watcher bugs and burns CPU.
- **Per-keystroke Tauri command for fuzzy search.** Round-trip latency kills the 60fps budget. Fetch the skill name list once, filter in JS.
- **Storing `bindings.ts` types in `useState` as serialised JSON.** They're already typed objects from the bindings — `setState<StatusReport_Serialize>` and pass them around as-is.
- **Re-fetching everything on every event.** A `library_changed` event does not need to re-fetch the doctor report (which is expensive). Each hook subscribes only to the events it actually depends on.
- **Building a "global state store" (Redux/Zustand).** Phase 26 has 4 views; each owns its own data via `useX()` hooks. URL state (which view is active, which skill is selected) lives in a tiny custom store or simple `useState` in `<App>`. Resist Redux/Zustand for alpha.
- **Hand-rolling ARIA roles** instead of using React Aria primitives. Every screen-reader bug we hit reading our own ARIA will cost more than learning React Aria's APIs.
- **Sharing the TUI's `App` state** with the GUI. The TUI has its own state machine; the GUI has React state. Share `tome::actions` (pure), not `browse::App` (UI-glue).
- **Skipping the `bindings.ts` regen + commit** when adding a new Tauri command. CI fails on stale bindings (Phase 25 freshness gate). Add → regen → commit in one PR.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Virtualised list at 60fps for 2000 items | Custom scroll-window component | React Aria's `<Virtualizer>` (1.17+) OR `@tanstack/react-virtual` 3.x | Both handle scroll math, repaint optimisation, dynamic row heights. Custom = bugs in scroll restoration + keyboard nav. |
| Markdown → HTML | Custom regex parser | `react-markdown` + `remark-gfm` | CommonMark + GFM has hundreds of edge cases; safe rendering requires not injecting `dangerouslySetInnerHTML`. |
| File system watching | `fs::metadata` polling | `notify` 8.2 + `notify-debouncer-full` 0.7 | Polling misses fast saves; OS-native FSEvents catches them. |
| Fuzzy search | Levenshtein hand-port | `fuse.js` (alpha) | Production-grade scoring includes BM25-style hits, character-position memoisation, etc. |
| Native macOS menu bar | HTML-based menu UI in the webview | `tauri::menu::MenuBuilder` | Native menus get free Cmd+`shortcuts`, Services menu, language localisation, VoiceOver integration. |
| Clipboard write | `navigator.clipboard.writeText` (no user-gesture flakes in webview) | `@tauri-apps/plugin-clipboard-manager` | Tauri's plugin uses the OS clipboard API directly; no permission prompt, no security context issues. |
| Open Finder | `Command::new("open")` shellout | `@tauri-apps/plugin-opener` `revealItemInDir` | The plugin handles macOS/Linux/Windows differences; for our macOS-only v1.0 it's still cleaner than shelling out. |
| Keyboard-accessible ListBox | Custom `<ul>` + `onKeyDown` arrows | React Aria `<ListBox>` | Free `aria-activedescendant`, free arrow/Home/End/PgUp/PgDn, free focus restoration, free screen-reader announcements. |
| Confirmation popover (preview-then-confirm) | Custom modal | React Aria `<Popover>` + `<Dialog>` | Free focus trap, free `Escape`-to-dismiss, free `aria-modal`, free outside-click dismissal. |
| Light/dark theme switching | `data-theme="dark"` toggle + JS detection | CSS `@media (prefers-color-scheme: dark)` | Browser-native, follows system in real time, no JS state. D-15 locked. |

**Key insight:** Tauri 2 + the official `@tauri-apps/plugin-*` set + React Aria covers ~80% of the GUI plumbing. The remaining 20% is the genuinely tome-specific code: fetching reports, rendering UI-SPEC components, file-watcher routing. Plans should be biased toward "wire the official thing" over "write the custom thing".

## Common Pitfalls

### Pitfall 1: Atomic-rename read race (NF-05 concurrency)
**What goes wrong:** CLI does `tempfile.persist(target)` (atomic rename) on `tome.lock`. notify fires `Modify(Name(From))` + `Modify(Name(To))` on the rename; the webview re-fetches `StatusReport` which re-reads the file — if the read lands between the rename and the FS publish, you can read either old or new content, never partial — but if you read **before** the watcher fires the event, you'll read the **old** content and re-render stale.
**Why it happens:** Atomic rename = no partial reads, but the watcher event is asynchronous; the re-render could trigger before the watcher has dispatched.
**How to avoid:** Trust the watcher — the read is initiated by the watcher event, not by polling. The event fires *after* the rename is committed (notify reads the FSEvent stream serially). One subtle case: if the rename fires *as the React refetch is mid-flight from a prior event*, the old read may win. **Mitigation:** debounce ~200ms on the watcher side (already planned via `notify-debouncer-full`). The race window is now 200ms wide and the user sees the up-to-date state within the same window.
**Warning signs:** Manual reproducer — run `tome sync` in a terminal twice in rapid succession (<200ms apart), watch whether the GUI's `Last sync` field updates exactly once or twice.

### Pitfall 2: TanStack Virtual + React Aria ListBox integration friction
**What goes wrong:** Per `react-spectrum#5356`, using TanStack Virtual inside a React Aria `ListBox` can cause selection-state desync (React Aria's `Collection` system expects to manage rendering; TanStack Virtual takes that over).
**Why it happens:** React Aria builds an internal `Collection` from the items prop and renders all of them virtually for keyboard/focus management. TanStack Virtual renders only visible items. The two layers fight over the DOM.
**How to avoid:** Use React Aria's native `<Virtualizer>` (1.17+) which is designed for this exact case. If TanStack Virtual is necessary (e.g., custom layout React Aria doesn't support), wrap a *plain* `<ul role="listbox">` instead of `<ListBox>` and re-implement keyboard navigation by hand.
**Warning signs:** Arrow-down stops working at row ~20 of a 2000-row list; selection focus ring disappears on scroll.

### Pitfall 3: react-markdown stripping characters
**What goes wrong:** Using `allowedElements` with the wrong list strips visible content (e.g., forgetting `<p>` leaves bare text floating; forgetting `<li>` leaves bullet markers without content).
**Why it happens:** `react-markdown` doesn't *unwrap* disallowed elements; it skips them entirely.
**How to avoid:** Snapshot tests against a fixture SKILL.md containing every allowed element; visually verify the rendered output matches the UI-SPEC. The Phase 26 fixture should include:
- H1, H2, H3
- A paragraph with `**bold**`, `*italic*`, `` `inline code` ``
- An unordered list with multiple items
- An ordered list
- A `[link](https://example.com)`
- A fenced code block

**Warning signs:** "Why is this skill description showing as one long string?" — missing `<p>` in `allowedElements`.

### Pitfall 4: Tauri opener / clipboard plugin permissions
**What goes wrong:** Plugin is installed and command is called, but it silently fails because the capability JSON doesn't grant the permission.
**Why it happens:** Tauri 2 is allow-list by default — every plugin command requires an explicit permission in `capabilities/main.json`. The default capability set is intentionally minimal.
**How to avoid:** Plan 26-03 adds:
```json
"permissions": [
  "core:default", "core:event:default",
  "opener:default",                       // covers revealItemInDir
  "clipboard-manager:allow-write-text",   // covers writeText
  // "fs:default" if OQ-3 lands on JS-side watcher
]
```
And document each addition in the plan so reviewers see the IPC surface growing.
**Warning signs:** "Open source folder" button does nothing; no error in the React console; Rust logs show `permission denied`.

### Pitfall 5: notify watching a directory before it exists
**What goes wrong:** `notify::watch` returns `Err(WatcherKind::PathNotFound)` if the path doesn't exist at startup. If the user runs the GUI before ever syncing, the library dir or manifest file won't exist yet → watcher silently doesn't watch them → user runs `tome sync` from CLI → no Tauri events → stale UI.
**Why it happens:** notify can't watch a non-existent path.
**How to avoid:** Watch the **parent** directory (`paths.config_dir()`, `paths.library_dir().parent()`), not the file itself. The watcher fires on `Create` when the file appears; we then know the manifest is fresh. The pattern shown in §"Pattern 5" already does this.
**Warning signs:** Fresh-install user opens GUI before first sync; runs `tome sync` from terminal; GUI shows nothing changed.

### Pitfall 6: bindings.ts drift
**What goes wrong:** A plan adds a Tauri command but forgets to run `gen-bindings`. CI catches it (Phase 25's freshness gate), but the planner can save time by including the regen step in every plan that touches `commands.rs` / `lib.rs::make_builder`.
**Why it happens:** Two-step process; easy to forget step 2.
**How to avoid:** Every plan that adds a command/event includes a checklist item: `cargo run -p tome-desktop --bin gen-bindings && git diff crates/tome-desktop/ui/src/bindings.ts | wc -l > 0 && git add crates/tome-desktop/ui/src/bindings.ts`.
**Warning signs:** PR fails CI with "bindings.ts is stale" diff.

### Pitfall 7: react-markdown does not support React 19 officially per peerDeps (`>=18`)
**What goes wrong:** `react-markdown@10.1.0`'s `peerDependencies.react` says `>=18` — npm install may warn but install succeeds. There's a chance of subtle hook-API incompatibilities.
**Why it happens:** The 10.x line was published before React 19 ratified.
**How to avoid:** Smoke-test in plan 26-04 — render a representative SKILL.md, verify no React 19 warnings in the console. If a real issue surfaces, fall back to `marked` + manual `<div dangerouslySetInnerHTML>` (with `DOMPurify` for sanitization) — but only if necessary.
**Warning signs:** Console warnings about deprecated React APIs from inside `react-markdown`.

### Pitfall 8: Variable SKILL.md description lengths breaking virtualised list height math
**What goes wrong:** UI-SPEC says list rows are 52px (two-line: primary 13px + secondary 12px). If a long source name wraps the secondary line, the row becomes 65px tall; the virtualiser thinks it's still 52px; scroll position drifts.
**Why it happens:** Fixed `rowHeight` in `ListLayout` (or `estimateSize` in TanStack Virtual) lies when content actually wraps.
**How to avoid:** Either (a) hard-clip the secondary line with `text-overflow: ellipsis; white-space: nowrap` (UI-SPEC implies this for the secondary line — fine), or (b) opt into measured rows. Recommendation: **(a) hard-clip the secondary line.** This is also better UX (predictable row geometry).
**Warning signs:** Scroll position jumps when scrolling fast; visible rows misalign with scrollbar.

### Pitfall 9: Native menu bar shortcut conflicts with React Aria FocusRing
**What goes wrong:** Native menu shortcuts (⌘C copy, ⌘O open) fire **before** the webview gets the keydown event. If a React Aria component (e.g., a SearchField) has focus, the user expects ⌘C to copy the selected text — but our menu-level ⌘C tries to copy the *focused skill's source path*. Confusion.
**Why it happens:** macOS routes menu shortcuts to the menu first; the webview never sees them.
**How to avoid:** Don't bind ⌘C / ⌘V / ⌘X / ⌘A / ⌘Z at the menu level — leave them as `PredefinedMenuItem` (`.cut()`, `.copy()`, `.paste()`) which correctly dispatch to the focused control. Use **non-conflicting custom shortcuts** for tome-specific actions: ⌘1/⌘2/⌘3 (view nav), ⌘F (search), ⌘O (open source — actually conflicts with macOS Open dialog; pick another), ⌘D (disable — conflicts with bookmarks; pick another).
**Action:** Plan 26-07 (a11y/HIG audit) reviews **every** shortcut binding against macOS HIG and the Predefined menu items to find conflicts before they ship. UI-SPEC's keyboard map may need revision.
**Warning signs:** "Why doesn't ⌘C copy text from the search box?"

### Pitfall 10: `set_skill_disabled` write doesn't trigger watcher (path normalisation)
**What goes wrong:** Watcher watches `~/.config/tome/` (expanded to `/Users/martin/.config/tome/`). `machine::save_checked` writes via a tempfile in the same dir — atomic rename. If notify on macOS doesn't fire for own-process writes (it should, but FSEvents has quirks), the silent-refresh loop breaks for the lone Phase 26 mutation.
**Why it happens:** FSEvents historically had "own-process suppression" but this was fixed; still worth verifying.
**How to avoid:** Plan 26-06 includes an integration test: trigger `set_skill_disabled`, expect a `machine_prefs_changed` event within 500ms. Test runs Rust-side (no Playwright) using `tauri::test` harness.
**Warning signs:** User clicks "Disable on this machine", badge doesn't appear until they switch views and back.

## Runtime State Inventory

Phase 26 is **not a rename/refactor/migration phase** — section omitted (greenfield UI feature on top of an existing backend).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust | Build | ✓ | rustc 1.85+ (workspace `edition = "2024"`) | — |
| cargo | Build | ✓ | bundled | — |
| Node.js | UI build | ✓ (assumed — Phase 25 already ran) | `^22` or `^20` for Vite 6 | — |
| npm | UI deps | ✓ | bundled with Node | — |
| Tauri CLI (`@tauri-apps/cli`) | Dev loop (`cargo tauri dev`) | ✓ (Phase 25 baseline) | `^2` | — |
| Playwright browsers | Perf bench (plan 26-08) | ✗ | — | First-run `npx playwright install chromium` in CI step |
| `slopcheck` | Package legitimacy gate | ✗ (no `pip` in research environment) | — | Manual review per Package Legitimacy Audit table |
| macOS hardware (M1 8GB equivalent) | NF-01 perf verification | depends | — | Plan 26-08 documents the CI runner spec; use `macos-latest` (Apple Silicon since GitHub Actions migration) |
| VoiceOver | Manual NF-02 verification | ✓ (built into macOS) | — | — |

**Missing dependencies with no fallback:** none — all required tools are installable or already present.
**Missing dependencies with fallback:** `slopcheck` (mitigated by per-install `checkpoint:human-verify`); Playwright (one-time install step).

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | tome is single-user, no auth surface |
| V3 Session Management | no | local desktop app, no sessions |
| V4 Access Control | partial | Tauri capability system controls IPC surface — every new command must appear in `capabilities/main.json` |
| V5 Input Validation | yes | All Tauri command inputs (skill_name, finding_id) validated by Rust newtypes (`SkillName::new` rejects invalid identifiers); `set_skill_disabled` writes go through `machine::save_checked` (round-trip TOML validation) |
| V6 Cryptography | no | No crypto in Phase 26 (no key material, no signing) |
| V7 Error Handling | yes | `TomeError` boundary already standardised in Phase 25; never leak raw stack traces; classify into stable codes |
| V14 Configuration | yes | `tauri.conf.json` `csp` set to `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'` (Phase 25 baseline; verify still appropriate for Phase 26 — `style-src 'unsafe-inline'` may be tightened) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious SKILL.md content rendered as raw HTML → XSS | Tampering / EoP | `react-markdown` safe-by-default — no `rehype-raw`; `allowedElements` further restricts |
| Path traversal via skill_name in Tauri command (e.g., `../../etc/passwd`) | Tampering / EoP | `SkillName` newtype rejects `.`, `..`, and path separators at construction; never construct via raw string |
| Malicious markdown link → `javascript:` URI | EoP | Link `onClick` handler calls `openUrl(href)` via Tauri opener plugin, which validates the URL scheme; reject `javascript:` / `data:` explicitly in the click handler |
| File watcher → unbounded event flood (DoS the webview) | DoS | `notify-debouncer-full` with 200ms window caps event rate; React refetch is idempotent |
| Tauri IPC surface expansion → unintended capability | EoP | Capability JSON is allow-list; every new permission requires deliberate add (caught in code review) |
| Markdown body sent to webview → could include massive payload (DoS the renderer) | DoS | Cap `SKILL.md` body length read by `get_skill_detail` at ~1MB (the lint already warns at 6000 chars; the cap is a defensive ceiling) `[ASSUMED — confirm during plan 26-03]` |
| `notify` watching paths outside the user's home → accidental info exposure | InformationDisclosure | Watcher only watches paths derived from `TomePaths` (user's own `tome_home` + machine config dir); never absolute paths from user input |

## Sources

### Primary (HIGH confidence — verified via tool/official docs)

- `npm view <pkg> version time.modified license peerDependencies.react` — independent verification of every JS package version and React 19 compatibility (run 2026-05-29)
- `cargo info notify` / `cargo info notify-debouncer-full` — confirmed `notify 8.2.0` is the latest stable line (9.0.0-rc.4 is a pre-release); confirmed `notify-debouncer-full 0.7.0` (cargo search returned `0.8.0-rc.2` which is also a pre-release)
- `crates/tome-desktop/Cargo.toml` + `crates/tome-desktop/ui/package.json` — read directly; Phase 25 scaffold versions verified
- `crates/tome-desktop/src/{lib,commands,error,sink,main}.rs` + `bindings.ts` — full Phase 25 IPC surface inspected
- `crates/tome/src/{status,doctor,lint,list,skill,manifest}.rs` + `crates/tome/src/browse/{app,fuzzy,markdown}.rs` — domain code inspected to ground every "reuse this" claim
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/ROADMAP.md` — milestone context grounded
- `.planning/phases/25-rust-core-extraction-tauri-integration-spike/25-CONTEXT.md` — D-01..D-17 carried forward
- `.planning/research/v1.0-frontend-framework-decision.md` — React 19 ADR

### Secondary (MEDIUM confidence — official docs via WebFetch)

- `https://v2.tauri.app/learn/window-menu/` — Tauri 2 `MenuBuilder` + `SubmenuBuilder` API surface
- `https://v2.tauri.app/plugin/file-system/` — Tauri 2 fs plugin `watch` + `delayMs` debounce (for OQ-3)
- `https://docs.rs/notify/8.2.0/notify/` — confirmed `notify 8.2.0` stable; FSEvents default backend; macOS FSEvents security caveats
- `https://docs.rs/notify/latest/notify/` — confirmed the latest crate version metadata
- `https://docs.rs/tauri/latest/tauri/menu/index.html` — Tauri 2 `tauri::menu` module structs
- `https://tanstack.com/virtual/latest/docs/api/virtualizer` — `useVirtualizer` + `measureElement` for dynamic heights
- `https://react-aria.adobe.com/getting-started` — `react-aria-components` is the current install path (confirmed via redirect from `react-spectrum.adobe.com/react-aria/getting-started.html`)
- `https://github.com/remarkjs/react-markdown` — `react-markdown` 10.1.0 features (safe by default, `allowedElements`, `remark-gfm`)
- `https://github.com/notify-rs/notify/releases` — confirmed 8.2.0 published 2025-08-03 and 9.0.0-rc.x is pre-release
- Web search (Brave-disabled / built-in WebSearch) — `react-aria-components` 1.17 native `<Virtualizer>` confirmed for `ListBox` (OQ-1 key finding)

### Tertiary (LOW confidence — single source, needs verification)

- `fuse.js` recommendation — common knowledge but specific version unverified in this session; planner verifies `npm view fuse.js version repository.url` before adding
- Exact `tauri::menu` builder method names (`.enabled(false)`, `.text(id, label)`, `.accelerator(str)`) — assumed standard Tauri 2 API; verify in `docs.rs/tauri/2/tauri/menu/struct.SubmenuBuilder` before plan 26-07 ships menu code
- React 19 + `react-markdown 10.1.0` compatibility — peerDeps say `>=18`, no explicit React 19 attestation; smoke-test required in plan 26-04
- FSEvents own-process write suppression — historical behaviour; assumed fixed in modern macOS; verify with the integration test in plan 26-06

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | React Aria 1.17's native `<Virtualizer>` covers VIEW-02 at 2000 items @ 60fps | OQ-1, §"Standard Stack — Virtualisation" | If perf fails, fall back to TanStack Virtual (~1-2 days of refactor; bench-discoverable in plan 26-08) |
| A2 | `react-markdown 10.1.0` is React-19 compatible despite peerDeps `>=18` | §"Standard Stack — Markdown", Pitfall 7 | If real issues, fall back to `marked` + `DOMPurify` (~1 day of refactor) |
| A3 | `fuse.js` ranking is "close enough" to nucleo's ranking that users won't complain | §"Standard Stack — Fuzzy search" | Beta user feedback may demand parity; hand-port nucleo to TS (~1-2 days) is the fallback |
| A4 | `notify 8.2.0` FSEvents backend fires events for own-process writes on macOS APFS | Pitfall 10 | If suppressed, the lone Phase 26 mutation (D-06) won't trigger silent refresh; mitigation = manual refetch after `set_skill_disabled` succeeds |
| A5 | Tauri 2's `MenuItemBuilder` has `enabled(false)` for disabled-with-tooltip menu items | §"Pattern 7", Pitfall 9 | If API differs, render disabled items via a custom solution or skip the tooltip (cosmetic) |
| A6 | The 200ms watcher debounce + Tauri event round-trip + React refetch fits within SC#1's "200ms refresh" target | §"Pattern 5", Pitfall 1 | If too slow, drop debounce to 100ms (more wakeups but tighter latency); measurable in plan 26-06 |
| A7 | Lockfile-state classification ("In sync" / "Out of sync") is content_hash comparison between lockfile and manifest | §"Standard Stack — Status dashboard" | If semantics differ (e.g., per-skill version comparison wanted), revisit during plan 26-01 |
| A8 | The browse TUI's clipboard + opener code does NOT need to be reused by the GUI — only the path computation does | §"Architecture Patterns — Action handler refactor" | If reviewer disagrees, refactor `arboard` use into `tome::actions::copy_to_clipboard` — adds Linux dep to GUI for no benefit |
| A9 | `DiagnosticIssue` finding ID via stable hash of (category, message, repair_kind) is unique enough to dispatch repairs | §"Pattern 4 — Doctor per-item fix" | Collision = wrong finding repaired; mitigation = include path fragment in hash |
| A10 | The 2000-skill synthetic-bench can use Playwright + `requestAnimationFrame` sampling to measure FPS reliably on macOS-latest CI runner | §"Pattern 6 — Perf bench" | If too flaky, switch to manual local benches with explicit `[skip-ci]` annotation |
| A11 | `slopcheck` will be runnable in the planning environment (next agent) | §"Package Legitimacy Audit" | If not, planner uses checkpoint:human-verify per package — already noted as the fallback |
| A12 | macOS-latest CI runner provides Apple Silicon (M-class) hardware comparable to the M1 8GB target | §"Pattern 6 — Perf bench" | If x86_64 runners still used in some matrix, perf numbers will be misleading; pin to `macos-latest` (current GitHub Actions = Apple Silicon since 2024) |
| A13 | `fuse.js` is the package name (not slopsquatted); needs `npm view fuse.js repository.url` confirmation | §"Package Legitimacy Audit" | Slop = exfiltrated dev-machine secrets via postinstall; mitigation is the checkpoint:human-verify before install |
| A14 | Phase 26 does not need a separate state-management library; React hooks + a tiny URL/router state suffice for 3 views + selection | §"Architecture Patterns — Anti-Patterns to Avoid" | If state grows during Phase 27 wire-up, drop Zustand in then (additive) |

## Open Questions

The Phase 25 / UI-SPEC research already resolved most of the open questions the user originally floated. These four remain and **belong to plan-time decisions**, not research:

1. **OQ-1: React Aria native `<Virtualizer>` (1.17+) vs TanStack Virtual for VIEW-02.**
   - What we know: React Aria 1.17 added `<Virtualizer layout={ListLayout}>` that wraps `<ListBox>` for a free keyboard-accessible virtualised list. TanStack Virtual is the more general-purpose primitive. UI-SPEC §"Design System" names TanStack Virtual; D-14 names TanStack Virtual. The research below `react-aria-components 1.17.0 release notes` confirms native virtualisation is now production-ready.
   - What's unclear: whether the native Virtualizer meets the NF-01 perf budget at 2000 rows on M1 8GB. No public benchmarks yet (released 2026-05-18, 11 days ago).
   - Recommendation: **Plan 26-02 starts with React Aria's native `<Virtualizer>`** (zero extra deps, free a11y). If plan 26-08's bench fails to hit 60fps, switch to TanStack Virtual + hand-wired ListBox semantics. Both UI-SPEC and D-14 should be revised to acknowledge the option flip — non-blocking; planner files a small spec amendment.

2. **OQ-2: `FindingId` derivation for `doctor::repair_one`.**
   - What we know: `DiagnosticIssue` has no stable ID today. We need one to dispatch per-item fixes (D-09 / D-10 / D-11). A hash of `(category, message, repair_kind, path_fragment)` would work, but the `message` is a human string that may change over time.
   - What's unclear: should the ID be content-derived (hash) or content-aware (e.g., `RemoveStaleTargetSymlink(/path/to/symlink)`)?
   - Recommendation: **Use a content-aware ID** like `enum FindingId { LibraryStaleManifest(SkillName), LibraryBrokenSymlink(PathBuf), TargetStaleSymlink { directory: DirectoryName, path: PathBuf }, TargetRealDirToSymlink { directory: DirectoryName, path: PathBuf } }`. This is more verbose but immune to message reword. Plan 26-05 decides.

3. **OQ-3: File watcher — Rust-side `notify` vs JS-side `@tauri-apps/plugin-fs`.**
   - What we know: Both are viable. Rust-side gives us typed events per file (`ManifestChanged`, `LockfileChanged`, …); JS-side keeps the Rust code thinner.
   - What's unclear: whether the JS plugin's `watch` honours rename events at the same granularity as `notify-debouncer-full` (which has rename-stitching specifically).
   - Recommendation: **Rust-side**, because (a) we want typed events (better for React refetch routing), (b) `notify-debouncer-full`'s rename stitching is best-in-class for atomic-rename writes (which is exactly the pattern `tome::manifest`/`lockfile` use), and (c) it keeps the IPC surface auditable (the JS plugin would need `fs:default` permissions, broadening attack surface). Decision can be revisited if Rust-side proves harder than expected.

4. **OQ-4: Should plan 26-01 also extend `StatusReport` with the lockfile state field, or is that a follow-up?**
   - What we know: UI-SPEC §"Per-view Design — Status" shows `LOCKFILE  In sync • ●green`. `StatusReport` doesn't carry this. The CLI's `tome status` doesn't surface it either — *the lockfile state classification is a new domain concept this UI motivates.*
   - What's unclear: whether it should ship in Phase 26 or be deferred (the UI could show a placeholder).
   - Recommendation: **Ship in plan 26-01** as an additive `StatusReport` field. The classification (`reconcile::classify_lockfile` already exists) can be wrapped into a `StatusReport::lockfile` field cheaply. CLI text output can be revised separately (or skipped — JSON-only carriers are fine).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `react-window` (the original virtualisation lib) | `@tanstack/react-virtual` 3.x OR `react-aria-components` `<Virtualizer>` 1.17+ | TanStack 3.0 released 2023; React Aria virtualizer 2026-05 | Both options are now mature; `react-window` is maintained but the ecosystem has migrated |
| `Electron` for desktop apps | `Tauri 2` | Tauri 2 stable Oct 2024 | Smaller bundle, Rust core, OS webview |
| Hand-rolled ARIA components | `react-aria-components` (replacing the older `@react-aria/*` hooks-only approach) | RAC 1.0 Sept 2024; 1.17 May 2026 | Drop-in headless components; less hook boilerplate |
| `marked` (returns HTML string) | `react-markdown` (returns React tree) | `react-markdown` 8.x onward | Safer (no `dangerouslySetInnerHTML`); composable with React components |
| `fs.watch` (Node.js) on the JS side | OS-native FSEvents/inotify/ReadDirectoryChangesW via `notify` (Rust) | always preferred for desktop apps | Lower CPU, no missed events |
| Polling for "did file change" | Watcher + event-driven refresh | always preferred for live UIs | No stale UI; lower CPU |

**Deprecated/outdated:**
- `react-aria` (hooks-only) → use `react-aria-components` (RAC) for new code
- `@tanstack/virtual` v2.x → use v3.x (`@tanstack/react-virtual`)
- `notify` 4.x / 5.x → 8.2.0 stable (major rewrite in 6.x for cross-platform stability)
- `marked` for React apps → `react-markdown` is safer
- `react-window` → maintained but ecosystem has moved on

## Code Examples

### Status view rendering — extends Phase 25 App.tsx

```tsx
// ui/src/views/StatusView.tsx (NEW)
// Source: extension of crates/tome-desktop/ui/src/App.tsx (Phase 25)
import { useStatus } from '../hooks/useStatus';
import { KeyValueRow } from '../components/KeyValueRow';
import { DirectoryTable } from '../components/DirectoryTable';
import { Pill } from '../components/Pill';

export function StatusView() {
  const { status, err, updatedAt } = useStatus();

  if (err) return <ErrorBanner err={err} />;
  if (!status) return <ContentPane title="Status"><LoadingSkeleton /></ContentPane>;

  return (
    <ContentPane title="Status">
      <section>
        <KeyValueRow label="TOME HOME"  value={status.library_dir.replace(/\/library$/, '')} mono />
        <KeyValueRow label="LIBRARY"    value={status.library_dir} mono
                     trailing={<span>{formatCount(status.library_count)} skills</span>} />
        <KeyValueRow label="LAST SYNC"  value={formatLastSync(status.last_sync)}
                     trailing={updatedAt && Date.now() - updatedAt < 2000 ? <Pill variant="updated">Updated</Pill> : null} />
        <KeyValueRow label="LOCKFILE"   value={formatLockfile(status.lockfile)}
                     trailing={<StatusDot ok={status.lockfile?.kind === 'InSync'} />} />
        <KeyValueRow label="MACHINE"    value={`${status.machine_prefs_summary?.disabled_count ?? 0} skills disabled`} />
      </section>
      <DirectoryTable directories={status.directories} />
    </ContentPane>
  );
}
```

### Virtualised skill list — recommended (React Aria native virtualizer)

```tsx
// ui/src/views/SkillsView.tsx (NEW)
// Source: react-aria.adobe.com/Virtualizer + UI-SPEC §"Per-view Design — Skills"
import { ListBox, ListBoxItem, ListLayout, Virtualizer, SearchField } from 'react-aria-components';
import { useFuzzySearch } from '../hooks/useFuzzySearch';
import { useSkills } from '../hooks/useSkills';

export function SkillsView() {
  const { skills } = useSkills();
  const [query, setQuery] = useState('');
  const filtered = useFuzzySearch(skills, query, { keys: ['name', 'source_name'] });
  const [selected, setSelected] = useState<string | null>(null);

  return (
    <div className={styles.split}>
      <div className={styles.listColumn}>
        <SearchField aria-label="Search skills" value={query} onChange={setQuery} />
        <Toolbar /* sort + group popups */ />
        <Virtualizer layout={ListLayout} layoutOptions={{ rowHeight: 52 }}>
          <ListBox aria-label="Skills" items={filtered} selectionMode="single"
                   selectedKeys={selected ? [selected] : []}
                   onSelectionChange={(s) => { const k = [...s][0] as string; setSelected(k ?? null); }}>
            {(skill) => (
              <ListBoxItem id={skill.name} textValue={skill.name}>
                <SkillListRow skill={skill} />
              </ListBoxItem>
            )}
          </ListBox>
        </Virtualizer>
      </div>
      <div className={styles.detailColumn}>
        {selected ? <SkillDetail skillName={selected} /> : <EmptySelectionPlaceholder />}
      </div>
    </div>
  );
}
```

### Doctor fix popover — preview-then-confirm (NF-04 / D-09)

```tsx
// ui/src/components/PreviewPopover.tsx
import { DialogTrigger, Button, Popover, Dialog, Heading } from 'react-aria-components';

export function FixButton({ finding, onApply, onError }: Props) {
  return (
    <DialogTrigger>
      <Button className={styles.fixSmall}>Fix</Button>
      <Popover>
        <Dialog aria-labelledby="preview-heading">
          {({ close }) => (
            <>
              <Heading id="preview-heading" slot="title">PREVIEW</Heading>
              <p>{finding.dry_run_description}</p>
              <p className={styles.helper}>This change is reversible by running tome sync.</p>
              <div className={styles.actions}>
                <Button onPress={close}>Cancel</Button>
                <Button className={styles.primary} onPress={async () => {
                  close();
                  try { await onApply(); } catch (e) { onError(e as TomeError); }
                }}>Apply</Button>
              </div>
            </>
          )}
        </Dialog>
      </Popover>
    </DialogTrigger>
  );
}
```

### Markdown body — SC#4 subset enforced via allow-list

```tsx
// ui/src/components/MarkdownBody.tsx
// Source: github.com/remarkjs/react-markdown + UI-SPEC §"MarkdownBody"
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { openUrl } from '@tauri-apps/plugin-opener';
import styles from './MarkdownBody.module.css';

const ALLOWED = ['h1','h2','h3','p','strong','em','code','ul','ol','li','a','pre'];

export function MarkdownBody({ body, skillName }: { body: string; skillName: string }) {
  return (
    <article aria-label={`${skillName} documentation`} className={styles.body}>
      <ReactMarkdown
        allowedElements={ALLOWED}
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ href, children }) => (
            <a href={href} onClick={async (e) => {
              e.preventDefault();
              if (href && /^https?:/.test(href)) await openUrl(href);
            }}>{children}</a>
          ),
        }}
      >{body}</ReactMarkdown>
    </article>
  );
}
```

## Project Constraints (from CLAUDE.md)

These are the actionable directives from `./CLAUDE.md` that constrain Phase 26 work; the planner must verify compliance in every plan:

1. **Non-interactive shell flags.** Always use `cp -f`, `mv -f`, `rm -f`, `rm -rf`. Never run plain `cp`/`mv`/`rm` (may be aliased to `-i` and hang).
2. **Rust edition 2024 + strict clippy.** `cargo clippy --all-targets -- -D warnings` is the gate. Phase 26 must pass on `ubuntu-latest` + `macos-latest`.
3. **Unix-only project.** No Windows support; symlinks via `std::os::unix::fs::symlink`. (Tauri-desktop is macOS-only per D-GUI-06, even tighter.)
4. **No CLI regression.** `crates/tome` ships unchanged; `crates/tome/tests/cli*.rs` must keep passing.
5. **Single user; backward compat: none.** New `StatusReport` fields (lockfile state, machine-prefs summary) can ship without migration.
6. **OpenSpec workflow** for substantial changes — Phase 26 is substantial; opens an OpenSpec change in plan 26-01.
7. **GitHub Issues + GSD** for execution state; no parallel TODO markdown files.
8. **Session completion** — work isn't done until `git push` succeeds; full quality gates (`make ci`) before push.
9. **No nested git** — git source clones go to `~/.tome/repos/`, not inside the library dir.
10. **GSD workflow enforcement** — every Edit/Write goes through a GSD command (already in play here).
11. **`make ci` matches CI** (fmt-check + clippy + tests). Plan 26-08 (perf bench) must not run as part of `make ci` unless the planner explicitly opts in (perf benches are slow).
12. **Tool preferences:** `fd`, `rg`, `jq` — not `find`/`grep`/`ls -R`/`cat | grep`.
13. **GitHub PRs as DRAFT, first commit empty.** Already standard for this repo.
14. **`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`** is already on `main.rs` — leave it; harmless on macOS.

Phase 26's draft plan list (26-01 through 26-08) maps cleanly to a single feature branch with per-plan commits; the planner should use `make ci` between each plan to catch regressions early.

## Metadata

**Confidence breakdown:**
- Standard stack (versions + capabilities): HIGH — every package version independently verified via `npm view` / `cargo info` / `docs.rs` on 2026-05-29
- Architecture patterns (commands, watcher, virtualizer, perf bench): HIGH — patterns match documented Tauri 2 + React Aria + notify usage
- VIEW-02 virtualisation choice: MEDIUM — OQ-1 resolution recommended (React Aria native) but bench-verified only in plan 26-08
- Detail-action handler refactor: HIGH — minimal Rust-side refactor, clear scope (`tome::actions` module)
- Doctor per-item fix design: MEDIUM — needs `FindingId` design decision (OQ-2) before plan 26-05
- Perf bench harness: MEDIUM — approach is standard, exact CI runner specs assumed (Apple Silicon)
- File watcher integration: HIGH — `notify 8.2` + `notify-debouncer-full` 0.7 is the standard pattern; recommended over the JS-side alternative
- Markdown subset rendering: HIGH — `react-markdown` + `allowedElements` is the textbook approach
- A11y testing strategy: MEDIUM — React Aria gives WCAG-AA defaults; axe-core verifies; VoiceOver smoke-test is manual

**Research date:** 2026-05-29
**Valid until:** ~2026-06-30 for the JS stack (npm packages move fast); ~2026-09-01 for Rust crates (slower-moving). Earlier expiry warranted on `react-aria-components` (1.17.0 is 11 days old; refresh on virtualizer perf claims if planning slips into Q3 2026).

## RESEARCH COMPLETE

**Phase:** 26 - read-only-views-alpha-cut
**Confidence:** HIGH on stack + patterns; MEDIUM on OQ-1 (virtualisation flavour), OQ-2 (FindingId shape), OQ-4 (lockfile state in plan 26-01 vs deferred). All four OQs are planning decisions, not research gaps.

### Key Findings

1. **React Aria 1.17 (May 2026) ships a native `<Virtualizer>` for `ListBox`** that may make TanStack Virtual redundant — UI-SPEC names TanStack Virtual, but the cheaper-and-equivalent path is the native one (zero extra dep, free a11y semantics). Recommend starting with React Aria native; switch only if NF-01 bench fails. (OQ-1)
2. **The browse TUI's markdown renderer is intentionally NOT reused** — it's 3 elements (headers + HRs + inline bold/italic/code) and ratatui-only. UI-SPEC's SC#4 markdown subset is much richer (headings + lists + links + code blocks + inline emphasis). D-08 already locks `react-markdown` + `remark-gfm`. VIEW-04's literal wording in REQUIREMENTS.md needs a cleanup commit.
3. **One new Rust module** (`tome::actions`) suffices to share path computation + machine.toml mutation between TUI and GUI. The TUI's clipboard/opener code is NOT shared — the GUI uses Tauri plugins for those, which is correct (different OS-call shape).
4. **`notify 8.2.0` stable + `notify-debouncer-full 0.7.0`** is the file watcher. `notify 9.0.0-rc.4` is a pre-release; do NOT adopt. Debounce ~200ms; watch four roots (manifest, lockfile, library dir, machine.toml).
5. **Fuzzy search runs JS-side** via `fuse.js` (verify package legitimacy before install). Per-keystroke Tauri command would eat 6-30% of the 60fps budget. Ranking divergence from CLI's `nucleo` is the documented trade-off; revisit in beta.
6. **Doctor per-item fix needs a new `FindingId`**; current `dispatch_repairs` is batch-only. Plan 26-05 designs the ID shape (OQ-2) — recommendation is content-aware enum rather than message hash.
7. **`StatusReport` needs two additive fields**: `lockfile: LockfileState` and `machine_prefs_summary: MachinePrefsSummary` (UI-SPEC asks for both; neither exists today). Plan 26-01 owns this.

### File Created

`/Users/martin/dev/opensource/tome/.planning/phases/26-read-only-views-alpha-cut/26-RESEARCH.md`

### Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Standard Stack | HIGH | Every package version independently verified via `npm view` / `cargo info` on 2026-05-29 |
| Architecture | HIGH | Patterns match official Tauri 2 + React Aria + notify documentation |
| Pitfalls | HIGH | Each pitfall has a documented warning sign + mitigation; ten enumerated |
| Virtualisation choice | MEDIUM | OQ-1 — React Aria native vs TanStack Virtual; bench-verifiable in plan 26-08 |
| Detail-action handler refactor | HIGH | Scope is narrow (new module + minimal TUI rewire) |
| Doctor per-item fix | MEDIUM | OQ-2 — `FindingId` shape decision deferred to plan 26-05 |
| Perf bench harness | MEDIUM | Standard approach; CI runner specs assumed |
| Markdown rendering | HIGH | `react-markdown` + `allowedElements` is textbook |

### Open Questions

OQ-1: React Aria native `<Virtualizer>` vs TanStack Virtual for VIEW-02 — recommend starting with native; UI-SPEC may need amendment.
OQ-2: `FindingId` derivation for `doctor::repair_one` — recommend content-aware enum; plan 26-05 decides.
OQ-3: File watcher Rust-side vs JS-side — recommend Rust-side for typed events + auditable IPC surface.
OQ-4: Should `StatusReport` lockfile field ship in plan 26-01 or be deferred — recommend ship in plan 26-01.

### Ready for Planning

Research complete. Planner can create 8 plans (26-01..26-08) per the draft list, with OQ-1..OQ-4 surfaced for the planner's first-cut decisions.

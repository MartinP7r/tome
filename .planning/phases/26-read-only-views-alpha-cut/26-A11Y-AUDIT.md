---
phase: 26-read-only-views-alpha-cut
plan: 07
type: audit
status: in-progress
requirements: [NF-02, NF-03]
created: 2026-05-29
---

# Phase 26 Plan 07 — Accessibility & HIG Audit

The cross-cutting a11y / macOS-HIG bar for the alpha cut. This doc captures:

1. The **keyboard shortcut conflict audit** that drove the UI-SPEC §Keyboard Map revision 3 (Pitfall 9).
2. The **VoiceOver smoke-test checklist** — every aria-label template from UI-SPEC §VoiceOver labels, walked once with VoiceOver on.
3. **Reduced-motion + reduced-transparency + dark-mode** verification — the System Settings toggles that NF-02 must honour.
4. The **axe-core CI gate baseline** — first-run output of Task 3's `npm run test:a11y`.

---

## 1. Keyboard shortcut audit — results

Walked every row of UI-SPEC §Keyboard Map against the macOS HIG conventional shortcuts, the Tauri Predefined Edit menu items, and the webview-default intercepts. Three conflicts surfaced; resolutions below applied in commit at the end of Task 2.

### Resolutions

| Original binding (plan 26-03) | Conflict | New binding | Justification |
|---|---|---|---|
| `⌘C` — copy source path, scoped to "skill selected" | Edit menu's **Predefined Copy** owns ⌘C; if the SearchField (or any input) had focus the two would race. Custom code firing first would block native text copy. | `⌘C` retained, **guarded** by `activeElement` check. When focus is on an `<input>` / `<textarea>` / contenteditable / `role="searchbox"` / `role="textbox"`, the skill-scoped handler abstains — the Predefined Copy item wins (Pitfall 9, T-26-07-01). | Lets users copy search text with ⌘C in the SearchField AND copy the selected skill's source path with ⌘C from the list/detail. Implemented as `isTextInputFocused()` helper in `SkillsView.tsx`. |
| `⌘O` — open source folder in Finder | macOS HIG: bare ⌘O is "Open…" dialog. Tome doesn't have an Open dialog in Phase 26, but Phase 27+ likely will (file picker for "Add directory"). Pre-empting the convention now means we'd have to rebind later. | `⌘⇧O` — explicit "shift" variant tied to "Open _Source_ folder" — the Shift is mnemonic. | Keeps bare ⌘O free for a future Open dialog. Existing button label "Open source folder" still wins as the discoverable surface; the shortcut is the power-user accelerator. |
| `⌘D` — Disable selected skill on this machine | macOS HIG: bare ⌘D variously means Don't Save (NSAlert), Duplicate (Finder/Pages), Add Bookmark (Safari). Every interpretation overlaps "destructive-ish toggle" in a way that risks misfire. The button is a single click from the keyboard via Tab. | **Removed entirely.** No keyboard shortcut for Disable in Phase 26. | The action is a deliberate toggle — discoverable via the DetailHeader's labelled button. Single-key shortcuts for rare deliberate actions add risk without payoff. Bare ⌘D is also held free for a future "Duplicate skill" surface, where it would map to HIG more naturally. |

### Bindings retained as-is

| Binding | Why no change |
|---|---|
| `⌘1` / `⌘2` / `⌘3` | No HIG conflict at the bare-Cmd-digit level. Now dispatched by the **native macOS menu** (View → Status / Skills / Health) → typed `MenuAction` event → `useMenuActions` hook. The previous document-level keydown listener in `App.tsx` was removed to avoid double-firing. |
| `⌘F` | macOS HIG: ⌘F is "Find" — and that's exactly what we use it for (focus the search field). Now dispatched by the native menu (View → Focus Search) → `MenuAction::FocusSearch` → `useMenuActions` focuses the SearchField by aria-label. Previous SkillsView-scoped keydown listener removed. |
| `Esc` | React Aria primitives handle this natively (clear search, close popover, close context menu). No custom binding needed. |
| `↑` / `↓` / `Home` / `End` / `Page Up` / `Page Down` / `Enter` / `Space` | React Aria ListBox / Virtualizer / Button defaults. |
| `Shift+F10` / `⌃Space` | React Aria Menu default for opening a contextual menu on the focused list row. |
| `⌘W` | OS-level Close Window via Tauri. The File menu's Predefined `close_window` item registers it. |
| `⌘,` | Reserved for Phase 28 Settings — explicitly not bound in Phase 26. |
| `⌘R` / `⌘N` / `⌘Z` / `⌘⇧Z` | Reserved for future phases. `⌘R` appears in the View menu disabled, which surfaces the breadcrumb without intercepting the accelerator. |

### Predefined macOS Edit menu — Pitfall 9 contract

The Edit menu (`menu.rs`) is built from `tauri::menu::PredefinedMenuItem` calls — `.undo()`, `.redo()`, `.cut()`, `.copy()`, `.paste()`, `.select_all()`. The OS registers ⌘Z / ⌘⇧Z / ⌘X / ⌘C / ⌘V / ⌘A against these items and routes them to the focused webview control automatically. Custom code MUST NOT bind any of these accelerators at the menu level (would intercept text input) or at the document level (would double-fire). The skill-scoped ⌘C in `SkillsView.tsx` is the only custom binding that touches a Predefined accelerator — and it gates itself via `isTextInputFocused()` so the Predefined item wins on text inputs.

---

## 2. VoiceOver smoke-test checklist

Sequence runs against `cargo tauri dev` with VoiceOver on (⌘F5). Each row's `aria-label` template comes verbatim from UI-SPEC §VoiceOver labels — explicit contracts. Check off each row after a single complete pass.

### 2.1 Pre-flight

- [ ] Enable VoiceOver (System Settings → Accessibility → VoiceOver → on, OR ⌘F5).
- [ ] Set VoiceOver verbosity to default (the OS shipping default).
- [ ] Launch `cargo tauri dev`.

### 2.2 Status view (default landing per D-02)

- [ ] Tab into the Window — first stop is the Sidebar.
- [ ] Sidebar selected NavItem reads: **"Status, Status section, selected"** (template: `${name}, ${section} section, selected`).
- [ ] Tab forward to next NavItem (Skills) — reads **"Skills, Skills section"** (no "selected" suffix).
- [ ] Tab to the Health NavItem — when `findings.length > 0`, reads **"Health, Health section, N health issues"** (template: `Health, Health section, ${count} health issues`).
- [ ] Tab into the Status pane — heading reads "Status".
- [ ] Tab through KeyValueRow entries — each label/value pair reads as expected.
- [ ] **Transient pill** — trigger a watcher-driven refresh (e.g. external `tome sync`) and verify the "Updated" pill announces once via `aria-live="polite"`.

### 2.3 Skills view

- [ ] Use the menu (View → Skills) or click the Sidebar NavItem to switch.
- [ ] Sidebar Skills NavItem now reads **"Skills, Skills section, selected"**.
- [ ] Tab into the Skills list pane — focus lands on the SearchField.
- [ ] SearchField reads **"Search skills"** (the SearchField's `aria-label`).
- [ ] Tab to Sort PopupMenu — reads **"Sort skills"** (the PopupMenu's `aria-label`).
- [ ] Tab to Group PopupMenu — reads **"Group skills"**.
- [ ] Tab into the ListBox — first row reads **"${name}, source ${sourceName}, managed"** or **"${name}, source ${sourceName}, local"** (templates: `SkillListRow` default).
- [ ] If a row is disabled, it reads **"${name}, source ${sourceName}, local, disabled on this machine"** (template: `SkillListRow` disabled).
- [ ] Select a skill (Enter) — focus moves to the detail column.

### 2.4 Detail header (skill selected)

- [ ] Tab through the DetailHeader action buttons.
- [ ] "Open source folder" button reads **"Open source folder for ${skillName} in Finder"** (verbatim template).
- [ ] "Copy path" button reads **"Copy source path for ${skillName} to clipboard"** (verbatim).
- [ ] "Disable on this machine" button reads **"Disable ${skillName} on this machine"** (verbatim).
- [ ] Trigger the Disable button — the aria-live region announces the resulting state ("Updated" pill or similar per `useSkillActions.announcement`).
- [ ] **Edge case (D-03)** — externally delete a skill that's selected. Verify VoiceOver reads **"Selected skill was removed."** (aria-live polite region).
- [ ] Verify ⌘C in the SearchField copies the search text (Predefined Edit > Copy wins); ⌘C in the list/detail invokes "Copy path" handler.
- [ ] Verify `⌘⇧O` invokes "Open source folder" (rebound from bare ⌘O).
- [ ] Verify bare `⌘D` does NOT toggle Disable (removed by audit) — pressing it should do nothing.

### 2.5 Health view

- [ ] Switch to Health (View → Health or ⌘3).
- [ ] If findings.length is 0, the all-clear state reads **"Library health: everything looks healthy"** (template: `role="status"`).
- [ ] Section heading **AUTO-FIXABLE (N)** appears in the VoiceOver headings rotor.
- [ ] Section heading **NEEDS ATTENTION (N)** appears in the headings rotor.
- [ ] Tab into an auto-fixable FindingRow — reads **"Warning finding: ${title}. ${description}. Fix available."** (verbatim).
- [ ] Tab into a manual FindingRow — reads **"Blocked finding: ${title}. ${description}. Manual remediation required."** (verbatim).
- [ ] SeverityIcon is decorative (aria-hidden) — VoiceOver does NOT read the icon as a separate element.

### 2.6 PreviewPopover (Health Fix)

- [ ] Trigger an auto-fixable FindingRow's Fix button.
- [ ] PreviewPopover opens as a Dialog — `role="dialog"`, `aria-modal="true"`.
- [ ] PreviewPopover's `aria-labelledby` resolves to the "PREVIEW" heading.
- [ ] Focus traps inside the dialog.
- [ ] Escape closes the dialog and returns focus to the Fix button.
- [ ] Apply: focus moves to the (now-likely-removed) row's parent section header.
- [ ] Helper text reads **"This change is reversible by running tome sync."** (verbatim).

### 2.7 Native menu bar

- [ ] App menu shows "tome" → About / Services / Hide / Hide Others / Show All / Quit (system-provided).
- [ ] File menu → "Close Window" (⌘W).
- [ ] Edit menu → Undo / Redo / Cut / Copy / Paste / Select All (Predefined; routes to webview).
- [ ] View menu → Status (⌘1) / Skills (⌘2) / Health (⌘3) / separator / Focus Search (⌘F) / separator / Reload (⌘R, disabled).
- [ ] Library menu → Sync (disabled) / Add Directory… (disabled).
- [ ] Help menu → Documentation / Report Issue — verify each opens the GitHub URL in the system browser.
- [ ] Click View → Status: switches view via `MenuAction::JumpStatus` event.
- [ ] Click View → Focus Search: focus moves to the SearchField (if not on Skills view, also switches first).

---

## 3. Reduced-motion / reduced-transparency / dark-mode verification

### Reduced motion (System Settings → Accessibility → Display → Reduce motion)

- [ ] Toggle ON.
- [ ] Trigger an "Updated" pill (watcher-driven refresh).
- [ ] Pill SHOULD snap visible then hide instantly (no fade transition). UI-SPEC §"Pill — Updated" / §Reduced motion.
- [ ] Toggle OFF and verify the fade returns.

### Reduced transparency (System Settings → Accessibility → Display → Reduce transparency)

- [ ] Toggle ON.
- [ ] Verify the Sidebar uses the solid fallback (`--sidebar-material` light or dark — no vibrancy/translucency).
- [ ] Toggle OFF and verify the vibrancy returns.

### Dark mode (System Settings → Appearance)

- [ ] Toggle to Dark.
- [ ] Every view recolours without app restart — Sidebar, Titlebar, ContentPane, DetailHeader, FindingRow, PreviewPopover, SearchField.
- [ ] Token values come from CSS variables (`--bg-window`, `--label-primary`, etc.) per UI-SPEC §Color; the dark Apple-pair placeholders track the live tokens.
- [ ] Toggle to Light and verify recolouring.
- [ ] Toggle to Auto and verify the OS schedule drives it.

---

## 4. axe-core/playwright CI gate baseline

Populated by Task 3 once `npm run test:a11y` runs against the built UI for the first time.

### First-run results

_(pending Task 3 — to be filled when `npm run test:a11y` is wired and run against the alpha cut.)_

| View | Violations (count) | Worst severity | Resolution |
|------|-------------------|----------------|------------|
| Status | — | — | — |
| Skills | — | — | — |
| Health | — | — | — |
| PreviewPopover (open) | — | — | — |

### Known exceptions (`disableRules` with justification)

_(none — empty unless a violation can't be fixed in this plan.)_

---

## Cross-references

- UI-SPEC §Keyboard Map (revision 3 — 2026-05-29) — the canonical key→action table.
- UI-SPEC §VoiceOver labels — every aria-label template the smoke-test walks.
- `crates/tome-desktop/src/menu.rs` — the native macOS menu owner.
- `crates/tome-desktop/ui/src/hooks/useMenuActions.ts` — the React subscriber.
- `crates/tome-desktop/ui/src/views/SkillsView.tsx::isTextInputFocused` — Pitfall 9 / T-26-07-01 guard.
- 26-RESEARCH.md Pitfall 9 — the upstream conflict catalogue.

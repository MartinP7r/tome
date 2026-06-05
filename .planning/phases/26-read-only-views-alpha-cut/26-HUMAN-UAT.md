---
status: partial
phase: 26-read-only-views-alpha-cut
source: [26-VERIFICATION.md]
started: 2026-05-29T11:00:00Z
updated: 2026-05-29T11:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. 3-column NavigationSplitView shell + macOS chrome
expected: Mail/Notes-style layout — translucent vibrancy sidebar on the left with Status/Skills/Health nav items, 44px unified titlebar (`tome — Status`) with traffic-light controls overlaid, content pane with KeyValueRows and DirectoryTable on initial Status view.
why_human: Visual layout, vibrancy material, and macOS chrome cannot be verified by grep or TS checks.
how: `cd crates/tome-desktop && cargo tauri dev`
result: [pending]

### 2. View switching via ⌘1/⌘2/⌘3 and native macOS menu
expected: ⌘1 → Status view, ⌘2 → Skills view (SearchField visible + virtualised list), ⌘3 → Health view. Native menu **View → Status/Skills/Health** produces identical transitions.
why_human: Native menu accelerators and view routing require a running Tauri app.
how: With app running, try each accelerator and each menu item.
result: [pending]

### 3. File watcher round-trip
expected: With app running on Status / Skills view, run `tome sync` from a separate terminal. Within ~200ms: Status `LAST SYNC` updates, Updated pill flashes for ~2s, Skills list reflects any additions/removals. No stale state.
why_human: File watcher round-trip with real on-disk changes requires live app + CLI.
how: Two terminals: (1) `cargo tauri dev`, (2) `tome sync`.
result: [pending]

### 4. VoiceOver full a11y checklist
expected: All 30+ aria-label templates from UI-SPEC §VoiceOver labels read correctly. Focus traps in PreviewPopover. SkillListRow reads name/source/managed-or-disabled. "Selected skill was removed." announces on external deletion.
why_human: VoiceOver screen-reader output requires real macOS Accessibility + live app; axe-core covers DOM-level rules but not verbal output quality.
how: Run `cargo tauri dev`; System Settings → Accessibility → VoiceOver ON; walk the checklist in `26-A11Y-AUDIT.md §2.2–§2.7`.
result: [pending]

### 5. Reduce motion
expected: System Settings → Accessibility → Display → Reduce motion ON: trigger a watcher-driven StatusView refresh. The Updated pill appears instantly and disappears instantly (no CSS fade). With reduce-motion OFF, the 2s fade animation runs.
why_human: `prefers-reduced-motion` CSS media query requires a real system-settings toggle.
how: Toggle System Settings while app is running; trigger refresh by `tome sync`.
result: [pending]

### 6. Reduce transparency
expected: System Settings → Accessibility → Display → Reduce transparency ON: Sidebar switches from vibrancy material to solid `--sidebar-material` fallback (no translucency).
why_human: Tauri windowEffects + macOS transparency system preference requires live app.
how: Toggle System Settings while app is running.
result: [pending]

### 7. Dark mode (no in-app theme switcher per D-16)
expected: System Settings → Appearance → Dark: every surface (Sidebar, Titlebar, ContentPane, DetailHeader, FindingRow, PreviewPopover, SearchField, MarkdownBody) switches to dark CSS token values driven by `prefers-color-scheme`. No restart required.
why_human: CSS custom-property dark mode requires live system appearance change + visual check.
how: Toggle System Settings → Appearance while app is running.
result: [pending]

### 8. Disable on this machine — end-to-end mutation
expected: Select a skill in SkillsView; click **Disable on this machine** in DetailHeader. Click → atomic `machine.toml` write → watcher fires `MachinePrefsChanged` → DetailHeader refetches → "disabled" badge appears within ~200ms. Aria-live region announces the state change.
why_human: End-to-end mutation + watcher round-trip requires live app with real `machine.toml`.
how: Pick any skill in the running app; click Disable; observe DetailHeader.
result: [pending]

### 9. Doctor fix — end-to-end repair
expected: Trigger a doctor finding (e.g. `ln -s /nonexistent ~/.tome/library/_broken`). Finding appears in HealthView **AUTO-FIXABLE** section → click **Fix** → PreviewPopover shows dry-run description → click **Apply** → finding disappears (watcher fires `LibraryChanged`) → sidebar Health badge decrements.
why_human: Real doctor findings require live filesystem state; repair execution requires actual Tauri commands.
how: Create the broken symlink, open the app, navigate to Health.
result: [pending]

### 10. ⌘C scoping guard + ⌘⇧O behaviour
expected: When SearchField has focus and user types ⌘C: text is copied (Predefined Edit > Copy wins). When list/detail has focus and user types ⌘C: skill source path is copied. ⌘⇧O opens Finder to skill source folder.
why_human: Keyboard shortcut scoping (`isTextInputFocused` guard) requires live interaction test.
how: With a skill selected, focus the SearchField (⌘F) and ⌘C; then click into the list and ⌘C; then ⌘⇧O.
result: [pending]

## Summary

total: 10
passed: 0
issues: 0
pending: 10
skipped: 0
blocked: 0

## Gaps

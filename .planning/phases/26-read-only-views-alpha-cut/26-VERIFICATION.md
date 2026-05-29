---
phase: 26-read-only-views-alpha-cut
verified: 2026-05-29T12:00:00Z
status: gaps_found
score: 14/17 must-haves verified
overrides_applied: 0

gaps:
  - truth: "cargo fmt --all -- --check passes on all phase-modified files"
    status: failed
    reason: "doctor.rs (4 spots), skill.rs (3 spots), commands.rs (1 spot) fail cargo fmt --check. All three files were modified by phase-26 commits (17e022d, fcf3bba, 8c021da). The deferred-items.md attributes this to 'pre-existing drift' but git log confirms all drift-bearing files were touched by this phase's commits. CI includes cargo fmt --all -- --check and would fail on merge."
    artifacts:
      - path: "crates/tome/src/doctor.rs"
        issue: "4 formatting drift spots (FindingId struct literal brace placement, lines 1483/1526/1566/3687)"
      - path: "crates/tome/src/skill.rs"
        issue: "3 formatting drift spots (Ok(json) arm collapse, lines 180/285/430)"
      - path: "crates/tome-desktop/src/commands.rs"
        issue: "1 formatting drift spot (import sort order, line 10)"
    missing:
      - "Run `cargo fmt --all` and commit the formatting fix before merging"

  - truth: "Skill list view group-by (none / source / role) visually renders grouped sections"
    status: partial
    reason: "The Group PopupMenu toolbar is wired (shows None/Source/Role options) but renders flat in all modes — no section headers are produced. The 26-02 SUMMARY documents this as 'open follow-up pending the 26-08 perf bench result' but it is not captured in deferred-items.md or explicitly assigned to a later ROADMAP phase. VIEW-02 in REQUIREMENTS.md requires 'group-by (none / source / role)' without qualification."
    artifacts:
      - path: "crates/tome-desktop/ui/src/views/SkillsView.tsx"
        issue: "Line 115: `void group;` — group variable consumed as no-op; comment says TODO 26-03+"
    missing:
      - "Implement section-header rendering for source/role grouping, OR formally defer this sub-feature to a later ROADMAP phase with an explicit entry in deferred-items.md"

  - truth: "Skill list view 'recent' sort mode sorts by last-sync timestamp"
    status: partial
    reason: "Recent sort falls back to alphabetical name sort. 26-02 SUMMARY documents the fallback (DiscoveredSkill has no synced_at field; that's a discovery-time projection). Not captured in deferred-items.md, not assigned to a later ROADMAP phase. VIEW-02 requires 'sort modes (name / source / recent)'."
    artifacts:
      - path: "crates/tome-desktop/ui/src/views/SkillsView.tsx"
        issue: "sortSkills() falls back to name sort for 'recent' mode with a code comment"
    missing:
      - "Either implement recent sort by extending DiscoveredSkill with a manifest-sourced timestamp, OR formally defer to a later ROADMAP phase"

deferred:
  - truth: "NF-01 perf bench passes (p95 < 18ms on 2000-skill list, search-as-you-type)"
    addressed_in: "Phase 27 or later (OQ-1 evaluation in 26-PERF-REPORT.md — CI on macos-latest is the source of truth)"
    evidence: "Local runs land p95 at 18.10–18.40ms (0.1–0.4ms over). p50 is exactly 60fps. 26-PERF-REPORT.md §OQ-1 explicitly documents that TanStack Virtual swap is on-the-table if CI also shows boundary failure. Plan instructions prohibited auto-fixing this."

human_verification:
  - test: "Launch cargo tauri dev and visually verify the 3-column NavigationSplitView shell"
    expected: "Mail/Notes-style layout: translucent vibrancy sidebar on the left with Status/Skills/Health nav items, 44px unified titlebar (tome — Status) with traffic-light controls overlaid, content pane with KeyValueRows and DirectoryTable on initial Status view"
    why_human: "Visual layout, vibrancy material, and macOS chrome cannot be verified by grep or TS checks"

  - test: "Switch views via ⌘1, ⌘2, ⌘3 and via the native macOS menu (View menu)"
    expected: "⌘1 → Status view, ⌘2 → Skills view (SearchField visible + virtualised list), ⌘3 → Health view. Native menu View → Status/Skills/Health produces identical transitions"
    why_human: "Native menu accelerators and view routing require a running Tauri app"

  - test: "Run tome sync from CLI while the GUI is open; observe StatusView and SkillsView"
    expected: "Within 200ms: Status LAST SYNC updates, Updated pill flashes for ~2s, Skills list reflects any additions/removals. No stale state."
    why_human: "File watcher round-trip with real on-disk changes requires live app + CLI"

  - test: "VoiceOver: full checklist from 26-A11Y-AUDIT.md §2.2–§2.7"
    expected: "All 30+ aria-label templates from UI-SPEC §VoiceOver labels read correctly. Focus traps in PreviewPopover. SkillListRow reads name/source/managed or disabled. 'Selected skill was removed.' announces on external deletion."
    why_human: "VoiceOver screen-reader output requires real macOS Accessibility + live app; axe-core covers DOM-level rules but not verbal output quality"

  - test: "System Settings → Reduce motion ON: trigger watcher-driven StatusView refresh"
    expected: "Updated pill appears instantly, disappears instantly (no CSS fade). With reduce-motion OFF, the 2s fade animation runs."
    why_human: "prefers-reduced-motion CSS media query requires real system settings change"

  - test: "System Settings → Reduce transparency ON: verify Sidebar"
    expected: "Sidebar switches from vibrancy material to solid --sidebar-material fallback (no translucency)"
    why_human: "Tauri windowEffects + macOS transparency system preference requires live app"

  - test: "System Settings → Dark mode: verify all views recolour without restart"
    expected: "Every surface (Sidebar, Titlebar, ContentPane, DetailHeader, FindingRow, PreviewPopover, SearchField, MarkdownBody) switches to dark CSS token values driven by prefers-color-scheme"
    why_human: "CSS custom-property dark mode requires live system appearance change + visual check"

  - test: "Select a skill, click Disable on this machine. Observe DetailHeader badge change."
    expected: "Click → machine.toml atomic write → watcher fires MachinePrefsChanged → DetailHeader refetches → 'disabled' badge appears within ~200ms. Aria-live region announces state change."
    why_human: "End-to-end mutation + watcher round-trip requires live app with real machine.toml"

  - test: "Health view: trigger a doctor finding (e.g. add a broken symlink in library dir). Click Fix."
    expected: "Finding appears in AUTO-FIXABLE section → click Fix → PreviewPopover shows dry-run description → click Apply → finding disappears (watcher fires LibraryChanged) → sidebar badge decrements"
    why_human: "Real doctor findings require live filesystem state; repair execution requires actual Tauri commands"

  - test: "SkillsView: ⌘C copies source path (not text from SearchField when it has focus)"
    expected: "When SearchField has focus and user types ⌘C: text is copied (Predefined Edit > Copy wins). When list/detail has focus and user types ⌘C: skill source path is copied. ⌘⇧O opens Finder to skill source folder."
    why_human: "Keyboard shortcut scoping (isTextInputFocused guard) requires live interaction test"
---

# Phase 26: Read-only views — alpha cut — Verification Report

**Phase Goal:** Ship the read-only half of the GUI: status dashboard, virtualised skill list, detail pane with markdown preview, doctor health view, and on-disk file watcher. After this phase, the desktop app is useful for inspection but does not mutate state. First user-visible UI; built on the React scaffold chosen in Phase 25.
**Verified:** 2026-05-29
**Status:** GAPS FOUND
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Status dashboard renders resolved tome_home, library dir, skill count, last sync, lockfile state, machine-prefs summary | ✓ VERIFIED | StatusView.tsx renders 5 KeyValueRows; status.rs gathers LockfileState + MachinePrefsSummary; bindings.ts exposes both |
| 2 | Every directory in StatusReport.directories renders with role badge and type badge | ✓ VERIFIED | DirectoryTable.tsx renders role→Badge subtype map; type-string→subtype map; tested in cargo test |
| 3 | LockfileState classifies as InSync / OutOfSync { drift_count } / Missing | ✓ VERIFIED | status.rs LockfileState enum with POLISH-04 sentinel; classify() reuses manifest::load + lockfile::load |
| 4 | MachinePrefsSummary exposes disabled_count and disabled_directory_count | ✓ VERIFIED | MachinePrefsSummary struct in status.rs; gather() populates from machine::load |
| 5 | cargo test -p tome passes; existing CLI snapshot tests updated in lock-step | ✓ VERIFIED | 909 unit tests + 44 cli tests pass; cli_status__status_empty_library.snap intentionally re-blessed |
| 6 | Skill list renders ≥2000 skills at 60fps with fuzzy search as-you-type | ? UNCERTAIN (human needed) | React Aria Virtualizer + fuse.js wired and substantive; local perf runs land p95 18.1–18.4ms (documented boundary); CI on dedicated macos-latest is source of truth |
| 7 | Skill list sort modes (name / source / recent) work | ✗ FAILED (partial) | name + source sort implemented; recent sort falls back to name — see gap #3 |
| 8 | Skill list group-by (none / source / role) renders grouped sections | ✗ FAILED (partial) | Group toolbar wired; renders flat in all modes — see gap #2 |
| 9 | Detail pane renders frontmatter, source path, hash, last sync, managed/local badge, disabled state + 3 actions | ✓ VERIFIED | DetailHeader.tsx with 3-row layout; 4 Tauri commands (get_skill_detail, set_skill_disabled, open_source_folder, copy_path); SkillDetail struct in skill.rs |
| 10 | Markdown preview renders SC#4 subset (H1-H3, p, bold/italic/code, ul/ol/li, links, pre/code blocks) | ✓ VERIFIED | MarkdownBody.tsx with 12-element ALLOWED list; react-markdown + remark-gfm; 5 Vitest tests covering allow-list + scheme guard |
| 11 | Health pane lists doctor findings with one-click fix actions through real repair handlers | ✓ VERIFIED | HealthView.tsx; FindingId enum + repair_one() in doctor.rs; PreviewPopover NF-04 confirm flow; 2 Tauri commands (get_doctor_report, doctor_repair_one) |
| 12 | File watcher reloads UI state on manifest/lockfile/library/machine.toml changes | ✓ VERIFIED | watcher.rs with notify 8.2 + debouncer; 4 typed tauri-specta events; watcher_smoke tests prove own-process + external writes fire events within 2000ms |
| 13 | No stale UI after CLI sync (VIEW-06) | ✓ VERIFIED | Per-hook event-subscription matrix; useStatus/useSkills/useSkillDetail/useDoctorReport all subscribe to appropriate subsets |
| 14 | Every interactive element is keyboard-accessible (NF-02) | ? UNCERTAIN (human needed) | React Aria primitives provide free keyboard nav; axe-core gate passes (4/4 with color-contrast deferred); VoiceOver checklist in 26-A11Y-AUDIT.md §2 not yet walked against live app |
| 15 | Native macOS menu bar (NF-03): tome/File/Edit/View/Library/Help | ✓ VERIFIED | menu.rs with 6 submenus; MenuAction typed event; install_menu() wired in main.rs::setup; CI a11y job confirmed |
| 16 | Disable on this machine mutation works (D-06) | ✓ VERIFIED | set_skill_disabled command routes through actions::set_skill_disabled; atomic machine.toml write; browse TUI apply_toggle Global scope refactored to same helper |
| 17 | cargo fmt --all -- --check passes on all phase-modified files | ✗ FAILED | doctor.rs (4 spots), skill.rs (3 spots), commands.rs (1 spot) fail fmt check; all three touched by phase-26 commits (17e022d, fcf3bba, 8c021da); CI fmt-check gate would fail on merge |

**Score:** 14/17 truths verified (12 VERIFIED, 2 FAILED/partial, 1 UNCERTAIN/human, 2 FAILED on sub-features)

### Deferred Items

Items not yet met but explicitly addressed in later milestone phases.

| # | Item | Addressed In | Evidence |
|---|------|-------------|---------|
| 1 | NF-01 p95 < 18ms on dedicated CI hardware | Phase 27 or later (CI run) | 26-PERF-REPORT.md §OQ-1: local runs land p95 18.10–18.40ms; CI on macos-latest is source-of-truth; TanStack Virtual swap is fallback if CI confirms regression |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/tome/src/status.rs` | LockfileState + MachinePrefsSummary added to StatusReport | ✓ VERIFIED | Both types present, specta-gated, with POLISH-04 sentinel |
| `crates/tome-desktop/ui/src/views/StatusView.tsx` | Renders all StatusReport fields via atoms | ✓ VERIFIED | 5 KeyValueRows + DirectoryTable; imports useStatus + relativeTime |
| `crates/tome-desktop/ui/src/views/SkillsView.tsx` | Virtualised list + fuzzy search + sort + group | ✓ VERIFIED (partial) | React Aria Virtualizer + fuse.js; group-by toolbar present but renders flat |
| `crates/tome-desktop/ui/src/views/HealthView.tsx` | Doctor findings + PreviewPopover fixes | ✓ VERIFIED | Grouped AUTO-FIXABLE/NEEDS ATTENTION; all-clear state; inline failure disclosure |
| `crates/tome-desktop/ui/src/components/MarkdownBody.tsx` | SC#4 allow-list + scheme guard | ✓ VERIFIED | 12-element ALLOWED const; https:// gate in onClick |
| `crates/tome-desktop/src/watcher.rs` | notify 8.2 file watcher + 4 typed events | ✓ VERIFIED | ManifestChanged/LockfileChanged/LibraryChanged/MachinePrefsChanged; 200ms debounce |
| `crates/tome-desktop/src/menu.rs` | Native macOS menu bar + MenuAction event | ✓ VERIFIED | 6 submenus, all Predefined items for Edit, MenuAction typed event |
| `crates/tome-desktop/ui/src/bindings.ts` | Regenerated — all new types exposed | ✓ VERIFIED | gen-bindings idempotent (exit 0 on git diff --exit-code) |
| `crates/tome/src/actions.rs` | Cross-surface action helpers | ✓ VERIFIED | resolve_source_path + set_skill_disabled; 7 unit tests |
| `crates/tome/src/doctor.rs` | FindingId enum + repair_one + DoctorView | ✓ VERIFIED | FindingId enum with 4 variants; repair_one API; collect_doctor_view |
| `crates/tome-desktop/tests/watcher_smoke.rs` | Integration tests for watcher events | ✓ VERIFIED | 2 tests pass (own-process + external write fire within 2000ms) |
| `crates/tome-desktop/tests/a11y/axe.spec.ts` | axe-core CI gate (4 tests) | ✓ VERIFIED | 4 tests with documented color-contrast deferral; wired in ci.yml |
| `crates/tome-desktop/tests/perf/synthetic_skills.rs` | 2000-skill fixture generator | ✓ VERIFIED | Cargo integration test; self-gates on PERF_FIXTURE_OUT env var |
| `crates/tome-desktop/tests/perf/60fps-search.spec.ts` | Playwright FPS bench | ✓ VERIFIED | p95 < 18ms assertion; fps-sampler.js injected via addInitScript |
| `.github/workflows/perf.yml` | macOS-only perf CI workflow | ✓ VERIFIED | macos-latest runner; narrow path triggers; independent of ci.yml |
| `.github/workflows/ci.yml` | a11y gate added to CI | ✓ VERIFIED | Job `a11y` on macos-latest; npx playwright install + npm run test:a11y |
| Shell components: Window/Titlebar/Sidebar/ContentPane | 3-column NavigationSplitView | ✓ VERIFIED | All 4 exist in shell/; tauri.conf.json has Overlay titlebar + windowEffects sidebar |
| Atom components: KeyValueRow/Badge/StatusDot/Pill/DirectoryTable | UI atoms per UI-SPEC | ✓ VERIFIED | All exist with .module.css siblings; substantive implementations |
| Hook: useStatus/useSkills/useSkillDetail/useDoctorReport | Data hooks per Pattern 2 | ✓ VERIFIED | All present; proper event subscriptions via useTauriEvent |
| tokens.css | Design tokens per UI-SPEC §Color/§Typography | ✓ VERIFIED | Full token system with light/dark + prefers-reduced-transparency |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| StatusView.tsx | commands.getStatus | useStatus hook + Result-narrowing | ✓ WIRED | useStatus.ts fetches and narrows Result union |
| SkillsView.tsx | commands.listSkills | useSkills hook + Virtualizer | ✓ WIRED | useSkills.ts fetches; Virtualizer renders filtered list |
| SkillsView (detail) | commands.getSkillDetail | useSkillDetail hook | ✓ WIRED | useSkillDetail.ts fetches on name change |
| SkillsView (actions) | commands.setSkillDisabled, openSourceFolder, copyPath | useSkillActions hook | ✓ WIRED | useSkillActions.ts dispatches 3 actions |
| HealthView.tsx | commands.getDoctorReport | useDoctorReport hook | ✓ WIRED | useDoctorReport.ts fetches + subscribes to events |
| HealthView (fix) | commands.doctorRepairOne | PreviewPopover Apply button | ✓ WIRED | FindingRow → PreviewPopover → commands.doctorRepairOne(id) |
| watcher.rs | React hooks | ManifestChanged/LockfileChanged/LibraryChanged/MachinePrefsChanged events | ✓ WIRED | spawn_watcher in main.rs; hooks subscribe via useTauriEvent |
| menu.rs | React router (App.tsx) | MenuAction event + useMenuActions hook | ✓ WIRED | install_menu in main.rs; useMenuActions in App.tsx |
| status.rs LockfileState | bindings.ts | gen-bindings (specta-gated) | ✓ WIRED | Idempotent — git diff --exit-code exits 0 |
| tome::actions::set_skill_disabled | browse/app.rs apply_toggle | Direct function call (Global scope arm) | ✓ WIRED | TUI refactored to call same helper as GUI |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| StatusView.tsx | status: StatusReport | commands.getStatus() → tome::status::gather() | Yes — reads manifest, lockfile, machine.toml | ✓ FLOWING |
| SkillsView.tsx | skills: DiscoveredSkill[] | commands.listSkills() → tome::list::collect() | Yes — walkdir scan of library | ✓ FLOWING |
| DetailHeader.tsx | detail: SkillDetail | commands.getSkillDetail() → tome::skill::collect_detail() | Yes — reads manifest + machine.toml + SKILL.md body | ✓ FLOWING |
| HealthView.tsx | report: DoctorView | commands.getDoctorReport() → tome::doctor::collect_doctor_view() | Yes — runs diagnose() against real filesystem | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| cargo build workspace | `cargo build --workspace` | Finished dev profile in 13.78s | ✓ PASS |
| cargo clippy -D warnings | `cargo clippy --workspace --all-targets -- -D warnings` | Clean (no output) | ✓ PASS |
| cargo test workspace | `cargo test --workspace` | 909 lib + 13 cli_status + 9 cli_status_details + 10 cli_doctor + 2 watcher_smoke + 3 cli_sync + others — all pass | ✓ PASS |
| cargo fmt check | `cargo fmt --all -- --check` | FAILS on doctor.rs (4 spots), skill.rs (3 spots), commands.rs (1 spot) | ✗ FAIL |
| tsc --noEmit | `cd crates/tome-desktop/ui && npx tsc --noEmit` | Exit 0 | ✓ PASS |
| npm test (Vitest) | `cd crates/tome-desktop/ui && npm test` | 5 passed in 715ms | ✓ PASS |
| gen-bindings idempotency | `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts` | Exit 0 — bindings fresh | ✓ PASS |

### Probe Execution

Step 7c: SKIPPED — no probe-*.sh scripts exist for this phase. Verification covered by the behavioral spot-checks and cargo test above.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| VIEW-01 | 26-01 | Status dashboard with all StatusReport fields | ✓ SATISFIED | StatusView.tsx + extended StatusReport + bindings regen |
| VIEW-02 | 26-02 | Virtualised skill list, fuzzy search, sort, group-by | ✗ PARTIAL | Virtualizer + fuzzy search VERIFIED; group-by renders flat; recent sort falls back to name |
| VIEW-03 | 26-03 | Skill detail pane + 3 actions | ✓ SATISFIED | DetailHeader + 4 Tauri commands; actions module shared with TUI |
| VIEW-04 | 26-04 | Markdown preview, SC#4 subset | ✓ SATISFIED | MarkdownBody + 12-element allow-list; REQUIREMENTS.md updated |
| VIEW-05 | 26-05 | Doctor health pane + one-click fixes | ✓ SATISFIED | HealthView + FindingId + repair_one + DoctorView |
| VIEW-06 | 26-06 | File watcher auto-refresh | ✓ SATISFIED | watcher.rs + 4 events + per-hook subscriptions + integration tests |
| NF-01 | 26-02 + 26-08 | 2000 skills at 60fps | ? UNCERTAIN | React Aria Virtualizer wired + bench harness deployed; local runs show p95 18.1–18.4ms (boundary); CI macos-latest is source of truth |
| NF-02 | 26-07 | Keyboard-navigable + VoiceOver labels | ? UNCERTAIN | axe-core passes (non-color rules); keyboard audit applied (⌘C guard, ⌘⇧O, ⌘D removed); VoiceOver checklist not walked on live app |
| NF-03 | 26-07 | Native macOS menu bar | ✓ SATISFIED | menu.rs with 6 submenus; MenuAction event; install_menu in main.rs |
| NF-05 | 26-06 | Shared tome.lock + manifest; watcher on external change | ✓ SATISFIED | watcher_smoke tests prove external writes fire events; no GUI-private state |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/tome/src/doctor.rs` | 1483, 1526, 1566, 3687 | cargo fmt brace-placement drift in FindingId struct literals | ✗ BLOCKER | CI fmt-check gate fails; blocks PR merge |
| `crates/tome/src/skill.rs` | 180, 285, 430 | cargo fmt single-line match-arm formatting drift | ✗ BLOCKER | CI fmt-check gate fails; blocks PR merge |
| `crates/tome-desktop/src/commands.rs` | 10 | cargo fmt import sort order drift | ✗ BLOCKER | CI fmt-check gate fails; blocks PR merge |
| `crates/tome-desktop/ui/src/views/SkillsView.tsx` | 115 | `void group;` with TODO comment — group-by no-op | ⚠ WARNING | VIEW-02 partially unimplemented |

### Human Verification Required

### 1. Visual Shell Layout

**Test:** Launch `cargo tauri dev` and inspect the main window.
**Expected:** 3-column NavigationSplitView with translucent vibrancy sidebar (Status/Skills/Health nav items), 44px unified titlebar showing "tome — Status", traffic-light controls overlaid on content, Status view landing page with KeyValueRows and DirectoryTable.
**Why human:** Visual layout, vibrancy material, and native macOS chrome require a running Tauri app.

### 2. View Switching

**Test:** Use ⌘1/⌘2/⌘3 and the native View menu to switch between Status, Skills, and Health.
**Expected:** Each transition shows the correct view; native menu items fire MenuAction events; SearchField focused on ⌘F.
**Why human:** Requires live Tauri app and native macOS menu interaction.

### 3. File Watcher Live Refresh

**Test:** Run `tome sync` from the CLI terminal while the GUI is open.
**Expected:** StatusView LAST SYNC updates within 200ms; Updated pill flashes ~2s; Skills list reflects any changes. No stale state persists.
**Why human:** Real file watcher + CLI + live app round-trip.

### 4. VoiceOver Checklist (26-A11Y-AUDIT.md §2.2–§2.7)

**Test:** Enable VoiceOver (⌘F5), launch cargo tauri dev, walk all 30+ checklist items in 26-A11Y-AUDIT.md §2.
**Expected:** Every aria-label template reads correctly; PreviewPopover focus-traps; "Selected skill was removed." announces on external deletion; sidebar Health badge count reads in VoiceOver label.
**Why human:** VoiceOver verbal output quality requires macOS Accessibility + live app; axe-core covers DOM structure but not audio announcements.

### 5. Reduced Motion / Transparency / Dark Mode

**Test:** Toggle System Settings → Accessibility → Reduce motion ON/OFF; Reduce transparency ON/OFF; Appearance Dark/Light.
**Expected:** Pill snap-visible-then-hide (reduced-motion); solid sidebar fallback (reduced-transparency); full dark token recolour across all surfaces without restart (dark mode).
**Why human:** CSS media queries require live system settings changes.

### 6. Disable Mutation End-to-End

**Test:** Select a skill, click Disable on this machine in the DetailHeader, observe badge change.
**Expected:** machine.toml atomic write → MachinePrefsChanged event fires → DetailHeader refetches within ~200ms → "disabled" badge appears; aria-live region announces state.
**Why human:** Real file mutation + watcher round-trip requires live app.

### 7. Doctor Fix Action End-to-End

**Test:** With a real library that has a finding (e.g. stale manifest entry), click Fix → confirm in PreviewPopover → Apply.
**Expected:** Finding disappears from HealthView; sidebar badge decrements; LibraryChanged or ManifestChanged event triggers refetch. Failed fix shows inline [Code] error.
**Why human:** Real doctor findings require live filesystem; repair handlers mutate disk.

### 8. Keyboard Shortcut Scoping (Pitfall 9)

**Test:** With SearchField focused, press ⌘C. Then with a skill selected (no text input focused), press ⌘C. Also test ⌘⇧O.
**Expected:** ⌘C in SearchField copies text (Predefined Edit wins); ⌘C on list/detail copies source path. ⌘⇧O opens Finder to skill source dir.
**Why human:** isTextInputFocused() guard behavior requires live interaction to confirm correct scoping.

---

### Gaps Summary

**Three gaps block goal achievement:**

**Gap 1 (BLOCKER): cargo fmt failure on phase-modified files.** `doctor.rs`, `skill.rs`, and `commands.rs` were all modified by phase-26 commits but have formatting drift that fails `cargo fmt --all -- --check`. The CI workflow includes this check and would fail on PR merge. The deferred-items.md incorrectly attributed this drift as pre-existing — git log shows all three files were touched by phase commits (17e022d, fcf3bba, 8c021da respectively). Fix is trivial: `cargo fmt --all` and commit.

**Gap 2 (WARNING): VIEW-02 group-by renders flat.** The Group PopupMenu shows None/Source/Role options but all three modes produce a flat list with no section headers. The 26-02 SUMMARY calls this an "open follow-up" pending the 26-08 perf bench, but it is not formally deferred to a later phase in ROADMAP.md or deferred-items.md. REQUIREMENTS.md VIEW-02 requires "group-by (none / source / role)" without qualification.

**Gap 3 (WARNING): VIEW-02 recent sort falls back to alphabetical.** "Recent" sort silently falls back to name sort because DiscoveredSkill has no synced_at field. The 26-02 SUMMARY documents this as an open follow-up (manifest-sourced timestamp needed), but it is not formally deferred to a later phase.

Gaps 2 and 3 are sub-features of VIEW-02 that the executor deliberately deferred with documented rationale, but without a formal ROADMAP entry. They may warrant treatment as follow-up issues on the Phase 27 backlog rather than blocking the alpha cut — the planner should decide.

---

_Verified: 2026-05-29T12:00:00Z_
_Verifier: Claude (gsd-verifier)_

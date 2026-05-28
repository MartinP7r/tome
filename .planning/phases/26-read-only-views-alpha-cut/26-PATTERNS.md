# Phase 26: Read-only views — alpha cut — Pattern Map

**Mapped:** 2026-05-29
**Files analyzed:** 27 (12 Rust, 15 React/TS) across 8 draft plans (26-01..26-08)
**Analogs found:** 15 / 27 — Rust side fully analogous; React/TS side is greenfield except for App.tsx scaffold

> **Bucket split (per phase guidance):**
> 1. **Rust** — every new Rust file (`actions.rs`, `watcher.rs`, `menu.rs`, commands, doctor extension, status extension) has a strong in-repo analog. Reuse derives, error chaining, `anyhow`+`.context()`, atomic temp+rename, presenter/gather split, `From<anyhow::Error> for TomeError` boundary.
> 2. **React/TS** — only `App.tsx` (Phase 25 single-view spike) and `bindings.ts` exist today. There is **no in-repo analog** for views/, components/, hooks/, or styles/. The planner pattern-matches against (a) the App.tsx Result-narrowing shape and (b) the official React Aria / TanStack Query / react-markdown reference patterns embedded in RESEARCH §"Code Examples".
>
> **Common-source compression:** every Rust file shares the same imports/error/anyhow/atomic-write conventions. Those appear once in §Shared Patterns rather than repeating per-file.

---

## File Classification

### Rust (CLI core + Tauri backend)

| New/Modified File | Plan | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|------|-----------|----------------|---------------|
| `crates/tome/src/status.rs` (MODIFY — add `lockfile: LockfileState` + `machine_prefs_summary: MachinePrefsSummary` fields, retain `gather`/`show` split) | 26-01 | model + presenter | request-response | itself (existing `StatusReport` + `CountOrError` already specta-gated) | **exact** |
| `crates/tome/src/actions.rs` (NEW — pure helpers shared by TUI + GUI) | 26-03 | service module | transform | `crates/tome/src/list.rs` (CORE-01 collect-shape) + `crates/tome/src/machine.rs::save` (atomic write) | **exact** |
| `crates/tome/src/doctor.rs` (MODIFY — add `FindingId` enum, `repair_one(finding_id)`, refactor `dispatch_repairs` arms into per-item helpers) | 26-05 | model + service | transform | itself (`DiagnosticIssue`, `RepairKind::ALL`, `dispatch_repairs`, `repair_kind_action_label`) | **exact** |
| `crates/tome-desktop/src/commands.rs` (MODIFY — add `list_skills`, `get_skill_detail`, `get_doctor_report`, `doctor_repair_one`, `set_skill_disabled`, `open_source_folder`, `copy_path`) | 26-01/03/05/06 | controller (Tauri command) | request-response | itself — `get_status` is the template (lines 33-41) | **exact** |
| `crates/tome-desktop/src/lib.rs` (MODIFY — extend `collect_commands!` + `collect_events!` lists; keep single `make_builder()`) | 26-01/03/05/06 | config / registry | — | itself (`make_builder` at line 26-38) | **exact** |
| `crates/tome-desktop/src/watcher.rs` (NEW — `notify` + `notify-debouncer-full` thread, 4 typed events) | 26-06 | service (event-driven) | event-driven (pub-sub) | `crates/tome-desktop/src/sink.rs` (typed `tauri-specta::Event` shape) | role-match (no in-repo `notify` precedent) |
| `crates/tome-desktop/src/menu.rs` (NEW — `MenuBuilder` + `MenuAction` event) | 26-07 | controller / event | event-driven | `crates/tome-desktop/src/sink.rs` (typed event shape) | role-match (no in-repo menu code) |
| `crates/tome-desktop/src/main.rs` (MODIFY — call `build_app_menu`, `install_menu_event_handler`, `watcher::spawn_watcher` in `setup`) | 26-06/07 | config / entry point | — | itself (Phase 25 `setup` closure at line 23-28) | **exact** |
| `crates/tome-desktop/capabilities/main.json` (NEW or MODIFY — allow-list opener/clipboard/fs perms) | 26-03/06 | config | — | none in-repo (Phase 25 scaffold may have a minimal one) | greenfield (small JSON; reference Tauri 2 plugin docs) |
| `crates/tome-desktop/tauri.conf.json` (MODIFY — add unified titlebar + vibrancy sidebar + `prefers-reduced-transparency` fallback) | 26-02 | config | — | itself (Phase 25 baseline) | partial (small additive edit) |
| `crates/tome-desktop/tests/perf/synthetic_skills.rs` (NEW — generate 2000 fake skills in TempDir) | 26-08 | test fixture | file-I/O (generator) | `crates/tome/tests/cli.rs` (assert_cmd + TempDir convention) | role-match (CLI tests don't generate at this scale, but the temp-dir pattern is the same) |
| `crates/tome/Cargo.toml` / `crates/tome-desktop/Cargo.toml` (MODIFY — add `notify`, `notify-debouncer-full`, `tauri-plugin-opener`, `tauri-plugin-clipboard-manager` deps) | 26-01/03/06 | config | — | itself (Phase 25 `Cargo.toml` shape with `=2.0.0-rc.25` pin on specta) | **exact** |

### React/TypeScript (frontend)

| New/Modified File | Plan | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|------|-----------|----------------|---------------|
| `crates/tome-desktop/ui/src/App.tsx` (REWRITE — replace single-scroll dashboard with 3-col shell + view router) | 26-02 | shell entry | request-response | itself (Phase 25 — Result-narrowing pattern at lines 24-30 to preserve) | partial (signature/imports kept; body rewritten) |
| `crates/tome-desktop/ui/src/bindings.ts` (REGENERATED — committed) | every plan | generated boundary | — | itself | **exact** (CI freshness gate from Phase 25) |
| `crates/tome-desktop/ui/src/tokens.css` (NEW — design-token CSS custom properties) | 26-02 | config / styles | — | itself (Phase 25 `styles.css`) | role-match (extends existing global stylesheet) |
| `crates/tome-desktop/ui/src/shell/{Window,Titlebar,Sidebar,ContentPane}.tsx` (NEW) | 26-02 | component (shell) | — | **NONE in repo** | greenfield (reference React Aria + UI-SPEC §Shell) |
| `crates/tome-desktop/ui/src/views/StatusView.tsx` (NEW) | 26-01 | view component | request-response | `App.tsx` (the existing single-view rendering, treat as template) | role-match (App.tsx IS the existing analog for a status-rendering view) |
| `crates/tome-desktop/ui/src/views/SkillsView.tsx` (NEW) | 26-02/03/04 | view component | request-response + filtering | **NONE in repo** | greenfield (RESEARCH §Code Examples "Virtualised skill list") |
| `crates/tome-desktop/ui/src/views/HealthView.tsx` (NEW) | 26-05 | view component | request-response | **NONE in repo** | greenfield (RESEARCH §"Doctor fix popover") |
| `crates/tome-desktop/ui/src/components/{Badge,Button,Pill,StatusDot,SeverityIcon,SearchField,PopupMenu,KeyValueRow,DirectoryTable,SkillListRow,DetailHeader,MarkdownBody,SectionHeader,FindingRow,PreviewPopover}.tsx` (NEW — one per UI-SPEC component) | 26-02 (shell atoms), 26-01 (KVR, DirectoryTable), 26-02 (SkillListRow, DetailHeader), 26-04 (MarkdownBody), 26-05 (FindingRow, PreviewPopover) | component (atom/molecule) | display-only | **NONE in repo** | greenfield (React Aria components + UI-SPEC contracts) |
| `crates/tome-desktop/ui/src/hooks/{useStatus,useSkills,useSkillDetail,useDoctorReport,useFuzzySearch,useTauriEvent,useMenuActions}.ts` (NEW) | 26-01/02/03/05/06/07 | hook | request-response + event-driven | **NONE in repo** | greenfield (RESEARCH §"React side — fetching + watcher-driven refresh") |
| `crates/tome-desktop/ui/src/lib/{relativeTime,ariaLabels}.ts` (NEW) | 26-01/02 | utility | transform | **NONE in repo** | greenfield (tiny pure helpers) |
| `crates/tome-desktop/ui/package.json` (MODIFY — add `react-aria-components`, `react-markdown`, `remark-gfm`, `@tauri-apps/plugin-opener`, `@tauri-apps/plugin-clipboard-manager`, `fuse.js`, dev deps) | 26-02/03/04 | config | — | itself (Phase 25 baseline) | **exact** |
| `crates/tome-desktop/tests/perf/playwright/*.ts` (NEW — Playwright FPS bench) | 26-08 | test (e2e) | — | **NONE in repo** | greenfield (RESEARCH §"Pattern 6 — Perf bench") |

---

## Pattern Assignments

### Rust patterns (in-repo analogs are strong; copy precisely)

#### `crates/tome/src/status.rs` (MODIFY) — additive specta-gated fields

**Analog:** itself. Already the canonical CORE-01 shape: pure `gather() -> Result<StatusReport>` + thin `show()` presenter. **Do not** touch the existing field set; only add the two new fields the UI-SPEC asks for (lockfile state + machine-prefs summary).

**Existing derive + specta-gate pattern to mirror for new types** (lines 13-21, 49-70, 73-93):
```rust
/// A count that may have failed with an error message.
#[derive(serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct CountOrError { pub count: Option<usize>, #[serde(skip_serializing_if = "Option::is_none")] pub error: Option<String> }
```

**Existing gather/show split (CORE-01 template)** (lines 98-165, 219-227):
```rust
pub fn gather(config: &Config, paths: &TomePaths) -> Result<StatusReport> { /* pure, no I/O beyond reads */ }

pub fn show(config: &Config, paths: &TomePaths, json: bool) -> Result<()> {
    let report = gather(config, paths)?;
    if json { println!("{}", serde_json::to_string_pretty(&report)?); }
    else { render_status(&report); }
    Ok(())
}
```

**Planner deltas (OQ-4 / RESEARCH "Standard Stack — Status dashboard"):**
- Add `pub lockfile: LockfileState` and `pub machine_prefs_summary: MachinePrefsSummary` to `StatusReport`. Both new enums/structs derive `serde::Serialize` + `#[cfg_attr(feature = "bindings", derive(specta::Type))]`.
- `LockfileState::classify(...)` reuses `reconcile.rs::classify_lockfile` shape (`reconcile.rs:304`) — content-hash comparison; map outcome to `InSync` / `OutOfSync { drift_count }` / `Missing`.
- `MachinePrefsSummary` reads from `machine::load(default_machine_path()?)` and exposes `disabled_count`, `disabled_directory_count`.
- Update `cmd_status` text rendering to print the new fields (a trailing `LOCKFILE:` / `MACHINE:` line) — JSON output ships the new fields automatically.

---

#### `crates/tome/src/actions.rs` (NEW) — shared TUI+GUI handlers

**Analog:** `crates/tome/src/list.rs` (the CORE-01 collect template) for the file shape; `crates/tome/src/machine.rs::save` for the atomic-write site.

**Collect-shape template** (`list.rs:1-47`) — module-doc + structured-return + pure `collect()`:
```rust
//! `tome list` domain computation — discover every skill and return it as a
//! structured [`ListReport`].
//!
//! This is the CORE-01 / D-GUI-08 extraction for the `list` command...

pub struct ListReport {
    pub skills: Vec<DiscoveredSkill>,
    pub warnings: Vec<String>,
}

pub fn collect(config: &Config) -> Result<ListReport> {
    let mut warnings = Vec::new();
    let mut skills = discover::discover_all(config, &BTreeMap::new(), &mut warnings)?;
    skills.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
    Ok(ListReport { skills, warnings })
}
```

**Machine.toml mutation analog** (`machine.rs:262-280`) — atomic temp+rename, parent-dir create, best-effort cleanup:
```rust
pub fn save(prefs: &MachinePrefs, path: &Path) -> Result<()> {
    let content = toml::to_string_pretty(prefs).context("failed to serialize machine prefs")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &content)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);  // best-effort cleanup
        return Err(e).with_context(|| format!("failed to rename to {}", path.display()));
    }
    Ok(())
}
```

**Existing toggle methods to call from `actions::set_skill_disabled`** (`machine.rs:204-210`):
```rust
pub(crate) fn toggle_global_disabled(&mut self, skill: SkillName, disable: bool) -> bool {
    if disable { self.disabled.insert(skill) }
    else { self.disabled.remove(skill.as_str()) }
}
```

**Planner targets:**
```rust
// crates/tome/src/actions.rs (NEW)
//! Cross-surface skill actions (TUI + GUI).
//! Pure-Rust helpers shared between browse::app and tome-desktop::commands.

pub fn resolve_source_path(skill_name: &SkillName, config: &Config, paths: &TomePaths) -> Result<PathBuf> { /* read manifest entry, return source_path */ }

pub fn set_skill_disabled(skill_name: &SkillName, disabled: bool, machine_path: &Path) -> Result<()> {
    let mut prefs = machine::load(machine_path)?;
    prefs.toggle_global_disabled(skill_name.clone(), disabled);
    machine::save(&prefs, machine_path)
}
```

Browse TUI keeps clipboard/opener glue (browse-only); GUI uses Tauri plugins. See RESEARCH §"Pattern 3 — Action handler refactor" for the rationale: only path-computation + machine.toml mutation is shared.

---

#### `crates/tome/src/doctor.rs` (MODIFY) — add `FindingId` + `repair_one`

**Analog:** itself. The existing `RepairKind::ALL` + exhaustive-match dispatcher (`doctor.rs:868-942`) IS the template; per-item repair refactors the loop body into separate helpers.

**Existing `RepairKind` enum + POLISH-04 exhaustiveness guard** (`doctor.rs:135-193`) — keep this convention for any new `FindingId` enum variants:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum RepairKind {
    RemoveStaleManifestEntry,
    RemoveBrokenLibrarySymlink,
    RemoveStaleTargetSymlink,
    ConsolidateTargetRealDirToSymlink,
}

impl RepairKind {
    pub const ALL: [Self; 4] = [ /* mirror order above */ ];
}

#[allow(dead_code)]
const fn _repair_kind_exhaustiveness_sentinel(k: RepairKind) {
    match k {
        RepairKind::RemoveStaleManifestEntry => {}
        RepairKind::RemoveBrokenLibrarySymlink => {}
        RepairKind::RemoveStaleTargetSymlink => {}
        RepairKind::ConsolidateTargetRealDirToSymlink => {}
    }
}
const _: () = { assert!(RepairKind::ALL.len() == 4); };
```

**Existing batch dispatcher** (`doctor.rs:868-942`) — refactor each arm into a per-item helper that takes a `&DiagnosticIssue` (or its content-aware `FindingId`):
```rust
fn dispatch_repairs(report: &DoctorReport, config: &Config, paths: &TomePaths) -> Result<()> {
    // ... iterates report.all_issues(), matches issue.repair_kind, batches per kind
    for issue in report.all_issues() {
        match issue.repair_kind {
            Some(RepairKind::RemoveStaleManifestEntry) | Some(RepairKind::RemoveBrokenLibrarySymlink) => {
                if !ran_library_repair { repair_library(paths)?; ran_library_repair = true; }
            }
            // ...
        }
    }
}
```

**Existing per-kind action labels** (`doctor.rs:677-688`) — reuse verbatim for `PreviewPopover` body text (D-09):
```rust
fn repair_kind_action_label(k: RepairKind) -> &'static str {
    match k {
        RepairKind::RemoveStaleManifestEntry => "will remove entry from manifest file (and broken symlink, if any)",
        RepairKind::RemoveBrokenLibrarySymlink => "will delete broken symlink",
        RepairKind::RemoveStaleTargetSymlink => "will delete stale symlink from distribution dir",
        RepairKind::ConsolidateTargetRealDirToSymlink => "will delete the real directory and replace it with a symlink into the library",
    }
}
```
**Action:** promote `repair_kind_action_label` from `fn` to `pub fn` so the Tauri command can include it in `Finding { dry_run_description }`.

**Planner deltas (OQ-2 — content-aware `FindingId`):**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FindingId {
    LibraryStaleManifest { skill: SkillName },
    LibraryBrokenSymlink { path: PathBuf },
    TargetStaleSymlink { directory: DirectoryName, path: PathBuf },
    TargetRealDirToSymlink { directory: DirectoryName, path: PathBuf },
}

impl DiagnosticIssue { pub fn id(&self) -> FindingId { /* derive from message + category */ } }

pub fn repair_one(finding_id: &FindingId, config: &Config, paths: &TomePaths) -> Result<()> {
    let report = check(config, paths)?;
    let issue = report.all_issues().find(|i| i.id() == *finding_id)
        .ok_or_else(|| anyhow!("finding {:?} no longer present", finding_id))?;
    let Some(kind) = issue.repair_kind else { bail!("finding is not auto-fixable"); };
    match kind {
        RepairKind::RemoveStaleManifestEntry | RepairKind::RemoveBrokenLibrarySymlink => repair_library_one(paths, issue)?,
        RepairKind::RemoveStaleTargetSymlink => repair_target_one(config, paths, issue)?,
        RepairKind::ConsolidateTargetRealDirToSymlink => consolidate_one(config, paths, issue)?,
    }
    Ok(())
}
```
Add the same `_finding_id_exhaustiveness_sentinel` + `const _` length assert as `RepairKind` (POLISH-04 convention).

---

#### `crates/tome-desktop/src/commands.rs` (MODIFY) — new Tauri commands

**Analog:** itself. Phase 25's `get_status` (lines 33-41) is the template every new command copies verbatim — same load_context + `.map_err(TomeError::from)` boundary.

**Existing template** (`commands.rs:33-41`):
```rust
#[tauri::command]
#[specta::specta]
pub fn get_status(_app: tauri::AppHandle) -> Result<tome::status::StatusReport, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::status::gather(&config, &paths).map_err(TomeError::from)
}
```

**`load_context()` to reuse for every new command** (`commands.rs:20-26`):
```rust
fn load_context() -> anyhow::Result<(Config, TomePaths)> {
    let config_path = tome::config::default_config_path()?;
    let config = Config::load_or_default(Some(&config_path))?;
    let tome_home = tome::config::default_tome_home()?;
    let paths = TomePaths::new(tome_home, config.library_dir().to_path_buf())?;
    Ok((config, paths))
}
```

**Planner targets (one wrapper per new command, all identically shaped):**
```rust
#[tauri::command] #[specta::specta]
pub fn list_skills(_app: tauri::AppHandle) -> Result<tome::list::ListReport, TomeError> {
    let (config, _paths) = load_context().map_err(TomeError::from)?;
    tome::list::collect(&config).map_err(TomeError::from)
}

#[tauri::command] #[specta::specta]
pub fn get_skill_detail(_app: tauri::AppHandle, name: SkillName) -> Result<SkillDetail, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    skill::collect_detail(&name, &config, &paths).map_err(TomeError::from)
}

#[tauri::command] #[specta::specta]
pub fn get_doctor_report(_app: tauri::AppHandle) -> Result<tome::doctor::DoctorReport, TomeError> { /* same shape */ }

#[tauri::command] #[specta::specta]
pub fn doctor_repair_one(_app: tauri::AppHandle, finding_id: FindingId) -> Result<(), TomeError> { /* calls tome::doctor::repair_one */ }

#[tauri::command] #[specta::specta]
pub fn set_skill_disabled(_app: tauri::AppHandle, name: SkillName, disabled: bool) -> Result<(), TomeError> {
    let machine_path = tome::machine::default_machine_path().map_err(/* ... */)?;
    tome::actions::set_skill_disabled(&name, disabled, &machine_path).map_err(TomeError::from)
}

#[tauri::command] #[specta::specta]
pub fn open_source_folder(app: tauri::AppHandle, name: SkillName) -> Result<(), TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    let src = tome::actions::resolve_source_path(&name, &config, &paths).map_err(TomeError::from)?;
    // Tauri-side: use tauri-plugin-opener's reveal_item_in_dir
    tauri_plugin_opener::OpenerExt::opener(&app)
        .reveal_item_in_dir(&src)
        .map_err(|e| TomeError::from(anyhow::anyhow!(e)))
}

#[tauri::command] #[specta::specta]
pub fn copy_path(_app: tauri::AppHandle, name: SkillName) -> Result<String, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    let src = tome::actions::resolve_source_path(&name, &config, &paths).map_err(TomeError::from)?;
    Ok(src.display().to_string())  // React-side calls clipboard plugin to write
}
```

**Register in `make_builder()`** (`lib.rs:26-38`):
```rust
pub fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::get_status,
            commands::list_skills,        // NEW
            commands::get_skill_detail,   // NEW
            commands::get_doctor_report,  // NEW
            commands::doctor_repair_one,  // NEW
            commands::set_skill_disabled, // NEW
            commands::open_source_folder, // NEW
            commands::copy_path,          // NEW
        ])
        .events(collect_events![
            sink::SyncProgress,
            watcher::ManifestChanged,     // NEW
            watcher::LockfileChanged,     // NEW
            watcher::LibraryChanged,      // NEW
            watcher::MachinePrefsChanged, // NEW
            menu::MenuAction,             // NEW
        ])
        .dangerously_cast_bigints_to_number()
}
```
**Every command/event add triggers `cargo run -p tome-desktop --bin gen-bindings` + `git add bindings.ts`** (Phase 25 freshness gate).

---

#### `crates/tome-desktop/src/watcher.rs` (NEW) — typed file-watcher events

**Analog:** `crates/tome-desktop/src/sink.rs` — same typed-event shape that crosses the boundary (`tauri-specta::Event` derive on a unit struct).

**Sink.rs event-declaration pattern to copy** (`sink.rs:18-26`):
```rust
#[derive(Debug, Clone, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct SyncProgress {
    pub stage: SyncStage,
    pub current: u32,
    pub total: u32,
}
```

**Planner targets — four unit-struct events + the spawn function:**
```rust
// crates/tome-desktop/src/watcher.rs (NEW)
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent};
use tauri_specta::Event;

#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)] pub struct ManifestChanged;
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)] pub struct LockfileChanged;
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)] pub struct LibraryChanged;
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)] pub struct MachinePrefsChanged;

pub fn spawn_watcher(app: tauri::AppHandle, paths: tome::TomePaths) -> anyhow::Result<()> {
    // Full implementation: RESEARCH §"Pattern 5 — File watcher" lines 618-687.
    // Key: watch PARENT dirs (Pitfall 5 — notify::watch fails on non-existent path);
    // debounce 200ms (Pitfall 1 — atomic-rename read race);
    // classify each event's paths and emit the right typed event.
}
```

**Error/`anyhow` style:** identical to every other module — `anyhow::Result<()>`, `.context(...)` on every failable I/O.

---

#### `crates/tome-desktop/src/menu.rs` (NEW) — native macOS menu bar

**Analog:** `crates/tome-desktop/src/sink.rs` for the typed-event shape; no in-repo menu code precedent.

**Planner targets:** see RESEARCH §"Pattern 7 — Native menu bar" (lines 746-822) for the full skeleton. The pattern is `SubmenuBuilder` × 5 (App / File / Edit / View / Library / Help) wired into a `MenuBuilder`, with `on_menu_event` emitting a typed `MenuAction` enum:

```rust
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
#[serde(tag = "kind")]
pub enum MenuAction { FocusSearch, JumpStatus, JumpSkills, JumpHealth, Reload }
```

Use the same exhaustiveness-sentinel pattern as `RepairKind::ALL` if `MenuAction` grows past 5 variants (POLISH-04 convention).

---

#### `crates/tome-desktop/src/main.rs` (MODIFY) — wire watcher + menu in setup

**Analog:** itself (Phase 25 `setup` closure at `main.rs:23-28`).

**Existing setup closure to extend** (`main.rs:21-30`):
```rust
tauri::Builder::default()
    .invoke_handler(builder.invoke_handler())
    .setup(move |app| {
        builder.mount_events(app);
        Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tome-desktop");
```

**Planner target — extend setup:**
```rust
.setup(move |app| {
    builder.mount_events(app);
    let handle = app.handle().clone();
    // Watcher (RESEARCH §Pattern 5)
    let (_, paths) = tome_desktop::commands::load_context()?;
    tome_desktop::watcher::spawn_watcher(handle.clone(), paths)?;
    // Native menu (RESEARCH §Pattern 7)
    let menu = tome_desktop::menu::build_app_menu(&handle)?;
    handle.set_menu(menu)?;
    tome_desktop::menu::install_menu_event_handler(&handle);
    Ok(())
})
```
**`load_context()` becomes `pub` instead of `fn`** so `main.rs` can call it.

---

### React/TypeScript patterns (greenfield — App.tsx is the only in-repo seed)

#### `crates/tome-desktop/ui/src/App.tsx` (REWRITE) — 3-col shell + view router

**Analog:** itself (Phase 25). The **Result-narrowing pattern** is the load-bearing piece that survives; the body of cards-and-tables is replaced by the router.

**Pattern to preserve verbatim** (`App.tsx:24-30`) — the typed discriminated-union narrowing:
```tsx
commands.getStatus().then((res) => {
  if (res.status === "ok") setStatus(res.data);
  else setErr(res.error);
});
```

**TomeError rendering pattern to lift into a shared `<ErrorBanner>` component** (`App.tsx:32-48`):
```tsx
if (err) {
  return (
    <div className="error-banner">
      <strong>[{err.code}]</strong> {err.message}
      {err.context.length > 0 && (
        <ul>{err.context.map((c, i) => <li key={i}>{c}</li>)}</ul>
      )}
    </div>
  );
}
```
This becomes the `--danger`-bordered banner described in UI-SPEC §"Error state copy" (verbatim shape, just restyled).

**Planner deltas:**
- New `App.tsx` body = `<Window><Titlebar/><Sidebar/><ContentPane>{view}</ContentPane></Window>`, with `view` switched by a tiny router (single `useState<'status' | 'skills' | 'health'>('status')` per CONTEXT D-02 "lands on Status").
- Move data fetching out of `App.tsx` into the per-view `useStatus`/`useSkills`/`useDoctorReport` hooks.

---

#### `crates/tome-desktop/ui/src/views/StatusView.tsx` (NEW)

**Closest in-repo analog:** `App.tsx` (Phase 25) — its render path for `status.directories`, `status.unowned`, `status.library_dir`, `status.last_sync` is reusable; just rehome each line into a `<KeyValueRow>` or `<DirectoryTable>` per UI-SPEC §"Per-view Design — Status".

**Pattern to copy from RESEARCH §Code Examples** (lines 1098-1128):
```tsx
export function StatusView() {
  const { status, err, updatedAt } = useStatus();
  if (err) return <ErrorBanner err={err} />;
  if (!status) return <ContentPane title="Status"><LoadingSkeleton /></ContentPane>;
  return (
    <ContentPane title="Status">
      <KeyValueRow label="TOME HOME"  value={...} mono />
      <KeyValueRow label="LIBRARY"    value={status.library_dir} mono
                   trailing={<span>{formatCount(status.library_count)} skills</span>} />
      <KeyValueRow label="LAST SYNC"  value={formatLastSync(status.last_sync)}
                   trailing={updatedAt && Date.now() - updatedAt < 2000 ? <Pill variant="updated">Updated</Pill> : null} />
      <KeyValueRow label="LOCKFILE"   value={formatLockfile(status.lockfile)}
                   trailing={<StatusDot ok={status.lockfile?.kind === 'InSync'} />} />
      <KeyValueRow label="MACHINE"    value={`${status.machine_prefs_summary?.disabled_count ?? 0} skills disabled`} />
      <DirectoryTable directories={status.directories} />
    </ContentPane>
  );
}
```

---

#### `crates/tome-desktop/ui/src/views/SkillsView.tsx` (NEW)

**No in-repo analog.** Reference RESEARCH §Code Examples (lines 1130-1167) — React Aria native `<Virtualizer>` + `<ListBox>` + `SearchField` + fuse.js fuzzy hook + selection state + `<SkillDetail>` right column. **OQ-1 recommendation: start with React Aria native Virtualizer**; bench in plan 26-08; fall back to TanStack Virtual only if NF-01 fails.

---

#### `crates/tome-desktop/ui/src/views/HealthView.tsx` (NEW) + `PreviewPopover.tsx`

**No in-repo analog.** Reference RESEARCH §Code Examples (lines 1172-1201) for the `DialogTrigger` + `Popover` + `Dialog` shape — React Aria gives free focus trap, Escape-to-dismiss, `aria-modal`.

```tsx
<DialogTrigger>
  <Button className={styles.fixSmall}>Fix</Button>
  <Popover>
    <Dialog aria-labelledby="preview-heading">
      {({ close }) => (<>
        <Heading id="preview-heading" slot="title">PREVIEW</Heading>
        <p>{finding.dry_run_description}</p>
        <p className={styles.helper}>This change is reversible by running tome sync.</p>
        <div className={styles.actions}>
          <Button onPress={close}>Cancel</Button>
          <Button className={styles.primary} onPress={async () => { close(); try { await onApply(); } catch (e) { onError(e as TomeError); } }}>Apply</Button>
        </div>
      </>)}
    </Dialog>
  </Popover>
</DialogTrigger>
```

---

#### `crates/tome-desktop/ui/src/components/MarkdownBody.tsx` (NEW)

**No in-repo analog.** Reference RESEARCH §Code Examples (lines 1205-1233) — `react-markdown` + `remark-gfm` + `allowedElements` (whitelist `['h1','h2','h3','p','strong','em','code','ul','ol','li','a','pre']` per UI-SPEC §`MarkdownBody`).

**Critical Pitfall 3:** strip rejected elements via `allowedElements`, not by omitting CSS — `react-markdown` skips disallowed elements entirely (drops their content).
**Critical Pitfall 7:** `react-markdown@10.1.0` peerDeps say React `>=18` — smoke-test on React 19 in plan 26-04.

---

#### `crates/tome-desktop/ui/src/hooks/useStatus.ts` (NEW) and siblings

**No in-repo analog.** Reference RESEARCH §"Pattern 2 — React side — fetching + watcher-driven refresh" (lines 499-538) for the exact shape — fetch, subscribe to watcher events, refetch on event, track `updatedAt` for the transient "Updated" pill.

**Critical:** every hook subscribes only to the events it depends on (anti-pattern: subscribe-everything-everywhere → unnecessary refetches). `useStatus` subscribes to all four; `useDoctorReport` subscribes to library+manifest only; `useSkillDetail` subscribes to manifest+machine-prefs only.

---

#### `crates/tome-desktop/ui/src/tokens.css` (NEW) + `*.module.css` files

**No in-repo analog.** UI-SPEC §Color and §Spacing provide the canonical tokens. CSS Modules per D-15: every component gets `<Name>.module.css` co-located with `<Name>.tsx`. `prefers-color-scheme: dark` drives the dark-token override (no JS theme state).

---

## Shared Patterns

### S-1: `anyhow::Result` + `.context()` (every Rust module)
**Source:** every `crates/tome/src/*.rs` module (e.g. `status.rs:3`, `machine.rs:12`, `doctor.rs:4`)
**Apply to:** every new Rust file (`actions.rs`, `watcher.rs`, `menu.rs`, new doctor helpers).
**Rule:** import `anyhow::{Context, Result}`; every fallible call gets `.context("operation description")` or `.with_context(|| format!("...{}...", arg))`; never lose the chain. The `TomeError` boundary flattens this chain to `Vec<String>` for the GUI.

### S-2: `TomeError` boundary at every Tauri command edge
**Source:** `crates/tome-desktop/src/error.rs:114-140` (the `From<anyhow::Error>` impl) + `commands.rs:33-41` (every call site uses `.map_err(TomeError::from)`)
**Apply to:** EVERY new Tauri command in `commands.rs`. Domain code stays `anyhow::Result`; classification happens exactly at the boundary; never the other direction.

**The boundary impl to leave untouched but reference** (`error.rs:114-140`):
```rust
impl From<anyhow::Error> for TomeError {
    fn from(err: anyhow::Error) -> Self {
        let code = err.chain().find_map(|cause| {
            cause.downcast_ref::<tome::DomainTagged>().map(|t| ErrorCode::from(&t.kind))
                .or_else(|| cause.downcast_ref::<tome::DomainErrorKind>().map(ErrorCode::from))
        }).unwrap_or(ErrorCode::Internal);
        TomeError {
            code,
            message: err.to_string(),
            context: err.chain().map(|c| c.to_string()).collect(),
        }
    }
}
```

**The `ErrorCode` enum is additive (D-15).** New variants are non-breaking for the GUI default arm; still update `ErrorCode::ALL` and the `_error_code_exhaustiveness_sentinel` (`error.rs:48-81`) when adding.

### S-3: Specta gate on every cross-boundary type
**Source:** `crates/tome/src/status.rs:15-21,49-70,73-93` (every struct double-derives `serde::Serialize` + `#[cfg_attr(feature="bindings", derive(specta::Type))]`)
**Apply to:** new `StatusReport` fields (`LockfileState`, `MachinePrefsSummary`), new `SkillDetail` struct, new `FindingId` enum, every event payload (`ManifestChanged` etc.), every command return type.
**Rule (verified Pitfall 6):** ANY add/change to a cross-boundary type → run `cargo run -p tome-desktop --bin gen-bindings` → `git add crates/tome-desktop/ui/src/bindings.ts` → commit in the same PR. CI's `git diff --exit-code` fails otherwise.

### S-4: Atomic temp+rename for all file writes
**Source:** `crates/tome/src/machine.rs:262-280` (cleanest in-repo example); same convention in `manifest.rs::save` (`manifest.rs:391`), `lockfile.rs` save.
**Apply to:** `actions::set_skill_disabled` (calls `machine::save`), and any future Rust-side writes. Single-user concurrency-safe by construction; NF-05 holds.

### S-5: POLISH-04 exhaustiveness guards on any new closed enum
**Source:** `doctor.rs:135-193` (`RepairKind::ALL` + `_repair_kind_exhaustiveness_sentinel` + `const _: () = { assert!(... .len() == N) };`), `error.rs:48-81` (same shape on `ErrorCode`)
**Apply to:** new `FindingId` enum variants, new `MenuAction` variants, `LockfileState` variants. Adding a variant without updating `ALL` is a `cargo check` failure.

### S-6: Tauri-specta `Event` derive shape (every event)
**Source:** `crates/tome-desktop/src/sink.rs:18-26`
**Apply to:** every new watcher/menu event (`ManifestChanged`, `LockfileChanged`, `LibraryChanged`, `MachinePrefsChanged`, `MenuAction`).
```rust
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
pub struct EventName { /* payload fields */ }
```
Unit structs (`pub struct ManifestChanged;`) are fine when the event has no payload — only the name carries info.

### S-7: Single `make_builder()` registry (no parallel command/event lists)
**Source:** `crates/tome-desktop/src/lib.rs:26-38`
**Apply to:** every new command and event — declare ONCE in `make_builder()`'s `collect_commands![]` / `collect_events![]`. `main.rs` mounts it; `gen-bindings` exports from it. Two registries = drift.

### S-8: React Result-narrowing pattern (every new hook)
**Source:** `crates/tome-desktop/ui/src/App.tsx:24-30`
**Apply to:** every new `useX()` hook calling a Tauri command.
```tsx
const res = await commands.someCommand();
if (res.status === 'ok') setData(res.data);
else setErr(res.error);
```
No try/catch on the command itself — `tauri-specta` returns a discriminated `{ status: 'ok' | 'error', data | error }` union; narrow it.

### S-9: TempDir test isolation
**Source:** `crates/tome/src/machine.rs:283-330` (every test uses `tempfile::TempDir::new().unwrap()`), `crates/tome/tests/cli.rs` (assert_cmd + TempDir integration tests).
**Apply to:** `crates/tome-desktop/tests/perf/synthetic_skills.rs` (generate 2000 fake skills in a TempDir), any new commands.rs unit tests, any new doctor::repair_one tests.

### S-10: Display-only computation in React (D-GUI-08 enforcement)
**Source:** the existing App.tsx — `count()` (line 11), `roleClass()` (line 16) are the ONLY JS computations; everything else comes from the Rust report.
**Apply to:** every new view. Allowed: relative-time formatting, fuse.js fuzzy ranking (display-only), markdown→HTML (presentation). Not allowed: validation, plan computation, machine.toml logic — those go through commands.

---

## No Analog Found

These files have **no in-repo analog** and the planner should use external references (RESEARCH §"Code Examples", official docs):

| File | Role | Reason | Reference to use |
|------|------|--------|------------------|
| `crates/tome-desktop/src/watcher.rs` | Rust `notify` integration | No `notify` precedent anywhere in repo | RESEARCH §"Pattern 5 — File watcher" (lines 618-701) — full Rust skeleton verified against `docs.rs/notify/8.2.0` |
| `crates/tome-desktop/src/menu.rs` | Tauri native menu | No menu code in repo | RESEARCH §"Pattern 7 — Native menu bar" (lines 746-822) — verified against `v2.tauri.app/learn/window-menu` |
| `crates/tome-desktop/capabilities/main.json` (Phase-26 expansion) | Tauri capability JSON | Phase 25 left a minimal one | RESEARCH §"Pitfall 4 — Tauri opener / clipboard plugin permissions" (lines 901-914) |
| `crates/tome-desktop/ui/src/shell/{Window,Titlebar,Sidebar,ContentPane}.tsx` | React shell components | Only `App.tsx` exists | UI-SPEC §Shell + React Aria docs (no in-repo precedent) |
| `crates/tome-desktop/ui/src/views/{Skills,Health}View.tsx` | React virtualised + dialog views | Phase 25's App.tsx is a single Status-shaped view; no virtualised or dialog precedent | RESEARCH §Code Examples (lines 1130-1201) |
| `crates/tome-desktop/ui/src/components/*.tsx` (atoms + molecules) | React Aria-based atoms/molecules | None in repo | UI-SPEC §"Component Contract" + React Aria primitive docs |
| `crates/tome-desktop/ui/src/hooks/*.ts` | React hooks for command + watcher | None in repo | RESEARCH §"Pattern 2 — React side — fetching + watcher-driven refresh" (lines 499-538) |
| `crates/tome-desktop/ui/src/lib/{relativeTime,ariaLabels}.ts` | Tiny utility helpers | None in repo | Trivial; no external reference needed |
| `crates/tome-desktop/ui/src/tokens.css` + `*.module.css` | CSS Modules + design tokens | Phase 25 has a single global `styles.css`; no CSS-Modules precedent | UI-SPEC §Color, §Typography, §Spacing — verbatim tokens |
| `crates/tome-desktop/tests/perf/playwright/*.ts` | Playwright FPS bench | No Playwright/E2E precedent in repo | RESEARCH §"Pattern 6 — Perf bench" (lines 703-742) |

---

## Plan-by-plan Pattern Routing (quick lookup for the planner)

| Plan | Primary files touched | Lead patterns |
|------|----------------------|---------------|
| **26-01** Status backend + view | `status.rs` (extend), `commands.rs` (`get_status` stays; new types specta-gated), `lib.rs::make_builder` (no new commands), `bindings.ts` (regen), `ui/views/StatusView.tsx`, `ui/components/{KeyValueRow,DirectoryTable,Pill,StatusDot,Badge}.tsx`, `ui/hooks/useStatus.ts` | S-1, S-3, S-7, S-8 |
| **26-02** App shell + Skills list view (no detail/actions yet) | `tauri.conf.json` (titlebar/vibrancy), `commands.rs` (`list_skills`), `lib.rs::make_builder`, `bindings.ts` (regen), `ui/App.tsx` (rewrite), `ui/shell/*.tsx`, `ui/views/SkillsView.tsx`, `ui/components/{SearchField,PopupMenu,SkillListRow}.tsx`, `ui/hooks/{useSkills,useFuzzySearch}.ts`, `ui/tokens.css`, `ui/package.json` (add react-aria-components, fuse.js) | S-1, S-2, S-3, S-7, S-8, S-10 |
| **26-03** Detail pane + 3 actions (incl. the lone mutation D-06) | `actions.rs` (NEW Rust), `browse/app.rs` (refactor to call actions), `commands.rs` (4 new commands), `lib.rs::make_builder`, `bindings.ts` (regen), `capabilities/main.json` (opener+clipboard perms), `ui/views/SkillsView.tsx` (extend), `ui/components/{DetailHeader,Button,Badge}.tsx`, `ui/hooks/useSkillDetail.ts`, `ui/package.json` (add @tauri-apps/plugin-opener, plugin-clipboard-manager) | S-1, S-2, S-3, S-4, S-7, S-8 |
| **26-04** Markdown preview | `commands.rs` (extend `get_skill_detail` to include body), `ui/components/MarkdownBody.tsx`, `ui/package.json` (add react-markdown, remark-gfm) | S-2, S-8 (Pitfall 3, 7) |
| **26-05** Doctor health view + per-item fix | `doctor.rs` (`FindingId`, `repair_one`, refactor `dispatch_repairs`), `commands.rs` (`get_doctor_report`, `doctor_repair_one`), `bindings.ts` (regen), `ui/views/HealthView.tsx`, `ui/components/{SectionHeader,FindingRow,PreviewPopover,SeverityIcon}.tsx`, `ui/hooks/useDoctorReport.ts` | S-1, S-2, S-3, S-5, S-7, S-8 |
| **26-06** File watcher (VIEW-06 silent refresh) | `watcher.rs` (NEW), `lib.rs::make_builder` (new events), `main.rs::setup` (spawn watcher), `bindings.ts` (regen), `capabilities/main.json` (if JS-side OQ-3 picked), `ui/hooks/useTauriEvent.ts` + all view-hooks subscribe | S-1, S-3, S-6, S-7 (Pitfalls 1, 5, 10) |
| **26-07** Native menu + a11y/HIG audit | `menu.rs` (NEW), `lib.rs::make_builder` (`MenuAction`), `main.rs::setup` (install menu), `bindings.ts` (regen), `ui/hooks/useMenuActions.ts`, `ui/lib/ariaLabels.ts`, package.json dev deps (axe-core/playwright) | S-1, S-3, S-5, S-6, S-7 (Pitfall 9) |
| **26-08** Perf bench harness (NF-01) | `crates/tome-desktop/tests/perf/synthetic_skills.rs` (NEW Rust generator), `tests/perf/playwright/*.ts` (NEW), `package.json` (add playwright) | S-9 |

---

## Metadata

**Analog search scope:** `crates/tome/src/` (40 module files), `crates/tome-desktop/src/` (Phase 25 scaffold), `crates/tome-desktop/ui/src/` (App.tsx + bindings.ts + main.tsx + styles.css)
**Files read (full or targeted ranges):** `commands.rs`, `error.rs`, `sink.rs`, `main.rs`, `lib.rs` (tome-desktop), `App.tsx`, `bindings.ts` (head), `status.rs` (1-230), `doctor.rs` (1-400, 670-690, 868-942), `machine.rs` (1-330), `browse/app.rs` (1-350), `list.rs`, `skill.rs` (1-120), `reconcile.rs` (head + classify_lockfile), `25-PATTERNS.md` (full)
**Patterns extracted directly from in-repo code:** S-1..S-10 + 6 file-specific assignments (status, actions, doctor, commands, watcher event-shape, main.rs setup)
**Patterns sourced from RESEARCH §Code Examples (no in-repo analog):** watcher.rs skeleton, menu.rs skeleton, every React view + hook + component, perf-bench harness
**Pattern extraction date:** 2026-05-29

---

## PATTERN MAPPING COMPLETE

**Phase:** 26 - read-only-views-alpha-cut
**Files classified:** 27 (12 Rust, 15 React/TS)
**Analogs found:** 15 / 27 — every Rust file has a strong in-repo analog; greenfield React/TS is documented as "no analog" with concrete external reference

### Coverage
- Files with exact analog: 11 (every modified Rust file + App.tsx + bindings.ts + package.json + Cargo.toml)
- Files with role-match analog: 4 (`actions.rs` ← `list.rs`+`machine.rs`; `watcher.rs`/`menu.rs` ← `sink.rs` event shape; `synthetic_skills.rs` ← `cli.rs` TempDir; `StatusView.tsx` ← App.tsx)
- Files with no analog: 12 (all new React component/hook/view files except StatusView; tokens.css/CSS Modules; Playwright bench)

### Key Patterns Identified
- **Tauri command boundary template** — `get_status` (`commands.rs:33-41`) is copy-paste-shaped: `load_context() + map_err(TomeError::from) + domain_fn()`. Apply verbatim to all 7 new commands.
- **Specta gate** — every cross-boundary type uses `#[derive(serde::Serialize)] #[cfg_attr(feature="bindings", derive(specta::Type))]`. `StatusReport` is the textbook example; mirror for `SkillDetail`, `FindingId`, `LockfileState`, `MachinePrefsSummary`, every event payload.
- **POLISH-04 exhaustiveness** — every closed enum has `ALL` + sentinel + const-length assert (`RepairKind`, `ErrorCode`, `DiagnosticIssueKind`). Apply to new `FindingId`, `MenuAction`, `LockfileState`.
- **Atomic temp+rename** — `machine::save` is the canonical example; reuse via `actions::set_skill_disabled`.
- **Single `make_builder()` registry** — every new command/event goes into `lib.rs::make_builder` ONCE; `main.rs` + `gen-bindings` share. CI freshness gate (`git diff --exit-code` on `bindings.ts`) keeps it honest.
- **App.tsx Result-narrowing** — `if (res.status === "ok") ... else setErr(res.error)` is the load-bearing JS pattern; every hook copies it.
- **No JS business logic (D-GUI-08)** — every view fetches → renders. Allowed exceptions: relative-time formatting, fuse.js fuzzy ranking (display-only), react-markdown rendering (presentation).
- **Browse TUI ≠ GUI** — share only `tome::actions` (path-computation + machine.toml mutation), NOT clipboard/opener glue (different OS-call shape).

### File Created
`/Users/martin/dev/opensource/tome/.planning/phases/26-read-only-views-alpha-cut/26-PATTERNS.md`

### Ready for Planning
Pattern mapping complete. Planner can now write per-plan `PLAN.md` files (26-01..26-08) referencing the analog excerpts and shared patterns above. Greenfield TSX files have explicit RESEARCH-section pointers in lieu of in-repo analogs.

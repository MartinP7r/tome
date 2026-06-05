# Phase 27: Sync + triage UI — Pattern Map

**Mapped:** 2026-06-05
**Files analyzed:** 23 new / extended (11 React components, 2 React hooks, 1 React view, 1 Rust module new, 6 Rust modules extended, 2 Rust tests new)
**Analogs found:** 22 / 23 (one net-new component has no direct analog — `StageStepper` outer container; falls back to RESEARCH §"Code Examples")

> Pattern extraction is anchored on `27-CONTEXT.md` D-01..D-20, `27-RESEARCH.md` §"Architectural Responsibility Map" / §"Code Examples", and `27-UI-SPEC.md` §Component Contract. Every excerpt below names the analog file path and line range; the planner can copy the shape verbatim and adjust types/copy.

---

## File Classification

### Rust crate `crates/tome` (domain)

| File | New/Modified | Role | Data Flow | Closest Analog | Match Quality |
|------|--------------|------|-----------|----------------|---------------|
| `crates/tome/src/progress.rs` | **modified** (D-08 adds `item: Option<String>` to `SyncStageProgress`) | model (event vocabulary) | event-driven | itself (existing variant) | exact (self-extension) |
| `crates/tome/src/discover.rs` | **modified** (D-16 adds `synced_at: Option<DateTime>` to `DiscoveredSkill`) | model | request-response | `DiscoveredSkill` itself + `SkillProvenance` shape (lines 207-226) | exact (self-extension) |
| `crates/tome/src/machine.rs` | **modified** (new `preview_save` fn returning `MachineTomlPreview { lines: Vec<DiffLine> }`) | service | transform | `machine::save` (lines 262-280) + `update::diff` (`update.rs:37-67`) | role-match |
| `crates/tome/src/library.rs` | unchanged (read-only reference for D-16 semantics confirmation) | — | — | n/a | n/a |
| `crates/tome/src/lib.rs::sync` | unchanged (already takes `&dyn ProgressSink + &CancelToken`) | — | — | n/a | n/a |
| `crates/tome/tests/sync_smoke.rs` | **new** (SYNC-04 integration test) | test | event-driven | `crates/tome-desktop/tests/watcher_smoke.rs` | role-match |

### Rust crate `crates/tome-desktop` (Tauri boundary)

| File | New/Modified | Role | Data Flow | Closest Analog | Match Quality |
|------|--------------|------|-----------|----------------|---------------|
| `crates/tome-desktop/src/sink.rs` | **modified** (mirror `item` field; D-09 fold-in formatting) | boundary translator | event-driven | itself (existing `impl ProgressSink for TauriEventSink`) | exact (self-extension) |
| `crates/tome-desktop/src/commands.rs` | **modified** (new `start_sync` / `cancel_sync` / `get_lockfile_diff` / `preview_machine_toml` / `apply_machine_toml`; possibly `retry_sync_from`) | controller (IPC) | request-response + async + transform | existing `set_skill_disabled` (lines 91-99), `get_doctor_report` (lines 146-151), `doctor_repair_one` (lines 168-176) | exact for sync commands; partial for async `start_sync` (see Pitfall 5 in RESEARCH) |
| `crates/tome-desktop/src/lib.rs::make_builder` | **modified** (register 5–6 new commands + 0–2 new events + extended `SyncProgress`) | config (registry) | n/a | itself (lines 28-64) | exact (registry append) |
| `crates/tome-desktop/src/menu.rs` | **modified** (add `JumpSync` to `MenuAction`; re-anchor `⌘3`→Sync / `⌘4`→Health; enable Library→Sync `⌘R`) | controller (menu) | event-driven | existing `JumpHealth` definition + `⌘3` accelerator (lines 49, 161-168) | exact (additive enum) |
| `crates/tome-desktop/src/commands.rs::SyncOutcome` type (new) | **new** | model (IPC type) | request-response | `error::TomeError` (lines 104-112), `update::UpdateDiff` (`update.rs:26-34`) | role-match |
| `crates/tome-desktop/src/commands.rs::PartialFailure` type (new) | **new** | model (IPC type) | request-response | `error::TomeError`, `doctor::DoctorFinding` | role-match |

### React UI `crates/tome-desktop/ui/src`

| File | New/Modified | Role | Data Flow | Closest Analog | Match Quality |
|------|--------------|------|-----------|----------------|---------------|
| `ui/src/views/SyncView.tsx` | **new** | view (3-state: idle / in-progress / terminal) | event-driven + request-response | `views/HealthView.tsx` (state branching) + `views/SkillsView.tsx` (split layout) | role-match (composition) |
| `ui/src/components/StageStepper.tsx` | **new** | component (composite) | event-driven | no exact analog; closest is `views/HealthView.tsx` SectionHeader+FindingRow grouping (lines 113-141) | partial — see Shared Pattern §"Stepper outer container" |
| `ui/src/components/StageRow.tsx` | **new** | component (variant-driven row) | event-driven | `components/FindingRow.tsx` (variant rendering + inline `[ErrorCode] message` + disclosure) | exact |
| `ui/src/components/TriagePanel.tsx` | **new** | component (sectioned list) | request-response | `views/SkillsView.tsx` (ListBox + Virtualizer) + `views/HealthView.tsx` (SectionHeader nesting) | role-match (replace `<ListBox>` with `<GridList>` per RESEARCH Pitfall 1) |
| `ui/src/components/TriageRow.tsx` | **new** | component (row) | n/a | `components/SkillListRow.tsx` (52px row + selection + secondary line) | exact |
| `ui/src/components/TriageDetail.tsx` | **new** | component (detail pane) | request-response | `components/DetailHeader.tsx` (title + KeyValueRow grid + action triplet) | exact |
| `ui/src/components/MachineTomlDiff.tsx` | **new** (slot content for PreviewPopover) | component (presentational) | request-response | `components/FindingRow.tsx` (typed payload → structured rows) | role-match |
| `ui/src/components/PreviewPopover.tsx` | **modified** (refactor `dryRunDescription: string` → `children: ReactNode` slot per RESEARCH Pitfall 3) | component (popover shell) | request-response | itself (lines 1-99) | exact (self-refactor) |
| `ui/src/components/SectionHeader.tsx` | unchanged (already exists, wired into two new consumers — closes Phase 26 carryover #1) | — | — | itself (lines 1-25) | exact |
| `ui/src/components/Sidebar.tsx` | **modified** (add 4th NavItem "Sync"; spinner+badge variants; `⌘3`→Sync re-anchor) | shell | n/a | itself (lines 17-89) | exact (additive) |
| `ui/src/components/SyncToast.tsx` | **new** ("Sync complete" / "Sync cancelled" transient) | component | event-driven | `components/Pill.tsx` (`role="status" aria-live="polite"` transient surface — lines 1-26) | role-match |
| `ui/src/hooks/useSync.ts` | **new** | hook (data + lifecycle) | event-driven + request-response | `hooks/useStatus.ts` + `hooks/useDoctorReport.ts` (refetch + Tauri-event subscription pattern) | role-match (extends with stage-state accumulator + start/cancel/retry handlers) |
| `ui/src/hooks/useLockfileDiff.ts` | **new** | hook | request-response | `hooks/useSkills.ts` (single-fetch + event-driven refetch) | exact |
| `ui/src/hooks/useMenuActions.ts` | **modified** (add `JumpSync` case + `⌘R`/`⌘.` global handlers) | hook (menu adapter) | event-driven | itself (lines 59-92) | exact (additive switch arm) |
| `ui/src/stores/router.ts` | **modified** (`View` union gains `"sync"`) | store | n/a | itself (lines 15) | exact (literal-union extension) |
| `ui/src/App.tsx` | **modified** (route `"sync"` → `<SyncView />`; title label `"Sync"`) | app shell | n/a | itself (lines 30-72) | exact (additive switch arm) |
| `ui/src/bindings.ts` | **regenerated** (CI freshness gate) | generated | n/a | n/a | n/a |
| `crates/tome-desktop/tests/a11y/axe.spec.ts` | **modified** (add Sync-view axe scan after `⌘3` navigation) | test | n/a | itself (existing pattern) | exact |

---

## Pattern Assignments

### `crates/tome/src/progress.rs` — extension of `SyncStageProgress` (D-08)

**Analog:** itself (existing variant).

**Existing variant** (`progress.rs:122-129`):
```rust
SyncStageProgress {
    /// Which stage is progressing.
    stage: SyncStage,
    /// Units processed so far.
    current: usize,
    /// Total units in this stage (0 if unknown).
    total: usize,
},
```

**Pattern to copy — add `item: Option<String>` field, keep `cfg_attr(feature = "bindings", derive(specta::Type))` derive:** the planner inserts the new field after `total`. Update the `RecordingSink` test (`progress.rs:269-308`) to include `item: None` (or `Some("…")`) in the constructed events.

**Compile-time drift guard pattern** (`progress.rs:83-100` and `error.rs:67-81`): every enum that crosses the IPC boundary follows the `ALL: [T; N]` + `_ensure_*_exhaustive` const-fn + `const _: () = { assert!(T::ALL.len() == N); };` trio. `SyncStage::ALL` is already in place; no extension needed unless a new stage is added (D-07 keeps 6 stages).

---

### `crates/tome/src/discover.rs` — extension of `DiscoveredSkill` (D-16)

**Analog:** existing `DiscoveredSkill` (`discover.rs:205-226`) + `SkillProvenance` (`discover.rs:111-121`) as the pattern for `Option`-shaped serialized fields.

**Existing shape** (`discover.rs:205-226`):
```rust
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct DiscoveredSkill {
    pub name: SkillName,
    pub path: PathBuf,
    pub source_name: DirectoryName,
    pub origin: SkillOrigin,
    #[allow(dead_code)]
    #[serde(skip)]
    #[cfg_attr(feature = "bindings", specta(skip))]
    pub frontmatter: Option<crate::skill::SkillFrontmatter>,
}
```

**Pattern to copy:** add `pub synced_at: Option<String>` (ISO-8601 timestamp, mirroring `SkillEntry::synced_at` in `manifest.rs`). `Option<String>` keeps `bindings.ts` as `string | null`. `discover_all` must be threaded with a way to read the manifest entry's `synced_at` — easiest is to accept the manifest at the discover call site (`lib.rs::sync`) and stamp it after the manifest read.

---

### `crates/tome/src/machine.rs` — `preview_save` helper (SYNC-03)

**Analog:** `machine::save` (`machine.rs:262-280`) for the path + atomic-write contract; `update::diff` (`update.rs:37-67`) for the "diff with structured output" pattern.

**`machine::save` pattern** (atomic temp+rename, lines 262-280) — reused verbatim by `apply_machine_toml`:
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
        let _ = std::fs::remove_file(&tmp_path); // best-effort cleanup
        return Err(e).with_context(|| format!("failed to rename to {}", path.display()));
    }
    Ok(())
}
```

**`update::diff` pattern** (structured-output diff, lines 37-67) — the shape SYNC-03 mirrors:
```rust
pub fn diff(old: &Lockfile, new: &Lockfile) -> UpdateDiff {
    let mut changes = BTreeMap::new();
    for (name, new_entry) in &new.skills {
        match old.skills.get(name) {
            None => { changes.insert(name.clone(), SkillChange::Added(new_entry.clone())); }
            Some(old_entry) if old_entry.content_hash != new_entry.content_hash => {
                changes.insert(name.clone(), SkillChange::Changed { old: old_entry.clone(), new: new_entry.clone() });
            }
            _ => {}
        }
    }
    // …
    UpdateDiff { changes }
}
```

**New `preview_save`** (per RESEARCH §"Code Examples — similar-based machine.toml diff", lines 482-518): use `similar::TextDiff::from_lines(current_text, proposed_text).iter_all_changes()` to build `Vec<DiffLine>`. `MachineTomlPreview` derives `serde::Serialize + specta::Type` behind `feature = "bindings"`.

---

### `crates/tome-desktop/src/sink.rs` — extend `SyncProgress` + D-09 fold-in

**Analog:** itself (`sink.rs:54-93`).

**Existing fold-in pattern** (lines 54-93):
```rust
impl ProgressSink for TauriEventSink {
    fn emit(&self, event: ProgressEvent) {
        let payload = match event {
            ProgressEvent::SyncStageStarted { stage }
            | ProgressEvent::SyncStageFinished { stage } => SyncProgress {
                stage, current: 0, total: 0,
            },
            ProgressEvent::SyncStageProgress { stage, current, total } => SyncProgress {
                stage,
                current: saturate_usize(current),
                total: saturate_usize(total),
            },
            ProgressEvent::GitCloneProgress { received, .. } => SyncProgress {
                stage: SyncStage::Reconcile,
                current: saturate_u64(received),
                total: 0,
            },
            ProgressEvent::BackupSnapshot { .. } => SyncProgress {
                stage: SyncStage::Save,
                current: 0, total: 0,
            },
        };
        let _ = payload.emit(&self.app);
    }
}
```

**Pattern to extend (D-08 + D-09):**
- Add `pub item: Option<String>` to `SyncProgress` (line 18-26).
- Pass through on `SyncStageProgress` arm.
- D-09: GitCloneProgress arm now sets `item: Some(format!("git: {directory} ({})", format_bytes(received)))`. `format_bytes` is sink-private (CONTEXT.md "Claude's Discretion" — `{:.1} MiB / GiB` per the existing saturating cast).
- BackupSnapshot arm: `item: Some(message)`.
- `SyncStageStarted` / `SyncStageFinished`: `item: None`.

---

### `crates/tome-desktop/src/commands.rs` — five new commands (SYNC-01..05)

**Analog (sync read-only commands):** `set_skill_disabled` (`commands.rs:91-99`), `get_doctor_report` (lines 146-151), `doctor_repair_one` (lines 168-176).

**Existing sync command pattern** (lines 91-99):
```rust
#[tauri::command]
#[specta::specta]
pub fn set_skill_disabled(
    _app: tauri::AppHandle,
    name: SkillName,
    disabled: bool,
) -> Result<(), TomeError> {
    let machine_path = tome::default_machine_path().map_err(TomeError::from)?;
    tome::actions::set_skill_disabled(&name, disabled, &machine_path).map_err(TomeError::from)
}
```

**Existing read-only command pattern** (lines 146-151):
```rust
#[tauri::command]
#[specta::specta]
pub fn get_doctor_report(_app: tauri::AppHandle) -> Result<tome::doctor::DoctorView, TomeError> {
    let (config, paths) = load_context().map_err(TomeError::from)?;
    tome::doctor::collect_doctor_view(&config, &paths).map_err(TomeError::from)
}
```

**Pattern to copy:**
- Every command starts with `load_context().map_err(TomeError::from)?` (RESEARCH §"Established Patterns").
- `.map_err(TomeError::from)` at every fallible call into the domain (Phase 25 D-13).
- `app: tauri::AppHandle` is the first parameter (sink commands clone it; read-only commands accept `_app`).

**Analog for `start_sync` (async + spawn_blocking):** No existing analog — see RESEARCH §"Code Examples — spawn_blocking for the sync command (Pitfall 5)" lines 549-577. The planner copies that template verbatim and threads `tauri::State<'_, SyncState>` for cancel-token sharing.

**Analog for `SyncOutcome` type:** `error::TomeError` (`error.rs:104-112`) shows the `serde::Serialize + specta::Type` derive shape for cross-IPC structs:
```rust
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct TomeError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Vec<String>,
}
```

Copy this derive line for `SyncOutcome`, `PartialFailure`, `MachineTomlPreview`, `DiffLine`, `LockfileDiff` projection, `TriageDecision`. Wrap `Result<(), TomeError>` per RESEARCH §Summary §3 recommendation.

---

### `crates/tome-desktop/src/lib.rs::make_builder` — registry append

**Analog:** itself (lines 28-64).

**Existing pattern:**
```rust
pub fn make_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::get_status,
            commands::list_skills,
            commands::get_skill_detail,
            commands::set_skill_disabled,
            commands::open_source_folder,
            commands::copy_path,
            commands::get_doctor_report,
            commands::doctor_repair_one,
        ])
        .events(collect_events![
            sink::SyncProgress,
            watcher::ManifestChanged,
            watcher::LockfileChanged,
            watcher::LibraryChanged,
            watcher::MachinePrefsChanged,
            menu::MenuAction,
        ])
        .dangerously_cast_bigints_to_number()
}
```

**Pattern to copy — additive only:** append `commands::start_sync, commands::cancel_sync, commands::get_lockfile_diff, commands::preview_machine_toml, commands::apply_machine_toml` (and possibly `commands::retry_sync_from`) to `collect_commands![]`. If new events `SyncStarted` / `SyncFinished` are added (D-06 toast routing — researcher flags this as optional), append them to `collect_events![]`. **Then run `cargo run -p tome-desktop --bin gen-bindings` and commit `ui/src/bindings.ts`**; the CI freshness gate (`git diff --exit-code`) enforces it.

---

### `crates/tome-desktop/src/menu.rs` — `MenuAction::JumpSync` + accelerator re-anchor

**Analog:** itself (lines 41-75 enum + ALL + sentinel; lines 154-188 menu builder; lines 229-252 click handler).

**Existing `MenuAction` enum** (lines 41-52):
```rust
#[derive(Clone, Debug, serde::Serialize, specta::Type, tauri_specta::Event)]
#[serde(tag = "kind")]
pub enum MenuAction {
    JumpStatus,
    JumpSkills,
    JumpHealth,
    FocusSearch,
}

impl MenuAction {
    pub const ALL: [&'static str; 4] = ["JumpStatus", "JumpSkills", "JumpHealth", "FocusSearch"];
}
```

**Existing accelerator registration** (lines 161-168):
```rust
.item(
    &MenuItemBuilder::with_id("jump-health", "Health")
        .accelerator("CmdOrCtrl+3")
        .build(app)?,
)
```

**Pattern to copy:**
1. Add `JumpSync` variant + update `ALL` to length 5 + extend `_menu_action_exhaustiveness_sentinel` match.
2. Add a `jump-sync` `MenuItemBuilder` with `CmdOrCtrl+3` accelerator; change the existing `jump-health` accelerator to `CmdOrCtrl+4`.
3. Add a `"jump-sync" => MenuAction::JumpSync` arm to the click handler (lines 233-237).
4. **Enable the Library → Sync item** (currently `.enabled(false)` at lines 196-198): change to `.enabled(true).accelerator("CmdOrCtrl+R")` and add a `"sync" => MenuAction::JumpSync` arm (or a sibling `SyncNow` event — planner picks per UI-SPEC §Open Items).
5. The existing `"reload" => ... .accelerator("CmdOrCtrl+R") .enabled(false)` View-menu item (lines 183-187) is removed (its `⌘R` slot is reclaimed by Library→Sync per D-02 + RESEARCH Pitfall 7).

---

### `ui/src/views/SyncView.tsx` — three-state view (idle / in-progress / terminal)

**Analog:** `views/HealthView.tsx` (`HealthView.tsx:27-144`) for the variant-branching shape; `views/SkillsView.tsx` (`SkillsView.tsx:144-233`) for the split-column layout when in-progress.

**`HealthView` branching pattern** (lines 27-91):
```tsx
export function HealthView() {
  const { report, err, refetch } = useDoctorReport();

  if (err) {
    return (
      <div className="error-banner">
        <strong>[{err.code}]</strong> {err.message}
        {err.context.length > 0 && (
          <ul>{err.context.map((c, i) => (<li key={i}>{c}</li>))}</ul>
        )}
      </div>
    );
  }
  if (!report) return <div>Loading…</div>;
  if (report.findings.length === 0) {
    return (
      <section className={styles.allClear} role="status" aria-label="Library health">
        {/* …idle / all-clear hero… */}
      </section>
    );
  }
  // …findings list…
}
```

**Pattern to copy:**
- Branch on `useSync()` shape: `(err, status: 'idle' | 'in-progress' | 'terminal-success' | 'terminal-failure' | 'terminal-cancelled' | 'terminal-partial')`.
- Idle → hero composition (per UI-SPEC §Idle state).
- In-progress → split layout (stepper + TriagePanel middle column; TriageDetail right column).
- Terminal variants → stepper transformed in-place + summary block + action buttons (per UI-SPEC §Terminal state).

---

### `ui/src/components/StageRow.tsx` — variant-driven row (D-07/D-18/D-20)

**Analog:** `components/FindingRow.tsx` (`FindingRow.tsx:53-106`).

**Existing variant + inline failure pattern** (lines 53-106):
```tsx
export function FindingRow({ finding, onApplyFix }: FindingRowProps) {
  const [localError, setLocalError] = useState<TomeError | null>(null);
  const fixable = finding.repair_kind != null;
  const severity = fixable ? "warning" : "blocked";
  const ariaLabel = `${severityWord} finding: ${finding.title}. ${finding.description}. ${
    fixable ? "Fix available" : "Manual remediation required"
  }.`;

  return (
    <div className={styles.row} role="group" aria-label={ariaLabel}>
      <div className={styles.icon}><SeverityIcon severity={severity} /></div>
      <div className={styles.text}>
        <div className={styles.title}>{finding.title}</div>
        <div className={styles.description}>{finding.description}</div>
        {localError != null && (
          <div className={styles.failed}>
            <span className={styles.errCode}>[{localError.code}]</span>{" "}
            {localError.message}
            {localError.context.length > 0 && (
              <details className={styles.disclosure}>
                <summary>Show context</summary>
                <ul>{localError.context.map((c, i) => (<li key={i}>{c}</li>))}</ul>
              </details>
            )}
          </div>
        )}
      </div>
      <div className={styles.trailing}>
        {/* trailing slot per variant */}
      </div>
    </div>
  );
}
```

**Pattern to copy verbatim:**
- Status-variant switch (`pending` / `active` / `complete` / `failed` / `cancelled`) drives icon + label weight (400 vs 600 per UI-SPEC §Typography) + trailing slot.
- Inline `[ErrorCode] message` + `<details><summary>Show error chain</summary><ul>` for the failed variant (D-18) AND for each `PartialFailure` in the complete-with-issues variant (D-20).
- `aria-label` template lives at the row level (UI-SPEC §VoiceOver labels lines 591-595).

---

### `ui/src/components/TriagePanel.tsx` — sectioned list with bulk actions

**Analog:** `views/SkillsView.tsx` (`SkillsView.tsx:192-222`) for the list rendering; `views/HealthView.tsx` (`HealthView.tsx:112-141`) for SectionHeader-driven grouping.

**SkillsView list pattern** (lines 192-222):
```tsx
<div className={styles.list}>
  <Virtualizer layout={ListLayout} layoutOptions={LAYOUT_OPTIONS}>
    <ListBox
      aria-label="Skills"
      items={sorted}
      selectionMode="single"
      selectedKeys={selected ? new Set([selected]) : new Set()}
      onSelectionChange={(keys) => { /* … */ }}
    >
      {(skill) => (
        <ListBoxItem id={skill.name} textValue={skill.name} aria-label={/* … */}>
          {({ isSelected }) => (
            <SkillContextMenuRow skill={skill} selected={isSelected} onSelect={/* … */} />
          )}
        </ListBoxItem>
      )}
    </ListBox>
  </Virtualizer>
</div>
```

**HealthView SectionHeader-driven grouping** (lines 112-141):
```tsx
return (
  <>
    {autoFixable.length > 0 && (
      <>
        <SectionHeader label="AUTO-FIXABLE" count={autoFixable.length} />
        <div>{autoFixable.map((f) => (<FindingRow key={…} finding={f} onApplyFix={onApplyFix} />))}</div>
      </>
    )}
    {manual.length > 0 && (
      <>
        <SectionHeader label="NEEDS ATTENTION" count={manual.length} />
        <div>{manual.map((f) => (<FindingRow key={…} finding={f} onApplyFix={onApplyFix} />))}</div>
      </>
    )}
  </>
);
```

**Pattern to copy — with the RESEARCH Pitfall 1 substitution:**
- Replace `<ListBox>` + `<ListBoxItem>` with `<GridList>` + `<GridListItem>` (+ `<GridListSection>` for the two nesting levels). The inline `[✓ keep]` chip toggle requires GridList semantics (RESEARCH §Pitfall 1 + §"Code Examples — React `<GridList>` with sections + inline button" lines 522-547).
- Use `<SectionHeader>` verbatim at the outer level (NEW / CHANGED / REMOVED) and inner level (PLUGINS / MY-REPO / UNOWNED). The component already renders `<h2>` — for inner sections, the planner may need to add a `level: 2 | 3` prop OR introduce an `<h3>`-rendering variant. Smallest change: extend `SectionHeaderProps` with `level?: 2 | 3` (default 2) and `<h2>` / `<h3>` switch.
- `[Apply N decisions]` button at the bottom-right of the panel → uses `<Button variant="primary">` from `components/Button.tsx` (lines 38-55) AND anchors the `<PreviewPopover>` (slot content = `<MachineTomlDiff />`).

---

### `ui/src/components/TriageRow.tsx` — middle-column row with inline chip

**Analog:** `components/SkillListRow.tsx` (`SkillListRow.tsx:28-45`).

**Existing 52px row pattern** (lines 28-45):
```tsx
export function SkillListRow({ skill, disabled = false, selected = false }: SkillListRowProps) {
  const managed = skill.origin.kind === "managed";
  const sourceDisplay = skill.source_name;
  const secondary = `${sourceDisplay} · ${managed ? "managed" : "local"}`;
  return (
    <div className={styles.row} data-selected={selected ? "true" : undefined}>
      <div className={styles.text}>
        <span className={styles.primary}>{skill.name}</span>
        <span className={styles.secondary}>{secondary}</span>
      </div>
      {disabled && (
        <span className={styles.trailing}>
          <Badge subtype="disabled">Disabled</Badge>
        </span>
      )}
    </div>
  );
}
```

**Pattern to copy:**
- Same 52px row metric (UI-SPEC §TriageRow declares this match).
- Same primary (`--text-body` 13px / 400 default, 600 when selected) + secondary (`--text-small` 12px / 400, `--label-secondary`) structure.
- Secondary line composes `${source} · ${managed ? 'managed' : 'local'} · synced ${relativeTime}` (`relativeTime` via existing `lib/relativeTime.ts`).
- Trailing slot = inline chip (Button variant — see UI-SPEC copy: `[✓ keep]` / `[⊘ disabled here]` / `[implicit remove]`). REMOVED rows: chip is 50% opacity, non-interactive.
- `data-selected="true"` + parent's `data-selected` CSS still applies (selection lives on the `<GridListItem>` parent per RESEARCH Pitfall 1).

---

### `ui/src/components/TriageDetail.tsx` — right-column detail pane

**Analog:** `components/DetailHeader.tsx` (`DetailHeader.tsx:54-134`).

**Existing detail header pattern** (lines 54-134) — title + KeyValueRow grid + action triplet:
```tsx
return (
  <header className={styles.header} aria-label={`${skillName} details`}>
    {/* Row 1: name + badges */}
    <div className={styles.titleRow}>
      <h2 className={styles.title}>{skillName}</h2>
      <div className={styles.badges}>
        {managed && <Badge subtype="managed">Managed</Badge>}
        {disabled && <Badge subtype="disabled">Disabled</Badge>}
      </div>
    </div>
    {/* Row 2: metadata grid */}
    <dl className={styles.metaGrid}>
      <div className={styles.metaCell}>
        <dt className={styles.metaLabel}>SOURCE</dt>
        <dd className={styles.metaValueMono} title={sourcePath}>{truncatePath(sourcePath)}</dd>
      </div>
      <div className={styles.metaCell}>
        <dt className={styles.metaLabel}>HASH</dt>
        <dd className={styles.metaValueMono} title={hash}>{truncateHash(hash)}</dd>
      </div>
      <div className={styles.metaCell}>
        <dt className={styles.metaLabel}>SYNC</dt>
        <dd className={styles.metaValue}>{formatRelative(lastSync)}</dd>
      </div>
    </dl>
    {/* Row 3: action buttons */}
    <div className={styles.actions}>
      <Button variant="secondary" onPress={onOpenSource} ariaLabel={openSourceLabel(skillName)}>Open source folder</Button>
      {/* … */}
    </div>
  </header>
);
```

**Pattern to copy:**
- Same 3-row composition (title + badges; metadata `<dl>` grid; actions).
- Replace badges with change-kind badge (`Badge--type-git` family — `New` / `Changed` / `Removed`).
- Metadata grid: SOURCE / CONTENT HASH (for Changed: `sha256:abc…  →  sha256:def…` with arrow glyph) / SYNCED. Reuse `truncateHash` (`DetailHeader.tsx:50`) verbatim.
- Replace action triplet with a `<RadioGroup>` (`react-aria-components`) for the canonical picker (D-12). For `Removed` rows: omit the picker and render the verbatim copy `"This skill will be removed from the lockfile. No action required."` (UI-SPEC §Copywriting).
- Empty-selection state: copy `views/SkillsView.tsx:228` (`<p className={styles.placeholder}>Select a skill to view details</p>`) → adapt copy to `"Select a change to view details"`.

---

### `ui/src/components/MachineTomlDiff.tsx` — slot content (D-15)

**Analog:** no exact analog — the closest is `FindingRow.tsx`'s typed-payload rendering of structured rows. The shape is a presentational mapping over `MachineTomlPreview.lines: Vec<DiffLine>`.

**Pattern to compose (per UI-SPEC §MachineTomlDiff lines 360-383):**
- Render a `<table role="table" aria-label="machine.toml diff, ${added} additions, ${removed} removals">`.
- Each `DiffLine` → `<tr role="row">` with three `<td>`s (line number / change glyph / content) per UI-SPEC §Color §`MachineTomlDiff` line-background tokens.
- The table sits inside the existing `<PreviewPopover>` slot (after refactor — see next entry).

---

### `ui/src/components/PreviewPopover.tsx` — slot refactor (RESEARCH Pitfall 3)

**Analog:** itself (`PreviewPopover.tsx:1-99`).

**Current shape** (lines 43-98):
```tsx
export function PreviewPopover({ dryRunDescription, onApply, onError }: PreviewPopoverProps) {
  return (
    <DialogTrigger>
      <AriaButton className={styles.fix} aria-label="Fix">Fix</AriaButton>
      <Popover className={styles.popover}>
        <Dialog className={styles.dialog} aria-labelledby="preview-heading">
          {({ close }) => (
            <>
              <Heading id="preview-heading" slot="title" className={styles.heading}>PREVIEW</Heading>
              <p className={styles.body}>{dryRunDescription}</p>
              <p className={styles.helper}>This change is reversible by running tome sync.</p>
              <div className={styles.actions}>
                <AriaButton className={styles.cancel} onPress={close} aria-label="Cancel">Cancel</AriaButton>
                <AriaButton className={styles.apply} aria-label="Apply" onPress={() => { close(); onApply().catch(/* … */); }}>Apply</AriaButton>
              </div>
            </>
          )}
        </Dialog>
      </Popover>
    </DialogTrigger>
  );
}
```

**Pattern to refactor:**
- Replace `dryRunDescription: string` prop with `children: ReactNode` (or `bodyContent: ReactNode`). The Doctor caller passes `<p>{description}</p>`; the Apply caller passes `<MachineTomlDiff preview={previewResult} />`.
- Add optional `width?: number` prop (default 320; the diff variant overrides to 480 per UI-SPEC §Spacing).
- Add optional `helperText: string` prop (default "This change is reversible by running tome sync."; the diff variant overrides to "Applying writes `~/.config/tome/machine.toml`. The CLI sees this change immediately.").
- The trigger Button is also a slot — currently hardcoded to "Fix"; the Apply caller needs `[Apply N decisions]`. Replace with a `trigger: ReactNode` prop OR accept a `triggerLabel: string` and `triggerAriaLabel: string`.
- Update the existing Doctor caller (`FindingRow.tsx:88-97`) in the same plan so the refactor is atomic.

---

### `ui/src/hooks/useSync.ts` — sync lifecycle hook (NEW)

**Analog:** `hooks/useStatus.ts` (`useStatus.ts:1-72`) + `hooks/useDoctorReport.ts` (`useDoctorReport.ts:1-66`) for the discriminated-union narrow + `useTauriEvent` subscription pattern.

**Existing pattern** (`useStatus.ts:29-71`):
```ts
export function useStatus(): UseStatusResult {
  const [status, setStatus] = useState<StatusReport_Serialize | null>(null);
  const [err, setErr] = useState<TomeError | null>(null);
  const [updatedAt, setUpdatedAt] = useState<number | null>(null);

  const fetchStatus = useCallback(async (fromEvent: boolean) => {
    const res = await commands.getStatus();
    if (res.status === "ok") {
      setStatus(res.data);
      setErr(null);
      if (fromEvent) setUpdatedAt(Date.now());
    } else {
      setErr(res.error);
    }
  }, []);

  const refetch = useCallback(() => fetchStatus(true), [fetchStatus]);

  useEffect(() => { fetchStatus(false); }, [fetchStatus]);

  useTauriEvent(events.manifestChanged, refetch);
  useTauriEvent(events.lockfileChanged, refetch);
  useTauriEvent(events.libraryChanged, refetch);
  useTauriEvent(events.machinePrefsChanged, refetch);

  return { status, err, updatedAt, refetch };
}
```

**Pattern to copy + extend for `useSync`:**
- Same `useState` + `useCallback` + `useEffect` shape.
- `useTauriEvent(events.syncProgress, handleProgress)` — the handler accumulates stage state into a `Map<SyncStage, StageStatus>` (per UI-SPEC §StageStepper props), records wall-clock at `SyncStageStarted` / `SyncStageFinished` for D-10 durations.
- Exposes `start: () => Promise<void>`, `cancel: () => void`, `retryFrom: (stage: SyncStage) => Promise<void>`, `retryFailedItems: () => Promise<void>`, `dismiss: () => void`, plus `stages`, `outcome`, `isRunning`.
- **RESEARCH Pitfall 6 discipline:** while `isRunning === true`, this hook does NOT subscribe to `manifestChanged` / `lockfileChanged` (the watcher fires for own-process Save writes; the running stepper must not bounce). Other hooks (idle-state summary) keep their subscriptions.

---

### `ui/src/hooks/useLockfileDiff.ts` — lockfile diff fetch (NEW)

**Analog:** `hooks/useSkills.ts` (`useSkills.ts:22-50`).

**Existing pattern** (lines 22-50):
```ts
export function useSkills(): UseSkillsResult {
  const [skills, setSkills] = useState<DiscoveredSkill[] | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [err, setErr] = useState<TomeError | null>(null);

  const refetch = useCallback(async () => {
    const res = await commands.listSkills();
    if (res.status === "ok") {
      setSkills(res.data.skills);
      setWarnings(res.data.warnings);
      setErr(null);
    } else {
      setErr(res.error);
    }
  }, []);

  useEffect(() => { refetch(); }, [refetch]);

  useTauriEvent(events.manifestChanged, refetch);
  useTauriEvent(events.libraryChanged, refetch);
  useTauriEvent(events.machinePrefsChanged, refetch);

  return { skills, warnings, err, refetch };
}
```

**Pattern to copy verbatim** — substitute `commands.getLockfileDiff()`, subscribe only to `lockfileChanged` (the diff is lockfile-derived).

---

### `ui/src/components/Sidebar.tsx` — 4th NavItem

**Analog:** itself (`Sidebar.tsx:31-84`).

**Existing pattern** (lines 31-84):
```tsx
export function Sidebar({ selected, onChange, badgeCount = 0 }: SidebarProps) {
  const { status } = useStatus();
  const skillCount = status?.library_count.count ?? null;
  const footerText = skillCount === null ? "tome" : `tome · ${skillCount} skills`;

  return (
    <aside className={styles.sidebar} aria-label="Sections">
      <div className={styles.caption}>LIBRARY</div>
      <ListBox
        className={styles.nav}
        aria-label="Sections"
        selectionMode="single"
        disallowEmptySelection
        selectedKeys={new Set([selected])}
        onSelectionChange={(keys) => { /* … */ }}
      >
        {SECTIONS.map((s) => {
          const showBadge = s.id === "health" && badgeCount > 0;
          const ariaLabel = s.id === "health" && badgeCount > 0
              ? `Health, Health section, ${badgeCount} health issues`
              : `${s.label}, ${s.label} section`;
          return (
            <ListBoxItem key={s.id} id={s.id} textValue={s.label} className={styles.navItem} aria-label={ariaLabel}>
              <span>{s.label}</span>
              {showBadge && <span className={styles.badge} aria-hidden="true">{badgeCount}</span>}
            </ListBoxItem>
          );
        })}
      </ListBox>
      <div className={styles.footer}>{footerText}</div>
    </aside>
  );
}
```

**Pattern to copy + extend:**
- Insert `{ id: "sync", label: "Sync" }` between `skills` and `health` in `SECTIONS` (line 18-22).
- Extend `Section` union: `"status" | "skills" | "sync" | "health"`.
- Extend `SidebarProps`: add `syncPendingCount?: number` and `syncInProgress?: boolean` (or unify into one richer `syncBadge: { kind: 'pending' | 'failures' | 'none'; count: number }` per UI-SPEC §Sidebar lines 192-200).
- Sync row renders the spinner variant when `syncInProgress` (replace icon with an inline `<svg>` spinner — small system spinner per CONTEXT.md "Claude's Discretion").
- Badge logic: two-meaning rendering per UI-SPEC; mutually exclusive (sync clears pre-sync count before failure badge can appear).
- A11y label template per UI-SPEC §VoiceOver labels lines 586-589.

---

### `ui/src/components/SyncToast.tsx` — transient toast (NEW)

**Analog:** `components/Pill.tsx` (`Pill.tsx:1-26`).

**Existing pattern** (lines 15-26):
```tsx
export function Pill({ variant, children }: PillProps) {
  return (
    <span
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className={[styles.pill, styles[variant]].join(" ")}
    >
      {children}
    </span>
  );
}
```

**Pattern to copy — per RESEARCH Pitfall 2 (hand-roll, not UNSTABLE_ToastRegion):**
- Same `role="status" aria-live="polite" aria-atomic="true"` triplet — Pill is precedent.
- Mount/unmount via `useEffect` + `setTimeout(5000)` (per RESEARCH §"Don't Hand-Roll" line 355).
- Positioned via the parent (CSS top-right per UI-SPEC §States §Terminal state; Apple HIG NSAlert transient style).
- Used only for "Sync complete" (D-06 success) and "Sync cancelled" (D-06 cancel). **NOT for failures** (D-18 stepper-persistence applies).

---

### `ui/src/stores/router.ts` — View union extension

**Analog:** itself (line 15).

**Existing:**
```ts
export type View = "status" | "skills" | "health";
```

**Pattern to copy — literal-union extension:**
```ts
export type View = "status" | "skills" | "sync" | "health";
```

The router file comment (`router.ts:11-13`) already anticipates this: *"Adding a new view (e.g. Sync, Config in Phase 27) is a literal-union extension."*

---

### `ui/src/hooks/useMenuActions.ts` — add `JumpSync` + `⌘R` / `⌘.` handlers

**Analog:** itself (lines 59-92).

**Existing switch** (lines 63-81):
```ts
events.menuAction
  .listen((evt) => {
    if (cancelled) return;
    switch (evt.payload.kind) {
      case "JumpStatus": setView("status"); break;
      case "JumpSkills": setView("skills"); break;
      case "JumpHealth": setView("health"); break;
      case "FocusSearch": focusSearchField(); break;
    }
  })
```

**Pattern to copy — additive switch arm + global keyboard handlers:**
- Add `case "JumpSync": setView("sync"); break;`.
- Add a parallel `useEffect` that binds `⌘R` (Run/Cancel/Retry per state) and `⌘.` (Cancel sync when running) at the `window` level. Use the existing `isTextInputFocused()`-style guard from `SkillsView.tsx:47-55` (referenced verbatim there — the planner may want to extract it to `lib/`).
- `⌘⇧A` triage Apply binding is contextual (Sync section only; opens PreviewPopover). Mount it inside `SyncView` rather than the global hook.

---

### `crates/tome/tests/sync_smoke.rs` — integration test (NEW)

**Analog:** `crates/tome-desktop/tests/watcher_smoke.rs` (path verified in `Bash` listing) — the canonical Phase 26 integration test that drives a domain subsystem with a recording sink + tempdir.

**Pattern to copy:**
- Use `tempfile::TempDir` for filesystem isolation.
- Use `RecordingSink` (`progress.rs:208-238`) to capture the emitted event sequence.
- Drive `tome::sync(&config, &paths, opts, &sink, &cancel)` with a pre-flipped `cancel` token to assert SC#4 invariant (library state consistent after cancellation).
- Also assert `RecordingSink` event ordering for D-09 / RESEARCH Pitfall 4 verification: `SyncStageStarted { Reconcile }` precedes the first `GitCloneProgress`.

---

## Shared Patterns

### Authentication / Authorization
**Not applicable** — single-user local app; no auth surface (RESEARCH §"Applicable ASVS Categories" V2/V3/V4 = no).

### Error Handling — IPC boundary classification

**Source:** `crates/tome-desktop/src/error.rs` (`error.rs:114-140`).
**Apply to:** Every new Tauri command in `commands.rs`.

```rust
impl From<anyhow::Error> for TomeError {
    fn from(err: anyhow::Error) -> Self {
        let code = err
            .chain()
            .find_map(|cause| {
                cause.downcast_ref::<tome::DomainTagged>()
                    .map(|t| ErrorCode::from(&t.kind))
                    .or_else(|| cause.downcast_ref::<tome::DomainErrorKind>().map(ErrorCode::from))
            })
            .unwrap_or(ErrorCode::Internal);
        TomeError {
            code,
            message: err.to_string(),
            context: err.chain().map(|c| c.to_string()).collect(),
        }
    }
}
```

**Convention to follow:** every command body ends in `.map_err(TomeError::from)?` or `.map_err(TomeError::from)` (verified in `commands.rs:48`, `:79`, `:97`, `:98`, `:113`, `:114`, `:150`, `:175`). For `start_sync`, also wrap the `JoinError` from `spawn_blocking`:
```rust
.await.map_err(|join_err| TomeError::from(anyhow::anyhow!("sync task panicked: {join_err}")))?
```

### Validation — input

**Source:** `crates/tome/src/discover.rs::SkillName` newtype + serde adapter (`discover.rs:38-42`, `:101-108`).

```rust
pub fn new(name: impl Into<String>) -> Result<Self> {
    let name = name.into();
    crate::validation::validate_identifier(&name, "skill name")?;
    Ok(Self(name))
}

impl<'de> serde::Deserialize<'de> for SkillName {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SkillName::new(s).map_err(serde::de::Error::custom)
    }
}
```

**Apply to:** every cross-IPC type that contains a `SkillName` or `DirectoryName` — including the new `TriageDecision` payload of `apply_machine_toml`. The newtypes already validate; the planner should re-use `SkillName` / `DirectoryName` for triage payloads rather than `String`.

### Front-end Result-narrowing (S-8 / Phase 25)

**Source:** `ui/src/hooks/useStatus.ts:43-51` (and every Phase 26 hook).

```ts
const res = await commands.getStatus();
if (res.status === "ok") {
  setStatus(res.data);
  setErr(null);
} else {
  setErr(res.error);
}
```

**Apply to:** every new `useSync` / `useLockfileDiff` command call. **Never `try/catch`** around the typed discriminated-union result (the comment `// S-8 Result-narrowing — no try/catch around the typed discriminated-union result.` is verbatim at every Phase 26 hook callsite).

### Event subscription discipline

**Source:** `ui/src/hooks/useTauriEvent.ts:35-55` (cleanup-on-unmount + late-listen-race guard) + per-hook subscription discipline at `useSkills.ts:45-47`, `useStatus.ts:65-68`, `useDoctorReport.ts:60-62`.

```ts
export function useTauriEvent<T = unknown>(event: EventListener<T>, handler: () => void): void {
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    event.listen(((_e: Event<T>) => { if (!cancelled) handler(); }) as EventCallback<T>)
      .then((un) => { if (cancelled) un(); else unlisten = un; });
    return () => { cancelled = true; unlisten?.(); };
  }, [event, handler]);
}
```

**Apply to:** `useSync.ts` for the `syncProgress` subscription. **Each hook subscribes only to the events it depends on** (anti-pattern guard documented at `useTauriEvent.ts:7-11`). Per RESEARCH Pitfall 6, `useSync` does NOT subscribe to `manifestChanged` / `lockfileChanged` (the idle-state hooks pick those up after the run resolves).

### "Structure at the edge" (Phase 25 D-17)

**Source:** `crates/tome-desktop/src/sink.rs:54-93` (already applied for SyncProgress).

**Apply to:** every new IPC type in Phase 27. The `SyncStage` discriminant crosses as a TS string-union (never stringified via `format!("{:?}", stage)`); `TriageDecision`, `BulkScope`, `DiffLineKind` all derive `specta::Type` for typed pattern-matching on the React side.

### Preview-then-confirm (NF-04 ergonomic)

**Source:** `ui/src/components/PreviewPopover.tsx` + `ui/src/components/FindingRow.tsx:88-97` (Doctor flow caller).

**Apply to:** SYNC-03 Apply flow. After the slot refactor (above), the caller in `TriagePanel.tsx` mirrors `FindingRow`'s shape — same `onApply` / `onError` contract, different `children` slot content (`<MachineTomlDiff />`).

### Plan/render/execute for mutating actions

**Source:** Existing CLI helpers (`remove.rs`, `reassign.rs`, `relocate.rs`, `eject.rs`) + Phase 26 doctor repair (`commands.rs::doctor_repair_one`).

**Apply to:** SYNC-03 → `preview_machine_toml` (plan/render) + `apply_machine_toml` (execute). The `PreviewPopover` slot is the "render" middle step.

### Cooperative cancellation

**Source:** `crates/tome/src/progress.rs:159-186` (`CancelToken`).

**Apply to:** SYNC-04. The token is created in `start_sync`, cloned into `tauri::State<'_, SyncState>` so `cancel_sync` can flip the bit. RESEARCH §"Code Examples — spawn_blocking for the sync command" lines 549-577 has the template.

### Exhaustiveness drift guards (POLISH-04)

**Source:** `crates/tome/src/progress.rs:83-100` (SyncStage::ALL); `crates/tome-desktop/src/error.rs:67-81` (ErrorCode::ALL); `crates/tome-desktop/src/menu.rs:54-75` (MenuAction::ALL).

**Apply to:** every new IPC enum (`TriageDecision`, `DiffLineKind`, `BulkScope.kind` if encoded as enum, etc.). Pattern:
1. Add an `ALL: [T; N]` associated constant.
2. Add an `_ensure_*_exhaustive(t: T)` const-fn with an exhaustive match.
3. Add a `const _: () = { assert!(T::ALL.len() == N); };` length-pin.
This trio fails compile when a new variant lands without updating the registry. Verified at every Phase 25/26 IPC enum.

---

## Stepper outer container (No analog)

**File:** `ui/src/components/StageStepper.tsx`

**Reason for no analog:** Phase 26 has no vertical-stepper or pipeline-progress affordance. The closest behavioural reference is `views/HealthView.tsx:112-141`'s SectionHeader-driven group rendering, but the visual shape (connector-line between rows; in-place transformation between live progress and terminal state) is net-new.

**Fall-back recipe from RESEARCH §"Code Examples" + UI-SPEC §StageStepper lines 213-238:**
- Outer `<section role="list" aria-label="Sync pipeline progress">` per UI-SPEC.
- Render 6 `<StageRow role="listitem">` from `stages: StageState[]` prop.
- A 2px vertical line in `--separator` between adjacent stage-icon centres (CSS pseudo-element on the container).
- Trailing slot above the stepper renders `[Cancel sync]` (active) or `[Dismiss]` + `[Retry from <stage>]` / `[Retry failed items]` (terminal). Use `<Button variant="secondary">` and `<Button variant="primary">` from `components/Button.tsx`.
- Live-region announcements via `role="status" aria-live="polite"` wrapper — pattern verbatim from `Pill.tsx:18-20`.

The planner builds this from primitives; no copy-paste source exists.

---

## Files with no analog (summary)

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `ui/src/components/StageStepper.tsx` | component (composite) | event-driven | No vertical-stepper precedent in Phase 26; assemble from primitives per UI-SPEC §StageStepper + RESEARCH §"Code Examples" |

Every other Phase 27 net-new file has at least a role-match analog in Phase 26 or in the domain crate.

---

## Metadata

**Analog search scope:**
- `crates/tome/src/{progress,discover,update,machine,lockfile,manifest,library,lib}.rs`
- `crates/tome-desktop/src/{commands,sink,lib,menu,watcher,error}.rs`
- `crates/tome-desktop/ui/src/{App.tsx,stores/router.ts,bindings.ts}`
- `crates/tome-desktop/ui/src/views/*.tsx`
- `crates/tome-desktop/ui/src/hooks/*.ts`
- `crates/tome-desktop/ui/src/components/*.tsx`
- `crates/tome-desktop/ui/src/shell/Sidebar.tsx`
- `crates/tome-desktop/tests/watcher_smoke.rs`

**Files scanned:** ~38 (selective reads on relevant sections; no full re-reads).

**Pattern extraction date:** 2026-06-05.

**Key insight:** Per RESEARCH §"Don't Hand-Roll" line 359, *"The Rust side already owns every primitive Phase 27 needs… The React side already owns every primitive… Phase 27 is at least 70% wiring and 30% net-new components."* This pattern map confirms it: 22 of 23 files map exact-or-role-match to an existing analog, and the shared patterns table compresses the cross-cutting concerns to seven copy-from-existing rules. The planner should be suspicious of any plan that proposes new abstractions beyond the seven shared patterns + the one no-analog stepper outer container.

# Phase 28: Configuration UI - beta cut - Research

**Researched:** 2026-08-13  
**Domain:** Tauri 2 / React configuration workflow over Rust-owned Tome configuration  
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Finish Phase 28 as the desktop beta cut, then return to TUI and CLI work. Phases 29-31 remain deferred rather than starting the remaining desktop milestone immediately.
- **D-02:** When no valid config exists, start in a dedicated setup screen. Main desktop sections are unavailable until setup saves a config or the user explicitly cancels.
- **D-03:** Mirror the CLI's brownfield and legacy choices: use existing, edit, reinitialize with backup, or cancel; legacy cleanup remains explicit.
- **D-04:** Show the resolved Tome data folder and its source. Preserve explicit flag, environment, or XDG choices; only an implicit default is editable. Offer XDG persistence when a custom folder is selected.
- **D-05:** Present auto-discovered directories as a preselected checklist with type, role, and path. Users can deselect entries or add a custom directory inline before review.
- **D-06:** Use a selectable directory list and editable detail pane, with Add as the primary action.
- **D-07:** Support drag-and-drop and accessible Move up/Move down controls. Reordering changes the draft only until explicit confirmation.
- **D-08:** Keep invalid edits visible and correctable, show field-level and affected-directory errors, and block preview/save until Rust validation succeeds.
- **D-09:** Remove means remove from the configuration draft only. Library and distribution cleanup stays in Phase 29. The resulting config change still requires confirmation.
- **D-10:** Confirm directory changes through one full `tome.toml` diff in the existing preview-then-confirm pattern.
- **D-11:** Validate and clone a Git source before adding it to the configuration draft; show progress in the app.
- **D-12:** Scan a cloned repository for skill layouts, prefill the best detected subdirectory with a clear label, and let the user override it before continuing. This folds in pending todo `2026-06-26-tome-add-auto-detect-subdir.md`.
- **D-13:** Default to the repository's default branch. Put mutually exclusive branch, tag, and revision pinning in an Advanced section.
- **D-14:** On clone or layout failure, retain the entered draft, show the Rust error inline with retry, and do not add an unverified source to config.
- **D-15:** Organize preferences as global disabled skills/directories plus a selectable directory list with adjacent per-directory enabled/disabled rules.
- **D-16:** Use a searchable picker of known library skills. Preserve unavailable existing selections and explain unknown entries rather than dropping them.
- **D-17:** A disabled directory retains its per-directory rules, but those controls are inactive until the directory is enabled again.
- **D-18:** Confirm all preference edits in one full `machine.toml` diff with an explicit Apply action.
- **D-19:** Configuration is a persistent fifth sidebar section after Sync and before Health, following Phase 27's sidebar-expansion contract.

### the agent's Discretion
- The configuration editor should feel like the existing desktop list/detail surfaces rather than a separate settings system.
- Automatic Git subdirectory detection is an assistive default, never an unreviewed configuration change.

### Deferred Ideas (OUT OF SCOPE)
- Full operational cleanup caused by removing a directory belongs to Phase 29.
- Phases 29-31 are deferred after the Phase 28 beta cut while work returns to the TUI and CLI.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|---|---|---|
| CFG-01 | First-run wizard, including greenfield/brownfield/legacy cases and discovered/custom directories | Expose the existing wizard state/probe and pure assembly helpers through typed commands; retain the CLI choice semantics. |
| CFG-02 | Add/edit/remove/reorder directories with live validation | Use Rust projection/validation commands over a draft; commit only via `Config::save_checked`. |
| CFG-03 | Add Git repository with URL/name/ref inputs and clone progress | Build a clone-and-inspect domain operation over the existing git safeguards and `ProgressSink`; only merge a verified candidate into draft. |
| CFG-04 | Edit all machine preferences with previewed `machine.toml` diff | Generalize the existing `MachineTomlPreview`/`MachineTomlDiff` flow to a complete typed preference draft. |
| CFG-05 | Prevent invalid config writes | Make Rust the sole TOML producer and save boundary; React holds only serializable draft inputs/results. |
| NF-04 | Destructive operations require explicit confirmation | Use `PreviewPopover` for config and preference Apply; directory removal affects draft only in this phase. |
</phase_requirements>

## Summary

Phase 28 should add one Configuration route composed of three dependent flows: setup gating, a portable `tome.toml` directory draft, and a machine-local preference draft. React owns selection, form focus, and unsaved draft state; Rust owns identifier parsing, role validity, TOML serialization, validation, cloning, layout detection, diff generation, and writes. This is the established desktop boundary, not a new settings subsystem. [VERIFIED: crates/tome-desktop/src/commands.rs:63-78] [VERIFIED: crates/tome/src/config/mod.rs:286-330]

The audit baseline is: CFG-01, CFG-02, CFG-03, and CFG-05 have no shipped desktop implementation; CFG-04 and NF-04 have only the Phase-27 triage-specific `machine.toml` preview/apply flow. The reusable core/desktop pieces are the CLI wizard, `Config::validate`/`Config::save_checked`, `MachinePrefs`, `add.rs`, `PreviewPopover`/`MachineTomlDiff`, the `make_builder()` command registry, and typed sync progress. [VERIFIED: .planning/REQUIREMENTS.md:62-66] [VERIFIED: crates/tome-desktop/src/commands.rs:298-387] [VERIFIED: crates/tome-desktop/src/lib.rs:37-99]

**Primary recommendation:** Deliver typed Rust draft/preview/save operations first, then compose setup, directory editing, Git verification, and preferences in React; do not write or validate TOML in JavaScript.

## Project Constraints (from AGENTS.md)

- Use the substantial-change workflow: GitHub issue → OpenSpec → GSD phase/plans → implementation → archive/close. [VERIFIED: AGENTS.md:61-68]
- Rust edition is 2024; clippy warnings are failures. [VERIFIED: AGENTS.md:70-72] [VERIFIED: AGENTS.md:122-140]
- Preserve the CLI; the desktop crate is cargo-dist excluded until the v1.0 desktop release. [VERIFIED: AGENTS.md:21-23]
- Keep changes surgical, use `anyhow` application errors, atomically write persisted state, and use co-located Rust unit tests plus CLI integration tests. [VERIFIED: AGENTS.md:174-185]
- Use non-interactive flags for shell file operations. [VERIFIED: AGENTS.md:37-59]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| Setup-state detection, path-source policy, legacy/brownfield actions | API / Backend | Frontend Server | Rust already probes config/legacy files and owns mutations; React renders choices. [VERIFIED: crates/tome/src/wizard.rs:915-965] |
| Directory draft validation, TOML preview, and save | API / Backend | Browser / Client | `Config::validate` and `save_checked` are the canonical safety boundary; client displays their result. [VERIFIED: crates/tome/src/config/validate.rs:18-36] [VERIFIED: crates/tome/src/config/mod.rs:286-330] |
| List/detail editing, drag/drop, keyboard movement, confirmation UI | Browser / Client | API / Backend | The browser manages transient selection/draft ordering; Rust evaluates and commits the final draft. [CITED: https://react-spectrum.adobe.com/react-aria/ListBox.html#drag-and-drop] |
| Git clone, ref selection, skill-layout inspection, progress | API / Backend | Browser / Client | Clone is a blocking subprocess with a typed progress sink; UI must not run Git or infer a saved entry. [VERIFIED: crates/tome/src/git.rs:105-189] |
| Machine preference projection, preview, and atomic save | API / Backend | Browser / Client | Machine preferences are a Rust schema with mutual-exclusion validation and atomic save. [VERIFIED: crates/tome/src/machine.rs:96-159] [VERIFIED: crates/tome/src/machine.rs:370-390] |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---|---:|---|---|
| `tauri` | 2.11.5 registry latest; repo allows `2.11` | Typed commands, managed state, events | Already hosts the desktop trust boundary and commands. [VERIFIED: crates/tome-desktop/Cargo.toml:47-53] [VERIFIED: crates.io via `cargo search tauri`] |
| `tauri-specta` / `specta` | 2.0.0-rc.25 | Generated IPC bindings | Existing exact pins and `make_builder()` prevent JS/Rust type drift. [VERIFIED: crates/tome-desktop/Cargo.toml:47-54] [VERIFIED: crates.io via `cargo search tauri-specta`] |
| React / React Aria Components | repo pins React 19.1.0 / RAC 1.18.0 | UI state, accessible collections/forms/drag-drop | This is the locked desktop UI substrate; RAC supports keyboard and screen-reader drag/drop. [VERIFIED: crates/tome-desktop/ui/package.json:18-40] [CITED: https://react-spectrum.adobe.com/react-aria/ListBox.html#drag-and-drop] |
| `similar` | =3.1.1 | Server-side TOML line diff | Already approved and powers the existing machine-pref preview. [VERIFIED: Cargo.toml:40-47] [VERIFIED: crates/tome/src/machine.rs:308-367] |

### Supporting

| Library | Version | Purpose | When to Use |
|---|---:|---|---|
| Existing `TauriEventSink` / `SyncProgress` | in-repo | Clone progress | Reuse for Git verification; do not add a second event channel. [VERIFIED: crates/tome/src/git.rs:110-117] [VERIFIED: crates/tome-desktop/src/commands.rs:389-499] |
| Existing `PreviewPopover` + `MachineTomlDiff` | in-repo | Explicit diff confirmation | Use for both final `tome.toml` and `machine.toml` Apply. The popover is already slot-based. [VERIFIED: crates/tome-desktop/ui/src/components/PreviewPopover.tsx:47-147] [VERIFIED: crates/tome-desktop/ui/src/components/MachineTomlDiff.tsx:70-103] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|---|---|---|
| New drag-and-drop dependency | React Aria `useDragAndDrop` | Do not add a dependency: RAC already supports reorder interaction. Rows with inline move controls must not be `ListBoxItem`s because that pattern forbids interactive descendants; use `GridList` or place controls outside the listbox rows. [CITED: https://react-spectrum.adobe.com/react-aria/ListBox.html#drag-and-drop] |
| JS TOML parser/diff/validator | Rust `Config` + `MachinePrefs` + `similar` | A JS implementation would violate CFG-05 and duplicate schema semantics. [VERIFIED: crates/tome/src/config/mod.rs:257-330] |

**Installation:** No new dependency is recommended. Reuse installed Tauri, React Aria, and `similar`.

## Architecture Patterns

### System Architecture Diagram

```text
Configuration sidebar route
        |
        +--> setup gate ----> Rust setup snapshot ----> greenfield / brownfield / legacy choice
        |                                                  |
        |                                                  +--> validated draft --> preview --> save_checked
        |
        +--> directory list/detail + draft reorder --------> Rust validate_config_draft
        |                                                       |
        |                                                       +--> tome.toml line diff --> PreviewPopover --> save
        |
        +--> Add Git form --> Rust clone + inspect --SyncProgress--> clone UI
        |                         |                                      |
        |                         +--> candidate subdir / error ----------+--> reviewed draft entry
        |
        +--> preference draft --> Rust MachinePrefs validation/diff --> MachineTomlDiff --> Apply
```

### Recommended Project Structure

```text
crates/tome/src/
├── config/                 # typed config, validation, atomic save
├── wizard.rs                # extract reusable non-dialoguer setup domain helpers
├── add.rs / git.rs          # ref parsing, safe clone; add inspection projection
└── machine.rs               # typed prefs, validation, atomic preview/save
crates/tome-desktop/src/
├── commands.rs              # thin typed IPC shells
├── config_types.rs          # specta wire projections / pure projections
└── lib.rs                   # register every command in make_builder
crates/tome-desktop/ui/src/
├── views/ConfigurationView.tsx
├── components/Configuration*.tsx
└── hooks/useConfiguration*.ts
```

### Pattern 1: Draft → Rust validation → preview → confirmed save
**What:** Send a typed draft to Rust for each validation/preview; save only after the user clicks Apply in `PreviewPopover`. Rust returns field/affected-directory errors without destroying the client draft.  
**When to use:** Directory edits, setup completion, and all preference changes.  
**Why:** `save_checked` expands paths, validates, verifies TOML round-trip equality, then atomically writes. [VERIFIED: crates/tome/src/config/mod.rs:286-330]

### Pattern 2: Extract pure wizard domain helpers before UI wiring
**What:** Promote operations currently embedded in dialoguer flow into non-interactive Rust helpers: setup snapshot, known-directory candidates, brownfield decision execution, legacy cleanup plan/execute, and config assembly.  
**When to use:** CFG-01.  
**Why:** The existing wizard already has pure `assemble_config`, machine-state detection, and explicit legacy/brownfield semantics; a Tauri command must not drive dialoguer. [VERIFIED: crates/tome/src/wizard.rs:566-586] [VERIFIED: crates/tome/src/wizard.rs:915-1077] [VERIFIED: crates/tome/src/lib.rs:518-579]

### Pattern 3: Verify Git before draft insertion
**What:** Clone into Tome's existing cache, scan the clone for candidate skill roots, return candidates and a selected recommendation, then add the reviewed selection to the `tome.toml` draft.  
**When to use:** CFG-03 only.  
**Why:** Current `add()` is config-only and explicitly does not sync/clone; its URL/ref parsing should be factored/reused, not invoked as the desktop verification flow. [VERIFIED: crates/tome/src/add.rs:245-254] [VERIFIED: crates/tome/src/add.rs:348-498]

### Pattern 4: Typed edge and registry freshness
**What:** Command returns are `Result<T, TomeError>` at the IPC edge; domain remains `anyhow`. Register every command in `make_builder()`, then regenerate `bindings.ts`.  
**When to use:** Every new Configuration command.  
**Why:** This is the Phase-25 boundary contract and current registry. [VERIFIED: .planning/phases/25-rust-core-extraction-tauri-integration-spike/25-CONTEXT.md:45-52] [VERIFIED: crates/tome-desktop/src/lib.rs:37-99]

### Anti-Patterns to Avoid
- **Direct TOML editing or serialization in React:** violates CFG-05 and loses `save_checked` protection. [VERIFIED: crates/tome/src/config/mod.rs:257-330]
- **Adding a Git directory before clone/layout validation:** violates D-11/D-14; current `add()` writes config without cloning. [VERIFIED: crates/tome/src/add.rs:245-254]
- **Using `ListBoxItem` with embedded Move buttons:** RAC documents this as inaccessible; use `GridList` for interactive rows, while retaining explicit Move up/down buttons. [CITED: https://react-spectrum.adobe.com/react-aria/ListBox.html#drag-and-drop]
- **Applying machine prefs through the triage-only `TriageDecision` model:** it can only add to global disabled skills. Create a configuration-specific complete preference draft/projection. [VERIFIED: crates/tome-desktop/src/commands.rs:298-337]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| Config validation / role matrix / overlap errors | JS validation rules | `Config::validate` | It contains role/type, Git field, and distribution overlap checks. [VERIFIED: crates/tome/src/config/validate.rs:18-180] |
| Safe `tome.toml` write | Browser filesystem write | `Config::save_checked` | Preserves portability, round-trip checking, and atomic rename. [VERIFIED: crates/tome/src/config/mod.rs:257-361] |
| Machine-pref diff and write | Custom diff / non-atomic save | `preview_save` + `machine::save` | Existing line diff and temp+rename are tested and bound to the desktop. [VERIFIED: crates/tome/src/machine.rs:308-390] |
| Git clone command construction | New subprocess wrapper | `git::clone_repo` safeguards | Clears hostile Git env vars, checks cancellation, tags Git errors, and emits progress. [VERIFIED: crates/tome/src/git.rs:1-14] [VERIFIED: crates/tome/src/git.rs:118-189] |
| UI confirmation primitive | Modal/popover variant | `PreviewPopover` | Existing slot API supports a diff body and catches Apply errors. [VERIFIED: crates/tome-desktop/ui/src/components/PreviewPopover.tsx:47-147] |

## Concrete Reuse Map

| File / function | Reuse action | Phase-28 gap to close |
|---|---|---|
| `wizard.rs::detect_machine_state`, `brownfield_decision`, `handle_legacy_cleanup` | Extract command-safe state/action projections | Dialoguer calls and non-public types cannot cross IPC unchanged. [VERIFIED: crates/tome/src/wizard.rs:915-1077] |
| `wizard.rs::find_known_directories_in`, `assemble_config` | Reuse discovery and pure config assembly | Return serializable candidate DTOs; preserve explicit `type`/`role`/`path`. [VERIFIED: crates/tome/src/wizard.rs:863-890] [VERIFIED: crates/tome/src/wizard.rs:566-586] |
| `Config::validate`, `Config::save_checked` | Canonical validation/save | Add a read-only TOML preview counterpart, ideally using the existing `similar` diff approach. [VERIFIED: crates/tome/src/config/validate.rs:18-36] [VERIFIED: crates/tome/src/config/mod.rs:286-330] |
| `DirectoryType::valid_roles` | Return allowed roles from Rust / validate final draft | This is the source for the 3×4 role matrix. [VERIFIED: crates/tome/src/config/types.rs:118-150] [VERIFIED: crates/tome/src/config/validate.rs:560-706] |
| `add.rs::parse_tree_suffix`, GitRef and role rules | Factor into a reusable candidate builder | `AddOptions` and `add_git` are crate-private and save immediately. [VERIFIED: crates/tome/src/add.rs:47-188] [VERIFIED: crates/tome/src/add.rs:348-498] |
| `git.rs::clone_repo`, `repo_cache_dir`, `effective_path` | Clone verified Git candidate and locate the selected subdirectory | Add bounded, deterministic `SKILL.md` root scanning and ambiguity return type. [VERIFIED: crates/tome/src/git.rs:76-89] [VERIFIED: crates/tome/src/git.rs:105-189] [VERIFIED: crates/tome/src/git.rs:253-260] |
| `MachinePrefs::validate`, `preview_save`, `save` | Complete preferences draft/preview/apply | Add public/projected access/mutators sufficient for global and per-directory rules without exposing raw fields. [VERIFIED: crates/tome/src/machine.rs:96-159] [VERIFIED: crates/tome/src/machine.rs:308-390] |
| `PreviewPopover`, `MachineTomlDiff` | Preserve confirmation and diff a11y | Generalize diff type/component naming only if both TOML files can share it; otherwise add a parallel config diff renderer. [VERIFIED: crates/tome-desktop/ui/src/components/PreviewPopover.tsx:47-147] [VERIFIED: crates/tome-desktop/ui/src/components/MachineTomlDiff.tsx:70-103] |
| `commands.rs`, `lib.rs::make_builder`, generated `bindings.ts` | Add command shells/registry/bindings | Re-run the freshness gate for every IPC type/command. [VERIFIED: crates/tome-desktop/src/commands.rs:73-94] [VERIFIED: crates/tome-desktop/src/lib.rs:37-99] |
| `App.tsx`, router, Sidebar, menu | Insert Configuration as fifth: Status, Skills, Sync, Configuration, Health | Update literal unions, navigation order, menu event enum/ALL/sentinel, labels, tests, and accelerator map atomically. [VERIFIED: crates/tome-desktop/ui/src/App.tsx:28-92] [VERIFIED: crates/tome-desktop/ui/src/stores/router.ts:19-45] [VERIFIED: crates/tome-desktop/src/menu.rs:48-93] |

## Common Pitfalls

### Pitfall 1: Treating an absent config as a valid, configured setup
**What goes wrong:** `Config::load_or_default` returns defaults when the file is missing, so a naïve `load_context()` looks valid but must lead to the setup gate.  
**Avoid:** Add a setup snapshot that distinguishes absent, valid, malformed, legacy, and brownfield state; block normal routes until save or explicit cancel. [VERIFIED: crates/tome/src/config/mod.rs:43-64] [VERIFIED: crates/tome/src/wizard.rs:915-965]

### Pitfall 2: Reordering a `BTreeMap` as if it persisted order
**What goes wrong:** `Config.directories` is a `BTreeMap`, so merely reordering the React list cannot survive serialization. [VERIFIED: crates/tome/src/config/types.rs:414-443]  
**Avoid:** Resolve this before implementation: either introduce a deliberate persisted order representation/schema change, or define D-07's reordered draft as display/order-of-review only. The requirement says reordering changes the draft; no existing config ordering field was found in the source read this session. [ASSUMED]

### Pitfall 3: Losing a draft on validation or clone failure
**What goes wrong:** Replacing form state from a failed response discards the user input D-08/D-14 require to keep.  
**Avoid:** Return structured errors from a read-only Rust draft validator; retain client draft until a successful save or user discard. Use `TomeError.code/message/context` for unexpected execution failures. [VERIFIED: .planning/phases/25-rust-core-extraction-tauri-integration-spike/25-CONTEXT.md:45-52]

### Pitfall 4: Inconsistent preview and Apply
**What goes wrong:** A source file can change after preview; Phase 27 accepts re-read-at-apply for the single-user app, but the configuration flow must still calculate both draft projections through the same Rust helper. [VERIFIED: crates/tome-desktop/src/commands.rs:339-386]  
**Avoid:** Make preview and save share one domain `apply_draft` projection; apply revalidates then writes.

### Pitfall 5: Clone safety and event ordering
**What goes wrong:** New clone code can bypass `git.rs` env clearing/cancellation/error tagging, or React can miss/duplicate event subscriptions. [VERIFIED: crates/tome/src/git.rs:1-14] [CITED: https://v2.tauri.app/develop/calling-rust/]  
**Avoid:** Reuse `TauriEventSink`, run blocking clone work through `tauri::async_runtime::spawn_blocking`, and use one shared configuration operation state provider if multiple components render clone progress. [VERIFIED: crates/tome-desktop/src/commands.rs:389-499]

### Pitfall 6: Invalid per-directory preference state
**What goes wrong:** A directory may not have both `enabled` and `disabled`; the disabled-directory UI must retain but deactivate its rules. [VERIFIED: crates/tome/src/machine.rs:52-67] [VERIFIED: crates/tome/src/machine.rs:146-159]  
**Avoid:** Model per-directory rule mode explicitly in the Rust wire projection and validate before preview; do not silently drop unknown/unavailable skill names.

## Code Examples

### Thin command boundary skeleton

```rust
// Source pattern: crates/tome-desktop/src/commands.rs:73-94
#[tauri::command]
#[specta::specta]
pub fn preview_config_draft(
    _app: tauri::AppHandle,
    draft: ConfigDraft,
) -> Result<ConfigTomlPreview, TomeError> {
    config_preview_from_draft(draft).map_err(TomeError::from)
}
```

The command must only bridge typed input to a Rust domain helper and map errors at the edge. The exact `ConfigDraft` and `ConfigTomlPreview` names are [ASSUMED] proposed wire names; define them with `serde` + `specta::Type` and regenerate bindings.

### React Aria reorder shape

```tsx
// Source: https://react-spectrum.adobe.com/react-aria/ListBox.html#drag-and-drop
const { dragAndDropHooks } = useDragAndDrop({
  getItems: (keys, items) => items.map((item) => ({ "text/plain": item.name })),
  onReorder(event) {
    // Update only the React draft; Rust validates/persists after confirmation.
    if (event.target.dropPosition === "before") draft.moveBefore(event.target.key, event.keys);
    else if (event.target.dropPosition === "after") draft.moveAfter(event.target.key, event.keys);
  },
});
```

Use this only in a row shape without nested interactive controls; otherwise use a `GridList` because D-07 also requires Move up/down buttons. [CITED: https://react-spectrum.adobe.com/react-aria/ListBox.html#drag-and-drop]

## Recommended Dependency-Aware Plan Structure

1. **28-01 — Rust configuration domain and setup snapshot (foundation).** Extract non-dialoguer wizard operations; add serializable setup/config draft DTOs; validation + TOML preview/save helper; tests for greenfield, valid/malformed brownfield, legacy, path-source restrictions, role matrix, and no-write-on-invalid. Depends on Phase 25 boundary conventions only.
2. **28-02 — Tauri command boundary and Configuration navigation shell.** Add read/validate/preview/save/setup commands; register in `make_builder()`, regenerate bindings; add fifth router/sidebar/menu section. Add menu enum/ALL/sentinel and accessibility/nav tests atomically. Depends on 28-01.
3. **28-03 — Setup and directory editor UI.** Build setup gate plus selectable list/detail form, draft-only removal and reorder controls, field/affected-directory errors, `tome.toml` preview confirmation. Depends on 28-02. Do not start Git or prefs UI before the reusable draft/save seam exists.
4. **28-04 — Git candidate verification flow.** Factor/add a clone-and-layout-inspection domain operation using `git.rs` plus progress; build Add Git form, Advanced ref selection, candidate review/retry, and test ambiguous/no-skill paths. Depends on 28-01/02 and uses the existing Sync event sink.
5. **28-05 — Complete machine preferences domain and UI.** Add a complete preference projection/mutator/preview/save path (global, directories, per-directory rules, unavailable skill preservation); compose it with `MachineTomlDiff` and `PreviewPopover`. Depends on 28-02; can run in parallel with 28-04 after 28-02.
6. **28-06 — Beta integration, a11y, bindings freshness, and manual UAT.** Exercise first-run, directory lifecycle, Git clone/subdir detection, preference rules, no silent writes, and watcher refresh; run workspace/UI gates. Depends on 28-03/04/05.

## State of the Art

| Old Approach | Current Approach | Impact |
|---|---|---|
| CLI dialoguer-only configuration | Tauri commands with React rendering | Extract domain behavior from prompts rather than replicate it. [VERIFIED: crates/tome/src/wizard.rs:201-492] |
| Doctor/triage-specific confirmation | Slot-based `PreviewPopover` + line diff | Configuration can reuse the same confirmation language and a11y shell. [VERIFIED: crates/tome-desktop/ui/src/components/PreviewPopover.tsx:1-30] |
| String/hand-maintained JS boundary types | generated `bindings.ts` from `make_builder()` | Every new command requires regeneration and freshness verification. [VERIFIED: crates/tome-desktop/src/lib.rs:37-99] |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | Persisting a user-defined order needs a new representation because current directories are a `BTreeMap`. | Common Pitfalls | D-07 may need a locked schema/UX interpretation before execution. |
| A2 | `ConfigDraft` / `ConfigTomlPreview` are suitable new wire-type names. | Code Examples | Naming only; planner may choose repository-consistent alternatives. |
| A3 | A complete machine preference projection needs new public accessors/mutators rather than exposing fields. | Common Pitfalls / Plan structure | Exact API shape requires implementation design. |

## Resolved Open Questions

1. **Directory order persistence — RESOLVED.**
    - Add an explicit `Config` schema field containing an ordered `Vec<DirectoryName>` alongside the lookup-oriented `directories: BTreeMap<DirectoryName, DirectoryConfig>`.
    - Validation requires the sequence to be complete and one-to-one with configured directories: every entry is configured, no name appears twice, and no configured name is omitted.
    - Existing TOML without the field loads with a deterministic migration default: configured names in the BTreeMap's stable key-sorted order. New saves emit the normalized complete ordered sequence. User order is never inferred from BTreeMap iteration.
    - D-07 drag/drop and Move up/down mutate this explicit schema field in the draft; Rust validates and persists it through the canonical config save path.
2. **Git skill-root selection — RESOLVED.**
    - Scan the cloned repository for candidate directories that directly contain `SKILL.md`. The repository root is a candidate when `<clone>/SKILL.md` exists and is represented by normalized relative path `.`.
    - Normalize candidate relative paths, sort first by component depth (shallowest first), then lexicographically by normalized relative path for same-depth ties. The first candidate is the deterministic prefill.
    - Return the complete sorted candidate list and the prefill to the UI. The UI labels the prefill as an assistive recommendation and requires review; an explicit user subdirectory always wins over the recommendation.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---:|---|---|
| Cargo / Rust | Rust domain and desktop checks | ✓ | cargo 1.97.1 | — |
| Node.js / npm | React typecheck/build/test | ✓ | Node v26.7.0 / npm 11.19.0 | — |
| Git CLI | CFG-03 clone verification | ✓ | [ASSUMED: not probed in this session] | Block CFG-03 test execution until available |

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V5 Input Validation | yes | Validating newtypes, Rust `Config::validate`, typed deserialization at IPC boundary. [VERIFIED: crates/tome/src/config/types.rs:14-92] [VERIFIED: crates/tome/src/config/validate.rs:18-36] |
| V4 Access Control | no | Single-user local desktop; no account/authorization surface in scope. [VERIFIED: .planning/REQUIREMENTS.md:130-137] |
| V6 Cryptography | no | No new cryptographic function or secret storage is in Phase 28 scope. [VERIFIED: .planning/REQUIREMENTS.md:58-66] |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| Malformed identifiers or illegal role/ref combinations from IPC | Tampering | Deserialize into Tome validated types; re-run `Config::validate` server-side. [VERIFIED: crates/tome/src/config/types.rs:14-92] [VERIFIED: crates/tome/src/config/validate.rs:39-120] |
| Git environment/repository escape | Tampering | Reuse existing Git environment clearing and ceiling-directory safeguards. [VERIFIED: crates/tome/src/git.rs:1-40] |
| Silent destructive config write | Repudiation | Preview exact TOML diff, explicit Apply, canonical atomic save. [VERIFIED: crates/tome/src/machine.rs:308-390] |
| Over-broad Tauri permissions | Elevation of privilege | Keep filesystem/shell permissions absent; perform server-resolved operations through narrow commands. [VERIFIED: crates/tome-desktop/src/commands.rs:1-11] |

## Sources

### Primary
- [Tauri official docs](https://v2.tauri.app/develop/calling-rust/) — commands, serializable results, async caveats, event listener cleanup. [CITED: https://v2.tauri.app/develop/calling-rust/]
- [React Aria ListBox docs](https://react-spectrum.adobe.com/react-aria/ListBox.html#drag-and-drop) — reorder support and nested-interactive warning. [CITED: https://react-spectrum.adobe.com/react-aria/ListBox.html#drag-and-drop]
- Live Tome sources cited throughout, especially config, wizard, machine, add, git, commands, registry, and UI components.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — existing pinned dependencies and live command/UI architecture were read; official docs confirm Tauri/RAC behavior.
- Architecture: HIGH — constrained by locked phase context and current sources.
- Pitfalls: HIGH except the directory-order persistence decision (ASSUMED pending product/schema choice).

**Research date:** 2026-08-13  
**Valid until:** 2026-09-12

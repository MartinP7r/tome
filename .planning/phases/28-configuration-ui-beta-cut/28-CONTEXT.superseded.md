# Phase 28: Configuration UI - beta cut - Context

**Gathered:** 2026-08-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver the desktop configuration experience for first-run setup, directory editing, Git sources, and all machine preferences. It is the v1.0 beta cut. Existing CLI/core behavior is reused through Tauri commands; the desktop must not duplicate validation or write TOML directly.

</domain>

<decisions>
## Implementation Decisions

### Milestone sequencing
- **D-01:** Finish Phase 28 as the desktop beta cut, then return to TUI and CLI work. Phases 29-31 remain deferred rather than starting the remaining desktop milestone immediately.

### First-run setup
- **D-02:** When no valid config exists, start in a dedicated setup screen. Main desktop sections are unavailable until setup saves a config or the user explicitly cancels.
- **D-03:** Mirror the CLI's brownfield and legacy choices: use existing, edit, reinitialize with backup, or cancel; legacy cleanup remains explicit.
- **D-04:** Show the resolved Tome data folder and its source. Preserve explicit flag, environment, or XDG choices; only an implicit default is editable. Offer XDG persistence when a custom folder is selected.
- **D-05:** Present auto-discovered directories as a preselected checklist with type, role, and path. Users can deselect entries or add a custom directory inline before review.

### Directory editor
- **D-06:** Use a selectable directory list and editable detail pane, with Add as the primary action.
- **D-07:** Support drag-and-drop and accessible Move up/Move down controls. Reordering changes the draft only until explicit confirmation.
- **D-08:** Keep invalid edits visible and correctable, show field-level and affected-directory errors, and block preview/save until Rust validation succeeds.
- **D-09:** Remove means remove from the configuration draft only. Library and distribution cleanup stays in Phase 29. The resulting config change still requires confirmation.
- **D-10:** Confirm directory changes through one full `tome.toml` diff in the existing preview-then-confirm pattern.

### Git sources
- **D-11:** Validate and clone a Git source before adding it to the configuration draft; show progress in the app.
- **D-12:** Scan a cloned repository for skill layouts, prefill the best detected subdirectory with a clear label, and let the user override it before continuing. This folds in pending todo `2026-06-26-tome-add-auto-detect-subdir.md`.
- **D-13:** Default to the repository's default branch. Put mutually exclusive branch, tag, and revision pinning in an Advanced section.
- **D-14:** On clone or layout failure, retain the entered draft, show the Rust error inline with retry, and do not add an unverified source to config.

### Machine preferences and navigation
- **D-15:** Organize preferences as global disabled skills/directories plus a selectable directory list with adjacent per-directory enabled/disabled rules.
- **D-16:** Use a searchable picker of known library skills. Preserve unavailable existing selections and explain unknown entries rather than dropping them.
- **D-17:** A disabled directory retains its per-directory rules, but those controls are inactive until the directory is enabled again.
- **D-18:** Confirm all preference edits in one full `machine.toml` diff with an explicit Apply action.
- **D-19:** Configuration is a persistent fifth sidebar section after Sync and before Health, following Phase 27's sidebar-expansion contract.

### Folded Todos
- **Auto-detect --subdir in `tome add` by scanning for SKILL.md:** incorporate into the Add Git repository flow as reviewed automatic subdirectory detection.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and requirements
- `.planning/ROADMAP.md` §"Phase 28: Configuration UI - beta cut" — restored Phase 28 goal, dependency, requirements, and success criteria.
- `.planning/REQUIREMENTS.md` §"Configuration UI (CFG)" — CFG-01 through CFG-05 and NF-04.
- `.planning/todos/pending/2026-06-26-tome-add-auto-detect-subdir.md` — folded subdirectory-detection requirement.

### Inherited desktop decisions
- `.planning/phases/27-sync-triage-ui/27-CONTEXT.md` — React/Tauri boundary, fifth-sidebar-section precedent, PreviewPopover, watcher, diff, and confirmation patterns.
- `.planning/phases/26-read-only-views-alpha-cut/26-CONTEXT.md` — desktop shell, design tokens, accessibility, and PreviewPopover conventions.
- `.planning/phases/25-rust-core-extraction-tauri-integration-spike/25-CONTEXT.md` — Rust-owned business logic, TomeError boundary, bindings generation, and Tauri command patterns.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/tome/src/wizard.rs` and `crates/tome/src/lib.rs` — CLI greenfield, brownfield, legacy, auto-discovery, and save semantics to expose through the desktop boundary.
- `crates/tome/src/config/mod.rs::Config::validate` and `Config::save_checked` — required validation and write boundary for every `tome.toml` mutation.
- `crates/tome/src/machine.rs` — machine preference schema and atomic writes.
- `crates/tome/src/add.rs` — Git input parsing, ref pin semantics, and local/Git directory configuration behavior.
- `crates/tome-desktop/ui/src/components/PreviewPopover.tsx` and `MachineTomlDiff` — established preview-then-confirm UI.
- `crates/tome-desktop/src/commands.rs` and `src/lib.rs::make_builder` — typed Tauri command registry and bindings-generation pattern.

### Established Patterns
- Rust owns validation, planning, and side effects; React renders typed command results and dispatches commands.
- Every Tauri command maps errors through `TomeError`; command additions require `bindings.ts` regeneration and the freshness gate.
- Mutations use PreviewPopover and an explicit Apply confirmation.

### Integration Points
- Add Configuration to the existing desktop router, Sidebar, native menu, and app shell.
- Add configuration read, validate, preview, save, setup-state, and Git-clone commands at the Tauri boundary.
- Reuse sync progress events for clone progress where practical; do not introduce frontend business logic.

</code_context>

<specifics>
## Specific Ideas

- The configuration editor should feel like the existing desktop list/detail surfaces rather than a separate settings system.
- Automatic Git subdirectory detection is an assistive default, never an unreviewed configuration change.

</specifics>

<deferred>
## Deferred Ideas

- Full operational cleanup caused by removing a directory belongs to Phase 29.
- Phases 29-31 are deferred after the Phase 28 beta cut while work returns to the TUI and CLI.

</deferred>

---

*Phase: 28-configuration-ui-beta-cut*
*Context gathered: 2026-08-09*

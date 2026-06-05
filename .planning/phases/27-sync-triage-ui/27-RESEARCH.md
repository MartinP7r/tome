# Phase 27: Sync + triage UI — Research

**Researched:** 2026-06-05
**Domain:** Cross-stage mutating pipeline rendered as a Tauri 2 / React 19 GUI (first of its kind in tome); lockfile-diff triage UI; structured `machine.toml` line-diff preview; cooperative cancellation; stage-resumable retry.
**Confidence:** HIGH (substrate verified by Read on Phase 25/26 code; design contract approved 2026-06-04; the five open items are pre-flagged for the planner, not for the researcher).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Twenty decisions D-01..D-20 from `27-CONTEXT.md` are locked. They span: spatial layout (D-01..D-06 — Sync as 4th sidebar row; `⌘1..⌘4` re-anchors; idle view IS post-sync summary; nav free during run; sidebar badge counts pending; auto-return to idle on terminal except failure); progress visualisation (D-07..D-10 — 6-stage vertical stepper; `item: Option<String>` field added to existing `SyncStageProgress` variant; GitClone/BackupSnapshot fold into active stage subtitle; per-stage durations are JS-only); triage panel + per-skill actions (D-11..D-15 — three vertical sections with source-group sub-sections; inline `[✓ keep]` chip toggles; bulk actions only on `NEW`; `view source` reuses `open_source_folder`; `PreviewPopover` + `MachineTomlDiff` for Apply); carryover #2 (D-16 — `synced_at` plumbing only, no domain-semantics change); failure + cancellation (D-17..D-20 — Cancel button always visible, no confirm; stepper transforms in place on terminal; `retry_from: Option<SyncStage>` hint domain-driven; partial-failure rendering via `FindingRow` lists inside completed `StageRow`s).

The planner MUST research within these — do not explore alternatives to locked decisions.

### Claude's Discretion

- Stage label wording (plain English; typed `SyncStage` variant name stays internal identity).
- Toast positioning + duration + dismissal (D-06 "Sync complete" / "Sync cancelled"; Phase 26 has no toast precedent yet — see Pitfall 2 below).
- Sidebar working-spinner style (D-04) — small system spinner.
- TOML diff rendering details inside `PreviewPopover`.
- Default expansion state of triage sections (NEW expanded, CHANGED + REMOVED collapsed).
- `[Retry failed items]` exact scope.
- Stage-duration display format (already pinned in UI-SPEC: `0.3s` / `8.2s` / `1m 14s`).
- Stepper layout responsiveness.
- `item: Option<String>` exact emission for git-clone fold-in (sink-side formatting).
- Where the `retry_from` hint lives (wrapping `SyncOutcome` struct vs extending `TomeError`) — see Open Item 1 below.

### Deferred Ideas (OUT OF SCOPE)

- Opening upstream URLs in browser for git-sourced skills (in-Finder reveal only this phase).
- Bulk "Retry all failed items" with per-item selection (single-button retry only).
- `CHANGED` bulk-disable.
- Sync activity log / history view (only "last sync" surface).
- Diff rendering for the actual SKILL.md content of `Changed` skills (right column shows old → new hash + metadata only).
- Real-time auto-sync on watcher events (out-of-scope per v1.0; watcher detects external sync but doesn't trigger new runs).
- `STATE.md` staleness fix (one-line edit, separate commit at start of execution).
- `CLAUDE.md` "Current State" header staleness.
- Interim `v0.17.0` release of #542 + Phase 25 `lib.rs` refactor.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SYNC-01 | "Sync" runs the same pipeline as `tome sync` with per-stage progress + current-directory indicator. | `tome::sync()` (`crates/tome/src/lib.rs:1762`) already takes `&dyn ProgressSink` + `&CancelToken` end-to-end; six `SyncStageStarted`/`SyncStageProgress`/`SyncStageFinished` emission sites are wired (lines 1871/1918, 1944/1959, 2006/2010, 2086/2116, 2144/2167, 2183/2206). `TauriEventSink` (`crates/tome-desktop/src/sink.rs`) bridges to a `SyncProgress` tauri-specta event. The `item: Option<String>` field per D-08 lands on the existing `ProgressEvent::SyncStageProgress` variant + the mirror `SyncProgress` payload. |
| SYNC-02 | Lockfile diff produces triage panel with per-skill `keep` / `disable on this machine` / `view source` actions and `Disable all new from <source>` bulk action. | `tome::update::diff(old, new) → UpdateDiff { changes: BTreeMap<SkillName, SkillChange> }` (`crates/tome/src/update.rs:37`) already produces structured `Added(LockEntry) / Changed { old, new } / Removed(LockEntry)` records. SYNC-02 surfaces this verbatim via a new `get_lockfile_diff` Tauri command with a specta projection. Existing `tome::actions::set_skill_disabled` + `open_source_folder` cover the per-skill mutations and view-source action. |
| SYNC-03 | Triage decisions render as a `machine.toml` diff before save. No silent writes. | `machine::save(&prefs, path)` (`crates/tome/src/machine.rs:262`) is the canonical atomic temp+rename writer. Two new boundary commands: `preview_machine_toml` returns `MachineTomlPreview { lines: Vec<DiffLine> }`; `apply_machine_toml` commits decisions. Diff produced server-side from `toml::to_string_pretty(&proposed_prefs)` against the current TOML file content using the `similar` crate (verified on crates.io — see Standard Stack). |
| SYNC-04 | Sync runs cancellable at stage boundaries; library state stays consistent. | `CancelToken` (`crates/tome/src/progress.rs:169`) is an `Arc<AtomicBool>` checked at every stage boundary in `sync()` (verified lines 1863, 1935, 1999, 2073, 2138, 2179). Stage boundaries sit before each `bail!`; save() runs as a single atomic unit (no mid-save cancel — verified line 2173 comment). The GUI clones the token into app state; the `cancel_sync` command flips it. The CLI passes a never-tripped token today — the contract is already satisfied. |
| SYNC-05 | Failed sync surfaces per-stage failure summary with stage-resumable retry. | `TomeError { code, message, context }` (`crates/tome-desktop/src/error.rs:104`) carries the flattened anyhow chain — Phase 26 D-11's `FindingRow` already renders this shape. The new wrapping `SyncOutcome { result: Result<(), TomeError>, retry_from: Option<SyncStage>, partial_failures: Vec<PartialFailure> }` (planner-confirmed in Open Item 1) carries D-19's stage-resumable hint. Partial-failure aggregation already exists in domain code (SAFE-01 — `distribution_cleanup_failures` line 2142, install failures line 2210) and surfaces via the existing `SyncReport`; SYNC-05 promotes selected fields to the IPC boundary. |

</phase_requirements>

## Summary

Phase 27 is the first GUI surface that **runs the sync pipeline end-to-end**, replacing both the CLI flow (`tome sync`) and the `update::present_changes` interactive triage with a visual flow. Substrate built in Phases 25 and 26 makes this an integration phase, not a research phase — `ProgressSink`/`CancelToken`/`TauriEventSink`/`TomeError`/`PreviewPopover`/`FindingRow`/`SectionHeader` all exist. **The work is mostly: (1) extend two existing types by one optional field each, (2) wire a small fanout of new Tauri commands + one new wrapping struct + 0–2 new events, (3) build five new React molecules atop existing Phase 26 atoms, (4) close two Phase 26 carryovers (group-by SectionHeader + Recent sort).**

Three nontrivial decisions where the planner must be deliberate:

1. **List-primitive choice for `TriagePanel`.** UI-SPEC §Component Contract pencils `TriageRow` as `role="option"` inside a `ListBox`. React Aria documentation explicitly forbids interactive children (buttons, chips) inside `<ListBoxItem>` — they break keyboard + screen-reader navigation. The inline `[✓ keep]` toggle chip (D-12) is a button; the existing Skills view uses `<ListBoxItem>` with no inline buttons inside rows (the menu lives outside). **Recommendation: use `<GridList>` + `<GridListItem>` + `<GridListSection>` instead** — supports interactive children by design, sections are documented (currently α-tagged but shipping in 1.18), and keyboard navigation behaviour is configurable. See Pitfall 1.

2. **TOML diff: do it server-side with a structured `Vec<DiffLine>` payload.** D-15's open item 5 names this. The `similar` crate (3.1.1, MIT, by Armin Ronacher — same author as `insta`) is the canonical Rust diff library, used by `cargo`'s diff plugins and by `insta` snapshot reviews. `TextDiff::from_lines(current, proposed).iter_all_changes()` yields `Change<&str>` records with `ChangeTag::Equal | Insert | Delete` — a one-pass map gives the planner exactly the `DiffLine` shape UI-SPEC §`MachineTomlDiff` declares. **Hand-rolling a line-diff is feasible** (the diff is tiny — at most a few dozen lines of `machine.toml`), but `similar` is one dependency, well-vetted, and gives us correct behaviour on edge cases like adjacent changes. Recommendation: add `similar` to `crates/tome` under a thin internal helper in `machine.rs::preview_save`; it is NOT a UI dep.

3. **The `SyncOutcome` IPC shape (Open Item 1).** The UI-SPEC already renders against the wrapping-struct shape. Picking the alternative ("extend `TomeError` with `retry_from`") would conflate "failure classification" with "control-flow hint" — the latter is non-error information (`SyncReport` partial-failures also carry a `retry_from` in their domain — they're not errors). **Recommendation: wrapping struct** — `SyncOutcome { result: Result<(), TomeError>, retry_from: Option<SyncStage>, partial_failures: Vec<PartialFailure> }`. The `result` discriminant tells the React side whether to render the failure path; `retry_from` is consulted whenever it's `Some` (failure or partial); `partial_failures` is consulted on success.

**Primary recommendation:** Wave 1 lands 27-01 (extends `ProgressEvent::SyncStageProgress` with `item: Option<String>`, extends `SyncProgress` mirror, extends `DiscoveredSkill` with `synced_at: Option<DateTime>`, registers the `start_sync` + `cancel_sync` commands, registers any new events, regenerates `bindings.ts`). Waves 2–4 run 27-02 (triage panel + `GridList`-based primitives + `SectionHeader` wiring for both consumers), 27-03 (`preview_machine_toml` / `apply_machine_toml` + `MachineTomlDiff` slot in `PreviewPopover`), 27-04 (cancellation wiring + integration tests), 27-05 (failure + retry + partial-failure aggregation). The biggest risk for plan-checker is mis-sizing 27-02 — it carries the most net-new component code AND closes both Phase 26 carryovers.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Sync pipeline orchestration (the 6-stage state machine) | Rust domain (`crates/tome/src/lib.rs::sync`) | — | Already lives here; SYNC-01 wires a GUI sink/cancel into the existing call site. Domain stays sync (no tokio). |
| Stage progress emission (typed events) | Rust domain (`progress::ProgressEvent`) | Boundary (`tome-desktop::sink::SyncProgress` mirror) | "Structure at the edge" Phase 25 D-17 — typed `SyncStage` discriminant crosses the boundary; React pattern-matches the variant. |
| Cancellation flag | Rust domain (`progress::CancelToken`) | Tauri command edge (`cancel_sync` flips the shared `Arc<AtomicBool>`) | Cooperative cancellation lives at stage boundaries in domain code; the GUI just owns a clone. |
| Lockfile diff computation | Rust domain (`update::diff`) | Boundary projection (`get_lockfile_diff` command returns specta-derived `UpdateDiff`) | Diff is already structured; SYNC-02 only adds an IPC projection. No JS-side diffing. |
| `machine.toml` text + structured diff generation | Rust domain (new helper in `machine.rs::preview_save`) | Boundary (`preview_machine_toml` command returns `MachineTomlPreview`) | Per D-GUI-08 / "no JS-side business logic", TOML serialization + Myers diff (via `similar`) happen Rust-side; React renders pre-computed `Vec<DiffLine>`. |
| `machine.toml` write | Rust domain (`machine::save` atomic temp+rename) | Boundary (`apply_machine_toml` command) | Phase 26's `MachinePrefsChanged` watcher event already fires for own-process writes — UI refresh is free. |
| Sync section render (idle / in-progress / terminal) | React (new `SyncView` under `views/`) | — | Pure presentation; consumes typed events + command results. |
| Stepper visualisation + duration tracking | React (`StageStepper` / `StageRow`) | — | UI-only affordance; wall-clock measured on `SyncStageStarted`/`SyncStageFinished` event arrival per D-10. No domain-API change. |
| Triage panel state (per-skill decisions) | React (controlled state inside `SyncView`) | Boundary (passes to `preview_machine_toml`) | Decisions are draft until Apply; live only in React state until committed via the preview-then-confirm flow. |
| Retry-from-stage gating (which stages are safe) | Rust domain (computes `retry_from: Option<SyncStage>` based on SC#5 rule) | Boundary (`SyncOutcome.retry_from` field) | Safety rule "rerunning distribute on partial manifest is not OK" is domain logic per D-19. The React side just renders one button label. |
| Partial-failure aggregation (SAFE-01) | Rust domain (already exists — `SyncReport.cleanup` + `install_failures`) | Boundary (selected fields lifted onto `SyncOutcome.partial_failures`) | Domain already aggregates; SYNC-05 promotes the structure to IPC. |
| File-system change observation (post-Apply refresh) | Rust domain (`tome-desktop::watcher`) | — | Phase 26 plan 26-06 already fires `MachinePrefsChanged` for `apply_machine_toml`'s write and `ManifestChanged`/`LockfileChanged` after sync. No new wiring. |
| Sidebar route + keyboard `⌘1..⌘4` | React (`stores/router.ts`, `shell/Sidebar.tsx`) + Rust menu (`menu.rs::MenuAction`) | — | View enum gains `"sync"`; `MenuAction` enum gains `JumpSync`; menu accelerator `⌘3` re-anchors to Sync, `⌘4` to Health. One Phase 26 breaking change. |

## Standard Stack

> Every package in this section is already in `Cargo.toml` / `ui/package.json` of `crates/tome-desktop` OR is in the canonical Rust ecosystem (verified via `cargo search`). **Phase 27 introduces at most ONE new Rust dep (`similar`) and ZERO new npm deps.**

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tauri-specta` | `=2.0.0-rc.25` [VERIFIED: existing Cargo.toml + Phase 25 lock] | Typed Tauri command + event registry; derives specta types | Already the IPC contract spine. New commands + the `SyncOutcome` wrapping struct register here verbatim. |
| `specta` | `=2.0.0-rc.25` [VERIFIED: existing Cargo.toml] | Rust → TypeScript type generation | Already used by every cross-boundary type. The new `SyncOutcome`, `PartialFailure`, `MachineTomlPreview`, `DiffLine`, `LockfileDiff` projection, `TriageDecision` all derive `specta::Type`. |
| `react-aria-components` | `^1.18.0` [VERIFIED: existing ui/package.json] | Headless accessible primitives | `GridList` + `GridListItem` + `GridListSection` for `TriagePanel` (NOT `ListBox` — see Pitfall 1). `RadioGroup` for `TriageDetail` canonical picker. `DialogTrigger` + `Popover` for `PreviewPopover` (reused unchanged). |
| `similar` | `3.1.1` [VERIFIED: crates.io `cargo search`; current as of 2026-06-05] | Myers-algorithm line diff for `machine.toml` preview | Canonical Rust diff crate (`mitsuhiko/similar`, MIT, used by `insta`, `cargo-dist` testing, `cargo-mutants`). `TextDiff::from_lines` returns iterable `Change<&str>` records the planner maps to `DiffLine` Vec. Default-features off needed (drops a transitive `console` dep we already have). **Package legitimacy: HIGH confidence — known-good author, 5+ year track record, broad reverse-dep graph.** |

[CITED: docs.rs/similar](https://docs.rs/similar/) — `TextDiff::from_lines(old, new).iter_all_changes()` yields `ChangeTag::{Equal, Delete, Insert}` records that map 1:1 to the UI-SPEC §`MachineTomlDiff` `kind: 'unchanged' | 'removed' | 'added'` shape.

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `notify` | `8.2` [VERIFIED: existing Cargo.toml] | File watcher backend | Phase 26 wired. SYNC-03 inherits: `apply_machine_toml` write → existing watcher fires `MachinePrefsChanged` → idle-state refreshes. Zero work. |
| `notify-debouncer-full` | `0.7` [VERIFIED: existing Cargo.toml] | Debounce window for watcher | Same — inherited. |
| `tauri-plugin-opener` | `^2` [VERIFIED: existing Cargo.toml] | `reveal_item_in_dir` for "view source" | Reused for D-14 git source reveal in Finder. Zero new IPC surface. |
| `react-markdown` + `remark-gfm` | `^10.1.0` / `^4.0.1` [VERIFIED: ui/package.json] | Markdown rendering | Not used in Phase 27. |
| `@tauri-apps/api/event` | `^2` [VERIFIED: ui/package.json] | Event subscription on the React side | `useTauriEvent` hook (`hooks/useTauriEvent.ts`) is the established pattern. The new `SyncProgress` subscription routes through it; failure subscriptions if any new event types are added route through it too. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `similar` for line diff | Hand-rolled LCS in `machine.rs` | Saves one dep. But: edge cases (adjacent inserts/deletes, deep blank-line runs) are exactly what Myers handles correctly — and `similar` is already in `insta`'s dep tree if we ever add snapshot testing. **`similar` wins.** |
| `similar` for line diff | `dissimilar` (rustc team's diff crate) | `dissimilar` is char-aware (great for prose) but its primary API outputs `Chunk<&str>` joined runs — converting to per-line `DiffLine` is more work than `similar`'s direct line API. |
| `similar` | `imara-diff` | Faster on huge inputs (>100k lines). Irrelevant — `machine.toml` is ~10–50 lines. `similar` is the simpler API and has the broader ecosystem. |
| `<GridList>` for `TriagePanel` | `<ListBox>` (matches Skills view) | `<ListBox>` forbids interactive children inside `<ListBoxItem>`. The inline `[✓ keep]` chip toggle (D-12) IS an interactive child. **`GridList` is the only A11y-correct primitive here.** |
| Server-side line-diff | Diff library in JS (`diff`, `jsdiff`) | Violates D-GUI-08 "no JS-side business logic" — diff semantics aren't presentation. Also adds an npm dep where zero are needed. |
| Wrap return in `SyncOutcome` | Extend `TomeError` with `retry_from: Option<SyncStage>` | The retry hint is non-error info on success-with-partial-failures. Sticking it on `TomeError` couples two concepts. Wrapping struct wins. |

**Installation:**

```bash
# Rust side (Cargo.toml — workspace or crates/tome):
cargo add similar --no-default-features --features text  # text-only line diff; drops the console pretty-print dep
# (verify version with `cargo search similar` before the plan commits)

# UI side: NOTHING. Every component is built from the existing stack.
```

**Version verification (2026-06-05):**

```
similar 3.1.1            # `cargo search similar --limit 1` 2026-06-05; MIT; canonical mitsuhiko/similar
```

The `notify` 9.0-rc.x and `notify-debouncer-full` 0.8-rc.x lines exist on crates.io as of Phase 26's research; Phase 27 inherits Phase 26's `^8.2` / `^0.7` resolver pin (no upgrade).

## Package Legitimacy Audit

> Phase 27 introduces exactly **one** new external package on the Rust side. Two evaluation methods were applied because `slopcheck` is unavailable in this environment.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `similar` | crates.io | ~5 years (first published 2021-03; current 3.1.1 on 2024-04) | ~12M total downloads (per crates.io listing) | `github.com/mitsuhiko/similar` (Armin Ronacher — Flask, sentry-sdk, insta) | unavailable (graceful degradation per gate) | **Approved (high confidence on reputation + reverse-dep graph; planner should still add a one-step `cargo deny check` after the `cargo add` lands as belt-and-braces)** |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

*slopcheck was unavailable at research time; the disposition rests on (a) crates.io version verification, (b) repo provenance from a well-known author, (c) presence in the `insta`/`cargo-mutants` reverse-dependency graph (already vetted by upstream). The planner should still gate the install behind `cargo deny check` (already used by the repo via `deny.toml`).*

## Architecture Patterns

### System Architecture Diagram

```
[User] ─ ⌘R / toolbar [Sync] / Library→Sync menu ─┐
                                                  ▼
                              [React] SyncView ── start_sync command ──► [Tauri Cmd Boundary]
                                                                                 │
                                                                                 ▼
                                                                    load_context() → Config + TomePaths
                                                                                 │
                                                                                 ▼
                                                                    tome::sync(config, paths, opts, sink, cancel)
                                                                                 │
                              ◄─── SyncProgress events ◄─── TauriEventSink.emit ◄┤  (6 stages, threaded)
                              ◄─── Watcher events (free) ◄─── notify watcher  ◄──┤  (post-Save refresh)
                                                                                 ▼
                                                                       SyncOutcome { result, retry_from, partial_failures }
                                                                                 │
                                                                                 ▼
                              ◄─────────── return ────────────────────────────────┘
                                                  │
                                                  ▼
                              [React] SyncView reconciles:
                                ├─ stepper terminal state (D-18)
                                ├─ if retry_from.is_some() → render [Retry from <stage>]
                                ├─ if partial_failures.len() > 0 → render FindingRow list inside affected StageRow + [Retry failed items]
                                └─ else auto-return to idle + toast

[User] ─ select TriageRow → onSelect ─► TriageDetail (right column) — pure render
[User] ─ click [✓ keep] chip ─► onDecisionToggle ─► local React state (no IPC)
[User] ─ click [Apply N decisions] ─► PreviewPopover opens
                                          │
                                          ▼
                              preview_machine_toml(decisions) command
                                          │
                                          ▼
                              [Rust] reads current machine.toml,
                                     applies decisions to a cloned MachinePrefs,
                                     serializes to TOML,
                                     similar::TextDiff::from_lines(current, proposed),
                                     maps to Vec<DiffLine>
                                          │
                                          ▼
                              MachineTomlPreview { lines: Vec<DiffLine> } returned
                                          │
                                          ▼
                              MachineTomlDiff renders inside PreviewPopover slot
                                          │
                                          ▼
                              [User] clicks Apply ─► apply_machine_toml(decisions) command
                                                          │
                                                          ▼
                                          machine::save(&prefs, path) — atomic temp+rename
                                                          │
                                                          ▼
                                          notify watcher fires MachinePrefsChanged
                                                          │
                                                          ▼
                                          idle-state hook refetches; badge clears

[User] ─ click [Cancel sync] ─► cancel_sync() command ─► CancelToken::cancel() (flips Arc<AtomicBool>)
                                                              │
                                                              ▼ (next stage boundary)
                                                  sync() returns Err(anyhow!("sync cancelled"))
                                                  → classified as TomeError (some code; message "sync cancelled")
                                                  → SyncOutcome { result: Err, retry_from: None }
                                                  → React renders D-18 cancelled stepper
```

### Recommended Project Structure

```
crates/tome/src/
├── progress.rs                # D-08: add `item: Option<String>` to ProgressEvent::SyncStageProgress + update RecordingSink test
├── discover.rs                # D-16: add `synced_at: Option<DateTime>` to DiscoveredSkill (sourced from manifest at discover time)
├── machine.rs                 # SYNC-03: add fn preview_save(prefs, path) -> Result<MachineTomlPreview> using `similar`
└── lib.rs                     # no surface change (sync() already takes sink + cancel)

crates/tome-desktop/src/
├── sink.rs                    # D-08: add `item: Option<String>` to SyncProgress mirror; D-09: fold GitCloneProgress + BackupSnapshot into `item`
├── commands.rs                # SYNC-01..05: add start_sync, cancel_sync, get_lockfile_diff, preview_machine_toml, apply_machine_toml, retry_sync (or fold into start_sync)
└── lib.rs                     # update make_builder() to register the 5–6 new commands + 0–2 new events

crates/tome-desktop/ui/src/
├── views/
│   └── SyncView.tsx           # NEW — owns idle / in-progress / terminal state
├── components/
│   ├── StageStepper.tsx       # NEW (D-07/D-10/D-18/D-19/D-20)
│   ├── StageRow.tsx           # NEW — pending / active / complete / failed / cancelled variants
│   ├── TriagePanel.tsx        # NEW (D-11/D-12/D-13)
│   ├── TriageRow.tsx          # NEW — inline chip + selection state
│   ├── TriageDetail.tsx       # NEW — right-column RadioGroup picker + diff metadata
│   ├── MachineTomlDiff.tsx    # NEW — slot content inside PreviewPopover (D-15)
│   ├── SectionHeader.tsx      # EXTENDED — second consumer added (existing Health-view consumer unchanged; SkillsView gets a separate wiring for VIEW-02 group-by carryover)
│   └── PreviewPopover.tsx     # REFACTORED — accept a content slot (currently hardcoded to Doctor's body sentence); see Pitfall 3
├── hooks/
│   ├── useSync.ts             # NEW — owns CancelToken-like cleanup, subscribes to SyncProgress, accumulates stage state, exposes start/cancel/retry handlers
│   └── useLockfileDiff.ts     # NEW — calls get_lockfile_diff after the Reconcile stage completes
├── shell/
│   └── Sidebar.tsx            # EXTENDED — 4th NavItem (Sync); badge can mean pending count OR failure count (mutually exclusive)
├── stores/
│   └── router.ts              # EXTENDED — View type gains "sync"
└── main.rs / lib.tsx          # update App.tsx to route to SyncView when view === "sync"

crates/tome-desktop/src/
└── menu.rs                    # EXTENDED — MenuAction gains JumpSync; Library menu Sync item activates ⌘R toolbar handler; ⌘3 re-anchors

crates/tome-desktop/tests/a11y/
└── axe.spec.ts                # EXTENDED — add Sync-view axe scan after navigating via ⌘3 (or sidebar option click)

crates/tome-desktop/tests/
└── sync_smoke.rs (NEW)        # SYNC-04 integration test — drive `tome::sync` with a fake sink, assert cancel-at-Distribute leaves no half-written manifest
```

### Pattern 1: "Structure at the edge"

**What:** Domain emits typed `ProgressEvent`; boundary mirror struct (`SyncProgress`) carries the same `SyncStage` discriminant; React pattern-matches the variant. No string formatting at the boundary except where D-09 explicitly delegates it (sink-side `item` formatting for git-clone bytes).
**When to use:** Every new IPC type Phase 27 adds.
**Example:**
```rust
// Source: crates/tome-desktop/src/sink.rs (existing) + D-08 extension
ProgressEvent::SyncStageProgress { stage, current, total, item } => SyncProgress {
    stage,
    current: saturate_usize(current),
    total: saturate_usize(total),
    item,  // D-08 — passes through typed; React renders directly
},
```

### Pattern 2: Preview-then-confirm (NF-04 ergonomic)

**What:** Mutating actions show a `PreviewPopover` before the write happens. Inside the popover: a content slot describes the change; Confirm fires the actual command.
**When to use:** SYNC-03 Apply flow.
**Example:**
```tsx
// Source: crates/tome-desktop/ui/src/components/PreviewPopover.tsx (existing — needs slot refactor; see Pitfall 3)
<PreviewPopover
  trigger={<Button>Apply {N} decisions</Button>}
  onApply={async () => {
    await commands.applyMachineToml(decisions);
    // watcher fires MachinePrefsChanged; idle-state refetches for free
  }}
>
  <MachineTomlDiff preview={previewResult} />
</PreviewPopover>
```

### Pattern 3: Cooperative cancellation via shared `Arc<AtomicBool>`

**What:** GUI clones a `CancelToken` into app state when sync starts; `cancel_sync` command flips the bit; domain checks at stage boundaries.
**When to use:** SYNC-04 — the only place a long-running command needs a kill switch.
**Example:**
```rust
// Source: crates/tome/src/progress.rs (existing) + commands.rs (new)
#[tauri::command]
#[specta::specta]
pub async fn start_sync(app: tauri::AppHandle, state: tauri::State<'_, SyncState>) -> Result<SyncOutcome, TomeError> {
    let cancel = CancelToken::new();
    *state.cancel.lock().unwrap() = Some(cancel.clone());     // share the bit
    let sink = TauriEventSink::new(app.clone());
    // run on a blocking thread so the IPC handler returns control to the runtime
    let result = tokio::task::spawn_blocking(move || tome::sync(&config, &paths, opts, &sink, &cancel)).await?;
    *state.cancel.lock().unwrap() = None;
    Ok(SyncOutcome::from(result))                              // wrap + classify
}

#[tauri::command]
#[specta::specta]
pub fn cancel_sync(state: tauri::State<'_, SyncState>) -> Result<(), TomeError> {
    if let Some(token) = state.cancel.lock().unwrap().as_ref() {
        token.cancel();
    }
    Ok(())  // idempotent
}
```

### Pattern 4: Plan/render/execute for mutating actions

**What:** Domain produces a structured "plan" (preview) the user reviews; on Confirm, a second command executes. Already used by `remove`/`reassign`/`relocate`/`eject` and Phase 26's doctor-repair flow.
**When to use:** SYNC-03 `preview_machine_toml` + `apply_machine_toml` is a textbook instance.

### Anti-Patterns to Avoid

- **Per-keystroke IPC for the triage panel.** The lockfile diff is fetched once after Reconcile completes (or on view mount if the user opens the section idle). All triage state lives in React; the diff payload is small (skill count bounded by lockfile size). Don't round-trip on every chip click.
- **Putting `[✓ keep]` button inside `<ListBoxItem>`.** Breaks ARIA + keyboard navigation (see Pitfall 1).
- **Hand-rolling a Myers diff.** Edge cases (adjacent inserts/deletes, deep equal runs) are precisely what `similar` exists to handle.
- **Stringifying `SyncStage` at the boundary.** The whole "structure at the edge" pattern (Phase 25 D-17) is the typed variant crossing as a TS string-union discriminant. Never `format!("{:?}", stage)`.
- **Single subscribe-to-everything React hook.** Phase 26's `useTauriEvent` doc explicitly warns against this; each new hook subscribes only to the events it depends on.
- **Storing wall-clock per stage on the Rust side.** D-10 is explicitly a React-side affordance — UI records timestamps on event arrival, computes the delta on render. No domain-API change.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Line-by-line text diff | A manual LCS / Myers loop | `similar::TextDiff::from_lines` | Adjacent inserts/deletes, equal-line runs, trailing-newline handling are all edge cases `similar` already gets right. ~10 lines of mapping code. |
| Cancellation channels | A custom tokio `CancellationToken` or watch channel | `tome::progress::CancelToken` (already exists; sync `Arc<AtomicBool>`) | Domain stays sync; the GUI shares a clone. Don't add tokio to `crates/tome`. |
| Sectioned list with interactive items | `<ListBox>` + custom workaround for chips | `<GridList>` + `<GridListSection>` (`react-aria-components`) | The A11y semantics for interactive children inside list items are built in. ListBox forbids them. |
| Per-skill radio picker | Custom radio buttons | `react-aria-components <RadioGroup>` | Accessibility, keyboard nav, focus management for free. Phase 26 already depends on the lib. |
| Toast for "Sync complete" / "Sync cancelled" | A `useEffect`+`setTimeout` ad-hoc div | **HONEST CALL:** Phase 26 has no toast precedent. **Two options** for the planner (Open Item 3 in this research, distinct from CONTEXT.md open items): (a) adopt `react-aria-components`' `UNSTABLE_ToastRegion` + `UNSTABLE_ToastQueue` — accessible + correct but UNSTABLE-prefixed in 1.18 (production-API-risk); (b) hand-roll a single live-region div with `role="status" aria-live="polite"` and a 5s `setTimeout` (matches WCAG transient-message guidance — see ARIA APG live-region pattern). Recommendation: **(b)** for v1.0 — the surface area is tiny, the UNSTABLE prefix would lock us into upgrade work later, and we have zero re-use of toasts elsewhere yet. |
| Fuzzy match on triage entries | (not applicable — the triage list is bounded and grouped; no search needed in Phase 27) | — | — |
| Atomic file writes for `machine.toml` | A custom temp+rename | `machine::save` (already does this) | Path verified in `crates/tome/src/machine.rs:262`. |

**Key insight:** The Rust side already owns every primitive Phase 27 needs (`ProgressSink`, `CancelToken`, `TomeError`, atomic `machine::save`, `update::diff`, `tome::actions::*`, `notify` watcher). The React side already owns every primitive (`PreviewPopover`, `FindingRow`, `Badge`, `Button`, `SectionHeader`, the alpha shell). **Phase 27 is at least 70% wiring and 30% net-new components.** The plan-checker should be suspicious of any plan that proposes new abstractions where the existing ones fit.

## Runtime State Inventory

> Not applicable — this is a greenfield phase (adding capability), not a rename / refactor / migration. The phase does extend two existing types (`ProgressEvent::SyncStageProgress` gains `item`, `DiscoveredSkill` gains `synced_at`), but these are additive `Option<…>` fields; nothing on-disk needs migrating.

**Stored data:** None changed. Manifest already stamps `synced_at` per skill (`crates/tome/src/manifest.rs:181` — `synced_at: String` exists; D-16 plumbs it through `DiscoveredSkill` to `ListReport` so the Skills view can sort by it).
**Live service config:** None.
**OS-registered state:** None.
**Secrets/env vars:** None.
**Build artifacts:** `bindings.ts` regenerates after each command/event change; the CI freshness gate (Phase 25, `git diff --exit-code`) enforces it. The planner adds a Wave 1 task to run `cargo run -p tome-desktop --bin gen-bindings` and commit the result.

## Common Pitfalls

### Pitfall 1: `ListBox` forbids interactive children — use `GridList` for `TriagePanel`

**What goes wrong:** UI-SPEC §`TriagePanel` pencils `<TriageRow>` as `role="option"` inside a `<ListBox>` with inline `[✓ keep]` chip toggles. React Aria's `<ListBoxItem>` documentation explicitly says: *"Interactive elements (e.g. buttons) within listbox items are not allowed. This will break keyboard and screen reader navigation."*
**Why it happens:** The Skills view uses `<ListBoxItem>` and stays compliant because it has no inline mutations — the context menu lives outside the row. The triage panel's defining UX is an inline chip toggle (D-12).
**How to avoid:** Use `<GridList>` + `<GridListItem>` + `<GridListSection>` instead. Per React Aria docs: GridList "supports interactive children" by design (drag handles, checkboxes, action buttons), has the same keyboard navigation semantics, and supports sections with sticky headers. UI-SPEC §VoiceOver labels translate verbatim: `<h2>` outer → `<GridListSection aria-label>` outer; same for inner `<h3>`. The planner's 27-02 should specify GridList in its task list, not ListBox.
**Warning signs:** axe-core/playwright failures on `nested-interactive` rule; VoiceOver announcing chip buttons as the row's primary action instead of as a secondary control.
**Source:** [CITED: react-aria.adobe.com/ListBox] "Interactive elements (e.g. buttons) within listbox items are not allowed."

### Pitfall 2: Toast affordance has no Phase 26 precedent — pick before 27-01

**What goes wrong:** D-06 calls for a "Sync complete" / "Sync cancelled" toast and CONTEXT.md "Claude's Discretion" defers position/duration to the planner — but Phase 26 didn't ship a toast component. The planner could either reach for `react-aria-components` `UNSTABLE_ToastRegion` or hand-roll a `role="status" aria-live="polite"` div.
**Why it happens:** Phase 26's surfaces were all non-transient — every notification was inline (FindingRow disclosures, validation banners). Toasts didn't appear in the alpha cut.
**How to avoid:** Make this an explicit 27-01 task. **Recommended choice: hand-roll a single `<div role="status" aria-live="polite">` with a 5-second setTimeout-driven mount/unmount cycle**, styled per UI-SPEC §Color tokens (semitransparent surface, top-right, 3s visible + 200ms fade — Apple HIG `NSAlert` transient style). Justification: the toast appears in exactly two places in v1.0 (D-06 success and D-06 cancellation; D-18 failures are NEVER toasts), so the UNSTABLE-prefixed React Aria API would be over-engineering. The `UNSTABLE_` prefix means the API can change in a non-major release of `react-aria-components` — production risk. (If a future phase needs toasts in more places, promote to the UNSTABLE component then.)
**Warning signs:** Plan check flags "no toast component exists"; planner reaches for a UNSTABLE-prefixed API.
**Source:** [CITED: react-aria.adobe.com/Toast] "Toast is UNSTABLE in v1.18."

### Pitfall 3: `PreviewPopover` is currently hardcoded to a single-sentence body

**What goes wrong:** UI-SPEC §`MachineTomlDiff` says "Reuses `PreviewPopover` verbatim — same outer shell as Doctor's Fix." But the existing `PreviewPopover.tsx` (verified at `crates/tome-desktop/ui/src/components/PreviewPopover.tsx`) takes a `dryRunDescription: string` prop and renders a `<p>` — not a content slot. Phase 27 needs the diff body, not a sentence.
**Why it happens:** Phase 26 D-09 shipped the popover for the Doctor flow which only needs one sentence. The "verbatim reuse" UI-SPEC language elides that the *outer shell* is reused; the *content slot* needs to be made a slot.
**How to avoid:** 27-03 includes a small `PreviewPopover` refactor — replace `dryRunDescription: string` with `children: ReactNode` (or a new `bodyContent` prop), keeping the rest of the contract (PREVIEW caption, button row, A11y dialog wrapper). Then both the Doctor flow (passes `<p>{description}</p>`) and the Apply flow (passes `<MachineTomlDiff preview={…} />`) compose. Update the existing Doctor caller in the same plan. The width override (480px for diff vs 320px default) goes onto an opt-in `width` prop.
**Warning signs:** Plan-checker flags the contradiction between "reused verbatim" and "must accept the diff body".

### Pitfall 4: `Reconcile` stage now spans BOTH git-clone events AND lockfile-diff drift detection

**What goes wrong:** D-09 folds `GitCloneProgress` into the active stage's `item` subtitle — and `TauriEventSink` already routes `GitCloneProgress` to `SyncStage::Reconcile` (verified `sink.rs:78`). But `sync()` (line 1953) actually triggers `resolve_git_directories` (which emits `GitCloneProgress`) inside the **Discover** stage span, not Reconcile. The CLI sink renames this away as a presentation detail; the GUI's typed routing means git-clone events would surface on the wrong stage row.
**Why it happens:** The sink does `match` on event variant, not on the currently-active stage. The git-clone events carry no stage of their own — the sink picks one to fold into.
**How to avoid:** Either (a) extend `GitCloneProgress` to carry a `stage: SyncStage` field, OR (b) emit a `SyncStageStarted { stage: Reconcile }` before `resolve_git_directories` runs (treating git resolution as part of Reconcile in event emission terms), OR (c) keep the current `Reconcile` routing in the sink BUT verify that `resolve_git_directories`'s span in `sync()` truly conceptually belongs to Reconcile (it does — drift detection includes "is the git source we resolve still here?"). Option (c) is cheapest and matches the user mental model ("Reconcile = checking what's already there"); requires only that the planner reads the line-1953 location and confirms before 27-01 commits.
**Warning signs:** A live test of `tome sync` shows the git-clone subtitle appearing on the Discover row in the GUI while the spinner is on Reconcile in the CLI. Either way, the planner should add a Wave-1 task to **explicitly assert in `RecordingSink` tests that `SyncStageStarted { Reconcile }` precedes the first `GitCloneProgress`** so the sink's "fold into Reconcile" routing is correct.

### Pitfall 5: `start_sync` must NOT block the Tauri main thread

**What goes wrong:** `tome::sync` is synchronous and can take many seconds (git clone of a remote source, hundreds of skills to consolidate). If `start_sync` is a `#[tauri::command]` that calls `tome::sync` directly, it blocks the JS-IPC handler thread; nothing else can return (including `cancel_sync`).
**Why it happens:** The default Tauri `#[command]` runs synchronously in the IPC handler. The PHASE 25 spike used `NullSink`/short-running flows so the issue didn't surface.
**How to avoid:** Make `start_sync` `async fn` and run the sync body on `tokio::task::spawn_blocking(...).await?`. The Tauri 2 runtime is already tokio-based — `spawn_blocking` exists and is the recommended pattern for sync long-runners. `cancel_sync` stays a fast sync command that flips the shared `Arc<AtomicBool>` and returns instantly. Validate the design in 27-01 (the Phase 25 spike already did `spawn_blocking` for the placeholder — there's a working precedent).
**Warning signs:** During a real run, clicking `[Cancel sync]` produces nothing visible — the cancel command is queued behind the sync command's response.
**Source:** [CITED: v2.tauri.app/develop/calling-rust/] "Commands can be async; use `tokio::spawn_blocking` for sync long-running work."

### Pitfall 6: Watcher's own-process self-fire causes a feedback loop if not gated

**What goes wrong:** SYNC-03 Apply writes `machine.toml`; the watcher fires `MachinePrefsChanged`; the idle-state's "last sync summary" refetches and re-renders. Fine. BUT — the watcher ALSO fires for the manifest + lockfile writes that `tome::sync` does in the Save stage. If `useSync` subscribes to `ManifestChanged` to refresh state mid-sync, the React state tracker could trip its own progress display.
**Why it happens:** Phase 26's watcher correctly fires for own-process writes (verified in `tests/watcher_smoke.rs`). The new `useSync` hook needs to know **not** to refetch from watcher events while a sync is in progress.
**How to avoid:** `useSync` ignores `ManifestChanged` / `LockfileChanged` events while `isRunning` is true; the idle-state hooks (which depend on these events for post-sync refresh) keep their existing subscriptions and fire AFTER `SyncOutcome` resolution. This is just a state-machine discipline — flag it explicitly in 27-01.
**Warning signs:** During a sync run, the stepper resets to idle then bounces back when the manifest is rewritten mid-Save.

### Pitfall 7: Re-anchoring `⌘3` is a breaking change to the keyboard map

**What goes wrong:** Phase 26 documents `⌘3 → Health` and ships menu accelerators for it (`menu.rs::JumpHealth`). UI-SPEC explicitly re-anchors `⌘3 → Sync` and `⌘4 → Health`. If the menu definitions aren't updated alongside the `MenuAction` enum + `useMenuActions` hook + Sidebar nav, the menu and the sidebar diverge.
**Why it happens:** Three files own the same fact: `menu.rs::install` (registers accelerators), `MenuAction` enum (typed event), `useMenuActions` (React-side switch). Phase 26's POLISH-04 exhaustiveness sentinel catches missing-variant compile errors; it does NOT catch "accelerator string drift".
**How to avoid:** 27-01 task list includes: (1) add `JumpSync` to `MenuAction::ALL` + match arms; (2) register `Sync` menu item with accelerator `⌘3` in `menu.rs::install`; (3) demote Health's `⌘3` to `⌘4`; (4) update `useMenuActions` switch; (5) update `axe.spec.ts` selectors; (6) add release-note line. The plan-checker should fail any 27-01 plan that omits any of these.

## Code Examples

Verified patterns from existing tome-desktop / tome code + cited library docs.

### Example: Domain-side typed event emission (D-08 extension target)

```rust
// Source: crates/tome/src/progress.rs (existing — D-08 adds the `item` field)
pub enum ProgressEvent {
    SyncStageStarted { stage: SyncStage },
    SyncStageProgress {
        stage: SyncStage,
        current: usize,
        total: usize,
        item: Option<String>,  // D-08 — directory name (Discover), skill name (Consolidate/Distribute), path (Cleanup), filename (Save), or "git: <dir> (<size>)" (Reconcile)
    },
    SyncStageFinished { stage: SyncStage },
    GitCloneProgress { directory: String, received: u64 },
    BackupSnapshot { message: String },
}
```

### Example: TauriEventSink folding — D-09 implementation site

```rust
// Source: crates/tome-desktop/src/sink.rs (existing) — D-08/D-09 modifications
impl ProgressSink for TauriEventSink {
    fn emit(&self, event: ProgressEvent) {
        let payload = match event {
            ProgressEvent::SyncStageProgress { stage, current, total, item } => SyncProgress {
                stage,
                current: saturate_usize(current),
                total: saturate_usize(total),
                item,                                       // D-08 — pass through
            },
            ProgressEvent::GitCloneProgress { directory, received } => SyncProgress {
                stage: SyncStage::Reconcile,                // (verify Pitfall 4)
                current: saturate_u64(received),
                total: 0,
                item: Some(format!("git: {directory} ({})", format_bytes(received))),   // D-09 — sink owns formatting
            },
            ProgressEvent::BackupSnapshot { message } => SyncProgress {
                stage: SyncStage::Save,
                current: 0,
                total: 0,
                item: Some(message),                        // D-09 — message becomes the subtitle verbatim
            },
            ProgressEvent::SyncStageStarted { stage } | ProgressEvent::SyncStageFinished { stage } => SyncProgress {
                stage, current: 0, total: 0, item: None,
            },
        };
        let _ = payload.emit(&self.app);
    }
}
```

### Example: similar-based machine.toml diff

```rust
// Source: docs.rs/similar/ TextDiff::from_lines → maps to UI-SPEC DiffLine shape
// Lands as new fn in crates/tome/src/machine.rs (or a sibling preview.rs).
use similar::{ChangeTag, TextDiff};

pub struct MachineTomlPreview {
    pub lines: Vec<DiffLine>,
    pub added_count: usize,
    pub removed_count: usize,
}

pub struct DiffLine {
    pub line_number: u32,
    pub kind: DiffLineKind,   // serializes as "removed" | "added" | "unchanged"
    pub content: String,
}

pub fn preview_save(current_text: &str, proposed_text: &str) -> MachineTomlPreview {
    let diff = TextDiff::from_lines(current_text, proposed_text);
    let mut lines = Vec::new();
    let mut added = 0;
    let mut removed = 0;
    let mut current_line = 1u32;
    let mut proposed_line = 1u32;

    for change in diff.iter_all_changes() {
        let (kind, line_number) = match change.tag() {
            ChangeTag::Equal  => { let n = proposed_line; proposed_line += 1; current_line += 1; (DiffLineKind::Unchanged, n) }
            ChangeTag::Delete => { removed += 1; let n = current_line; current_line += 1; (DiffLineKind::Removed, n) }
            ChangeTag::Insert => { added   += 1; let n = proposed_line; proposed_line += 1; (DiffLineKind::Added, n) }
        };
        lines.push(DiffLine { line_number, kind, content: change.to_string().trim_end_matches('\n').to_string() });
    }

    MachineTomlPreview { lines, added_count: added, removed_count: removed }
}
```

### Example: React `<GridList>` with sections + inline button

```tsx
// Source: react-aria.adobe.com/GridList — interactive children are SUPPORTED inside GridListItem
// Pattern for TriagePanel (D-11/D-12).
import { GridList, GridListItem, GridListSection, Button } from "react-aria-components";

<GridList aria-label="Triage decisions" selectionMode="single" onSelectionChange={onSelect}>
  <GridListSection>
    <Header>NEW ({newCount})</Header>
    <GridListSection>
      <Header>PLUGINS ({pluginsCount})</Header>
      {newPluginsSkills.map(skill => (
        <GridListItem key={skill.name} id={skill.name} textValue={skill.name}>
          <div className={styles.row}>
            <div className={styles.primary}>{skill.name}</div>
            <div className={styles.secondary}>{skill.source} · {skill.managed ? 'managed' : 'local'} · synced {relativeTime}</div>
            <Button onPress={() => onDecisionToggle(skill.name)} className={styles.chip}>
              {decision === 'keep' ? '✓ keep' : '⊘ disabled here'}
            </Button>
          </div>
        </GridListItem>
      ))}
    </GridListSection>
  </GridListSection>
  {/* … CHANGED, REMOVED sections … */}
</GridList>
```

### Example: spawn_blocking for the sync command (Pitfall 5)

```rust
// Source: v2.tauri.app/develop/calling-rust/ — async commands + spawn_blocking pattern
#[tauri::command]
#[specta::specta]
pub async fn start_sync(
    app: tauri::AppHandle,
    state: tauri::State<'_, SyncState>,
) -> Result<SyncOutcome, TomeError> {
    let cancel = CancelToken::new();
    *state.cancel.lock().expect("cancel mutex poisoned") = Some(cancel.clone());

    let (config, paths) = load_context().map_err(TomeError::from)?;
    let machine_path = tome::default_machine_path().map_err(TomeError::from)?;
    let machine_prefs = tome::machine::load(&machine_path).map_err(TomeError::from)?;
    let opts = build_sync_options(&machine_path, &machine_prefs);
    let sink = TauriEventSink::new(app.clone());

    let result = tokio::task::spawn_blocking(move || {
        tome::sync(&config, &paths, opts, &sink, &cancel)
    })
    .await
    .map_err(|join_err| TomeError::from(anyhow::anyhow!("sync task panicked: {join_err}")))?;

    *state.cancel.lock().expect("cancel mutex poisoned") = None;
    Ok(SyncOutcome::from_result(result))      // wraps + classifies + populates retry_from + partial_failures
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| CLI `update::present_changes` (interactive `dialoguer::MultiSelect`) | GUI triage panel with per-skill chip + canonical RadioGroup picker | This phase | First time the GUI triages ALL three buckets (today's CLI only triages Added). |
| Single CLI spinner spans the whole pipeline | 6-stage vertical stepper with per-stage progress + per-item subtitle | This phase (D-07/D-08) | Honest to the pipeline structure; per-stage timing diagnoses slow stages. |
| `Result<(), TomeError>` at every command boundary | `SyncOutcome { result, retry_from, partial_failures }` for sync only | This phase (D-19/D-20) | Retry hint is non-error info on success-with-partial-failures; needs its own carrier. |
| Skills view's "Recent" sort silently degrades to alphabetical (Phase 26 carryover) | `synced_at: Option<DateTime>` on `DiscoveredSkill`, plumbed through `ListReport` → `bindings.ts` → comparator | This phase (D-16) | VIEW-02 flips from `partial` → `complete`. |
| Skills view's group-by toolbar is a visual no-op (Phase 26 carryover) | `SectionHeader` wired into both `TriagePanel` (new) and `SkillsView` (back-fill) | This phase (D-11) | VIEW-02 group-by closure. |

**Deprecated/outdated:** none. Every type extended in Phase 27 is additive (`Option<…>` fields, additional struct field). No serialized type breaks.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `similar` crate is the right diff lib for `machine.toml` (Myers, structured output, broad reverse-dep graph). | Standard Stack | Low — if `dissimilar` or `imara-diff` is preferred for an unrelated reason, swap is mechanical (10 lines of mapping). |
| A2 | Hand-rolled `role="status" aria-live="polite"` toast is preferable to `react-aria-components::UNSTABLE_ToastRegion`. | Pitfall 2 / Don't Hand-Roll | Low — if the planner picks UNSTABLE, the only consequence is a future-tracking item when the API stabilizes. |
| A3 | `spawn_blocking` is the correct pattern to run `tome::sync` from an async Tauri command without blocking the IPC thread. | Pitfall 5 | Low — Phase 25's spike already used this; Tauri docs confirm. |
| A4 | `GitCloneProgress` should remain folded into `Reconcile` (not Discover) for D-09 purposes. | Pitfall 4 | Medium — requires planner to verify by reading `lib.rs:1953` and the Phase 25 sink test. The fix if wrong is small (re-route in the sink or add a stage field). |
| A5 | The watcher's own-process self-fire during the Save stage will cause UI bouncing unless `useSync` ignores `ManifestChanged`/`LockfileChanged` events while sync is running. | Pitfall 6 | Medium — discoverable only by running a real sync; integration test could catch it. |
| A6 | `<GridList>` is the right primitive for `TriagePanel` (its `Sections` API is α-tagged in 1.18 but documented and usable). | Pitfall 1 | Low — if α-tag causes axe-core failures, fall back to a non-virtualised `<table role="grid">` (the triage list is small enough — typical user has <30 entries). |
| A7 | The `MachinePrefsChanged` watcher event is sufficient to refresh the idle-state post-Apply with no manual invalidation. | Architecture Diagram | Low — verified Phase 26 contract; integration test `watcher_smoke.rs` confirms self-fire. |
| A8 | `SyncOutcome` wrapping struct is preferable to extending `TomeError` with `retry_from`. | Summary §3 | Low — pure shape decision; UI-SPEC already renders against the wrapping shape. |
| A9 | The planner can build `StageStepper` / `StageRow` without virtualisation. | Code Examples | Low — 6 rows fixed; no perf concern. |
| A10 | Per-stage durations measured on event-arrival timestamps (D-10) are accurate enough — Tauri event latency is sub-millisecond. | Architecture Map | Low — even 10ms of event latency wouldn't matter at the `0.1s` display granularity. |

**Planner action on A4 + A5:** add a verification task in 27-01 that runs the existing `RecordingSink` tests under a real `cargo test` AND adds two new tests: (1) `SyncStageStarted { Reconcile }` precedes `GitCloneProgress`; (2) the Save stage emits the manifest/lockfile writes correctly when watched.

## Open Questions

These map 1:1 to the UI-SPEC §"Open Items" — the researcher is surfacing them again because they reach into research territory in addition to design territory.

1. **`SyncOutcome` shape (wrapping struct vs error-attached).** Recommendation: wrapping struct. Planner picks during 27-04 / 27-05.
2. **Dark-mode token completion.** Phase 26 carry-forward; no Phase 27 introductions.
3. **`⌘1..⌘4` re-anchoring release note.** Single sentence in 27-01. Help → Keyboard Shortcuts cheatsheet (Phase 26 26-07 artifact) needs the matching update.
4. **`Sync` NavItem SF-symbol substitute.** Whatever the Phase 26 icon library shipped (`lucide-react` filtered subset, or hand-curated SVGs). Planner picks in 27-01 — `Refresh` or `RefreshCw` in lucide should be close enough to `arrow.triangle.2.circlepath`.
5. **`MachineTomlPreview` line-diff algorithm.** Recommendation: `similar::TextDiff::from_lines` Myers diff. Planner picks in 27-03.

**Additional questions surfaced by this research (NOT in the UI-SPEC's five):**

6. **Should `GitCloneProgress` carry a `stage: SyncStage` field?** Pitfall 4. Recommendation: NO — leave the sink-side fold in place; verify the event-order test in 27-01.
7. **Toast implementation strategy.** Pitfall 2. Recommendation: hand-rolled `role="status"` live region, 5s setTimeout.
8. **`PreviewPopover` content-slot refactor.** Pitfall 3. Sized at ~20 LOC; lands in 27-03 alongside `MachineTomlDiff`.
9. **`useSync` event-subscription discipline during sync run.** Pitfall 6. Recommendation: ignore `ManifestChanged`/`LockfileChanged` while `isRunning === true`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust + cargo | All Rust work | ✓ | 1.85+ (edition 2024) | — |
| `npm` | `bindings.ts` regeneration + UI build | ✓ (Phase 26 infrastructure) | — | — |
| `git` | Real sync runs (git source clone) | ✓ | — | — |
| Tauri 2.11 SDK | `tauri-desktop` builds | ✓ | 2.11 in workspace | — |
| `playwright` + `@axe-core/playwright` | a11y CI gate (Sync view scan) | ✓ | (existing) | — |
| `similar` crate | SYNC-03 TOML diff | ✗ (new dep) | 3.1.1 on crates.io | Hand-rolled LCS in `machine.rs` (small but error-prone — see Don't Hand-Roll). |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** `similar` (fallback is hand-rolling; researcher recommends taking the dep).

## Validation Architecture

Skipped — `workflow.nyquist_validation: false` in `.planning/config.json`.

## Security Domain

> `security_enforcement` is not explicitly disabled in config; default = enabled. Phase 27 surfaces:

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | n/a (single-user local app) |
| V3 Session Management | no | n/a |
| V4 Access Control | no | n/a |
| V5 Input Validation | yes | `tauri-specta` enforces typed IPC at the boundary; `SkillName` newtype rejects path separators (verified `discover.rs:39`); `DirectoryName` same; `MachineTomlPreview` body is read-only (no inbound user TOML). |
| V6 Cryptography | no | n/a (content hashes via existing `sha2` for manifest, not new) |

### Known Threat Patterns for Tauri 2 + React 19 on macOS

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Webview executes injected JS via IPC string payloads | Tampering | All cross-boundary types derive `specta::Type`; no `eval`-equivalent surface. New `start_sync` returns a structured `SyncOutcome`. |
| Malicious lockfile triggers TOML parse pathologies | DoS | `toml` crate is used by both CLI and GUI; no untrusted external inputs (the lockfile is local). |
| Watcher event flood causes UI thrashing | DoS | Debounced (Phase 26 200ms window); `useSync` further gates by `isRunning` state. |
| `cancel_sync` is invoked maliciously by JS to interrupt a user's sync | Repudiation | Sync is locally invoked + always recoverable; cancellation is by design always safe (SC#4 invariant). Threat-model neutral. |
| `apply_machine_toml` writes outside `~/.config/tome/machine.toml` | Tampering / EoP | Path is resolved server-side via `tome::default_machine_path()` (`crates/tome/src/machine.rs:237`); no path argument crosses the boundary. |
| `view source` reveals a path outside the user's library | Information Disclosure | Reuses Phase 26's `open_source_folder` command which resolves via `tome::actions::resolve_source_path` — already audited. |

**Phase 27 does NOT widen the Tauri capability surface.** No `fs:default`, no `shell:default`. Only existing `opener:default` + `clipboard-manager:allow-write-text` + `core:default`/`core:event:default` capabilities are used. The five new commands all route through `load_context()` + `TomeError::from` boundary — Phase 25's threat model (T-25-04-EoP) stays intact.

## Sources

### Primary (HIGH confidence)

- `crates/tome/src/progress.rs` — Read (full file). `ProgressEvent`, `SyncStage`, `ProgressSink`, `CancelToken`, `RecordingSink` definitions + tests.
- `crates/tome-desktop/src/sink.rs` — Read (full file). `TauriEventSink::emit` event routing.
- `crates/tome-desktop/src/commands.rs` — Read (full file). `load_context`, existing command boundary pattern, `.map_err(TomeError::from)` discipline.
- `crates/tome-desktop/src/error.rs` — Read (full file). `TomeError`, `ErrorCode`, sentinel-downcast classification.
- `crates/tome-desktop/src/lib.rs` — Read (full file). `make_builder` command + event registry.
- `crates/tome-desktop/src/watcher.rs` — Read (lines 1-120). `notify` watcher, four event types, debounce window.
- `crates/tome-desktop/src/menu.rs` — Read (lines 1-100). `MenuAction` enum, `⌘1..⌘3` accelerators.
- `crates/tome/src/update.rs` — Read (full file). `SkillChange`, `UpdateDiff`, `diff(old, new)`, current CLI `present_changes` flow.
- `crates/tome/src/lockfile.rs` — Read (lines 1-100). `Lockfile`, `LockEntry` shape.
- `crates/tome/src/machine.rs` — Read (lines 1-90, 160-280). `MachinePrefs`, atomic `save`, mutators.
- `crates/tome/src/manifest.rs` — Read (lines 1-200). `synced_at` field already present (D-16 plumbing target).
- `crates/tome/src/discover.rs` — Read (lines 1-220). `DiscoveredSkill`, `SkillName`, `SkillOrigin`.
- `crates/tome/src/list.rs` — Read (full file). `ListReport::collect`.
- `crates/tome/src/lib.rs::sync` — Read (lines 1490-2260). Pipeline orchestration, 6 stage emission sites, cancellation checks.
- `crates/tome/src/library.rs` — Read (lines 170-330). `consolidate_managed` / `consolidate_local`, `synced_at` stamp semantics.
- `crates/tome-desktop/ui/src/App.tsx` — Read. Current 3-view router.
- `crates/tome-desktop/ui/src/stores/router.ts` — Read. `View` type, `setView`, subscription.
- `crates/tome-desktop/ui/src/shell/Sidebar.tsx` — Read. ListBox nav, badge slot.
- `crates/tome-desktop/ui/src/components/SectionHeader.tsx` — Read.
- `crates/tome-desktop/ui/src/components/PreviewPopover.tsx` — Read. Confirms slot refactor is needed (Pitfall 3).
- `crates/tome-desktop/ui/src/components/FindingRow.tsx` — Read. `[ErrorCode] message` + disclosure pattern.
- `crates/tome-desktop/ui/src/hooks/useTauriEvent.ts` — Read. Subscription pattern.
- `crates/tome-desktop/ui/src/views/SkillsView.tsx` — Read (head). `<ListBox>` + `<ListBoxItem>` usage confirms no inline buttons today (Pitfall 1 evidence).
- `crates/tome-desktop/ui/src/lib/relativeTime.ts` — Read.
- `crates/tome-desktop/ui/package.json` — Read. Confirms `react-aria-components ^1.18.0`, no diff lib.
- `crates/tome-desktop/Cargo.toml` — Read (head). Confirms `tauri-specta =2.0.0-rc.25`, `specta =2.0.0-rc.25`, `notify 8.2`, plugins.
- `crates/tome-desktop/tests/a11y/axe.spec.ts` — Read. WCAG-AA pattern — Sync view scan extends this.
- `.planning/phases/27-sync-triage-ui/27-CONTEXT.md` — Read (full file).
- `.planning/phases/27-sync-triage-ui/27-UI-SPEC.md` — Read (full file).
- `.planning/REQUIREMENTS.md` — Read (relevant sections). SYNC-01..05 verbatim; NF-04/NF-05; VIEW-02.
- `.planning/ROADMAP.md` — Read (lines 320-360). Phase 27 detail block, 5-plan structure.
- `.planning/STATE.md` — Read (head). Project state confirmation.
- `.planning/phases/26-read-only-views-alpha-cut/deferred-items.md` — Read. VIEW-02 carryover acceptance criteria.

### Secondary (MEDIUM confidence — official docs not in primary read)

- [CITED: react-aria.adobe.com/ListBox] — "Interactive elements (e.g. buttons) within listbox items are not allowed. This will break keyboard and screen reader navigation."
- [CITED: react-aria.adobe.com/GridList] — Supports interactive children by design; sections via `GridListSection` + `GridListHeader` (currently α-tagged in 1.18 but documented).
- [CITED: react-aria.adobe.com/Toast] — `UNSTABLE_Toast` / `UNSTABLE_ToastRegion` / `UNSTABLE_ToastQueue` in v1.18; API not yet stable.
- [CITED: docs.rs/similar] — `TextDiff::from_lines(old, new).iter_all_changes()` yields `Change<&str>` with `ChangeTag::{Equal, Delete, Insert}`. MIT, by mitsuhiko (Armin Ronacher).
- [CITED: v2.tauri.app/develop/calling-rust/] — async commands + `tokio::task::spawn_blocking` for sync long-running work.

### Tertiary (LOW confidence — search-only, flagged for verification before commit)

- WebSearch confirmed `similar` 3.1.1 is current on crates.io; planner re-runs `cargo search similar --limit 1` at 27-03 plan time as belt-and-braces.
- WebSearch on `tauri-specta` async commands returned the official `specta-rs/tauri-specta` repo but no version-specific docs; the planner reads the upstream README / examples directory before committing 27-01.

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — every dep is either already in `Cargo.toml`/`package.json` or (in the case of `similar`) verified via `cargo search` and broadly used.
- Architecture: HIGH — read against the existing code; the 6-stage pipeline, the boundary classifier, the `TauriEventSink`, the watcher, the `PreviewPopover` shape, the menu/router/sidebar wiring are all known.
- Patterns (preview-then-confirm, plan/render/execute, structure-at-the-edge, cooperative cancellation): HIGH — all proven in Phase 26.
- Pitfalls: MEDIUM-HIGH — Pitfalls 1, 3, 5, 7 are mechanically verifiable. Pitfalls 2 (toast precedent), 4 (Reconcile vs Discover for GitClone routing), 6 (watcher self-fire mid-sync) are MEDIUM — they require either reading another file (which the planner should do during plan generation) or running a real sync to trigger.
- Open questions / IPC shape: MEDIUM — the planner picks; the UI contract is shape-(a)-leaning but works for both.

**Research date:** 2026-06-05.
**Valid until:** 2026-07-05 — Tauri/React Aria/specta release cadence is steady; recheck `cargo search similar` and `react-aria-components` ToastRegion stability before the phase actually commits.

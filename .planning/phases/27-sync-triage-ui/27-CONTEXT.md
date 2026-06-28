# Phase 27: Sync + triage UI - Context

**Gathered:** 2026-06-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace `tome sync`'s CLI flow (and `update.rs::present_changes`'s interactive triage prompt) with a visual flow rendering five surfaces:

- **SYNC-01** Per-stage progress with current-item indicator
- **SYNC-02** Triage panel over the lockfile diff with per-skill actions + bulk actions
- **SYNC-03** Previewable `machine.toml` diff before save (no silent writes)
- **SYNC-04** Cancellation that leaves library state consistent
- **SYNC-05** Per-stage failure summary with stage-resumable retry

**Highest-UX-risk phase of v1.0** — first cross-stage *mutating* pipeline rendered in the GUI; first command that streams typed `ProgressEvent`s through `TauriEventSink`; first capability lift over the CLI flow it replaces (today's `update.rs` only triages ADDED skills, this expands to all three buckets).

**Two Phase 26 carryovers explicitly close in this phase** (see `26-deferred-items.md`):
- **#1 VIEW-02 group-by section headers** — `SectionHeader` component exists but isn't wired into the virtualised list; closure happens for free since SYNC-02's three-section triage layout *requires* `SectionHeader` at two nesting levels. Build once, ship in both surfaces.
- **#2 VIEW-02 "Recent" sort** — needs `synced_at` exposed on `DiscoveredSkill` / `ListReport` / `bindings.ts`. Manifest already stores it with the correct "last material change" semantics (verified in `library.rs:176-201`); only plumbing remains. Logical home is the 27-01 manifest extension (the same plan that adds `item: Option<String>` to `ProgressEvent`).

Carried-forward locked decisions (do **NOT** relitigate):
- **D-GUI-04 / D-GUI-08** Frontend = React; structured types stay in Rust; GUI dispatches commands + renders results. No JS-side business logic.
- **D-GUI-07 / NF-05** App + CLI share `tome.lock` + `.tome-manifest.json`; the Phase 26 file watcher reloads on external change.
- **Phase 26 D-09 / NF-04** "Preview-then-confirm" via `PreviewPopover` is the established pattern for mutating actions. SYNC-03 inherits it.
- **Phase 26 D-11** Inline `[ErrorCode] message` + disclosure chain (FindingRow pattern) is the failure-rendering precedent. SYNC-05 inherits it.
- **Phase 26 D-13..D-16** Shell, design tokens, React Aria, TanStack Virtual, native macOS chrome, follow system light/dark. Phase 27 reuses the shell unchanged.
- **Phase 25 D-09..D-12** `ProgressSink` / typed `ProgressEvent` / `CancelToken` — Phase 27 *uses* them; one variant gains a new field (see D-07).
- **NF-04** is formally Phase 28's, but `PreviewPopover` extends NF-04 spirit to the SYNC-03 `machine.toml` write.

</domain>

<decisions>
## Implementation Decisions

### Sync entry point & spatial layout

- **D-01:** Sync is a **4th sidebar row** alongside Status / Skills / Health. **Middle column** = progress stepper + triage list (when changes pending). **Right column** = per-skill diff detail with the canonical action picker. Replaces the option of a transient modal — by making Sync a permanent section, **Phases 28–30 can follow the same pattern** (5th / 6th / 7th sidebar rows for Configuration / Mutating-ops / Backup) rather than each inventing a new spatial slot.
- **D-02:** **Trigger placement = three discoverable entry points**: toolbar `[⌘R Sync]` button, ⌘R keyboard shortcut (aligns with Phase 26 plan 26-07's a11y shortcut map), and Library menu → "Sync" item (NF-03 menu bar). All three auto-switch to the Sync section when fired.
- **D-03:** **Idle state of the Sync section** (no pending sync, no run in progress) = **last-sync summary + prominent "Run sync" CTA**. Summary shows last-sync timestamp, counts (new / changed / removed since last sync), collapsible recent-changes list. Empty state shown **only if never synced**. The idle view IS the post-sync summary — there is no separate "result" view.
- **D-04:** **While the pipeline runs, navigation is free**. Status / Skills / Health remain interactive. NF-05 already guarantees concurrent-write tolerance via the Phase 26 watcher. The Sync sidebar row shows a **working spinner**; clicking it returns to the in-progress view. **The Cancel button stays on the Sync section** (single canonical location — clicking the sidebar row is the only path back to it during a run).
- **D-05:** **Sidebar badge on the Sync row counts pending triage decisions** (8 new + 3 changed + 1 removed = `(12)`); clears to zero on Apply. Reuses the same badge primitive Phase 26 ships for the Health row.
- **D-06:** **Post-completion (success, no-op, or cancel) auto-returns to idle state** with a transient "Sync complete" / "Sync cancelled" toast. The just-completed run's outcome populates the idle "last sync" summary. No persistent "result" panel; no auto-dismiss timer; no result-view-third-state. Two views in the section: **idle** and **in-progress** (which includes terminal failure / cancellation rendering until dismissed — see D-17).

### Progress visualisation

- **D-07:** **Progress UI = 6-stage vertical stepper** (Reconcile / Discover / Consolidate / Distribute / Cleanup / Save). Completed stages = checkmark + duration. Active stage expanded with progress bar + current-item subtitle. Future stages = dim outline circle. **Mirrors the macOS Installer / Xcode build-phases idiom**; honest to the pipeline's actual structure; per-stage timing helps diagnose slow stages.
- **D-08:** **Domain-API change**: extend `ProgressEvent::SyncStageProgress { stage, current, total }` to `SyncStageProgress { stage, current, total, item: Option<String> }`, and mirror the new field in the boundary `SyncProgress` struct (`crates/tome-desktop/src/sink.rs`). Free-form per stage (Discover = directory name; Consolidate/Distribute = skill name; Cleanup = path being removed; Save = filename being written). **Stays a single event variant** — no sibling `SyncStageItem` (variant proliferation rejected). Requires `bindings.ts` regen + CI freshness gate.
- **D-09:** **`GitCloneProgress` and `BackupSnapshot` fold into the active stage's `item` subtitle** via `TauriEventSink` formatting. Git-clone → `item = "git: my-repo (4.2 MB)"` on Reconcile (no progress bar since `total` is unknown). BackupSnapshot → `item = "<message>"` on Save with zeroed counts. **Uniform stepper rendering**: one row per stage, one subtitle line, optional bar — no per-event-type special case in the React layer. Sink owns the byte-count formatting (`{:.1} MB`, etc.) so JS stays presentation-only.
- **D-10:** **Per-stage durations show on completed rows** (e.g., `0.3s`, `8.2s`). UI records wall-clock at `SyncStageStarted` / `SyncStageFinished` event arrival; delta shown on the completed row. **No domain-API change** — purely a JS-side affordance. Helps diagnose anomalously slow stages (e.g., "Distribute took 12s? probably hitting an APFS clone-fail loop").

### Triage panel & per-skill actions

- **D-11:** **Triage layout = three vertical sections** (`▼ NEW` / `▼ CHANGED` / `▼ REMOVED`), **grouped by source within each section** ("plugins (5)", "my-repo (3)", "unowned (1)"). The `SectionHeader` component is reused at **both nesting levels** (change-type + source-group) — VoiceOver reads as nested headings. **This closes Phase 26 carryover #1 by construction**: the section-header abstraction Phase 27 needs is the same one the Skills view's group-by-Source / group-by-Role rendering needs; build once, wire into both consumers.
- **D-12:** **Per-skill inline action**: the chip in the middle-column row (`[✓ keep]`) **toggles keep ⇄ disable on click**. The **right column has the canonical full picker** (`● keep / ○ disable here / ○ view source` radios) plus the diff metadata (old → new content_hash, source, sync timestamps). One-click for the common case; right column for the full picker. Mirrors Phase 26's "detail pane is where the truth lives" pattern without making every change require row selection.
- **D-13:** **Bulk actions live at two granularities**: per-section header (`[Disable all NEW]`) AND per-source-group header (`[Disable all new from plugins]`). **Only `NEW` carries both**: `CHANGED` bulk-disable is unusual enough to omit from alpha; `REMOVED` is implicit (the skill is going away regardless of the user's input). Power users with noisy single sources hit the source-group shortcut; broader bulk available too.
- **D-14:** **"View source" action for git-sourced skills = reveal cloned repo directory in Finder**. Reuses the existing `tome::actions::resolve_source_path` + the Phase 26 `open_source_folder` Tauri command (maps to `open -R` via `tauri-plugin-opener`). **Zero new IPC surface**; consistent with the Skills view's same action. (Opening upstream URLs in browser is deferred — see Deferred Ideas.)
- **D-15:** **Apply flow / SYNC-03 `machine.toml` diff preview = `PreviewPopover`** anchored to the `[Apply N decisions]` button. The popover shows the literal TOML diff with red-/green-highlighted lines, Confirm / Cancel buttons inline. **Reuses the Phase 26 D-09 component verbatim** — same `PreviewPopover` Doctor's Fix flow uses. Consistent NF-04 ergonomic; no new modal primitive; the user already knows this shape. The Rust side returns the proposed `machine.toml` text (current vs proposed) via a new dry-run-style command; the React side renders the diff (existing diff lib OR small hand-rolled line-diff since the input is structured).

### Phase 26 carryover #2 (synced_at plumbing)

- **D-16:** **"Recent" sort semantics = manifest's existing `synced_at` behavior**, NOT a new domain concept. Verified in `crates/tome/src/library.rs:176-201` (`consolidate_managed`) + `:300+` (`consolidate_local`): `record_in_manifest` (which calls `SkillEntry::new()` and stamps `now_iso8601()`) is **only called when content_hash differs OR managed-flag flips OR the skill is first-seen**. Unchanged-skill sync increments `result.unchanged` and returns early WITHOUT touching `synced_at`. **Phase 27 work = plumbing only**: extend `DiscoveredSkill` with `synced_at: Option<DateTime>`, surface in `ListReport`, regenerate `bindings.ts`, wire the Sort=Recent comparator. No domain-semantics change. Tiebreaker for skills sharing a timestamp = alphabetical name (per Phase 26 deferred-items.md acceptance criteria).

### Failure + cancellation

- **D-17:** **Cancel button is always visible** during the pipeline run, next to the stepper. Click → `CancelToken::cancel()` → domain bails at the next stage boundary (SC#4 invariant). **No confirm dialog** — SC#4's "consistent library state on cancel" guarantee means cancellation is always recoverable; a confirm dialog would add friction with zero protective value most of the time.
- **D-18:** **Stepper transforms in place on terminal state** (failure OR cancellation). Same component renders live progress and terminal state — no separate "outcome panel". Icons: `✓` succeeded, `!` failed (red), `⊘` cancelled (amber), `—` not run (gray). Failed stage row shows `[ErrorCode] message` chip + `▶ Show error chain` disclosure for the full `anyhow` chain (Phase 26 D-11 / FindingRow pattern). **The terminal-state stepper persists** until the user clicks `[Dismiss]` or `[Retry from <stage>]` — there is no auto-dismiss for failures (it's the one exception to D-06's auto-return-to-idle).
- **D-19:** **Retry strategy = domain-driven via a `retry_from: Option<SyncStage>` hint** on the failure return. The Rust side knows SC#5's safety rule ("re-running discover + consolidate is acceptable; rerunning distribute on a partial manifest is not"); the React side just renders one `[Retry from <stage>]` button. Distribute failure on partial manifest → `retry_from = Some(Reconcile)` (back up to drift detection). Consolidate failure → `Some(Discover)`. Stale-lock or otherwise non-recoverable → `None` (button hides; only `[Dismiss]` available). **Matches the "no JS-side business logic" constraint** — safety rules live in Rust.
- **D-20:** **Partial-failure rendering inside a successful stage** (SAFE-01 K-failures semantics — e.g., Distribute symlinks 8/10 skills, 2 fail, pipeline continues): the stage icon stays `✓` (it completed), an amber `[⚠ K issues]` badge surfaces the count, and the row **expands by default** to a per-operation `FindingRow`-style list (skill name + `[ErrorCode]` + disclosure chain per item). The terminal summary line reads "Sync complete with K issues" + a `[Retry failed items]` action (scope = re-run only the failed individual operations within their stage, not the whole pipeline). **Honors Phase 26 D-11 ("failures must never be silently swallowed")** without alarming the user about an otherwise-successful sync.

### Claude's Discretion

- **Stage label wording** (e.g., "Reconcile" vs "Checking for changes") — Claude picks plain-English labels per stage, keeping the typed `SyncStage` variant name as the internal identity. Tooltips/aria-labels match the canonical name.
- **Toast positioning + duration + dismissal** (D-06 "Sync complete" / "Sync cancelled") — follow Phase 26 toast conventions if any exist; otherwise standard macOS toast (top-right, ~3s, fade-out, dismissible). Failure does NOT use toast (D-18 stepper-persistence applies).
- **Sidebar working-spinner style** (D-04) — small system spinner inline with the row text; HIG-aligned.
- **TOML diff rendering inside `PreviewPopover`** (D-15) — exact font, line-numbering presence/absence, colour intensity. Defer to Phase 26 design tokens; line-level red/green is the floor.
- **Default expansion state** of triage section headers (NEW/CHANGED/REMOVED) — Claude picks: NEW expanded by default (most actionable); CHANGED + REMOVED collapsed (user expands if interested).
- **`[Retry failed items]` exact scope** (D-20) — retry the specific per-operation failures within their stage (not the full pipeline). Implementation detail: the failed-item list is the retry input; the domain processes each individually with the same `ProgressSink` / `TomeError` machinery.
- **Stage-duration display format** (D-10) — sub-second = `0.1s`, multi-second = `8.2s`, minute+ = `1m 14s`. Numbers right-aligned for readability.
- **Stepper layout responsiveness** — default vertical stack at full Sync-column width; no horizontal collapse needed (Phase 27 isn't embedded anywhere narrow).
- **`item: Option<String>` exact emission for git-clone fold-in** (D-09) — sink formats `"git: <directory> (<size>)"`; size formatting `{:.1} MiB / GiB` per the existing saturating cast.
- **Where the `retry_from` hint lives in the IPC type system** — extend `TomeError` with an optional retry hint field, or carry it on a wrapping `SyncOutcome` struct? Planner picks; default is `SyncOutcome { result: Result<(), TomeError>, retry_from: Option<SyncStage> }`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 27 scope + locked requirements
- `.planning/REQUIREMENTS.md` §"Sync + triage (SYNC)" — SYNC-01..05 full text; NF-04 (destructive ops confirm — applies to SYNC-03 in spirit); NF-05 (CLI+app concurrency safety).
- `.planning/ROADMAP.md` §"Phase 27: Sync + triage UI" — promoted detail block: goal, depends-on, 6 success criteria, 5 draft plans, Phase 26 carryover folding.
- `.planning/milestones/v1.0-ROADMAP.md` §"Phase 12: Sync + Triage UI" — milestone draft this phase was promoted from (local Phase 12 == global Phase 27); draft plan stubs 12-01..12-05.

### Phase 26 inheritance (the substrate Phase 27 builds on UI-side)
- `.planning/phases/26-read-only-views-alpha-cut/26-CONTEXT.md` — D-01 (3-col shell), D-09 (PreviewPopover preview-then-confirm), D-11 (inline `[ErrorCode]` + disclosure failure rendering), D-13..D-16 (HIG polish, React Aria + TanStack Virtual, CSS Modules + design tokens, native chrome). **All inherited unchanged.**
- `.planning/phases/26-read-only-views-alpha-cut/deferred-items.md` — VIEW-02 group-by section headers + "Recent" sort closure criteria (folded into D-11 / D-16 of this phase).
- `.planning/phases/26-read-only-views-alpha-cut/26-UI-SPEC.md` — Phase 26's UI contract; Phase 27 should match its visual conventions (button styles, badge styles, section header rendering, popover anchoring).

### Phase 25 substrate (the domain APIs Phase 27 uses + extends)
- `.planning/phases/25-rust-core-extraction-tauri-integration-spike/25-CONTEXT.md` — D-09/D-10 (`ProgressSink` + typed `ProgressEvent`), D-12 (`CancelToken`, no tokio), D-13..D-16 (`TomeError` boundary, `ErrorCode` enum, `DomainErrorKind` sentinels, anyhow-downcast classification).
- `.planning/research/v1.0-frontend-framework-decision.md` — React + the `Result<T, TomeError>` discriminated-union narrowing pattern used at every command boundary.

### Code being extended / reused
- `crates/tome/src/progress.rs` — `SyncStage` enum + `SyncStage::ALL` (with compile-time drift guards), `ProgressEvent` variants (D-08 adds a field to `SyncStageProgress`), `ProgressSink` trait, `CancelToken`. **D-08 lands here.**
- `crates/tome/src/update.rs` — the CLI's `update::diff` + `present_changes` flow being functionally replaced. `diff()` is reused (it's already structured: `UpdateDiff { changes: BTreeMap<SkillName, SkillChange> }`). `present_changes` is bypassed by GUI sync (which substitutes its own triage UI).
- `crates/tome/src/lockfile.rs` — `Lockfile` + `LockEntry` (`source_name`, `previous_source`, `content_hash`, `registry_id`, `version`, `git_commit_sha`). Diff source for SYNC-02.
- `crates/tome/src/machine.rs` — `MachinePrefs` + atomic temp+rename write path. SYNC-03 writes through this.
- `crates/tome/src/manifest.rs` — `SkillEntry`, `synced_at` field (D-16 plumbing target). `SkillEntry::new()` stamps `now_iso8601()` only on material change (verified in `library.rs`).
- `crates/tome/src/library.rs` — `consolidate_managed:176-201` + `consolidate_local:311+` — the code paths whose semantics confirm D-16's "Recent = last material change".
- `crates/tome/src/discover.rs` — `DiscoveredSkill` (D-16 extension target: add `synced_at: Option<DateTime>`).
- `crates/tome-desktop/src/progress`/`sink.rs` — `TauriEventSink` + `SyncProgress` event. D-08 extends `SyncProgress` mirror; D-09 owns the git-clone / backup-snapshot fold-in formatting.
- `crates/tome-desktop/src/commands.rs` — established command boundary pattern (`load_context()` + `.map_err(TomeError::from)`); new commands land here for SYNC-01 (`start_sync`), SYNC-02 (`get_lockfile_diff`), SYNC-03 (`preview_machine_toml` + `apply_machine_toml`), SYNC-04 (`cancel_sync`), SYNC-05 (`retry_sync`).
- `crates/tome-desktop/src/watcher.rs` — Phase 26's file watcher; `MachinePrefsChanged` event already fires for SYNC-03 writes, so the UI refresh path post-Apply is free.
- `crates/tome-desktop/src/lib.rs::make_builder` — single command/event registry source of truth; new commands + events register here, then `bindings.ts` regen + CI freshness gate.
- `crates/tome-desktop/ui/src/components/` — `SectionHeader` (D-11 enabler, closes Phase 26 carryover #1), `PreviewPopover` (D-15), `FindingRow` (D-18/D-20 inline failure pattern), `Badge`/`Pill` (D-20 amber `[⚠ K issues]` badge), `Button`/`PopupMenu` (the toolbar trigger + bulk-action dropdowns).
- `crates/tome-desktop/src/menu.rs` — NF-03 native macOS menu bar; D-02 adds a "Sync" item to the Library menu.

### Architecture maps
- `.planning/codebase/ARCHITECTURE.md` — layer map (CLI / Config / Discovery / Consolidation / Distribution / Metadata / Cleanup / Lint / Sync-orchestration). Grounds where SYNC-01's `start_sync` command lives (boundary calls `tome::sync` which orchestrates all six stages) and where the lockfile-diff command (`get_lockfile_diff`) belongs.
- `.planning/codebase/INTEGRATIONS.md` — file-system surfaces (`~/.tome/.tome-manifest.json`, `~/.tome/tome.lock`, `~/.config/tome/machine.toml`); SYNC-03 writes to the last.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`tome::sync(config, paths, options, sink, cancel)`** — already in place from Phase 25. Threads `&dyn ProgressSink` + `&CancelToken` end-to-end through all 6 stages. SYNC-01 wires `TauriEventSink::new(app.clone())` into the `sink` parameter; SYNC-04 wires a per-run `CancelToken` (cloned into the app state so a `cancel_sync` command can flip it).
- **`tome::update::diff(old, new)` → `UpdateDiff { changes: BTreeMap<SkillName, SkillChange> }`** — already produces structured per-skill change records (`Added(LockEntry)` / `Changed { old, new }` / `Removed(LockEntry)`). SYNC-02 surfaces this directly via a new command (no new diff logic, just a specta-derived projection for the boundary).
- **`PreviewPopover`** (`crates/tome-desktop/ui/src/components/PreviewPopover.tsx`) — Phase 26 D-09 component, anchored-to-button popover with Confirm/Cancel slots. **Drop-in for SYNC-03's machine.toml diff preview (D-15)** — same component, different content.
- **`FindingRow`** (`crates/tome-desktop/ui/src/components/FindingRow.tsx`) — Phase 26 D-11 component. Inline `[ErrorCode] message` + `▶ Show error chain` disclosure pattern. **Drop-in for D-18 stepper failure rendering and D-20 partial-failure aggregation.**
- **`SectionHeader`** (`crates/tome-desktop/ui/src/components/SectionHeader.tsx`) — exists but **unwired into virtualised list** today (Phase 26 carryover #1). D-11 wires it into both the triage panel (two nesting levels) AND back-fills the Skills view's group-by-Source / group-by-Role rendering. Build wiring once, ship in both.
- **`tome::actions::set_skill_disabled`** + **`open_source_folder`** + **`resolve_source_path`** — Phase 26 actions module. D-14's "view source" is `open_source_folder` verbatim; SYNC-02's per-skill "disable on this machine" is `set_skill_disabled` via the same atomic temp+rename code path the TUI uses.
- **`MachinePrefsChanged` watcher event** — Phase 26 plan 26-06. Already fires for own-process `machine.toml` writes. SYNC-03 → Apply → watcher fires → idle-state last-sync summary refreshes for free. No manual UI invalidation needed.

### Established Patterns

- **`commands.rs::load_context()`** — every new SYNC-* command starts here. Resolves the real `Config` + `TomePaths` the same way the flag-free CLI does; ensures GUI observes identical state.
- **`.map_err(TomeError::from)`** at every command edge — Phase 25 D-13 boundary classification. Pipeline failures (per SYNC-05) cross as `TomeError { code: ErrorCode, message, context }`; the React side pattern-matches on `code` for rendering, never on `message`. **D-19's `retry_from` hint extends this pattern** — wrap the command return in `SyncOutcome { result, retry_from }` rather than overloading `TomeError`.
- **Single `make_builder()` registry** (`crates/tome-desktop/src/lib.rs`) — every new command + event registered here, `gen-bindings` bin regenerates `bindings.ts`, CI `git diff --exit-code` enforces freshness. **Five new commands + likely 1–2 new events** (e.g., `SyncStarted` / `SyncFinished` for the toast/auto-return) flow through this single source of truth.
- **Plan/render/execute pattern** — established in Phase 14 (`remove`/`reassign`/`relocate`/`eject`) and used by Phase 26's doctor repair flow. SYNC-03 fits naturally: `preview_machine_toml` returns the proposed text + diff; `apply_machine_toml` executes the write. The `PreviewPopover` is "render" between them.
- **"Structure at the edge" symmetry** (Phase 25 D-17) — typed `ProgressEvent` → typed `SyncProgress` → typed React pattern-match. D-08 maintains this: the new `item: Option<String>` field is **added** to the existing variant, not stringified or formatted in Rust before serialization. Front-end gets the structured value.
- **"No JS-side business logic"** — D-19's `retry_from` hint and D-16's "no domain-semantics change for `synced_at`" both honor this. The React side is presentation-only; safety rules + semantics live in Rust.

### Integration Points

- **Five new Tauri commands** crossing the IPC boundary:
  1. `start_sync` — invokes `tome::sync` with `TauriEventSink` + a per-app `CancelToken`. Returns `SyncOutcome { result: Result<(), TomeError>, retry_from: Option<SyncStage>, partial_failures: Vec<PartialFailure> }`.
  2. `cancel_sync` — flips the active `CancelToken`. Idempotent (matches the token's design).
  3. `get_lockfile_diff` — returns `UpdateDiff`-shaped projection over the boundary (uses existing `update::diff`).
  4. `preview_machine_toml` — given the triage decisions, returns proposed `machine.toml` text + a structured diff.
  5. `apply_machine_toml` — commits the triage decisions through `MachinePrefs` + atomic temp+rename. `MachinePrefsChanged` watcher event fires.
  6. (Plus possibly `retry_sync_from { stage }` — or fold into `start_sync(from: Option<SyncStage>)`.)
- **One domain change** (`item: Option<String>` on `SyncStageProgress`) ripples through: domain emission sites in `sync()` / `consolidate()` / `distribute()` / `cleanup()` / `save()` / `reconcile()`; `RecordingSink` tests update; `TauriEventSink::emit` passes through; `bindings.ts` regenerates.
- **One discovery change** (`synced_at: Option<DateTime>` on `DiscoveredSkill`) ripples through: discovery reads it from manifest; `list::collect` exposes it on `ListReport`; `bindings.ts` regenerates; Skills view's "Recent" sort comparator finally works.
- **Watcher subscriptions** — Phase 27 listens to `SyncProgress` events (new from D-08), and inherits `MachinePrefsChanged` / `LockfileChanged` / `ManifestChanged` from Phase 26 for post-Apply refresh.

</code_context>

<specifics>
## Specific Ideas

- **macOS Installer + Xcode build phases** are the spatial reference for the stepper (D-07). The vertical stack, per-stage check/duration/spinner, and the "expand the active row" affordance all come from there.
- **Mail.app "messages didn't send" pattern** as the reference for partial-failure aggregation (D-20) — the stage succeeded overall, but K issues warrant their own expandable list with per-item disclosure.
- **The PreviewPopover is the unifier** — same component used by Doctor's Fix (Phase 26 D-09) and SYNC-03's Apply (D-15). Consistency is the asset; users who learn the pattern in one place use it in the other.
- **"The sidebar is the contract"** — adding Sync as a 4th sidebar row commits Phase 28 (Configuration), 29 (Mutating ops), 30 (Backup) to the same sidebar-expansion pattern. The sidebar carrying 6+ sections by v1.0 is a feature, not bloat — it's the spatial promise the user makes once and re-uses everywhere.

</specifics>

<deferred>
## Deferred Ideas

- **Opening upstream URLs in browser for git-sourced skills** — D-14 ships "reveal in Finder" only. A second action ("Open on GitHub" / "Open upstream") could land in a polish pass or in Phase 28+ (where directory editing already touches the git URL). Captured here so it isn't lost.
- **Bulk "Retry all failed items"** (D-20) — alpha ships `[Retry failed items]` as a single-button retry for the per-stage K-failure list; finer-grained "select which items to retry" can be added if real-world failures cluster in ways that make selective retry useful.
- **`CHANGED` bulk-disable** — explicitly omitted from D-13 alpha. Bulk-disabling already-installed-and-now-changed skills is unusual; if a real workflow surfaces it, add later.
- **Sync activity log / sync history view** — Phase 27 ships "last sync" only (most-recent run). A history list of past runs (timestamps, durations, what changed) is its own surface — not in scope.
- **Diff rendering for CHANGED skill content** — D-12 right column shows old → new content_hash + metadata; rendering the actual SKILL.md diff between old and new versions is a richer surface (probably belongs with the v2 SKILL.md editor / GUI-EDIT-01).
- **Real-time auto-sync on watcher events** — explicitly out-of-scope per v1.0 REQUIREMENTS.md (deferred to GUI-WATCH-01). The watcher detects external CLI sync (post-completion refresh); it does NOT trigger new sync runs from inside the GUI.
- **STATE.md staleness fix** — `.planning/STATE.md` currently reads `status: milestone_complete` because Phase 26 was the alpha cut, not because the whole v1.0 milestone is done. Should be corrected at the start of Phase 27 execution (one-line edit; separate commit).
- **CLAUDE.md "Current State" header staleness** — still reads `v0.9.0 (shipped 2026-04-29)`; flagged in Phase 26's deferred-items.md, still pending. Non-blocking for Phase 27 planning.
- **Interim `v0.17.0` release of #542 `SkillOwnership` migration + Phase 25 `lib.rs` refactor** — flagged in Phase 26 CONTEXT.md as optional pre-v1.0 release. Non-blocking; user decides separately.

</deferred>

---

*Phase: 27-sync-triage-ui*
*Context gathered: 2026-06-02*

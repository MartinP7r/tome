---
phase: 27-sync-triage-ui
plan: 01a
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/progress.rs
  - crates/tome/src/discover.rs
  - crates/tome/src/list.rs
  - crates/tome/src/lib.rs
  - crates/tome-desktop/src/sink.rs
autonomous: true
requirements:
  - SYNC-01
tags:
  - rust
  - progress
  - manifest
  - sink
  - discover

must_haves:
  truths:
    - "ProgressEvent::SyncStageProgress carries a new item: Option<String> field (D-08); every emission site in tome::sync passes a value (Discover = directory name, Consolidate/Distribute = skill name, Cleanup = path, Save = filename, Reconcile = None)."
    - "SyncProgress mirror struct in crates/tome-desktop/src/sink.rs gains item: Option<String>; D-09 fold-in is implemented sink-side (GitCloneProgress → Reconcile + Some('git: <dir> (<size>)'); BackupSnapshot → Save + Some(message))."
    - "DiscoveredSkill carries synced_at: Option<String> ISO-8601 sourced from the manifest at discover-call boundary; ListReport surfaces the field for downstream consumers (closes Phase 26 VIEW-02 carryover #2 plumbing)."
    - "RecordingSink event-order test asserts SyncStageStarted{Reconcile} precedes the first GitCloneProgress (Pitfall 4 / Assumption A4 anchor; confirms the sink-side fold-in routing of git-clone events into Reconcile is correct)."
    - "No CLI regression: existing tome integration tests under crates/tome/tests/cli*.rs continue to pass (additive Option<…> fields only)."
  artifacts:
    - path: "crates/tome/src/progress.rs"
      provides: "ProgressEvent::SyncStageProgress.item field (D-08) + RecordingSink tests"
      contains: "item: Option<String>"
    - path: "crates/tome/src/discover.rs"
      provides: "DiscoveredSkill.synced_at field (D-16 plumbing)"
      contains: "synced_at: Option<String>"
    - path: "crates/tome/src/list.rs"
      provides: "ListReport entries surface synced_at"
      contains: "synced_at"
    - path: "crates/tome-desktop/src/sink.rs"
      provides: "SyncProgress mirror gains item; D-09 fold-in for GitCloneProgress + BackupSnapshot"
      contains: "item: Option<String>"
    - path: "crates/tome/src/lib.rs"
      provides: "Six emission sites in tome::sync updated to pass item per stage"
      contains: "SyncStageProgress"
  key_links:
    - from: "crates/tome/src/lib.rs::sync"
      to: "crates/tome/src/progress.rs::ProgressEvent::SyncStageProgress"
      via: "Each of the six stages constructs SyncStageProgress { stage, current, total, item }"
      pattern: "SyncStageProgress \\{ .*item.*\\}"
    - from: "crates/tome-desktop/src/sink.rs::TauriEventSink::emit"
      to: "SyncProgress payload"
      via: "GitCloneProgress arm sets stage=Reconcile + item=Some(format!(\"git: {dir} ({size})\")); BackupSnapshot arm sets stage=Save + item=Some(message)"
      pattern: "format!\\(\"git:"
    - from: "crates/tome/src/lib.rs::sync (post-discover_all)"
      to: "DiscoveredSkill.synced_at"
      via: "After discover_all returns and Manifest is in scope, populate synced_at for known skills before handing to ListReport::collect"
      pattern: "synced_at"
---

<objective>
Extend the Rust domain types that Phase 27 requires (D-08 `item: Option<String>` on `ProgressEvent::SyncStageProgress`; D-16 `synced_at: Option<String>` on `DiscoveredSkill`); implement the D-09 sink-side fold-in for `GitCloneProgress` + `BackupSnapshot` in `TauriEventSink`; pin Pitfall 4 / Assumption A4 with a `RecordingSink` event-order test that asserts `SyncStageStarted { Reconcile }` precedes the first `GitCloneProgress`. This plan is the FIRST HALF of SYNC-01 — Rust domain only, no Tauri boundary commands, no React wiring, no `bindings.ts` regen. Per the revision split, the Tauri boundary (`start_sync` + `cancel_sync` + `MenuAction::JumpSync`) and the React skeleton (`SyncView` + `useSync` + Sidebar Sync NavItem + `bindings.ts` regen + axe scan) ship in 27-01b.

Purpose: lands the typed domain substrate that 27-01b, 27-02, 27-03, 27-04, and 27-05 all build against. Splitting SYNC-01 in half keeps the file-touch count below the blocker threshold (5 files here, ~11 in 27-01b) and gives a clean Wave-1 boundary for the Tauri / React work in Wave-2.

Output: extended `ProgressEvent::SyncStageProgress`; extended `DiscoveredSkill`; extended `SyncProgress` mirror with D-09 fold-in; six updated emission sites in `tome::sync`; new RecordingSink tests (D-08 round-trip, D-09 fold-in, Pitfall 4 ordering, D-16 plumbing).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/27-sync-triage-ui/27-CONTEXT.md
@.planning/phases/27-sync-triage-ui/27-RESEARCH.md
@.planning/phases/27-sync-triage-ui/27-PATTERNS.md
@.planning/phases/26-read-only-views-alpha-cut/deferred-items.md
@crates/tome/src/progress.rs
@crates/tome/src/discover.rs
@crates/tome/src/list.rs
@crates/tome/src/manifest.rs
@crates/tome/src/library.rs
@crates/tome-desktop/src/sink.rs

<interfaces>
<!-- Pre-extracted contracts so the executor does not re-explore. -->

From crates/tome/src/progress.rs (existing; D-08 adds `item` to SyncStageProgress):
- `pub enum SyncStage { Reconcile, Discover, Consolidate, Distribute, Cleanup, Save }`
- `pub const ALL: [SyncStage; 6]` + `_ensure_sync_stage_exhaustive` const-fn + length-pin guard (POLISH-04 trio)
- `pub enum ProgressEvent { SyncStageStarted { stage }, SyncStageProgress { stage, current, total }, SyncStageFinished { stage }, GitCloneProgress { directory, received }, BackupSnapshot { message } }`
- `pub trait ProgressSink { fn emit(&self, event: ProgressEvent); }`
- `pub struct CancelToken(Arc<AtomicBool>)` with `new()`, `cancel()`, `is_cancelled()`, `Clone`
- `pub struct RecordingSink { events: Mutex<Vec<ProgressEvent>> }` for tests

From crates/tome/src/discover.rs (existing; D-16 adds `synced_at`):
- `pub struct DiscoveredSkill { pub name: SkillName, pub path: PathBuf, pub source_name: DirectoryName, pub origin: SkillOrigin, #[serde(skip)] pub frontmatter: Option<SkillFrontmatter> }`
- `pub fn discover_all(config: &Config, paths: &TomePaths) -> Result<(Vec<DiscoveredSkill>, Vec<String>)>`

From crates/tome/src/manifest.rs (read-only — synced_at already exists):
- `pub struct SkillEntry { ..., pub synced_at: String, ... }` (ISO-8601, stamped only on material change per library.rs:176-201)
- `pub fn load(path: &Path) -> Result<Manifest>`

From crates/tome-desktop/src/sink.rs (existing — D-08/D-09 extension target):
- `pub struct SyncProgress { stage: SyncStage, current: u64, total: u64 }` (mirror; needs `item: Option<String>`)
- `impl ProgressSink for TauriEventSink { fn emit(&self, event: ProgressEvent) { … match arms … } }`
- helper `saturate_usize` / `saturate_u64`
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Extend ProgressEvent::SyncStageProgress with item field; update six emission sites in tome::sync; add D-08 RecordingSink tests + Pitfall 4 ordering test</name>
  <files>
    crates/tome/src/progress.rs,
    crates/tome/src/lib.rs
  </files>
  <read_first>
    crates/tome/src/progress.rs (full file — extract enum ALL + RecordingSink shape + the existing mod tests block),
    crates/tome/src/lib.rs (lines 1490-2260 — locate the six SyncStageStarted/Progress/Finished emission sites + the resolve_git_directories call that triggers GitCloneProgress),
    .planning/phases/27-sync-triage-ui/27-RESEARCH.md §"Code Examples — Domain-side typed event emission" + §"Pitfall 4",
    .planning/phases/27-sync-triage-ui/27-PATTERNS.md §"crates/tome/src/progress.rs"
  </read_first>
  <behavior>
    - Test (D-08 round-trip): emitting `ProgressEvent::SyncStageProgress { stage: SyncStage::Discover, current: 5, total: 10, item: Some("axiom-build".into()) }` and reading it back via `RecordingSink::events()` returns the value verbatim including `item`.
    - Test (D-08 None case): emitting `ProgressEvent::SyncStageProgress { stage: SyncStage::Reconcile, current: 0, total: 0, item: None }` round-trips with `item: None`.
    - Test (Pitfall 4 ordering): emitting `ProgressEvent::SyncStageStarted { stage: SyncStage::Reconcile }` followed by `ProgressEvent::GitCloneProgress { directory: "my-repo".into(), received: 4_200_000 }` and inspecting `RecordingSink::events()` shows the Reconcile-started event at index 0 and the GitCloneProgress event at index 1. Confirms that `resolve_git_directories` emits `Reconcile` before any git-clone event so the sink-side fold-in routing to `SyncStage::Reconcile` is correct (Assumption A4 anchor).
    - Test (no CLI regression): `cargo test -p tome` passes — the only change to `ProgressEvent` is an additive field; if any snapshot test fails because `tome list --json` or similar output changed, accept via `cargo insta review`.
  </behavior>
  <action>
    1. In `crates/tome/src/progress.rs` extend `ProgressEvent::SyncStageProgress` to add `item: Option<String>` after `total`. Keep all existing derives intact (`#[cfg_attr(feature = "bindings", derive(specta::Type))]` on the enum, `serde::Serialize`).
    2. Update every existing emission site in `crates/tome/src/lib.rs::sync` that constructs `ProgressEvent::SyncStageProgress { stage, current, total }` to pass an additional `item`. Concrete per-stage assignment (locate by stage label):
       - Reconcile: `item: None` (git-clone fold-in via sink per D-09; no per-event subtitle from domain side).
       - Discover: `item: Some(directory_name.to_string())` where `directory_name` is the current `DirectoryName` being scanned in the discover loop.
       - Consolidate: `item: Some(skill_name.to_string())` for the current skill being consolidated.
       - Distribute: `item: Some(skill_name.to_string())` for the current skill being symlinked.
       - Cleanup: `item: Some(path.display().to_string())` for the path being removed.
       - Save: `item: Some(filename.to_string())` (e.g., `"tome.lock"`, `".tome-manifest.json"`, `"machine.toml"`) for each save sub-op.
    3. Update the `RecordingSink` test fixtures already in `progress.rs:269-308` to pass `item: None` (or `Some(...)`) — every existing test that constructed `SyncStageProgress` needs the field. Failing to add the field is a compile error which makes the change easy to find.
    4. Add the three new RecordingSink tests in the `mod tests` block per the behavior list (D-08 round-trip, D-08 None, Pitfall 4 ordering). The Pitfall 4 test does NOT run `tome::sync` — it just emits the two events into a `RecordingSink` directly and asserts the order. The real-sync-driven version of the ordering check lives in 27-04's `sync_cancel.rs` (where the pipeline is exercised end-to-end against a real fixture).
    5. Do NOT regenerate `bindings.ts` here — that ships in 27-01b after all events + the Tauri boundary commands are registered.
  </action>
  <verify>
    <automated>cargo test -p tome --lib progress::tests -- --nocapture &amp;&amp; cargo test -p tome &amp;&amp; cargo clippy -p tome --all-targets -- -D warnings</automated>
  </verify>
  <done>
    `ProgressEvent::SyncStageProgress` carries `item: Option<String>`; every emission site in `lib.rs::sync` passes a per-stage value; existing RecordingSink fixtures updated; three new tests pass (D-08 round-trip, D-08 None, Pitfall 4 ordering); `cargo test -p tome` clean (CLI regression-free); no clippy warnings.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Extend DiscoveredSkill with synced_at field; populate from Manifest in sync(); surface on ListReport</name>
  <files>
    crates/tome/src/discover.rs,
    crates/tome/src/list.rs,
    crates/tome/src/lib.rs
  </files>
  <read_first>
    crates/tome/src/discover.rs (lines 1-260 — DiscoveredSkill + SkillProvenance pattern + discover_all signature),
    crates/tome/src/list.rs (full file — ListReport::collect shape),
    crates/tome/src/library.rs (lines 170-330 — consolidate_managed + consolidate_local; confirm synced_at material-change semantics; do not modify),
    crates/tome/src/manifest.rs (lines 160-220 — SkillEntry.synced_at field already present, ISO-8601 string),
    crates/tome/src/lib.rs (locate where discover_all is called inside sync() and where the Manifest is loaded; the population point is the post-discover boundary where both are in scope),
    .planning/phases/27-sync-triage-ui/27-CONTEXT.md §"D-16" (Recent sort semantics; plumbing only, no domain-semantics change),
    .planning/phases/26-read-only-views-alpha-cut/deferred-items.md (VIEW-02 acceptance criteria)
  </read_first>
  <behavior>
    - Test (synced_at populated from manifest): given a TempDir fixture with a manifest containing `SkillEntry { name: "foo", synced_at: "2026-06-05T10:00:00Z", ... }` and a discover scan that finds `foo`, the returned `DiscoveredSkill { name: "foo", synced_at: Some("2026-06-05T10:00:00Z"), .. }`. Skills with no manifest entry have `synced_at: None`.
    - Test (ListReport surfaces synced_at): `ListReport::collect` returns per-skill records that include `synced_at`; round-trip through `serde_json::to_string` includes the field (assert the JSON contains `"synced_at":"2026-06-05T10:00:00Z"` for the populated entry and `"synced_at":null` for the missing-manifest case).
    - Test (no CLI regression): the existing `tome list` CLI integration tests pass; if `tome list --json` output now includes the field, accept the snapshot via `cargo insta review`. The CLI human-readable output (tabled) is unchanged because no column for synced_at is added.
  </behavior>
  <action>
    1. In `crates/tome/src/discover.rs`: add `pub synced_at: Option<String>` to `DiscoveredSkill`, placed alongside the other optional metadata fields. Mirror the existing serde derives. Initialize as `None` at the original discover sites (the manifest is loaded inside `sync()`, not inside `discover_all`; populating from inside `discover_all` would require threading the manifest through, which is the wrong layering).
    2. In `crates/tome/src/lib.rs::sync`: immediately after `discover_all` returns (and the Manifest has been loaded earlier in the pipeline — verify the precise line and ordering), iterate the returned `Vec<DiscoveredSkill>` and populate `synced_at` for each skill that has a corresponding `SkillEntry` in the manifest by `skill.synced_at = manifest.lookup(&skill.name).map(|entry| entry.synced_at.clone())`. This is a single pass; the cost is negligible compared to discover itself.
    3. In `crates/tome/src/list.rs`: surface `synced_at` on `ListReport`'s per-skill records. If `ListReport::Entry` (or whatever the per-skill struct is called) already mirrors `DiscoveredSkill`, the change is just adding the field; mirror the existing serde derives. The Skills view comparator wiring is NOT in this plan — that lives in the new 27-02b plan (extracted per warning W7).
    4. Add a unit test in `discover.rs` mod tests verifying `synced_at` plumbing using a fixture manifest under a TempDir + a small fixture skill dir. Add a unit test in `list.rs` mod tests verifying that `ListReport::collect` round-trips `synced_at` through serde_json.
  </action>
  <verify>
    <automated>cargo test -p tome --lib discover::tests &amp;&amp; cargo test -p tome --lib list::tests &amp;&amp; cargo test -p tome &amp;&amp; cargo clippy -p tome --all-targets -- -D warnings</automated>
  </verify>
  <done>
    `DiscoveredSkill.synced_at: Option<String>` exists; populated for known skills inside `sync()` before the result reaches `ListReport::collect`; `ListReport` per-skill records surface the field; unit tests for both modules pass; no CLI regression.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Extend SyncProgress mirror with item field; implement D-09 sink-side fold-in for GitCloneProgress + BackupSnapshot in TauriEventSink</name>
  <files>
    crates/tome-desktop/src/sink.rs
  </files>
  <read_first>
    crates/tome-desktop/src/sink.rs (full file — TauriEventSink::emit match arms + SyncProgress mirror struct + existing saturate_u64/saturate_usize helpers),
    crates/tome/src/progress.rs (post-Task-1 — confirm ProgressEvent::SyncStageProgress.item is present in the public API),
    .planning/phases/27-sync-triage-ui/27-RESEARCH.md §"Example: TauriEventSink folding — D-09 implementation site" (lines 447-477),
    .planning/phases/27-sync-triage-ui/27-PATTERNS.md §"crates/tome-desktop/src/sink.rs"
  </read_first>
  <behavior>
    - Test (D-09 GitCloneProgress fold-in): a `TauriEventSink`-equivalent unit test using the helper functions (or a `MockTauriEmitter` if Phase 25/26 ships one — check existing tests in `sink.rs`) receives `ProgressEvent::GitCloneProgress { directory: "my-repo".into(), received: 4_200_000 }` and produces a `SyncProgress { stage: SyncStage::Reconcile, current: 4_200_000, total: 0, item: Some(s) }` where `s` starts with `"git: my-repo"` and contains a byte-size suffix per the existing saturating-cast format (MiB / GiB, 1 decimal).
    - Test (D-09 BackupSnapshot fold-in): receives `ProgressEvent::BackupSnapshot { message: "writing snapshot".into() }` and produces `SyncProgress { stage: SyncStage::Save, current: 0, total: 0, item: Some("writing snapshot".into()) }`.
    - Test (D-08 pass-through): receives `ProgressEvent::SyncStageProgress { stage: Discover, current: 5, total: 10, item: Some("foo".into()) }` and produces `SyncProgress { stage: Discover, current: 5, total: 10, item: Some("foo".into()) }`.
    - Test (started/finished item=None): `SyncStageStarted` and `SyncStageFinished` produce `SyncProgress { stage, current: 0, total: 0, item: None }`.
  </behavior>
  <action>
    1. Extend the `SyncProgress` mirror struct in `crates/tome-desktop/src/sink.rs` with `pub item: Option<String>` after `total`. Keep existing derives (`serde::Serialize`, `specta::Type`, `tauri_specta::Event` if present).
    2. Add a sink-private helper `fn format_bytes(received: u64) -> String` that renders the byte count per the existing project format (verify by reading the current `sink.rs` — the helper may already exist for git-clone CLI rendering elsewhere; if so, lift it or call it directly). The format MUST match what CLI users already see for git-clone progress (e.g., `"4.0 MiB"`, `"1.2 GiB"`). 1 decimal place.
    3. Update every match arm of `impl ProgressSink for TauriEventSink::emit` to set `item`:
       - `ProgressEvent::SyncStageProgress { stage, current, total, item }` → `SyncProgress { stage, current: saturate_usize(current), total: saturate_usize(total), item }` (pass through).
       - `ProgressEvent::GitCloneProgress { directory, received }` → `SyncProgress { stage: SyncStage::Reconcile, current: saturate_u64(received), total: 0, item: Some(format!("git: {directory} ({})", format_bytes(received))) }`.
       - `ProgressEvent::BackupSnapshot { message }` → `SyncProgress { stage: SyncStage::Save, current: 0, total: 0, item: Some(message) }`.
       - `ProgressEvent::SyncStageStarted { stage }` → `SyncProgress { stage, current: 0, total: 0, item: None }`.
       - `ProgressEvent::SyncStageFinished { stage }` → `SyncProgress { stage, current: 0, total: 0, item: None }`.
    4. Add unit tests in `sink.rs` mod tests covering the four behaviors above. If no `MockTauriEmitter` exists in Phase 25/26 code, the tests can assert by constructing the expected `SyncProgress` value directly from the helper functions and comparing — the goal is to lock the fold-in formatting against regression.
    5. Do NOT regenerate `bindings.ts` here — 27-01b owns the regen after all new commands + events are registered.
  </action>
  <verify>
    <automated>cargo test -p tome-desktop --lib sink::tests &amp;&amp; cargo build -p tome-desktop --features bindings &amp;&amp; cargo clippy -p tome-desktop --features bindings -- -D warnings</automated>
  </verify>
  <done>
    `SyncProgress` mirror gains `item: Option<String>`; `TauriEventSink::emit` passes through `item` for `SyncStageProgress` and folds in `GitCloneProgress` + `BackupSnapshot` per D-09; four unit tests pass; `cargo build -p tome-desktop --features bindings` is clean; clippy clean; bindings.ts NOT regenerated yet.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Domain `tome::sync` → ProgressSink | typed `ProgressEvent` records cross to whatever sink is wired (CLI's `NullSink`, GUI's `TauriEventSink`) |
| `TauriEventSink::emit` → Tauri event channel | `SyncProgress` payloads will eventually flow to the webview (the event channel is wired in 27-01b) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-27-01a-01 | Tampering | New `item: Option<String>` field carries unbounded user-controlled string | accept | Source is the domain's own per-stage data (directory name, skill name, path, filename) — all newtype-validated upstream (`SkillName`, `DirectoryName`) or filesystem-derived paths owned by the user. No untrusted input. |
| T-27-01a-02 | Information Disclosure | Path leakage via `item` subtitle into eventual webview console | accept | Same user owns both sides; paths are local; no PII. D-09 sink-side formatting is the canonical sanitization point. |
| T-27-01a-03 | Tampering | `format_bytes` panics on pathological `u64` (e.g., near `u64::MAX`) | accept | `saturate_u64` clamps to a sane range; the byte-format helper uses safe arithmetic (no `unwrap` on division). |
| T-27-01a-SC | Tampering | npm/cargo new deps | accept | This plan adds ZERO new external packages. All work is additive to existing types. |
</threat_model>

<verification>
- `cargo test -p tome --lib progress::tests` — D-08 round-trip + Pitfall 4 ordering tests pass.
- `cargo test -p tome --lib discover::tests` and `cargo test -p tome --lib list::tests` — D-16 plumbing tests pass.
- `cargo test -p tome-desktop --lib sink::tests` — D-09 fold-in tests pass.
- `cargo test -p tome` — full CLI + domain test suite passes (additive field changes only; accept any `cargo insta review` requests from `tome list --json` snapshot drift).
- `cargo build -p tome --features bindings && cargo build -p tome-desktop --features bindings` — both compile clean.
- `cargo clippy -p tome --all-targets -- -D warnings && cargo clippy -p tome-desktop --features bindings -- -D warnings` — zero warnings.
- `bindings.ts` is NOT regenerated in this plan — 27-01b owns the regen.
</verification>

<success_criteria>
- D-08 substrate landed: every emission site in `tome::sync` passes a per-stage `item` value; `RecordingSink` round-trips the field; Pitfall 4 ordering asserted.
- D-09 sink-side fold-in implemented: `GitCloneProgress` → Reconcile + formatted byte-size subtitle; `BackupSnapshot` → Save + message subtitle.
- D-16 plumbing landed: `DiscoveredSkill.synced_at` populated from manifest before `ListReport::collect`; per-skill records surface the field.
- No CLI regression; all existing `crates/tome` tests pass.
- 27-01b can proceed in Wave 2 against the typed types this plan ships.
</success_criteria>

<output>
Create `.planning/phases/27-sync-triage-ui/27-01a-SUMMARY.md` when done.
</output>

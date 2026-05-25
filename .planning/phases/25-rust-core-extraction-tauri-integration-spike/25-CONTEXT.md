# Phase 25: Rust core extraction + Tauri integration spike - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Reshape `crates/tome` so its domain operations return structured types callable from any front-end, add `crates/tome-desktop` as a sibling Tauri 2 app crate, wire `specta` + `tauri-specta` bindings, add a progress-event channel and a stable `TomeError` boundary, and pick the frontend framework via a 3-way spike. Delivers `CORE-01..05`. **No production UI ships in this phase** — the spike apps are throwaway except the winner's scaffold.

Carried-forward locked decisions (from `v1.0-REQUIREMENTS.md` / PROJECT.md / STATE.md — do NOT relitigate):
- **D-GUI-01** Tauri 2 (not Electron + napi-rs)
- **D-GUI-02** new `crates/tome-desktop` workspace member; CLI stays in `crates/tome`
- **D-GUI-03** `specta` + `tauri-specta` for `bindings.ts`
- **D-GUI-06** macOS only for v1.0
- **D-GUI-07** app + CLI share `tome.lock` + `.tome-manifest.json` (no GUI-private state)
- **D-GUI-08** CLI's `lib.rs::run` decomposes into a presenter layer over the same domain calls
- **#542 absorption** — Owned/Unowned enum migration folded into CORE-01

</domain>

<decisions>
## Implementation Decisions

### Framework spike (D-GUI-04)
- **D-01:** Build the spike in **all three** candidates — React, Solid, Svelte. The framework choice is irreversible from Phase 26 onward, so the comparison is built, not assumed.
- **D-02:** Each spike renders **only the real `StatusReport`** (`tome status` data) as a single-page dashboard — no list virtualization, no interactions. That is exactly Phase 25 success-criterion #2; richer surfaces (list, actions) belong to Phase 26.
- **D-03:** Compare across **all four criteria**: (1) `bindings.ts` ergonomics in each framework's idioms, (2) production bundle size + cold-start TTI, (3) dev-loop speed (HMR latency, error quality, type-check speed), (4) ecosystem fit for v1.0 reqs (virtualized lists for VIEW-02/NF-01, keyboard-accessible widgets for NF-02, macOS-HIG-aligned components for NF-03).
- **D-04:** Record the decision as a **scoring table (1–5 per criterion) + a short ADR** at `.planning/research/v1.0-frontend-framework-decision.md`, capturing rationale and what would invalidate the choice. Also update **D-GUI-04** in `v1.0-REQUIREMENTS.md` with the chosen framework. Two losing spikes are deleted after the decision.

### Structured types + specta gating (CORE-01, CORE-03)
- **D-05:** Structured types **stay in `crates/tome`**, co-located in their producing modules (`status.rs::StatusReport`, `remove.rs::RemovePlan`, `manifest.rs::SkillEntry`, …). `crates/tome-desktop` depends on `crates/tome` as a path dep and imports them directly. No `tome-core` crate; no wrapper/mirror types.
- **D-06:** `specta::Type` is gated behind an optional **`bindings` cargo feature** on `crates/tome`: `[features] bindings = ["dep:specta", ...]`, and cross-boundary types use `#[cfg_attr(feature = "bindings", derive(specta::Type))]`. The CLI builds with default features (no specta cost); `tome-desktop` enables `tome/bindings`.
- **D-07:** `bindings.ts` is generated **at `tome-desktop` compile time** via a `build.rs` calling `tauri_specta::ts::export(...)`, written to `crates/tome-desktop/ui/src/bindings.ts` and **committed**. CI freshness gate: `cargo build -p tome-desktop` then `git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts`.

### Owned/Unowned migration (#542, part of CORE-01)
- **D-08:** Replace `SkillEntry::source_name: Option<DirectoryName>` with `provenance: SkillProvenance` where `enum SkillProvenance { Owned { source: DirectoryName }, Unowned { last_owner: Option<DirectoryName> } }`. Lifts the existing `last_directory_name` field into the `Unowned` variant where it belongs. Specta-derives as a TS discriminated union and forces exhaustive handling in the GUI. **Manifest JSON migration strategy for this field-shape change is a planning detail** — must preserve the existing `#[serde(default)]` round-trip tolerance described in LIB-01..05 / D-10.

### Progress events (CORE-04)
- **D-09:** Long-running domain ops take an injected **`ProgressSink` trait**: `trait ProgressSink: Send + Sync { fn emit(&self, event: ProgressEvent); }`, passed as `sink: &dyn ProgressSink`. The domain **stays synchronous** — no tokio runtime dep added to `crates/tome`. CLI impl wraps `indicatif`; GUI impl wraps `tauri::AppHandle::emit`.
- **D-10:** `ProgressEvent` is a **per-op typed enum** (e.g. `SyncStageStarted/Progress/Finished { stage: SyncStage, … }`, `GitCloneProgress`, `BackupSnapshot`) — semantically rich so the GUI pattern-matches rather than string-matches.
- **D-11:** The trait + `ProgressEvent` enum live in a new **`crates/tome/src/progress.rs`**. The CLI `IndicatifSink` lives in `lib.rs` next to the `cmd_*` presenters; a `NullSink` is provided for tests + `--quiet`. `tome-desktop` ships its own `TauriEventSink`.
- **D-12:** Cancellation (needed by SYNC-04 in Phase 27) is threaded **now** as a per-op `&CancellationToken` arg alongside `sink`. Domain checks `cancel.is_cancelled()` at stage boundaries. CLI passes a never-tripped token; the real cancel behavior + cancel button land in Phase 27, but the API shape is fixed here so later phases don't re-sign every domain fn.

### TomeError boundary (CORE-05)
- **D-13:** **Classify at the IPC boundary.** The domain keeps `anyhow::Result` internally (zero refactor; no CLI regression). `tome-desktop` (and a thin CLI exit-code mapper) wrap results in `TomeError` at the Tauri command edge. This mirrors D-09's "structure at the edge" pattern — one mental model for how the GUI sees the core.
- **D-14:** Classification uses **typed sentinel errors via downcast**, not message string-matching. A small `thiserror` `enum DomainErrorKind` is attached at the specific failure sites the GUI must distinguish (via anyhow `.context()`); the boundary does `err.downcast_ref::<DomainErrorKind>()` to pick the code; anything unmatched → `Internal`. Only GUI-relevant sites get sentinels.
- **D-15:** `ErrorCode` is **coarse — ~6 categories**: `enum ErrorCode { Validation, NotFound, Permission, Conflict, Git, Io, Internal }`, mapping to broad GUI error-UI families. The enum is set up to grow additively (additive variants are non-breaking for the GUI's default arm); finer codes are added per-phase only when a phase's UI needs them.
- **D-16:** The error payload is `struct TomeError { code: ErrorCode, message: String, context: Vec<String> }`, where `context` is the flattened anyhow `.context()` chain (the `error: a: b: c` shown on stderr today). GUI surfaces `message` prominently and `context` in a details/disclosure view — no debugging info lost vs the CLI.

### Cross-cutting design principle
- **D-17:** **Symmetry "structure at the edge":** both progress (D-09/D-11) and errors (D-13) keep the domain ergonomic (sync, `anyhow`) and put GUI-facing structure at the boundary (`TauriEventSink`, `TomeError` wrapper). Plans should preserve this symmetry — it is one mental model, not two.

### Claude's Discretion
- `lib.rs::run` decomposition mechanics (cmd_* presenters inline vs a `presenters/` module) — deliberately left to research/planning (see Deferred Ideas). The planner proposes a shape grounded in the current 3,101-line `lib.rs`.
- Exact `SyncStage` enum members, `ProgressEvent` variant fields, and which call sites get `DomainErrorKind` sentinels — implementation detail for the planner, constrained by the decisions above.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone scope + decisions
- `.planning/milestones/v1.0-REQUIREMENTS.md` — CORE-01..05 full text; D-GUI-01..09 key decisions; constraints (no CLI regression, no JS-side business logic, strict Tauri 2.x, hardened runtime).
- `.planning/milestones/v1.0-ROADMAP.md` §"Phase 10" — Phase 25's success criteria (the roadmap's Phase 10 == this Phase 25), plan stubs 10-01..06, and the framework-spike framing.
- `.planning/ROADMAP.md` §"Phase 25" + §"Open questions Q1–Q7" — ratified milestone phase list and the open questions (Q1 framework = this phase's spike; Q2 Tauri pin policy).

### Background / cost model
- `.planning/research/v1.0-rust-to-typescript-migration-inventory.md` — exhaustive module-by-module surface area + dependency map; useful for sizing the type-extraction work and identifying which modules own which structured types.
- `.planning/STATE.md` §"v1.0 design context" — the binding notes (no CLI regression; library-canonical types are the contract; #542 absorption; specta/no-hand-rolled-TS; macOS-only).

### Code being reshaped
- `crates/tome/src/lib.rs` — the 3,101-line `run()` dispatcher + `cmd_*` presenters + `sync` pipeline orchestration being decomposed (CORE-01, D-GUI-08).
- `crates/tome/src/manifest.rs` — `SkillEntry` with the `source_name: Option<DirectoryName>` Owned/Unowned encoding being lifted to `SkillProvenance` (D-08, #542).
- `crates/tome/src/status.rs` — existing `StatusReport` (already structured; gains `specta` derive).
- `crates/tome/src/remove.rs` — existing `RemovePlan` (plan/render/execute pattern; reference for other plan types).
- `.planning/codebase/ARCHITECTURE.md` — layer map (CLI / config / discover / consolidate / distribute / metadata / cleanup); grounds where each structured type and the `ProgressSink` belong.

### Decision artifact to be produced this phase
- `.planning/research/v1.0-frontend-framework-decision.md` — (NEW, D-04) scoring table + ADR for the framework choice.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `StatusReport` (`status.rs`) and `RemovePlan` (`remove.rs`) already exist as structured types — CORE-01 is largely *adding specta derives + naming consistency*, not inventing types from scratch.
- The plan/render/execute pattern (`add`, `remove`, `reassign`, `relocate`, `eject`) already separates "compute a plan" from "render it" from "execute it" — the GUI consumes the plan struct directly; the CLI keeps the render step.
- `indicatif::ProgressBar`/`MultiProgress` usage in `library.rs`/`distribute.rs` is the presentation-in-domain smell that `ProgressSink` replaces; the `IndicatifSink` re-homes that existing code.
- `Config::save_checked` (expand → validate → round-trip → write) is the validation gate later phases (CFG-05) route through — unchanged here but its types cross the boundary.

### Established Patterns
- Newtype identifiers (`SkillName`, `DirectoryName`, `ContentHash`) with `#[serde(transparent)]` + validating `Deserialize` — these become TS branded-ish types via specta; verify specta handles transparent newtypes cleanly during the spike.
- `#[serde(default)]` tolerance on manifest/lockfile fields is the migration mechanism — the `SkillProvenance` change (D-08) must preserve round-trip tolerance.
- `anyhow::Result` + `.context()` everywhere — the chain is the value `TomeError.context` preserves (D-16).

### Integration Points
- `crates/tome-desktop` → `crates/tome` path dep with `features = ["bindings"]` (D-06).
- Tauri command layer → domain fns: passes a `TauriEventSink` (D-11) + `CancellationToken` (D-12), and wraps the `anyhow::Result` into `TomeError` (D-13).
- `build.rs` in `tome-desktop` → `tauri_specta::ts::export` → committed `bindings.ts` → consumed by the chosen framework's UI (D-07).

</code_context>

<specifics>
## Specific Ideas

- The spike's StatusReport view should render **real data from the user's actual `tome_home`**, not fixtures — that's what makes the framework comparison honest (Phase 25 SC #2).
- Keep the "structure at the edge" symmetry visible in code organization: `progress.rs` (trait in core, sinks at edges) and the `TomeError` boundary wrapper should read as the same idea applied twice.

</specifics>

<deferred>
## Deferred Ideas

- **`lib.rs::run` decomposition mechanics** — whether `cmd_*` presenters stay inline in `lib.rs` or move to a `presenters/` module. Deliberately not locked (user chose to wrap up over exploring it); the planner should propose a shape grounded in the current code. Belongs in Phase 25 planning, not a future phase.
- **Tauri 2.x minor-version pin policy (Q2)** — not discussed. Default: pin a specific `2.M.N`, bump at milestone boundaries, unless the planner finds a reason otherwise.
- **CI matrix shape for `tome-desktop`** — does macOS CI build the `.app`? where does the bindings-freshness gate sit relative to the existing fmt/clippy/test jobs? Planning detail for Phase 25.
- **Per-code structured `ErrorDetail` payloads** — coarse codes ship first (D-15); structured detail (e.g. `PathOverlap` carrying the two conflicting paths) is added when a later phase's UI needs it.
- **`tome lint` failure surfacing in the GUI (Q3)**, **tray-icon (Q4)**, **Sparkle vs `tauri-plugin-updater` (Q6)**, **telemetry (Q7)** — all out of Phase 25 scope; tracked in ROADMAP Q1–Q7 for later phases.

</deferred>

---

*Phase: 25-rust-core-extraction-tauri-integration-spike*
*Context gathered: 2026-05-24*

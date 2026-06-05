# Phase 25: Rust core extraction + Tauri integration spike - Research

**Researched:** 2026-05-25
**Domain:** Rust workspace reshaping + Tauri 2 IPC architecture + specta/tauri-specta TypeScript binding generation + frontend framework spike
**Confidence:** HIGH on Rust-side patterns (verified against codebase + anyhow/serde official docs); MEDIUM on tauri-specta v2 specifics (verified against the live `specta-rs/tauri-specta` example app, but the crates are still `2.0.0-rc`)

## Summary

This phase is **architectural plumbing, not feature work**. Most of the structured types CORE-01 needs already exist (`StatusReport`, `RemovePlan`, `SkillEntry`, `DirectoryStatus`) — the job is to (a) decompose the 3,101-line `lib.rs::run` dispatcher into a thin CLI presenter layer over already-structured domain calls, (b) add a sibling `crates/tome-desktop` Tauri 2 crate, (c) gate `specta::Type` derives behind a `bindings` cargo feature so the CLI build stays specta-free, (d) introduce a synchronous `ProgressSink` trait + typed `ProgressEvent` enum, and (e) classify `anyhow` errors into a stable `TomeError` at the IPC boundary. **No production UI ships** — three throwaway spike apps render the real `StatusReport` to settle the React/Solid/Svelte decision.

The codebase is unusually well-prepared for this. It already proves the two hardest patterns: **typed-sentinel-through-anyhow downcast** is live in `LintFailed`/`MigrationPartialOrFailed` (`main.rs` downcasts both at the exit-code boundary — this is exactly D-13/D-14's shape), and **plan/render/execute** separation already exists in `remove`/`reassign`/`relocate`/`eject`. The risk is concentrated in **the external toolchain**, not tome's own code.

**Primary recommendation:** Pin `tauri = "2.11"`, `specta = "=2.0.0-rc.25"`, `tauri-specta = "=2.0.0-rc.25"`, `specta-typescript = "0.0.12"` with exact `=` pins on the specta trio (they require exact-version lockstep — `tauri-specta 2.0.0-rc.25` depends on `specta =2.0.0-rc.25`). Generate `bindings.ts` from the **app's `main.rs` under `#[cfg(debug_assertions)]`** using the `tauri_specta::Builder` — **not** from `build.rs** (the registered commands don't exist in the build-script link unit). This is the single most important divergence from the literal wording of locked decision D-07; see the Common Pitfalls and Open Questions sections. For cancellation, hand-roll a ~10-line `CancelToken(Arc<AtomicBool>)` newtype — do not pull `tokio-util`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Framework spike (D-GUI-04):**
- **D-01:** Build the spike in **all three** candidates — React, Solid, Svelte. The framework choice is irreversible from Phase 26 onward, so the comparison is built, not assumed.
- **D-02:** Each spike renders **only the real `StatusReport`** (`tome status` data) as a single-page dashboard — no list virtualization, no interactions.
- **D-03:** Compare across **all four criteria**: (1) `bindings.ts` ergonomics in each framework's idioms, (2) production bundle size + cold-start TTI, (3) dev-loop speed (HMR latency, error quality, type-check speed), (4) ecosystem fit for v1.0 reqs (virtualized lists for VIEW-02/NF-01, keyboard-accessible widgets for NF-02, macOS-HIG-aligned components for NF-03).
- **D-04:** Record the decision as a **scoring table (1–5 per criterion) + a short ADR** at `.planning/research/v1.0-frontend-framework-decision.md`. Also update **D-GUI-04** in `v1.0-REQUIREMENTS.md` with the chosen framework. Two losing spikes are deleted after the decision.

**Structured types + specta gating (CORE-01, CORE-03):**
- **D-05:** Structured types **stay in `crates/tome`**, co-located in producing modules (`status.rs::StatusReport`, `remove.rs::RemovePlan`, `manifest.rs::SkillEntry`, …). `crates/tome-desktop` depends on `crates/tome` as a path dep and imports them directly. No `tome-core` crate; no wrapper/mirror types.
- **D-06:** `specta::Type` is gated behind an optional **`bindings` cargo feature** on `crates/tome`: `[features] bindings = ["dep:specta", ...]`, cross-boundary types use `#[cfg_attr(feature = "bindings", derive(specta::Type))]`. CLI builds with default features (no specta cost); `tome-desktop` enables `tome/bindings`.
- **D-07:** `bindings.ts` is generated **at `tome-desktop` compile time** via a `build.rs` calling `tauri_specta::ts::export(...)`, written to `crates/tome-desktop/ui/src/bindings.ts` and **committed**. CI freshness gate: `cargo build -p tome-desktop` then `git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts`.

**Owned/Unowned migration (#542, part of CORE-01):**
- **D-08:** Replace `SkillEntry::source_name: Option<DirectoryName>` with `provenance: SkillProvenance` where `enum SkillProvenance { Owned { source: DirectoryName }, Unowned { last_owner: Option<DirectoryName> } }`. Lifts the existing `last_directory_name`/`previous_source` field into `Unowned`. Specta-derives as a TS discriminated union. **Manifest JSON migration strategy is a planning detail** — must preserve existing `#[serde(default)]` round-trip tolerance.

**Progress events (CORE-04):**
- **D-09:** Long-running domain ops take an injected **`ProgressSink` trait**: `trait ProgressSink: Send + Sync { fn emit(&self, event: ProgressEvent); }`, passed as `sink: &dyn ProgressSink`. Domain **stays synchronous** — no tokio runtime dep in `crates/tome`. CLI impl wraps `indicatif`; GUI impl wraps `tauri::AppHandle::emit`.
- **D-10:** `ProgressEvent` is a **per-op typed enum** (`SyncStageStarted/Progress/Finished { stage: SyncStage, … }`, `GitCloneProgress`, `BackupSnapshot`) — GUI pattern-matches rather than string-matches.
- **D-11:** The trait + `ProgressEvent` enum live in new **`crates/tome/src/progress.rs`**. CLI `IndicatifSink` lives in `lib.rs` next to `cmd_*` presenters; a `NullSink` for tests + `--quiet`. `tome-desktop` ships its own `TauriEventSink`.
- **D-12:** Cancellation (SYNC-04, Phase 27) is threaded **now** as a per-op `&CancellationToken` arg alongside `sink`. Domain checks `cancel.is_cancelled()` at stage boundaries. CLI passes a never-tripped token; real cancel lands Phase 27, but the API shape is fixed here.

**TomeError boundary (CORE-05):**
- **D-13:** **Classify at the IPC boundary.** Domain keeps `anyhow::Result` internally (zero refactor; no CLI regression). `tome-desktop` (and a thin CLI exit-code mapper) wrap results in `TomeError` at the Tauri command edge.
- **D-14:** Classification uses **typed sentinel errors via downcast**, not message string-matching. A small `thiserror` `enum DomainErrorKind` is attached at specific failure sites via anyhow `.context()`; the boundary does `err.downcast_ref::<DomainErrorKind>()` to pick the code; unmatched → `Internal`. Only GUI-relevant sites get sentinels.
- **D-15:** `ErrorCode` is **coarse — ~6 categories**: `enum ErrorCode { Validation, NotFound, Permission, Conflict, Git, Io, Internal }`, growing additively.
- **D-16:** Payload is `struct TomeError { code: ErrorCode, message: String, context: Vec<String> }` where `context` is the flattened anyhow `.context()` chain.

**Cross-cutting design principle:**
- **D-17:** **Symmetry "structure at the edge":** both progress (D-09/D-11) and errors (D-13) keep the domain ergonomic (sync, `anyhow`) and put GUI-facing structure at the boundary. Plans should preserve this symmetry — one mental model, not two.

### Claude's Discretion
- `lib.rs::run` decomposition mechanics (cmd_* presenters inline vs a `presenters/` module) — research/planning proposes a shape grounded in the current 3,101-line `lib.rs`.
- Exact `SyncStage` enum members, `ProgressEvent` variant fields, and which call sites get `DomainErrorKind` sentinels — implementation detail constrained by the decisions above.

### Deferred Ideas (OUT OF SCOPE)
- `lib.rs::run` decomposition mechanics deep-dive — propose a shape, don't over-engineer.
- Tauri 2.x minor-version pin policy (Q2) — default: pin a specific `2.M.N`, bump at milestone boundaries.
- CI matrix shape for `tome-desktop` — does macOS CI build the `.app`? where does the bindings-freshness gate sit? Planning detail.
- Per-code structured `ErrorDetail` payloads — coarse codes ship first; structured detail added when a later phase's UI needs it.
- `tome lint` failure surfacing in the GUI (Q3), tray-icon (Q4), Sparkle vs `tauri-plugin-updater` (Q6), telemetry (Q7) — all out of Phase 25 scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CORE-01 | Domain operations return structured Rust types; `lib.rs::run` decomposed into thin CLI presenter over the same domain calls | Most types already exist (`StatusReport`, `RemovePlan`, `SkillEntry`). Decomposition shape proposed in Architecture Patterns. #542 `SkillProvenance` migration via `#[serde(from)]` intermediate type (Code Examples). |
| CORE-02 | New `crates/tome-desktop` workspace member, path dep on `crates/tome`; CLI unchanged | Workspace layout in Architecture Patterns. macOS needs only Xcode CLT, no extra system libs. `members = ["crates/*"]` auto-picks up the new crate — verify cargo-dist doesn't bundle it (Common Pitfalls). |
| CORE-03 | All boundary types generate `bindings.ts` via specta + tauri-specta; no hand-rolled TS | `tauri_specta::Builder` + `specta_typescript::Typescript` export. `bindings` feature flag wiring (Code Examples). Transparent-newtype gotcha + `io::Error` field gotcha flagged (Common Pitfalls). |
| CORE-04 | Long-running ops emit progress via Tauri events; front-end renders without blocking IPC reply | `ProgressSink` (sync, `Send + Sync`) → `TauriEventSink` wrapping `AppHandle::emit`. Typed events via `#[derive(tauri_specta::Event)]`. Thread-safety analysis in Common Pitfalls. |
| CORE-05 | All Rust errors crossing into front-end carry stable `code` enum + `message` | `TomeError`/`ErrorCode`/`DomainErrorKind` via anyhow downcast at the boundary. Pattern already proven in-repo by `LintFailed`. anyhow `.context()` preserves downcastability (verified, Code Examples). |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Domain logic (sync, status, plans, validation) | `crates/tome` (Rust library) | — | D-05/D-GUI-08: library is canonical. No JS-side business logic (constraint). |
| CLI presentation (stdout formatting, indicatif, dialoguer) | `crates/tome::lib.rs` presenter layer | — | CLI stays in `crates/tome` unchanged behaviorally; `run` becomes a thin presenter. |
| IPC commands + error classification + event emission | `crates/tome-desktop` (Tauri Rust side) | `crates/tome` (returns `anyhow::Result` + emits via `&dyn ProgressSink`) | D-13/D-17: structure at the edge. Boundary wraps `anyhow` → `TomeError`, injects `TauriEventSink`. |
| TS type generation | `crates/tome-desktop` build (debug) | `crates/tome` (gated `specta::Type` derives) | D-06/D-07: derives gated behind `bindings` feature; export driven from the app crate that links the commands. |
| UI rendering | `crates/tome-desktop/ui/` (spike framework) | — | Throwaway spike; renders bindings.ts-typed `StatusReport`. No logic. |
| Progress trait + event enum definition | `crates/tome/src/progress.rs` | — | D-11: trait + `ProgressEvent` are domain vocabulary; sinks live at the edges. |

## Standard Stack

### Core (new dependencies for `crates/tome-desktop`)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tauri` | `2.11` (latest 2.11.2, May 2026) [VERIFIED: crates.io] | Desktop app shell, IPC, events, AppHandle::emit | D-GUI-01 locked. Calls Rust directly; ~8 MB bundle vs Electron's ~150 MB. |
| `tauri-build` | `2` (build-dep) [CITED: tauri-specta example Cargo.toml] | Tauri build script (`tauri_build::build()` in build.rs) | Standard Tauri 2 build-dep; generates context, processes capabilities. |
| `specta` | `=2.0.0-rc.25` [VERIFIED: crates.io, cargo search] | Rust type → language-agnostic type model; `#[derive(specta::Type)]` | D-GUI-03 locked. Still rc — see landmines. `tauri-specta` pins `=2.0.0-rc.25` exactly. |
| `tauri-specta` | `=2.0.0-rc.25` [VERIFIED: crates.io, cargo search] | `Builder` to collect commands/events + export TS | D-GUI-03. Features: `["derive", "typescript"]`. |
| `specta-typescript` | `0.0.12` [VERIFIED: crates.io, cargo search] | The actual TS exporter (`Typescript::default()`) | **v2 moved TS export out of `tauri_specta::ts` into this crate.** Required by `tauri-specta`'s `typescript` feature. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | `2` [VERIFIED: crates.io — `tauri-specta` already depends on `thiserror ^2`] | Derive `DomainErrorKind` sentinel enum (D-14) and `TomeError` (D-16) | Domain-side sentinels + boundary error type. Already a transitive dep. |
| `serde` | `1` (already a workspace dep) | `Serialize` on `TomeError`/`ProgressEvent`/all boundary types | Already present. `TomeError` and event payloads must derive `Serialize` to cross IPC. |
| `tauri-plugin-updater` | `2.10.1` [VERIFIED: crates.io] | Auto-update | **Phase 31 only — NOT this phase.** Listed so planner doesn't pull it early. |

**Cancellation (D-12) — recommendation:** **hand-roll, no new dep.** A `pub struct CancelToken(std::sync::Arc<std::sync::atomic::AtomicBool>)` with `is_cancelled()` / `cancel()` / `new()` is ~12 lines, `Send + Sync`, zero deps, and honors "no tokio in `crates/tome`". `tokio_util::sync::CancellationToken` pulls in tokio (violates D-09's no-tokio rule). `async-cancellation-token` (crates.io, updated Dec 2025) is single-threaded + async-oriented — wrong fit for a sync, thread-crossing core. **Use the AtomicBool newtype; put it in `progress.rs` next to `ProgressSink`.** [VERIFIED: tokio-util docs confirm it lives in tokio-util; ASSUMED that AtomicBool newtype suffices — trivially true for a `is_cancelled()`-at-stage-boundaries model]

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| specta/tauri-specta (rc) | Hand-rolled `ts-rs` derive | `ts-rs` is stable but has no Tauri command/event integration — you'd hand-wire the IPC glue tauri-specta generates. D-GUI-03 already chose specta; rc-pinning is the accepted cost. |
| AtomicBool CancelToken | `tokio-util::CancellationToken` | Pulls tokio into the sync core. Violates D-09. Rejected. |
| Export in app `main.rs` (debug) | `build.rs` export (literal D-07) | build.rs cannot see the registered `#[tauri::command]` fns (different link unit). The Builder must be constructed where commands are in scope. See Open Questions Q-A. |

**Installation (add to `crates/tome-desktop/Cargo.toml`):**
```toml
[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tome = { path = "../tome", features = ["bindings"] }
tauri = { version = "2.11", features = [] }
tauri-specta = { version = "=2.0.0-rc.25", features = ["derive", "typescript"] }
specta = { version = "=2.0.0-rc.25", features = ["derive"] }
specta-typescript = "0.0.12"
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = "2"
```

And in `crates/tome/Cargo.toml`:
```toml
[features]
bindings = ["dep:specta"]

[dependencies]
specta = { version = "=2.0.0-rc.25", features = ["derive"], optional = true }
```

**Version verification (run before locking):**
```bash
cargo search tauri-specta          # confirm rc.25 still latest, or bump trio together
cargo search specta-typescript     # confirm 0.0.12
cargo search tauri                 # confirm 2.11.x
```
The specta trio (`specta`, `specta-typescript`, `tauri-specta`) and macros move in lockstep — if any bumps, bump all and re-pin with `=`.

## Package Legitimacy Audit

> All packages verified directly on crates.io via `cargo search` (authoritative registry for Rust). slopcheck targets npm/PyPI and was unavailable; for Rust, `cargo search` against the official registry plus confirmed `specta-rs` / `tauri-apps` org ownership is the equivalent gate.

| Package | Registry | Age | Source Repo | Verification | Disposition |
|---------|----------|-----|-------------|--------------|-------------|
| `tauri` 2.11.2 | crates.io | mature (2.x stable line) | github.com/tauri-apps/tauri | cargo search confirmed | Approved |
| `tauri-build` 2 | crates.io | mature | github.com/tauri-apps/tauri | tauri-apps org | Approved |
| `specta` 2.0.0-rc.25 | crates.io | rc series (active to May 2026) | github.com/specta-rs/specta | cargo search confirmed | Approved (rc — pin `=`) |
| `tauri-specta` 2.0.0-rc.25 | crates.io | rc series (active to May 2026) | github.com/specta-rs/tauri-specta | cargo search confirmed | Approved (rc — pin `=`) |
| `specta-typescript` 0.0.12 | crates.io | 0.0.x (specta-rs) | github.com/specta-rs/specta | cargo search confirmed | Approved (0.0.x — pin) |
| `thiserror` 2 | crates.io | mature, dtolnay | github.com/dtolnay/thiserror | already transitive dep | Approved |
| `tauri-plugin-updater` 2.10.1 | crates.io | mature | github.com/tauri-apps/plugins-workspace | cargo search confirmed | Approved (Phase 31 only) |

**Packages removed:** none.
**Packages flagged suspicious:** none. **Caveat:** `specta`/`tauri-specta`/`specta-typescript` are pre-1.0 (`rc` / `0.0.x`) — not "slop," but pre-stable. Pin exact versions; treat upgrades as deliberate planned work. This is the accepted cost of D-GUI-03.

## Architecture Patterns

### System Architecture Diagram

```
                          ┌─────────────────────────────────────────────┐
   `tome` CLI binary      │             crates/tome (library)            │
   (crates/tome/main.rs)  │                                              │
        │                 │   run(cli)  ──►  cmd_* PRESENTERS            │
        │ parse args      │                    │  (format stdout,        │
        ▼                 │                    │   indicatif, dialoguer) │
   run(cli) ──────────────┼────────────────────┤                        │
   downcast LintFailed/   │                    ▼                        │
   MigrationFailed →exit  │            DOMAIN FUNCTIONS (sync, no tokio) │
                          │            status::gather → StatusReport     │
                          │            remove::plan   → RemovePlan       │
                          │            sync(... sink: &dyn ProgressSink, │
                          │                 cancel: &CancelToken)        │
                          │                    │         │               │
                          │   ProgressSink ◄───┘         │ anyhow::Result│
                          │   (trait, progress.rs)       │ + .context(   │
                          │   ┌──────────┬─────────┐      │  DomainError- │
                          │   │Indicatif │ Null    │      │  Kind sentinel)│
                          │   │Sink (CLI)│Sink     │      │               │
                          │   └──────────┴─────────┘      ▼               │
                          │  specta::Type derives gated by `bindings` feat │
                          └───────────────┬──────────────┬────────────────┘
                                          │ path dep      │ structured types cross here
                                          │ +bindings     ▼
        ┌─────────────────────────────────┴──────────────────────────────┐
        │                  crates/tome-desktop (Tauri 2)                  │
        │                                                                  │
        │  #[tauri::command] get_status(app) -> Result<StatusReport,       │
        │                                              TomeError>          │
        │     ├─ injects TauriEventSink(app.clone()) ──► AppHandle::emit   │
        │     ├─ injects CancelToken                                       │
        │     └─ maps anyhow::Result → TomeError (downcast DomainErrorKind │
        │            → ErrorCode; chain() → context: Vec<String>)          │
        │                                                                  │
        │  main.rs (#[cfg(debug_assertions)]):                            │
        │     Builder::<Wry>::new()                                        │
        │       .commands(collect_commands![get_status, sync, ...])        │
        │       .events(collect_events![SyncProgress, ...])                │
        │       .export(Typescript::default(), "ui/src/bindings.ts")  ◄────┼─ committed; CI diff gate
        │       └─ .invoke_handler(builder.invoke_handler())               │
        │          .setup(|app| builder.mount_events(app))                 │
        └───────────────────────────────┬──────────────────────────────────┘
                                         │ IPC (commands return JSON; events stream)
                                         ▼
        ┌──────────────────────────────────────────────────────────────┐
        │   ui/  (vite + React | Solid | Svelte — 3 spike apps)          │
        │   import { commands, events } from "./bindings"                 │
        │   commands.getStatus() : Promise<Result<StatusReport,TomeError>>│
        │   events.syncProgress.listen(...)                               │
        │   renders StatusReport dashboard (real tome_home data)          │
        └──────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
tome/
├── Cargo.toml                        # workspace; members=["crates/*"] already picks up tome-desktop
├── crates/
│   ├── tome/                         # unchanged CLI; +bindings feature, +progress.rs
│   │   └── src/
│   │       ├── lib.rs                # run() → presenter layer; IndicatifSink + NullSink here
│   │       ├── progress.rs           # NEW: ProgressSink trait, ProgressEvent, SyncStage, CancelToken
│   │       ├── status.rs             # StatusReport gains cfg_attr specta::Type
│   │       ├── manifest.rs           # SkillEntry.source_name → provenance: SkillProvenance
│   │       └── ... (domain fns gain `sink: &dyn ProgressSink, cancel: &CancelToken`)
│   └── tome-desktop/                 # NEW crate (CORE-02)
│       ├── Cargo.toml                # path dep on tome+bindings; tauri/specta deps
│       ├── build.rs                  # `tauri_build::build()` (NOT the specta export)
│       ├── tauri.conf.json           # frontendDist: "ui/dist", beforeDevCommand, devUrl
│       ├── capabilities/main.json    # permission set
│       ├── icons/                    # app icons
│       └── src/
│           ├── main.rs               # Builder + commands + events + #[cfg(debug)] export
│           ├── commands.rs           # #[tauri::command] wrappers
│           ├── error.rs              # TomeError, ErrorCode, From<anyhow::Error>
│           └── sink.rs               # TauriEventSink impl ProgressSink
│       └── ui/                       # frontend (per-framework spike; gitignored dist/)
│           ├── package.json
│           ├── vite.config.ts
│           ├── index.html
│           └── src/
│               ├── bindings.ts       # GENERATED + COMMITTED (D-07)
│               └── App.{tsx|jsx|svelte}
```

> **Spike layout note (D-01):** Three frameworks need three frontends but should share one Tauri Rust backend + one `bindings.ts`. Cleanest: keep one `crates/tome-desktop` with the Rust side fixed, and three sibling `ui-react/`, `ui-solid/`, `ui-svelte/` dirs (or git branches). After the decision (D-04), collapse the winner into `ui/` and delete the losers. The Rust backend and bindings are identical across all three — only `tauri.conf.json`'s `beforeDevCommand`/`devUrl`/`frontendDist` point at the active frontend.

### Pattern 1: `lib.rs::run` presenter decomposition (CORE-01 / D-GUI-08)

**What:** Today `run(cli)` is a giant `match cli.command { ... }` with domain logic + stdout formatting inline. Decompose so each arm calls a pure domain fn returning a structured type, then a `cmd_*` presenter formats it.

**When to use:** Every command. The domain fns are what `tome-desktop` calls; the `cmd_*` presenters are CLI-only.

**Recommended shape (grounded in the actual 3,101-line file — Deferred Idea resolution):**
- Domain fns already partly exist (`status::gather`, `remove::plan`). Where a command's logic is still inline in `run`, extract it to its module as `fn collect(...) -> Result<XxxReport>` / `fn plan(...) -> Result<XxxPlan>`.
- Keep `cmd_*` presenters **inline in `lib.rs`** next to `run` (they already are — `cmd_remove_dir`, `cmd_remove_skill` at lines 662/779). A `presenters/` module is unnecessary churn for a 25-command surface and would not buy anything the GUI needs. **Recommendation: do NOT create a `presenters/` module; keep presenters in `lib.rs`.** This minimizes the diff and preserves the `insta` snapshot bytes.
- The `insta` snapshots (8 tests, per migration inventory) + 130 `assert_cmd` integration tests are the regression gate. The decomposition must preserve CLI stdout/stderr **byte-for-byte** — verify with `cargo test --all` and `cargo insta test`.

**Anti-pattern:** Don't rewrite the domain fns' output shape "while you're in there." The library-canonical types are the contract (STATE.md). Adding a specta derive is allowed; changing a field is a separate, explicit decision.

### Pattern 2: `bindings` feature gating (D-06)

**What:** `specta::Type` derive present only when the `bindings` feature is on.
```rust
// crates/tome/src/status.rs
#[derive(serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct StatusReport { /* ... */ }
```
**When to use:** Every type that crosses the IPC boundary. The CLI (`cargo build`, default features) compiles without specta entirely — zero binary-size or compile-time cost, satisfying D-GUI-02's "CLI binary stays slim."

### Pattern 3: ProgressSink injection (D-09/D-11) — "structure at the edge"

**What:** Domain fns take `sink: &dyn ProgressSink` and call `sink.emit(ProgressEvent::...)` at meaningful points. The trait is `Send + Sync` so a GUI sink holding an `AppHandle` is legal across threads.
```rust
// crates/tome/src/progress.rs
pub trait ProgressSink: Send + Sync {
    fn emit(&self, event: ProgressEvent);
}
pub enum ProgressEvent {
    SyncStageStarted { stage: SyncStage },
    SyncStageProgress { stage: SyncStage, current: usize, total: usize },
    SyncStageFinished { stage: SyncStage },
    GitCloneProgress { directory: String, received: u64 },
    BackupSnapshot { message: String },
}
pub enum SyncStage { Reconcile, Discover, Consolidate, Distribute, Cleanup, Save }
```
The current `spinner()` helper + the four `sp.finish_and_clear()` call sites in `lib.rs` (lines 148–157, 1675–1815) are exactly the presentation-in-domain smell `IndicatifSink` re-homes. **When to use:** thread `&dyn ProgressSink` + `&CancelToken` through `sync()` and `git::clone`/`backup::*` now; other long-running ops can adopt incrementally.

### Pattern 4: TomeError boundary via anyhow downcast (D-13/D-14/D-16)

**What:** The domain stays `anyhow`. At GUI-relevant failure sites, attach a `DomainErrorKind` sentinel via `.context()`. The Tauri command boundary downcasts to pick an `ErrorCode`, and flattens `err.chain()` into `context: Vec<String>`. **This pattern is already proven in-repo** — `main.rs` downcasts `LintFailed` and `MigrationPartialOrFailed` through anyhow at the exit-code boundary (lines 36–43). D-13/D-14 generalize that exact idea. (See Code Examples for the verified mechanics.)

### Anti-Patterns to Avoid
- **Exporting bindings from `build.rs`:** the literal D-07 wording. The `tauri_specta::Builder` needs the `#[tauri::command]` fns in scope; build scripts run in a separate link unit and can't see them. Export from the app's `main.rs`/`lib.rs` under `#[cfg(debug_assertions)]` instead (see Open Questions Q-A for how to keep the CI freshness-diff gate).
- **Deriving `specta::Type` on a type with `std::io::Error` fields:** `RemovePlan`'s `RemoveFailure`/`RemoveSkillFailure` carry `error: std::io::Error` (remove.rs lines 123, 216) — not `Serialize`, not `Type`. The tauri-specta example handles this with `#[serde(skip)]` on the field (stringify into a sibling `String` field instead). See Common Pitfalls.
- **Putting tokio in `crates/tome`:** violates D-09. Use the AtomicBool CancelToken.
- **Changing structured-type field shapes mid-decomposition:** the library-canonical types are the contract (STATE.md). Only `SkillProvenance` (D-08) changes shape, and that's an explicit decision with a migration plan.
- **A `tome-core` crate:** D-05 explicitly forbids it. Types stay in `crates/tome`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Rust→TS type generation | A custom `Serialize`-walking TS emitter | `specta` + `tauri-specta` + `specta-typescript` | D-GUI-03 locked. tauri-specta also generates the typed `commands.*` / `events.*` IPC glue, not just types. |
| Typed Tauri command/event IPC glue | Manual `invoke("get_status")` + hand-cast | `tauri-specta` `collect_commands!` / `collect_events!` + generated `commands`/`events` objects | Eliminates JS-side type drift; gives `Result<T, TomeError>` typed returns. |
| anyhow chain flattening for `TomeError.context` | String-split the `{:#}` Display | `err.chain().map(\|e\| e.to_string()).collect::<Vec<_>>()` | `chain()` is the documented, structured iterator over the cause chain (anyhow docs). |
| Cancellation token | A bespoke channel + select loop | `CancelToken(Arc<AtomicBool>)` newtype (~12 lines) | No tokio; sync `is_cancelled()` at stage boundaries is all SYNC-04 needs. |
| Backward-compatible manifest migration | Manual two-pass JSON parse | `#[serde(from = "SkillEntryRepr")]` intermediate type | Standard serde migration idiom; preserves round-trip (Code Examples). |

**Key insight:** tauri-specta's value isn't "TS types" — it's the *typed command + event contract*. The generated `commands.getStatus()` returns a typed `Result<StatusReport, TomeError>` discriminated union, and `events.syncProgress.listen()` is typed. Hand-rolling that glue is where Tauri apps accrue the most drift bugs.

## Runtime State Inventory

> This phase touches the `.tome-manifest.json` on-disk shape (D-08 `SkillProvenance` migration). The migration is **read-tolerant**, not a one-shot data rewrite, so the inventory is light.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `.tome-manifest.json` files in any `tome_home` carry `SkillEntry` with the old `source_name`/`previous_source` flat fields. The spike reads the **user's real** `tome_home` (per Specifics + SC#2). | **Code edit only — migration-on-read.** `#[serde(from = "SkillEntryRepr")]` maps old `source_name: Some(x)` → `Owned{source:x}`, `source_name: None` (+ optional `previous_source`) → `Unowned{last_owner}`. New writes use the enum shape. No standalone data-migration task; next `tome sync` rewrites in the new shape naturally. Preserve `#[serde(default)]` tolerance (LIB-01..05 / D-10). |
| Live service config | None — tome has no external services or daemons. Verified: REQUIREMENTS.md "No external services or network requirements." | None. |
| OS-registered state | None — tome registers nothing with the OS (no launchd, no scheduler). Verified by ARCHITECTURE/CLAUDE.md (standalone binary). | None. |
| Secrets/env vars | `TOME_HOME` env var is read for path resolution — **unchanged** by this phase. No secret keys involved. | None. |
| Build artifacts | A new `crates/tome-desktop/target` and `ui/node_modules` + `ui/dist` appear. `ui/dist` and `node_modules` must be gitignored; `ui/src/bindings.ts` must be **committed** (D-07). | Add `.gitignore` entries; commit `bindings.ts`. No stale-artifact risk to existing `tome` binary. |

**Manifest round-trip is the load-bearing migration.** The existing manifest already deserializes the old shape (manifest.rs has tests `deserialize_old_shape_with_source_name_string`, `deserialize_new_shape_with_null_source_name`). The `SkillProvenance` change must keep those JSON inputs parsing — add equivalent tests for the new enum shape.

## Common Pitfalls

### Pitfall 1: `bindings.ts` cannot be generated from `build.rs` (the literal D-07)
**What goes wrong:** D-07 says "via a `build.rs` calling `tauri_specta::ts::export(...)`." Two problems: (1) the `tauri_specta::ts::export` free function is the **v1** API; v2 uses `Builder::export` with `specta_typescript::Typescript`. (2) A `build.rs` runs as a separate compilation unit and **cannot see the `#[tauri::command]` functions** defined in the crate's `src/` — so it cannot build the command list the Builder needs.
**Why it happens:** The v1→v2 API churn + the natural assumption that "compile-time generation = build.rs."
**How to avoid:** Construct the `Builder` in the app's `main.rs`/`lib.rs` where commands are in scope, and call `.export(Typescript::default(), "ui/src/bindings.ts")` under `#[cfg(debug_assertions)]`. The official example does exactly this (main.rs lines 195–231). The CI freshness gate still works: `cargo run -p tome-desktop` (or `cargo tauri dev` startup) in debug regenerates the file, then `git diff --exit-code`. See Open Questions Q-A for the exact CI recipe.
**Warning signs:** `build.rs` referencing command fn names; "cannot find function" errors; or an empty/stale `bindings.ts`.
[VERIFIED: specta-rs/tauri-specta examples/app/src-tauri/{main.rs,build.rs}]

### Pitfall 2: `std::io::Error` fields block `specta::Type`/`Serialize` derive
**What goes wrong:** `RemovePlan`'s failure structs (`RemoveFailure`, `RemoveSkillFailure`) carry `pub error: std::io::Error`. `io::Error` is neither `Serialize` nor `specta::Type`. Deriving either fails to compile.
**Why it happens:** These structs predate the IPC boundary; they were CLI-internal.
**How to avoid:** Either (a) `#[serde(skip)]` + `#[cfg_attr(feature="bindings", specta(skip))]` the `error` field and add a sibling `error_message: String` populated from `error.to_string()`, or (b) change the field type to `String` outright (the GUI can't use a live `io::Error` anyway). Option (b) is cleaner for a boundary type but is a field-shape change — flag it as a deliberate sub-decision. The official example uses the `#[serde(skip)]`-on-`io::Error` approach (main.rs lines 86–90).
**Warning signs:** "the trait `Serialize` is not implemented for `std::io::Error`" when adding the `bindings` feature.
[VERIFIED: codebase remove.rs:123,216 + tauri-specta example MyError]

### Pitfall 3: Naming collision — `SkillProvenance` already exists
**What goes wrong:** `discover.rs` already defines `pub struct SkillProvenance { registry_id, version, git_commit_sha }` (package-manager metadata). D-08 introduces a **different** `enum SkillProvenance { Owned, Unowned }` in `manifest.rs`. Two public types, same name, same crate.
**Why it happens:** D-08 was written without grepping for the existing name.
**How to avoid:** Rename one. The existing struct is provenance-of-*managed-source* metadata; the new enum is provenance-of-*ownership*. Recommend naming the new D-08 enum **`SkillOwnership`** (or `ProvenanceKind`) to avoid the collision and to read better (`provenance: SkillOwnership`). Confirm with the user since D-08 spelled out `SkillProvenance` — this is a naming-only deviation, not a semantic one. Alternatively rename the discover.rs struct to `ManagedProvenance`. **Flag for planner: pick one rename; don't ship two `SkillProvenance` types.**
**Warning signs:** ambiguous-import or duplicate-definition errors; confused readers.
[VERIFIED: codebase discover.rs:109-118 vs D-08 text]

### Pitfall 4: cargo-dist accidentally building `tome-desktop`
**What goes wrong:** `members = ["crates/*"]` auto-includes `crates/tome-desktop`. cargo-dist (which owns `release.yml`) might try to build/ship it, or pull Tauri+webview deps into the CLI artifact build, bloating it or breaking the release.
**Why it happens:** Workspace globbing + cargo-dist's default "build all bin targets."
**How to avoid:** `crates/tome-desktop` should set `publish = false` and, in `[workspace.metadata.dist]` or per-package dist config, ensure only `tome` is a dist target. cargo-dist supports `dist = false` on a package's `[package.metadata.dist]`. Verify `cargo dist plan` still lists only the `tome` CLI artifacts after adding the crate. **Do not hand-edit `release.yml`** (CLAUDE.md: cargo-dist owns it; run `cargo dist init` after metadata changes). This is a STATE.md "Blockers/Concerns" item — address it in Phase 25, not Phase 31.
**Warning signs:** `cargo dist plan` listing a `tome-desktop` artifact; Tauri deps in the CLI release build log.
[VERIFIED: Cargo.toml members glob + CLAUDE.md cargo-dist ownership note; ASSUMED that `package.metadata.dist.dist = false` is the current cargo-dist 0.30 opt-out key — confirm against cargo-dist 0.30.3 docs at plan time]

### Pitfall 5: Thread-safety when a Tauri command calls the sync core which emits events
**What goes wrong:** An `async #[tauri::command]` (or a sync one on Tauri's thread pool) calls a synchronous domain fn that invokes `sink.emit(...)`, which calls `app_handle.emit(...)`. Concern: is this `Send`/thread-safe?
**Why it doesn't actually go wrong:** `tauri::AppHandle` is `Clone + Send + Sync`, and `AppHandle::emit` is callable from any thread (it's the documented cross-thread event mechanism). `ProgressSink: Send + Sync` (D-09) plus an `AppHandle`-holding `TauriEventSink` is sound. If the domain fn is heavy (sync, blocking), run it off the IPC thread via `tauri::async_runtime::spawn_blocking` (or a plain `std::thread`) inside the command so the IPC reply isn't blocked (CORE-04: "render progress without blocking the IPC reply"). The sink emits from that worker thread; the frontend listener fires independently.
**How to avoid:** make `#[tauri::command]` either `async` and offload the sync core to `spawn_blocking`, or keep it sync but ensure long ops emit progress (so the UI updates) and return when done. Don't hold a lock across `emit`.
**Warning signs:** UI frozen during sync (command blocking the reply with no events flowing); `!Send` compile errors if a non-Send value is held across an await.
[VERIFIED: Tauri docs confirm AppHandle is Send+Sync and emit is cross-thread; ASSUMED the spawn_blocking offload pattern — standard Tauri 2 guidance]

### Pitfall 6: Transparent newtypes (`SkillName`, `DirectoryName`, `ContentHash`) under specta
**What goes wrong:** These use `#[serde(transparent)]` + a *custom* validating `Deserialize` impl (discover.rs:99, validation.rs:52). specta derives off the type structure, not the serde impl. A `#[serde(transparent)]` newtype-over-`String` should specta-derive to a TS `string`, but the custom `Deserialize` (not derived) means `#[derive(specta::Type)]` sees a tuple struct.
**Why it happens:** specta's `Type` derive and serde's `Deserialize` are independent; a hand-written `Deserialize` doesn't inform specta.
**How to avoid:** add `#[cfg_attr(feature="bindings", derive(specta::Type))]` and verify in the spike that `SkillName` emits as `type SkillName = string` (or a branded string) — not `[string]` or an opaque struct. If specta mis-renders, use `#[specta(transparent)]` on the newtype to mirror the serde behavior. **This is explicitly called out in CONTEXT.md code_context: "verify specta handles transparent newtypes cleanly during the spike."** Make it a spike acceptance check.
**Warning signs:** `bindings.ts` showing `SkillName` as a tuple/array type or `unknown` instead of `string`.
[VERIFIED: codebase serde(transparent) newtypes; ASSUMED specta `#[specta(transparent)]` is the fix — confirm in spike]

## Code Examples

### anyhow `.context()` preserves downcastability (D-14 mechanics — verified)
```rust
// VERIFIED against anyhow docs: after `.context(C)` on an error E,
// BOTH downcast_ref::<C>() and downcast_ref::<E>() succeed.
// The existing repo already relies on this (LintFailed / MigrationPartialOrFailed).

// Domain side (crates/tome) — attach a sentinel at a GUI-relevant site:
use thiserror::Error;
#[derive(Debug, Error)]
pub enum DomainErrorKind {
    #[error("validation failed")]   Validation,
    #[error("not found")]            NotFound,
    #[error("permission denied")]    Permission,
    #[error("conflict")]             Conflict,
    #[error("git operation failed")] Git,
    #[error("io failure")]           Io,
}

fn do_thing() -> anyhow::Result<()> {
    something()
        .with_context(|| DomainErrorKind::NotFound)   // sentinel attached
        .context("while loading directory 'foo'")?;   // human context on top — still downcastable
    Ok(())
}

// Boundary side (crates/tome-desktop/error.rs):
impl From<anyhow::Error> for TomeError {
    fn from(err: anyhow::Error) -> Self {
        // walk the chain; first DomainErrorKind wins; else Internal
        let code = err
            .chain()
            .find_map(|cause| cause.downcast_ref::<DomainErrorKind>())
            .map(ErrorCode::from)
            .unwrap_or(ErrorCode::Internal);
        TomeError {
            code,
            message: err.to_string(),                       // top-level message
            context: err.chain().map(|c| c.to_string()).collect(), // flattened chain (D-16)
        }
    }
}
```
> **Subtlety:** `Error::downcast_ref::<T>()` on `anyhow::Error` checks the *outermost* type and the directly-attached context type. To find a sentinel buried under further `.context()` calls, iterate `err.chain()` and `downcast_ref` each `&(dyn Error)` — the snippet above does this. (anyhow docs confirm context wraps, not replaces; the cause chain is iterable via `chain()`.)
[VERIFIED: docs.rs/anyhow Error::context + downcast_ref semantics; repo main.rs:36-43 proves the pattern works]

### SkillProvenance / SkillOwnership migration-on-read (D-08 — verified serde idiom)
```rust
// crates/tome/src/manifest.rs
// New canonical shape:
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "lowercase")]   // → TS discriminated union: {kind:"owned",source} | {kind:"unowned",last_owner}
pub enum SkillOwnership {                            // renamed from D-08's SkillProvenance to avoid collision (see Pitfall 3)
    Owned { source: DirectoryName },
    Unowned { last_owner: Option<DirectoryName> },
}

// Intermediate "old shape" type, used only for deserialization:
#[derive(serde::Deserialize)]
struct SkillEntryRepr {
    source_path: PathBuf,
    #[serde(default)] source_name: Option<DirectoryName>,
    #[serde(default)] previous_source: Option<DirectoryName>,
    content_hash: ContentHash,
    synced_at: String,
    #[serde(default)] managed: bool,
}
impl From<SkillEntryRepr> for SkillEntry {
    fn from(r: SkillEntryRepr) -> Self {
        let ownership = match r.source_name {
            Some(source) => SkillOwnership::Owned { source },
            None => SkillOwnership::Unowned { last_owner: r.previous_source },
        };
        SkillEntry { source_path: r.source_path, ownership,
                     content_hash: r.content_hash, synced_at: r.synced_at, managed: r.managed }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
#[serde(from = "SkillEntryRepr")]   // old JSON → SkillEntryRepr → SkillEntry. New writes serialize the enum.
pub struct SkillEntry {
    pub source_path: PathBuf,
    pub ownership: SkillOwnership,
    pub content_hash: ContentHash,
    pub synced_at: String,
    #[serde(default)] pub managed: bool,
}
```
> Preserves the existing `#[serde(default)]` tolerance (old manifests parse). New serialization emits the enum shape; next `tome sync` rewrites naturally. Mirror the existing manifest tests (`deserialize_old_shape_*`) for the new enum. **Note:** `#[serde(from)]` makes the type deserialize-only via the intermediate; `Serialize` is still derived directly on `SkillEntry` (asymmetric serde is fine here). Verify serialize/deserialize round-trip in a unit test.
[VERIFIED: serde.rs container-attrs `#[serde(from)]` semantics; codebase manifest.rs existing migration tests]

### Tauri command + event + export (CORE-03/04/05 — from the live v2 example)
```rust
// crates/tome-desktop/src/main.rs  (structure mirrors specta-rs/tauri-specta examples/app)
use tauri_specta::{Builder, collect_commands, collect_events};
use specta_typescript::Typescript;

#[derive(serde::Serialize, Clone, specta::Type, tauri_specta::Event)]
pub struct SyncProgress { stage: String, current: u32, total: u32 } // bridged from ProgressEvent

#[tauri::command]
#[specta::specta]
fn get_status(app: tauri::AppHandle) -> Result<tome::status::StatusReport, TomeError> {
    let (config, paths) = load_context(&app)?;          // resolve real tome_home
    tome::status::gather(&config, &paths).map_err(TomeError::from)   // anyhow → TomeError
}

fn main() {
    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![get_status /*, sync, ... */])
        .events(collect_events![SyncProgress]);

    #[cfg(debug_assertions)]                              // NOT build.rs — see Pitfall 1
    builder
        .export(Typescript::default(), "ui/src/bindings.ts")
        .expect("export bindings.ts");

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| { builder.mount_events(app); Ok(()) })
        .run(tauri::generate_context!())
        .expect("run tome-desktop");
}
```
[VERIFIED: specta-rs/tauri-specta examples/app/src-tauri/main.rs (rc.25 era, fetched 2026-05-25)]

### CancelToken (D-12 — hand-rolled, no tokio)
```rust
// crates/tome/src/progress.rs
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);
impl CancelToken {
    pub fn new() -> Self { Self::default() }
    pub fn cancel(&self) { self.0.store(true, Ordering::SeqCst); }
    pub fn is_cancelled(&self) -> bool { self.0.load(Ordering::SeqCst) }
}
// Domain: `if cancel.is_cancelled() { anyhow::bail!("cancelled"); }` at stage boundaries.
// CLI passes CancelToken::new() (never tripped). GUI (Phase 27) clones it into a cancel-command.
```
[ASSUMED — trivial std-only construction; satisfies "no tokio" + "check at stage boundaries" exactly]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `tauri_specta::ts::export(...)` free fn (v1) | `Builder::new().commands(...).export(Typescript::default(), path)` (v2) | tauri-specta v2 (rc series) | D-07's cited API is the **v1** signature. Use the v2 `Builder`. |
| TS export inside `tauri_specta` crate | Separate `specta-typescript` crate (`Typescript`, `JSDoc`) | specta v2 rc | Must add `specta-typescript = "0.0.12"` as an explicit dep. |
| Electron + napi-rs | Tauri 2 `#[tauri::command]` | D-GUI-01 (this milestone) | ~8 MB vs ~150 MB; no N-API shim. |
| specta/tauri-specta 1.x (Tauri 1) | 2.0.0-rc.25 (Tauri 2) | ongoing rc since 2024 | **No stable 2.0 as of May 2026.** Pin `=` exact. |

**Deprecated/outdated:**
- Any tutorial referencing `tauri_specta::ts::builder()` / `tauri_specta::ts::export()` — that's v1. The v2 entry point is `tauri_specta::Builder`.
- `specta::Type` from a single `specta` crate import for TS export — TS lives in `specta-typescript` now.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `package.metadata.dist.dist = false` is the cargo-dist 0.30.3 opt-out for excluding `tome-desktop` from release artifacts | Pitfall 4 | If the key changed, `cargo dist plan` would still list tome-desktop; planner must check cargo-dist 0.30.3 docs. Low risk — `cargo dist plan` makes it self-verifying. |
| A2 | `#[specta(transparent)]` correctly maps tome's transparent newtypes to TS `string` | Pitfall 6 | If specta mis-renders, bindings.ts has wrong newtype shapes; caught by the spike's acceptance check (CONTEXT.md already mandates this verification). |
| A3 | `spawn_blocking` offload is the right pattern for the sync core in an async command without blocking the IPC reply | Pitfall 5 | If wrong, UI freezes during sync; fixable in Phase 27 (real sync UI). Standard Tauri guidance. |
| A4 | The AtomicBool CancelToken is sufficient for SYNC-04's "consistent state on cancel" | Code Examples / D-12 | SYNC-04's atomicity comes from the domain's existing atomic temp+rename writes + checking the flag only at *stage boundaries*, not the token type. Low risk. |
| A5 | tauri-specta rc.25 remains the latest at plan time | Standard Stack | rc series is active (rc.24→rc.25 in ~5 weeks). Planner should re-run `cargo search` and bump the trio together if a newer rc shipped. |
| A6 | Renaming D-08's `SkillProvenance` to `SkillOwnership` (or renaming the discover.rs struct) is acceptable to the user | Pitfall 3 | D-08 literally said `SkillProvenance`; this is a naming-only deviation forced by an existing same-named type. Needs user confirmation. |

## Open Questions

1. **Q-A: How to keep D-07's CI freshness-diff gate when export must happen at runtime, not in build.rs?**
   - What we know: The Builder must export from `main.rs` under `#[cfg(debug_assertions)]`; build.rs can't see the commands. The export *does* still happen "at compile/dev time" in the sense that a debug run/`cargo tauri dev` regenerates the file.
   - What's unclear: the exact CI step. Two viable recipes: (a) a tiny `--bin gen-bindings` or `xtask` that constructs the same `Builder` and calls `.export()` then exits — run it in CI, then `git diff --exit-code ui/src/bindings.ts`; or (b) a `#[test]` (gated on `bindings`) that builds the Builder and asserts the on-disk file matches a fresh export. Option (a) is cleanest and keeps export logic in one place (factor the Builder construction into a shared `fn make_builder()`).
   - Recommendation: factor `make_builder()` into a function shared by `main.rs` and a small `gen-bindings` bin; CI runs `cargo run -p tome-desktop --bin gen-bindings --features ...` then `git diff --exit-code`. Resolves D-07's intent without build.rs. **Planner should pick (a) or (b) explicitly.**

2. **Q-B: Does macOS CI building `cargo build -p tome-desktop` require a frontend build first?**
   - What we know: `cargo build -p tome-desktop` compiles the Rust side; `tauri.conf.json`'s `frontendDist` is only consulted by `cargo tauri build`/`dev`, not bare `cargo build`. For the *bindings freshness gate* you don't need the frontend at all (Q-A). For a full `.app` you'd need `beforeBuildCommand` (vite build).
   - Recommendation: CI's bindings gate runs bare `cargo run --bin gen-bindings` (no npm). A separate, optional `cargo tauri build` smoke job (npm + vite) can come in Phase 31; this phase only needs the gen-bindings + `cargo build -p tome-desktop` jobs. Keeps macOS CI cheap and avoids regressing the cargo-dist release path.

3. **Q-C: One `crates/tome-desktop` with three UI dirs, or three throwaway crates for the spike?**
   - What we know: The Rust backend + bindings are identical across frameworks (D-02 renders the same StatusReport). Duplicating the Rust crate three times wastes effort.
   - Recommendation: one Rust crate, three sibling `ui-*` frontends sharing one `bindings.ts`. Switch active frontend via `tauri.conf.json` `beforeDevCommand`/`devUrl`. After D-04, collapse winner to `ui/`, delete losers. Lets the framework comparison be purely frontend-side (fair for D-03 criteria).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All Rust work | ✓ (assumed — existing project) | 1.85.0+ (edition 2024) | — |
| Xcode Command Line Tools | Tauri macOS build (`cargo tauri build`, webview link) | likely ✓ on dev mac | — | `xcode-select --install` |
| Node.js + npm/pnpm | Vite frontend (spike `ui/`) | unknown | — | install via brew/volta; needed only for the spike frontends, not the bindings gate |
| Tauri CLI (`cargo tauri` / `tauri-cli`) | `cargo tauri dev`/`build` | likely ✗ (new) | 2.x | `cargo install tauri-cli --version "^2"` |
| macOS webview (WKWebView) | Tauri runtime | ✓ (system, all macOS) | system | — (no extra system libs on macOS, unlike Linux's webkit2gtk) |

**Missing dependencies with no fallback:** none — all are installable.
**Missing dependencies with fallback:** Node + Tauri CLI (install steps above). The **bindings freshness gate needs neither** — it's pure `cargo` (Q-A/Q-B), so the most CI-critical path has no JS toolchain dependency.

**macOS advantage:** Unlike Linux (webkit2gtk-4.1, libsoup, etc.), macOS Tauri builds need only Xcode CLT + the system WKWebView. This is why D-GUI-06 (macOS-only v1.0) keeps the CI surface small. [VERIFIED: Tauri v2 prerequisites docs — macOS section lists only Xcode]

## Validation Architecture

> `nyquist_validation` config key not located in `.planning/config.json` during research; treating as enabled per the absent-means-enabled rule. The project's existing test infrastructure (608 tests) is the primary gate.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `assert_cmd` (CLI integration) + `insta` (snapshots) |
| Config file | none (cargo defaults); `insta` snapshots in `crates/tome/tests/snapshots/` |
| Quick run command | `cargo test -p tome <name>` |
| Full suite command | `cargo test --all` (then `cargo insta test --review` for snapshots) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CORE-01 | CLI output byte-identical after `run` decomposition | integration + snapshot | `cargo test --all && cargo insta test` | ✅ (130 assert_cmd + 8 insta) |
| CORE-01 / D-08 | Old manifest JSON parses to `Owned`/`Unowned`; round-trips | unit | `cargo test -p tome manifest::tests` | ✅ (extend existing `deserialize_old_shape_*`) |
| CORE-02 | `tome-desktop` builds; CLI unaffected | build | `cargo build -p tome-desktop && cargo build -p tome` | ❌ Wave 0 (new crate) |
| CORE-03 | `bindings.ts` matches a fresh export | gen+diff | `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code` | ❌ Wave 0 (new gate) |
| CORE-03 | transparent newtypes emit as TS `string` | spike assertion | inspect generated `bindings.ts` | ❌ Wave 0 |
| CORE-04 | sync emits ≥1 event per stage | unit (NullSink/recording sink) | `cargo test -p tome progress::tests` | ❌ Wave 0 |
| CORE-05 | anyhow+sentinel downcasts to correct `ErrorCode` | unit | `cargo test -p tome-desktop error::tests` (mirror `lint_failed_downcast_through_anyhow`) | ❌ Wave 0 (new), pattern ✅ exists |
| CORE-05 | no CLI regression in exit codes | integration | `cargo test --all` | ✅ |

### Sampling Rate
- **Per task commit:** `cargo test -p tome <touched-module>` + `cargo clippy --all-targets -- -D warnings`
- **Per wave merge:** `cargo test --all` + `cargo insta test`
- **Phase gate:** `make ci` (fmt-check + clippy -D warnings + test) green; `cargo dist plan` lists only the `tome` CLI artifact; `git diff --exit-code` on `bindings.ts`.

### Wave 0 Gaps
- [ ] `crates/tome/src/progress.rs` + a `RecordingSink` test double (assert event sequence) — covers CORE-04
- [ ] `crates/tome-desktop/src/error.rs` + downcast unit tests mirroring `lint.rs::lint_failed_downcast_through_anyhow` — covers CORE-05
- [ ] Manifest migration tests for the new `Owned`/`Unowned` enum shape — covers CORE-01/D-08
- [ ] CI job: `gen-bindings` + `git diff --exit-code` — covers CORE-03 freshness gate
- [ ] CI job: `cargo build -p tome-desktop` on macos-latest — covers CORE-02
- [ ] Spike acceptance check: transparent-newtype TS rendering — covers CORE-03 newtype gotcha
- [ ] Snapshot preservation verification after `run` decomposition — covers CORE-01 no-regression

*Existing infra covers the regression suite (608 tests, the byte-for-byte snapshot gate). New tests are additive for the new crate + the new abstractions.*

## Security Domain

> `security_enforcement` config not located; included for completeness. This phase ships **no production UI** and **no network surface** — security scope is narrow.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Single-user local desktop tool; no auth. |
| V3 Session Management | no | No sessions. |
| V4 Access Control | partial | Tauri **capabilities/permissions** (`capabilities/main.json`) restrict which commands the webview can invoke — define a minimal allowlist; don't enable broad fs/shell plugins this phase. |
| V5 Input Validation | yes | Domain already validates via newtypes (`validate_identifier`) + `Config::save_checked`. The IPC boundary inherits this — no JS-side bypass (constraint: no JS business logic). |
| V6 Cryptography | no | No new crypto. SHA-256 hashing is content-integrity, not security; unchanged. |

### Known Threat Patterns for Tauri 2 desktop

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Overbroad webview→Rust command surface | Elevation of Privilege | Tauri 2 capabilities: explicit per-window command allowlist in `capabilities/main.json`. Only expose the spike's `get_status` this phase. |
| Loading remote/untrusted content in webview | Tampering | `frontendDist` is local bundled assets only; no remote `devUrl` in production. CSP via tauri.conf.json. |
| `tauri.conf.json` enabling dangerous plugins (shell, fs with wide scope) | EoP | Don't add `tauri-plugin-shell`/`-fs` this phase; DIST-05's "show in terminal" is Phase 31. |
| Hardened-runtime entitlement creep | EoP | DIST-04 (Phase 31) mandates minimum entitlements, library validation ON. Not this phase, but don't add entitlements speculatively. |

**This phase's security posture:** spike-only, local-only, single command exposed. The real surface lands incrementally in later phases; keep capabilities minimal now so they're easy to audit later.

## Sources

### Primary (HIGH confidence)
- Codebase: `crates/tome/src/{lib.rs,manifest.rs,status.rs,remove.rs,discover.rs,validation.rs,main.rs,lint.rs}`, `Cargo.toml`, `crates/tome/Cargo.toml` — current shapes, the proven `LintFailed` downcast pattern, transparent newtypes, `io::Error` plan fields, existing manifest migration tests.
- `specta-rs/tauri-specta` `examples/app/src-tauri/{main.rs,build.rs,Cargo.toml}` (fetched 2026-05-25, rc.25 era) — canonical v2 Builder + export + events + io::Error-skip pattern.
- crates.io API (`/api/v1/crates/{tauri,specta,tauri-specta}`) — verified latest versions: tauri 2.11.2, specta/tauri-specta 2.0.0-rc.25, specta-typescript 0.0.12 (May 2026).
- `cargo search` — confirmed crate existence + versions on the registry.
- docs.rs/anyhow `Error` — `context()` wraps (preserves downcastability), `chain()` iterates cause chain, `downcast_ref` semantics.
- serde.rs container-attrs — `#[serde(from = "...")]` migration idiom.
- v2.tauri.app prerequisites — macOS needs only Xcode (no extra system libs).

### Secondary (MEDIUM confidence)
- DeepWiki specta-rs/tauri-specta (getting started, API reference) — Builder/export/mount_events overview.
- Tauri v2 project-structure + configuration docs — workspace member layout, `frontendDist`/`beforeDevCommand`/`devUrl`.

### Tertiary (LOW confidence — flagged)
- WebSearch on cargo-dist per-package opt-out key (A1) — verify against cargo-dist 0.30.3 docs / `cargo dist plan` at plan time.
- `spawn_blocking` offload pattern (A3) — standard guidance, confirm in Phase 27 when real sync UI lands.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH on versions (crates.io-verified) — MEDIUM on rc stability (pre-1.0, will churn).
- Architecture (presenter decomposition, ProgressSink, TomeError): HIGH — patterns either already exist in-repo or follow verified anyhow/serde idioms.
- tauri-specta v2 wiring: MEDIUM-HIGH — verified against the live example app, but rc API may shift between rc.25 and a future rc/stable.
- Pitfalls: HIGH — each is grounded in a specific codebase fact (io::Error fields, name collision, transparent newtypes) or verified external behavior (build.rs limitation, AppHandle thread-safety).

**Research date:** 2026-05-25
**Valid until:** ~2026-06-08 (14 days — specta/tauri-specta are in active rc churn; re-verify versions at plan time). Codebase-grounded findings (decomposition, downcast, migration) are stable longer.

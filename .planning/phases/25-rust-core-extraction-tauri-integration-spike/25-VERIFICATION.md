---
phase: 25-rust-core-extraction-tauri-integration-spike
verified: 2026-05-27T00:00:00Z
status: human_needed
score: 10/10 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run `cargo tauri dev` pointing at the React ui/ scaffold and confirm it renders a live StatusReport from the real tome_home"
    expected: "The app window shows configured=true, library dir path, directory list with role badges, skill count, last sync time, health count — populated from actual on-disk data, not fixtures"
    why_human: "Cannot verify webview rendering or live Tauri app execution with grep. The Rust boundary and bindings.ts are verified programmatically, but the actual UI render requires a display/webview."
---

# Phase 25: Rust Core Extraction + Tauri Integration Spike — Verification Report

**Phase Goal:** Reshape `crates/tome` so its domain operations return structured types callable from any front-end, add `crates/tome-desktop` as a sibling Tauri 2 app crate, wire `specta` + `tauri-specta` bindings, add a progress-event channel and a stable `TomeError` boundary, and pick the frontend framework via a 3-way spike. NO production UI ships in this phase — the spike apps are throwaway except the winner's scaffold.
**Verified:** 2026-05-27
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `SkillEntry` uses `SkillOwnership` enum; old manifests deserialize via `SkillEntryRepr` migration-on-read | ✓ VERIFIED | `rg "enum SkillOwnership" manifest.rs` → 1; `#[serde(from = "SkillEntryRepr")]` at line 171; 4 migration test functions at lines 776, 796, 811, 860 |
| 2 | `bindings` feature compiles specta derives; default CLI build has zero specta cost | ✓ VERIFIED | `cargo build -p tome --features bindings` succeeds; `cargo tree -p tome -e normal \| rg specta` returns nothing |
| 3 | `crates/tome-desktop` exists as Tauri 2 workspace member, excluded from cargo-dist | ✓ VERIFIED | `Cargo.toml` `members = ["crates/*"]` glob includes it; `[package.metadata.dist] dist = false` in tome-desktop/Cargo.toml; `cargo dist plan` output contains no `tome-desktop` artifact |
| 4 | `gen-bindings` bin shares `make_builder()` with `main.rs`; exactly ONE `bindings.ts` tree-wide; CI freshness gate present | ✓ VERIFIED | Both `main.rs` and `gen-bindings.rs` call `tome_desktop::make_builder()`; `fd '^bindings\.ts$' crates/` returns exactly one file; CI `bindings` job runs `cargo run -p tome-desktop --bin gen-bindings` + `git diff --exit-code`; local gate passes (bindings fresh) |
| 5 | `progress.rs` defines `ProgressSink/ProgressEvent/SyncStage/CancelToken` (no tokio); threaded through `sync()` | ✓ VERIFIED | All types present in progress.rs; `rg "tokio" progress.rs` returns only comment text (no dep); `sync()` signature at lib.rs:1720 takes `sink: &dyn ProgressSink, cancel: &CancelToken`; `rg -c "sink\.emit" lib.rs` = 13; `rg -c "is_cancelled" lib.rs` = 6 |
| 6 | `TauriEventSink` bridges `ProgressEvent` to typed `SyncProgress` event with saturating casts | ✓ VERIFIED | `impl ProgressSink for TauriEventSink` in sink.rs:54; `SyncProgress.stage: SyncStage` typed (not stringly); `u32::try_from(n).unwrap_or(u32::MAX)` saturating cast in both `saturate_usize` and `saturate_u64` |
| 7 | `TomeError/ErrorCode` IPC boundary in tome-desktop with `From<anyhow::Error>` chain-downcast; `DomainErrorKind` sentinels in `crates/tome` | ✓ VERIFIED | `crates/tome-desktop/src/error.rs`: `struct TomeError`, `enum ErrorCode` (7 variants with ALL + exhaustiveness guard); `From<anyhow::Error>` walks `err.chain().find_map(downcast_ref::<DomainTagged>)` → `ErrorCode`; `crates/tome/src/errors.rs`: `enum DomainErrorKind` via thiserror; re-exported as `tome::DomainErrorKind` + `tome::DomainTagged`; `bindings.ts` contains `TomeError` and `ErrorCode` |
| 8 | All GUI-callable domain fns are `pub`; `list::collect` extracted; `plan` fns promoted | ✓ VERIFIED | `pub fn gather` (status.rs:95); `pub fn diagnose` (doctor.rs); `pub fn collect` (list.rs:42); `pub fn lint_library/lint_skill` (lint.rs); `pub fn plan` in all 4 of remove.rs/reassign.rs/relocate.rs/eject.rs (zero `pub(crate) fn plan` remaining) |
| 9 | Frontend framework decided (React); ADR at `.planning/research/v1.0-frontend-framework-decision.md`; D-GUI-04 updated in active REQUIREMENTS.md | ✓ VERIFIED | ADR exists with 4-criteria scoring table (React 16, Svelte 16, Solid 15); D-GUI-04 row in `.planning/REQUIREMENTS.md` names React with pointer to ADR; archive copy `.planning/milestones/v1.0-REQUIREMENTS.md` also updated |
| 10 | Losing spikes deleted; only winning React scaffold survives in `ui/`; full test suite green with no snapshot drift | ✓ VERIFIED | `ui-react/`, `ui-solid/`, `ui-svelte/` do not exist; `crates/tome-desktop/ui/` contains React scaffold; `cargo test --all` → 872+ tests, 0 failures; `git diff --stat crates/tome/tests/snapshots/` shows no changes |

**Score:** 10/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/tome/src/manifest.rs` | `SkillOwnership` enum + migration via `SkillEntryRepr` | ✓ VERIFIED | `enum SkillOwnership` at line 145; `#[serde(from = "SkillEntryRepr")]`; 5 migration tests |
| `crates/tome/Cargo.toml` | `bindings = ["dep:specta"]` feature | ✓ VERIFIED | Exact line present; `specta = { version = "=2.0.0-rc.25", features = ["derive"], optional = true }` |
| `crates/tome/src/progress.rs` | `ProgressSink`, `ProgressEvent`, `SyncStage`, `CancelToken`, `NullSink`, `RecordingSink` | ✓ VERIFIED | All 6 types present; unit tests cover CancelToken, RecordingSink ordering, NullSink; `SyncStage::ALL` + `const _` exhaustiveness guard |
| `crates/tome/src/lib.rs` | `IndicatifSink`, `sync(... sink, cancel)` signature, presenter decomposition | ✓ VERIFIED | `struct IndicatifSink` at line 214; sync at 1720 takes `sink` + `cancel`; all domain fns pub |
| `crates/tome-desktop/` | Tauri 2 workspace crate with `publish = false` + dist opt-out | ✓ VERIFIED | All scaffold files present; `publish = false`; `[package.metadata.dist] dist = false` |
| `crates/tome-desktop/src/bin/gen-bindings.rs` | Shares `make_builder()` with `main.rs` | ✓ VERIFIED | Both call `tome_desktop::make_builder()`; exporter writes to `crates/tome-desktop/ui/src/bindings.ts` |
| `crates/tome-desktop/ui/src/bindings.ts` | Generated TS bindings containing `StatusReport`, `TomeError`, `ErrorCode`, `SyncStage` | ✓ VERIFIED | All 4 types present; `SyncProgress.stage` is typed `SyncStage` (not `string`); transparent newtypes render as `string`; freshness gate passes |
| `crates/tome-desktop/src/sink.rs` | `impl ProgressSink for TauriEventSink` | ✓ VERIFIED | Present at line 54; typed `SyncStage` bridged (no stringly conversion); saturating usize/u64→u32 casts |
| `crates/tome-desktop/src/error.rs` | `struct TomeError`, `enum ErrorCode`, `From<anyhow::Error>` | ✓ VERIFIED | All present; downcast via `DomainTagged` wrapper; 5 unit tests covering sentinel→code, no-sentinel→Internal, context flattening |
| `crates/tome/src/errors.rs` | `enum DomainErrorKind` (thiserror) | ✓ VERIFIED | 6 variants; re-exported at crate root as `tome::DomainErrorKind` + `tome::DomainTagged`; `WithDomainKind` trait for attaching sentinels |
| `.planning/research/v1.0-frontend-framework-decision.md` | Scoring table + ADR | ✓ VERIFIED | 4-criteria table; React/Solid/Svelte scored; invalidation conditions section present |
| `.github/workflows/ci.yml` | `bindings` freshness job + `desktop-build` macOS job | ✓ VERIFIED | `bindings` job: `cargo run -p tome-desktop --bin gen-bindings` + `git diff --exit-code`; `desktop-build` job on macos-latest: `cargo build -p tome-desktop` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/tome-desktop/src/commands.rs` | `tome::status::gather` | `#[tauri::command] get_status` | ✓ WIRED | `commands.rs` imports and calls `tome::status::gather` via `load_context()`; returns `Result<StatusReport, TomeError>` |
| `.github/workflows/ci.yml` | `crates/tome-desktop/ui/src/bindings.ts` | `gen-bindings` + `git diff --exit-code` | ✓ WIRED | Both steps in `bindings` job; local run confirms gate passes |
| `crates/tome/src/lib.rs::sync` | `crates/tome/src/progress.rs::ProgressSink` | `sink.emit(ProgressEvent::SyncStage...)` | ✓ WIRED | 13 `sink.emit` calls in lib.rs; stage boundary cancellation checks (6 `is_cancelled` calls) |
| `crates/tome-desktop/src/error.rs` | `tome::DomainTagged` / `tome::DomainErrorKind` | `err.chain().find_map(downcast_ref::<DomainTagged>)` | ✓ WIRED | Typed-sentinel downcast in `From<anyhow::Error>` implementation |
| `crates/tome/src/lib.rs::make_builder` | `crates/tome-desktop/src/bin/gen-bindings.rs` | shared `make_builder()` | ✓ WIRED | Both call `tome_desktop::make_builder()`; single source of truth |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `commands.rs::get_status` | `StatusReport` | `tome::status::gather(&config, &paths)` after `Config::load_or_default` + `TomePaths::new` from user's actual tome_home | Yes — reads real manifest/config from disk | ✓ FLOWING |
| `ui/src/App.tsx` | `status: StatusReport_Serialize` | `commands.getStatus()` (tauri IPC call to get_status) | Yes — typed call to real Rust backend | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo build -p tome-desktop` compiles | `cargo build -p tome-desktop` | `Finished dev profile ... in 0.65s` | ✓ PASS |
| gen-bindings writes fresh bindings.ts | `cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts` | exit 0, no diff | ✓ PASS |
| tome builds with bindings feature | `cargo build -p tome --features bindings` | `Finished dev profile` | ✓ PASS |
| specta absent from default CLI tree | `cargo tree -p tome -e normal \| rg specta` | no output | ✓ PASS |
| cargo dist excludes tome-desktop | `cargo dist plan` | no `tome-desktop` in output | ✓ PASS |
| Full test suite passes | `cargo test --all` | 872+ tests, 0 failed, no snapshot drift | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| CORE-01 | 25-01, 25-03 | Domain operations return structured types; lib.rs decomposed into CLI presenters | ✓ SATISFIED | `pub fn gather/diagnose/collect/lint_library/lint_skill/plan` all verified; `list::collect` extracted; `IndicatifSink` presenter in lib.rs |
| CORE-02 | 25-04 | `crates/tome-desktop` Tauri 2 workspace member, path dep on crates/tome | ✓ SATISFIED | Crate exists; `tome = { path = "../tome", features = ["bindings"] }`; workspace glob picks it up |
| CORE-03 | 25-04, 25-06 | specta+tauri-specta generate committed bindings.ts; CI freshness gate | ✓ SATISFIED | bindings.ts committed with all boundary types; CI gate installed; local freshness gate passes |
| CORE-04 | 25-02, 25-03, 25-04 | ProgressSink/ProgressEvent/SyncStage/CancelToken (no tokio); TauriEventSink bridges to typed SyncProgress | ✓ SATISFIED | All types in progress.rs; no tokio dep; sync() wired; TauriEventSink in sink.rs |
| CORE-05 | 25-05 | TomeError/ErrorCode IPC boundary; DomainErrorKind sentinels via downcast | ✓ SATISFIED | error.rs has TomeError+ErrorCode; errors.rs has DomainErrorKind+DomainTagged; bindings.ts contains TomeError+ErrorCode |
| D-GUI-04 | 25-06 | Frontend framework chosen; ADR recorded; losing spikes deleted | ✓ SATISFIED | React chosen; ADR at .planning/research/v1.0-frontend-framework-decision.md; ui-react/ui-solid/ui-svelte all deleted; REQUIREMENTS.md updated |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | No TBD/FIXME/XXX markers found in phase-modified files | — | — |
| crates/tome/src/progress.rs | 39-40 | Comment references `unexpected_cfgs` warning about `feature = "bindings"` from a wave-merging context | ℹ Info | Harmless historical note; feature exists and compiles correctly |

No `TBD`, `FIXME`, or `XXX` debt markers found in any phase-modified source files. No stub implementations. No hollow props or disconnected data sources.

### Human Verification Required

#### 1. React UI Renders Live StatusReport Data

**Test:** With the dev machine's real tome_home configured, run `cargo tauri dev` (or `cargo tauri build`) from `crates/tome-desktop/` (with Node/npm installed, `npm install` run in `ui/`) and confirm the app window opens.
**Expected:** The window shows a populated StatusReport dashboard — configured=true, real library dir path, the actual configured directories listed with role badges (managed/synced/source/target), real skill count, last sync timestamp, health issue count. The `if (res.status === "ok")` React branch renders (not the error banner), confirming the real `get_status` command returned successfully.
**Why human:** Webview rendering and the Tauri app startup require a macOS display and the npm/Node toolchain. The Rust boundary (`cargo build -p tome-desktop` passes), bindings.ts is fresh, and App.tsx calls `commands.getStatus()` from verified bindings — but the actual UI render with a live webview cannot be checked with grep or cargo.

---

## Gaps Summary

No gaps. All 10 observable truths verified against the codebase. The only remaining item is a human UI smoke test to confirm the webview renders live data from the real tome_home.

---

## Notable Implementation Decisions (for next-phase context)

1. **DomainTagged wrapper (not `.with_context(|| DomainErrorKind::X)`)**: The RESEARCH proposed `.with_context()` for attaching sentinels, but this does not work — anyhow context values are Display-only and not recoverable via `downcast_ref`. The implementation uses a transparent `DomainTagged` wrapper error that delegates Display to the underlying error's top-level message and skips it in `source()`, preserving byte-for-byte CLI output while being downcastable. Documented in errors.rs module docs.

2. **`bindings.ts` transparent-newtype check**: `SkillName` and `DirectoryName` are not used in the CORE-05-phase boundary types (StatusReport uses `String` fields); the transparent-newtype Pitfall 6 did not surface. If future phases expose newtypes directly, `#[specta(transparent)]` annotations may be needed.

3. **`io::Error` → `String` in `RemoveFailure`**: The `pub error` field was changed from `std::io::Error` to `String` (stringified at constructor call sites) to allow `RemovePlan` to cross the specta IPC boundary. This is deliberate and recorded in 25-01 PLAN as a field-shape sub-decision.

4. **React scaffold imports bindings via `./bindings`** (relative, not `@bindings` alias): After collapsing the winner into `ui/`, the app source and bindings.ts are co-located, so the Vite alias is not needed. The `vite.config.ts` documents this explicitly.

---

_Verified: 2026-05-27_
_Verifier: Claude (gsd-verifier)_

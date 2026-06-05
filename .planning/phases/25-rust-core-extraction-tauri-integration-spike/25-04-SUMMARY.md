---
phase: 25-rust-core-extraction-tauri-integration-spike
plan: 04
subsystem: infra
tags: [tauri, specta, tauri-specta, typescript-bindings, cargo-dist, ipc, ci]

# Dependency graph
requires:
  - phase: 25-01
    provides: "tome `bindings` cargo feature + gated specta::Type derives on cross-boundary types"
  - phase: 25-03
    provides: "ProgressSink/ProgressEvent/SyncStage/CancelToken vocabulary + pub plan() fns"
provides:
  - "crates/tome-desktop Tauri 2 crate (workspace member, path dep on tome+bindings)"
  - "make_builder() single source of truth for the IPC command/event registry"
  - "get_status #[tauri::command] returning a real tome::status::StatusReport"
  - "TauriEventSink: ProgressSink impl bridging ProgressEvent -> SyncProgress over AppHandle::emit"
  - "Committed ui/src/bindings.ts (Wave-3 partial snapshot, pre-TomeError)"
  - "CI bindings-freshness gate + tome-desktop macOS build job"
  - "cargo-dist opt-out so release ships only the tome CLI"
affects: [25-05, 25-06, 26-read-only-views, 27-sync-triage-ui]

# Tech tracking
tech-stack:
  added:
    - "tauri 2.11.2"
    - "tauri-build 2 (build-dep)"
    - "tauri-specta =2.0.0-rc.25 (derive, typescript)"
    - "specta =2.0.0-rc.25 (derive)"
    - "specta-typescript 0.0.12"
    - "thiserror 2 (declared for 25-05's TomeError; unused this wave)"
  patterns:
    - "make_builder() shared by main.rs (debug export) + gen-bindings bin (CI export)"
    - "Builder::dangerously_cast_bigints_to_number() at the export edge (no library type change)"
    - "TauriEventSink carries typed SyncStage enum (no stringification); saturating u32::try_from casts"
    - "per-package [package.metadata.dist] dist = false to exclude a workspace crate from cargo-dist"

key-files:
  created:
    - "crates/tome-desktop/Cargo.toml"
    - "crates/tome-desktop/build.rs"
    - "crates/tome-desktop/tauri.conf.json"
    - "crates/tome-desktop/capabilities/main.json"
    - "crates/tome-desktop/src/lib.rs"
    - "crates/tome-desktop/src/main.rs"
    - "crates/tome-desktop/src/commands.rs"
    - "crates/tome-desktop/src/sink.rs"
    - "crates/tome-desktop/src/bin/gen-bindings.rs"
    - "crates/tome-desktop/ui/src/bindings.ts"
  modified:
    - "crates/tome/src/lib.rs (status: pub(crate) -> pub)"
    - "crates/tome/src/manifest.rs (SkillEntryRepr gated specta::Type)"
    - ".github/workflows/ci.yml (bindings + desktop-build jobs)"
    - ".gitignore (ui artifacts + gen/, NOT bindings.ts)"

key-decisions:
  - "Exported bindings from a gen-bindings bin sharing make_builder() (D-07 corrected — build.rs cannot see #[tauri::command] fns)"
  - "Builder::dangerously_cast_bigints_to_number() to export StatusReport's Option<usize> counts as TS number, avoiding a library-canonical CountOrError shape change"
  - "Ran gen-bindings + bindings CI gate on macos-latest (gen-bindings links tauri/WKWebView; no Linux webkit2gtk deps, no npm)"
  - "get_status returns Result<StatusReport, String> this wave (Wave-3 snapshot); TomeError arrives in 25-05"

patterns-established:
  - "Single-source-of-truth IPC registry: make_builder() feeds both the live app and the bindings exporter so bindings.ts cannot drift from what the app exposes"
  - "Export-edge bigint policy: configure the exporter (not the domain types) when specta forbids usize/u64"

requirements-completed: [CORE-02, CORE-03, CORE-04]

# Metrics
duration: ~55min
completed: 2026-05-26
---

# Phase 25 Plan 04: tome-desktop Tauri crate + typed bindings Summary

**Stood up the `crates/tome-desktop` Tauri 2 IPC shell with a real `get_status` command, a `TauriEventSink` bridging typed `ProgressEvent`s, a `make_builder()`-driven committed `bindings.ts`, a CI freshness gate + macOS build job, and a cargo-dist opt-out keeping the release CLI-only.**

## Performance

- **Duration:** ~55 min
- **Tasks:** 3 implementation tasks (Task 0 supply-chain checkpoint pre-approved)
- **Files modified:** 16 (10 created, 4 modified, + Cargo.lock, icon)

## Accomplishments
- New `crates/tome-desktop` workspace member: path dep on `tome` with `features=["bindings"]`, tauri 2.11 + the specta trio pinned exactly (`=2.0.0-rc.25`) + `specta-typescript 0.0.12`.
- `get_status` `#[tauri::command]` resolves the user's real `tome_home` (default config path + default `tome_home`, mirroring the CLI's flag-free path) and returns a real `tome::status::gather` `StatusReport`.
- `TauriEventSink` implements `tome::progress::ProgressSink`, bridging each `ProgressEvent` to a typed `SyncProgress { stage: SyncStage, current, total }` over `AppHandle::emit` — `SyncStage` crosses the boundary as a TS string-union (no stringification); numeric casts are saturating (`u32::try_from(..).unwrap_or(u32::MAX)`).
- `make_builder()` is the single source of truth for the command/event registry, shared by `main.rs` (debug-only export) and the `gen-bindings` bin (CI export). `bindings.ts` is generated + committed.
- CI gains a `bindings` freshness gate (`gen-bindings` + `git diff --exit-code`) and a `desktop-build` macOS job; `cargo dist plan` lists only the `tome` CLI artifact.

## Task Commits

1. **Task 1: tome-desktop crate scaffold + cargo-dist opt-out** - `3154289` (feat)
2. **Task 2: get_status + TauriEventSink + make_builder + gen-bindings + committed bindings.ts** - `0d324fa` (feat)
3. **Task 3: CI bindings-freshness gate + tome-desktop macOS build job** - `e851fd8` (ci)

## Confirmed specta-trio versions (Task 0 checkpoint, pre-approved)
- `specta = "=2.0.0-rc.25"`, `tauri-specta = "=2.0.0-rc.25"`, `specta-typescript = "0.0.12"` — installed exactly as pinned; the lockstep rc.25 set is current. `tauri = "2.11"` resolved to **2.11.2**.

## cargo-dist opt-out key used
- `[package.metadata.dist]\ndist = false` on `crates/tome-desktop/Cargo.toml` (verified self-consistent: `dist plan` via cargo-dist **0.30.3** lists only `tome`'s three target tarballs + Homebrew formula, no `tome-desktop` artifact). `release.yml` was NOT hand-edited; `cargo dist init` was NOT required (only per-package metadata changed, workspace-level dist config is unchanged).

## `#[specta(transparent)]` fixups
- **None were needed.** The transparent newtypes (`SkillName`, `DirectoryName`, `ContentHash`) are **not reachable from `StatusReport`** this wave — its fields use plain `String` (`name`, `path`, `role_description`) and the `DirectoryRole` enum (which renders as a clean TS string-union). No newtype mis-rendered, so no `#[specta(transparent)]` annotation was required. The transparent-newtype acceptance check is deferred until a future plan exposes a command/event whose reachable graph includes those newtypes directly.

## Wave-3 partial-snapshot note (REQUIRED)
The committed `crates/tome-desktop/ui/src/bindings.ts` is an **INTENTIONAL Wave-3 partial snapshot**. At this wave `get_status` returns `Result<StatusReport, String>` (a `// TODO(25-05): TomeError` marker is in `commands.rs`) because the `TomeError`/`ErrorCode` error boundary (CORE-05) does not land until **Wave 4, plan 25-05 Task 2**. 25-05 Task 2 will **regenerate + re-commit** `bindings.ts` once `TomeError` enters the boundary — that re-commit is an expected, legitimate change, NOT a freshness violation. The CI freshness gate is installed here and is internally consistent against the Wave-3 snapshot (it passes locally now); it is first *meaningfully* validated against the final boundary shape at phase end after 25-05.

## bindings.ts content verification (Wave-3)
- `StatusReport` present (`getStatus` returns `Result<StatusReport_Serialize, string>` — `string` error confirms the pre-`TomeError` snapshot).
- `SyncProgress` carries `stage: SyncStage`, and `SyncStage` renders as a typed string-union (`"Reconcile" | "Discover" | ... | "Save"`) — WARNING 4's typed option satisfied (front-end can discriminate stages, not a bare `string`).
- `StatusReport` count fields render as TS `number` (via the bigint→number export cast), not `bigint`.

## Decisions Made
- **D-07 correction honored:** export from a `gen-bindings` bin + `main.rs` (`#[cfg(debug_assertions)]`), not `build.rs` (build scripts can't see the registered commands). Both share `make_builder()`.
- **Bigint export at the edge:** `Builder::dangerously_cast_bigints_to_number()` keeps `StatusReport`'s `Option<usize>` counts as TS `number`. These are small bounded tallies (skill/health counts) so the cast is lossless in practice, and it avoids changing the library-canonical `CountOrError` shape (which STATE.md forbids mid-milestone without explicit decision). This is the documented tauri-specta escape hatch for the default usize-forbidden rule.
- **CI placement:** `bindings` and `desktop-build` jobs run on `macos-latest` because `gen-bindings` links `tauri` (system WKWebView), avoiding Linux `webkit2gtk` dev deps and any npm/vite step (Q-A/Q-B).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `tome --features bindings` failed to compile (SkillEntryRepr missing specta::Type)**
- **Found during:** Task 2 (first build of tome-desktop, which enables `tome/bindings`)
- **Issue:** `SkillEntry` carries `#[serde(from = "SkillEntryRepr")]` + a gated `derive(specta::Type)` (added in 25-01). specta honors serde's `from` and requires the source type (`SkillEntryRepr`) to also implement `Type`; it did not, so `cargo build -p tome --features bindings` failed with E0277. This is a latent issue introduced by 25-01 that only surfaces on the first `bindings` build — which is this plan's responsibility.
- **Fix:** Added `#[cfg_attr(feature = "bindings", derive(specta::Type))]` to `SkillEntryRepr`. `SkillEntry` is not reachable from any registered command/event this wave, so this type does not appear in `bindings.ts`; the derive exists only to keep the feature compiling.
- **Files modified:** `crates/tome/src/manifest.rs`
- **Verification:** `cargo build -p tome --features bindings` and `cargo clippy -p tome --features bindings --all-targets -- -D warnings` pass; manifest migration unit tests (62) still pass.
- **Committed in:** `0d324fa` (Task 2 commit)

**2. [Rule 3 - Blocking] `status` module was `pub(crate)`, blocking cross-crate `tome::status::gather`**
- **Found during:** Task 2 (carry-forward from 25-03, anchored at the cross-crate call)
- **Issue:** `get_status` calls `tome::status::gather` and consumes `tome::status::StatusReport`, but the `status` module was `pub(crate)`.
- **Fix:** Widened `mod status` to `pub` in `crates/tome/src/lib.rs` (minimal — only the structured `gather`/`StatusReport` surface; the CLI presenter `status::show` stays in-crate).
- **Files modified:** `crates/tome/src/lib.rs`
- **Verification:** `cargo build -p tome-desktop` succeeds; no new clippy warnings.
- **Committed in:** `0d324fa` (Task 2 commit)

**3. [Rule 3 - Blocking] specta forbids exporting `usize`/`u64` to TS (CountOrError.count)**
- **Found during:** Task 2 (first `gen-bindings` run panicked on `CountOrError.count: Option<usize>`)
- **Issue:** specta-typescript's default exporter forbids BigInt-style types (usize/u64) to guard precision; `StatusReport` reaches `usize` count fields.
- **Fix:** `Builder::dangerously_cast_bigints_to_number()` in `make_builder()` (single point; applies to both `main.rs` and `gen-bindings`). Counts are small bounded integers — lossless in practice — and the library type is left unchanged.
- **Files modified:** `crates/tome-desktop/src/lib.rs`
- **Verification:** `gen-bindings` runs clean; counts render as TS `number`.
- **Committed in:** `0d324fa` (Task 2 commit)

**4. [Rule 3 - Blocking] `anyhow` not a direct dependency of tome-desktop**
- **Found during:** Task 2 (`load_context` returns `anyhow::Result`)
- **Issue:** `tome`'s public fns return `anyhow::Result`; `commands.rs` needs `anyhow` to propagate with `?`. The plan's dep list omitted it.
- **Fix:** Added `anyhow = { workspace = true }` to `crates/tome-desktop/Cargo.toml` (existing workspace dep — not a new external install).
- **Files modified:** `crates/tome-desktop/Cargo.toml`
- **Verification:** `cargo build -p tome-desktop` succeeds.
- **Committed in:** `0d324fa` (Task 2 commit)

---

**Total deviations:** 4 auto-fixed (all Rule 3 - blocking).
**Impact on plan:** All four were necessary to make the cross-crate `bindings` build compile and the exporter run. No scope creep — each is the minimal change at the build-failure site. The `manifest.rs` fix corrects a latent 25-01 omission; the `status` widening was the anticipated 25-03 carry-forward.

## Issues Encountered
- **Disk pressure during the Tauri build.** The root APFS volume hit 100% mid-build (Tauri pulls a large dep tree); `cargo clean -p tome-desktop` reclaimed headroom and the build completed. No code impact. Worth noting for 25-05/25-06 (the spike adds three frontends + node_modules).
- **Local `cargo dist` was 0.19.1** (workspace pins 0.30.3, which refuses to run under the old binary). Installed cargo-dist 0.30.3 (ships as the `dist` binary) and ran `~/.cargo/bin/dist plan` to self-verify the opt-out. No workspace dist metadata version change was made.

## User Setup Required
None - no external service configuration required. (A placeholder `icons/icon.png` was generated for `tauri::generate_context!`; real app icons are a later-phase polish item.)

## Next Phase Readiness
- **25-05 (Wave 4, error boundary):** `commands.rs` has a `// TODO(25-05): TomeError` marker and returns `Result<StatusReport, String>`. 25-05 wires `TomeError`/`ErrorCode`/`From<anyhow::Error>` at the command edge, then regenerates + re-commits `bindings.ts` (the CI freshness gate stays green on the re-commit). `thiserror 2` is already declared in `tome-desktop/Cargo.toml`.
- **25-06 (spike):** `make_builder()` + `bindings.ts` are ready for the three throwaway frontends (`ui-react`/`ui-solid`/`ui-svelte`); `.gitignore` already excludes `ui-*` node_modules/dist.
- **Threat surface:** `capabilities/main.json` exposes only `core:default` + `core:event` (no shell/fs plugins) — T-25-04-EoP mitigation in place.

---
*Phase: 25-rust-core-extraction-tauri-integration-spike*
*Completed: 2026-05-26*

## Self-Check: PASSED
- All 10 created source/config files + SUMMARY present on disk.
- All 3 task commits (`3154289`, `0d324fa`, `e851fd8`) reachable in git.

---
phase: 25-rust-core-extraction-tauri-integration-spike
plan: 05
subsystem: api
tags: [tauri, anyhow, thiserror, specta, error-boundary, ipc, downcast]

# Dependency graph
requires:
  - phase: 25-04
    provides: "tome-desktop Tauri crate, get_status command (Result<StatusReport,String> stub), make_builder()/gen-bindings, Wave-3 partial bindings.ts, CI freshness gate"
  - phase: 25-03
    provides: "git clone/update threaded through ProgressSink + CancelToken; status::gather pub surface"
provides:
  - "tome::DomainErrorKind — coarse typed sentinel enum (Validation/NotFound/Permission/Conflict/Git/Io) in crates/tome"
  - "tome::DomainTagged + WithDomainKind::with_domain_kind — transparent chain-preserving sentinel attachment"
  - "tome-desktop ErrorCode (7 variants incl Internal) + TomeError { code, message, context } + From<anyhow::Error>"
  - "get_status now returns Result<StatusReport, TomeError>"
  - "bindings.ts regenerated with TomeError + ErrorCode (supersedes 25-04 Wave-3 snapshot)"
affects: [26-read-only-views, 27-sync-triage-ui, 28-configuration-ui]

# Tech tracking
tech-stack:
  added: ["thiserror 2 (crates/tome)"]
  patterns:
    - "Typed-sentinel-through-anyhow downcast classification at the IPC boundary (D-13/D-14)"
    - "Transparent DomainTagged wrapper preserves the {e:#} chain byte-for-byte while staying downcastable"
    - "ErrorCode::ALL + const _ exhaustiveness guard (POLISH-04) on the boundary code enum"

key-files:
  created:
    - crates/tome/src/errors.rs
    - crates/tome-desktop/src/error.rs
  modified:
    - crates/tome/src/lib.rs
    - crates/tome/src/git.rs
    - crates/tome/src/config/mod.rs
    - crates/tome/src/config/validate.rs
    - crates/tome/Cargo.toml
    - crates/tome-desktop/src/commands.rs
    - crates/tome-desktop/src/lib.rs
    - crates/tome-desktop/ui/src/bindings.ts

key-decisions:
  - "DomainTagged transparent wrapper replaces the RESEARCH .with_context(|| kind) pattern, which is provably NOT downcastable through anyhow's chain"
  - "Boundary classifies BOTH DomainTagged (with_domain_kind sites) AND bare DomainErrorKind (direct anyhow::Error::new sites); unmatched -> Internal"
  - "Validation vs Conflict split inside Config::validate (role/type -> Validation; path-overlap -> Conflict)"

patterns-established:
  - "WithDomainKind ext trait: tag a fallible result with a DomainErrorKind at a GUI-relevant site without changing the human-readable chain"
  - "From<&DomainErrorKind> for ErrorCode is exhaustive (no _ arm) so a new domain sentinel forces a boundary mapping (T-25-05-T)"

requirements-completed: [CORE-05]

# Metrics
duration: 17min
completed: 2026-05-26
---

# Phase 25 Plan 05: TomeError IPC-boundary error classification Summary

**Coarse `TomeError { code, message, context }` classified at the Tauri edge by downcasting typed `DomainErrorKind` sentinels out of the anyhow cause chain — the domain stays `anyhow::Result` with zero CLI regression, and `bindings.ts` is regenerated to carry the final `TomeError` + `ErrorCode` shape.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-05-26T15:19:46Z
- **Completed:** 2026-05-26T15:36Z
- **Tasks:** 2
- **Files modified:** 8 created/modified (2 created)

## Accomplishments
- `crates/tome/src/errors.rs`: `DomainErrorKind` (coarse sentinels, no `Internal`), `DomainTagged` transparent wrapper, `WithDomainKind` ext trait — sentinels attached at GUI-relevant sites without changing the `{e:#}` chain.
- `crates/tome-desktop/src/error.rs`: `ErrorCode` (7 variants incl `Internal`) with `ALL` + `const _` exhaustiveness guard, exhaustive `From<&DomainErrorKind>`, `TomeError`, and `From<anyhow::Error>` that classifies via `chain()` downcast and flattens the chain into `context`.
- `get_status` now returns `Result<StatusReport, TomeError>` (stub + `TODO(25-05)` removed).
- `bindings.ts` regenerated + re-committed: now contains `TomeError` + `ErrorCode`, superseding the 25-04 Wave-3 partial snapshot; `git diff --exit-code` clean (CI freshness gate satisfied against the final boundary shape).

## GUI-relevant sites that received DomainErrorKind sentinels

| Site | Sentinel | File |
|------|----------|------|
| `Config::validate` role/type/git-field/library-is-a-file checks | `Validation` | `crates/tome/src/config/validate.rs` |
| `Config::validate` library_dir vs distribution-dir path-overlap (Cases A/B/C) | `Conflict` | `crates/tome/src/config/validate.rs` |
| `Config::load_or_default` bad explicit `--config` (parent dir missing) | `NotFound` | `crates/tome/src/config/mod.rs` |
| `git::clone_repo` (all failures) | `Git` | `crates/tome/src/git.rs` |
| `git::update_repo` (all failures) | `Git` | `crates/tome/src/git.rs` |

`Permission` and `Io` are defined in the enum and mapped at the boundary but were not attached at a specific site this plan — they are available for the small additional sites later GUI phases reach (deliberately kept to a small set per D-14; no over-attachment).

## Task Commits

1. **Task 1: DomainErrorKind sentinels + GUI-site tagging** — `cd0d888` (feat)
2. **Task 2: TomeError/ErrorCode boundary + regenerate bindings.ts** — `aa6513b` (feat)

_TDD note: both tasks were sentinel/boundary infrastructure; tests were written alongside the types in each file and verified green before commit (4 tests in `errors.rs`, 6 in `error.rs`)._

## Files Created/Modified
- `crates/tome/src/errors.rs` (created) - `DomainErrorKind`, `DomainTagged`, `WithDomainKind`, 4 unit tests.
- `crates/tome-desktop/src/error.rs` (created) - `ErrorCode`, `TomeError`, `From<anyhow::Error>`, exhaustiveness guard, 6 unit tests.
- `crates/tome/src/lib.rs` - `pub mod errors;` + `pub use errors::{DomainErrorKind, DomainTagged};`
- `crates/tome/src/git.rs` - `clone_repo`/`update_repo` tag failures with `Git` via inner-fn delegation.
- `crates/tome/src/config/validate.rs` - split `validate()` into `validate_roles_and_fields` (`Validation`) + `validate_no_path_overlap` (`Conflict`).
- `crates/tome/src/config/mod.rs` - bad `--config` path tagged `NotFound` (preserving first-run tolerance).
- `crates/tome/Cargo.toml` - `thiserror = "2"`.
- `crates/tome-desktop/src/commands.rs` - `get_status -> Result<StatusReport, TomeError>` via `map_err(TomeError::from)`.
- `crates/tome-desktop/src/lib.rs` - `pub mod error;`
- `crates/tome-desktop/ui/src/bindings.ts` - regenerated with `TomeError` + `ErrorCode`.

## Decisions Made
- **`DomainTagged` wrapper over `.with_context(|| kind)`.** The RESEARCH Code Example (lines 401-406) used `.with_context(|| DomainErrorKind::NotFound)`. Verified empirically (probe crate) that a thiserror type used as an anyhow *context value* is wrapped Display-only and is **not** recoverable via `chain().find_map(downcast_ref::<DomainErrorKind>())` (returns `None`). Replaced with a concrete `DomainTagged { kind, source }` wrapper whose `Display` delegates to the underlying top message and whose `source()` skips that link — so the `{e:#}` chain reads byte-for-byte identical (verified for single- and multi-link chains) while remaining downcastable. See Deviations.
- **Dual-path classification at the boundary.** `From<anyhow::Error>` downcasts `DomainTagged` first (the `with_domain_kind` sites), then falls back to a bare `DomainErrorKind` (a future `anyhow::Error::new(kind)` site — verified that path *is* chain-downcastable). This keeps the literal `downcast_ref::<...DomainErrorKind>` classification (D-14) genuine and exercised.
- **Validation/Conflict split in `validate()`.** Role/type/git-field failures carry `Validation`; the library_dir/distribution path-overlap (Cases A/B/C) carry `Conflict`, matching the plan's "path-overlap collisions -> Conflict".

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] RESEARCH `.with_context(|| DomainErrorKind::X)` sentinel pattern is not downcastable**
- **Found during:** Task 1 (DomainErrorKind sentinels)
- **Issue:** The RESEARCH Code Examples (lines 401-406, 412-414) and the plan's `key_links.via` both specified attaching the sentinel via `.with_context(|| DomainErrorKind::X)` and recovering it via `err.chain().find_map(|c| c.downcast_ref::<DomainErrorKind>())`. Empirically (isolated probe crate) anyhow wraps a context *value* as a Display-only layer; `chain().find_map(downcast_ref::<DomainErrorKind>())` returns `None`. The documented pattern silently fails to classify — every error would fall to `Internal`, defeating CORE-05.
- **Fix:** Introduced a concrete `DomainTagged { kind, source }` wrapper attached via `anyhow::Error::new(...)` behind a `WithDomainKind::with_domain_kind` ext trait. The wrapper is transparent: `Display` delegates to the underlying top message, `source()` skips that already-printed link — so the `{e:#}` chain is byte-for-byte unchanged (no CLI regression) while `chain().find_map(downcast_ref::<DomainTagged>())` recovers the `kind`. The boundary also still downcasts a bare `DomainErrorKind` (which *is* chain-downcastable when attached via `anyhow::Error::new`), keeping the D-14 `downcast_ref::<DomainErrorKind>` path real.
- **Files modified:** `crates/tome/src/errors.rs`, `crates/tome-desktop/src/error.rs`
- **Verification:** `errors.rs` tests assert the sentinel survives layered `.context()` and that the `{:#}` rendering is identical to the un-tagged error (single- and multi-link); `error.rs` tests pin each sentinel->code mapping, the bare-sentinel path, no-sentinel->Internal, and no-duplicate-link in `context`. `cargo test --all` green; CLI snapshots unchanged.
- **Committed in:** `cd0d888` (Task 1) + `aa6513b` (Task 2)

---

**Total deviations:** 1 auto-fixed (1 bug in the documented pattern)
**Impact on plan:** The deviation is a corrected mechanism for the same decision (D-13/D-14) — the boundary still classifies via typed-sentinel downcast (not string-matching), still falls back to `Internal`, still preserves the full chain in `context`, and the domain stays `anyhow::Result`. No scope creep; all acceptance criteria (incl. `downcast_ref::<.*DomainErrorKind>` count = 1, bindings contain `TomeError` + `ErrorCode`, freshness gate clean) met.

## Issues Encountered
- The bindings re-commit is the expected, legitimate supersession of the 25-04 Wave-3 partial snapshot (documented in the 25-04 SUMMARY and the plan's `bindings_regen_expectation`). After regeneration the committed `bindings.ts` matches the generator output (`git diff --exit-code` clean).

## Quality Gates
- `cargo fmt --all -- --check` — clean
- `cargo clippy -p tome --all-targets -- -D warnings` — clean
- `cargo clippy -p tome --all-targets --features bindings -- -D warnings` — clean
- `cargo clippy -p tome-desktop --all-targets -- -D warnings` — clean
- `cargo build -p tome-desktop` — ok
- `cargo test --all` — 26 result blocks OK, 0 FAILED (incl. 4 `errors::` + 6 `error::` tests)
- `cargo run -p tome-desktop --bin gen-bindings` + `git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts` — clean
- CLI snapshots (`crates/tome/tests/snapshots/`) — unchanged (no regression)

## Threat surface scan
No new security-relevant surface beyond the plan's `<threat_model>`. The single trust boundary (Rust error -> webview) is exactly what `TomeError` carries; mitigations T-25-05-S (typed-sentinel classification + pinned mapping tests) and T-25-05-T (`ErrorCode::ALL` + `const _` guard + exhaustive `From<&DomainErrorKind>`) are implemented. T-25-05-I (path disclosure) is `accept` per the register (single-user local tool; GUI surfaces no more than the CLI's `{e:#}`).

## Next Phase Readiness
- The IPC boundary error shape is final for the milestone: `TomeError`/`ErrorCode` are stable, additive-growth types in `bindings.ts`. Phase 26 (read-only views) can pattern-match `result.error.code` and render `result.error.context` as a details view.
- Future GUI-reachable commands should `map_err(TomeError::from)` and, where a meaningful classification exists, attach a `DomainErrorKind` sentinel at the domain failure site via `WithDomainKind::with_domain_kind` (`Permission`/`Io` sites can be added as those paths surface in the GUI).
- No blockers.

---
*Phase: 25-rust-core-extraction-tauri-integration-spike*
*Completed: 2026-05-26*

## Self-Check: PASSED
- Files: `crates/tome/src/errors.rs`, `crates/tome-desktop/src/error.rs`, `crates/tome-desktop/ui/src/bindings.ts`, `25-05-SUMMARY.md` — all FOUND.
- Commits: `cd0d888` (Task 1), `aa6513b` (Task 2) — both FOUND.

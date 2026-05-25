---
phase: 25-rust-core-extraction-tauri-integration-spike
plan: 01
subsystem: api
tags: [specta, serde, tauri, type-bridge, manifest-migration, skill-ownership, ipc-boundary]

# Dependency graph
requires:
  - phase: 14-unowned-library-lifecycle
    provides: "SkillEntry source_name/previous_source flat fields + new_unowned constructor (the shape this plan replaces)"
  - phase: 11-library-canonical-core
    provides: "Option<DirectoryName> Unowned schema + serde(default) manifest tolerance"
provides:
  - "SkillOwnership { Owned { source }, Unowned { last_owner } } enum on SkillEntry — TS-discriminated-union ownership model (#542 / D-08)"
  - "Migration-on-read via SkillEntryRepr + #[serde(from)] — old manifests still parse; reads both old flat shape AND new enum shape (round-trip safe)"
  - "source_name()/previous_source() accessor methods mirroring the removed flat fields"
  - "bindings cargo feature gating specta::Type on every cross-IPC-boundary type; default CLI build stays specta-free"
  - "RemovePlan is now pub + Serialize + specta-derivable (GUI plan-preview-confirm ready)"
  - "io::Error → String on RemoveFailure/RemoveSkillFailure (boundary-serializable)"
affects: [26-read-only-views, 27-sync-triage-ui, 29-mutating-operations-ui, tome-desktop, specta-bindings, tauri-ipc]

# Tech tracking
tech-stack:
  added: ["specta =2.0.0-rc.25 (optional, behind bindings feature)"]
  patterns:
    - "cfg_attr(feature=bindings, derive(specta::Type)) gating on boundary types (D-06 / Pattern 2)"
    - "serde(from = SkillEntryRepr) migration-on-read with a repr that tolerates old AND new shapes"
    - "Enum accessor methods (source_name()/previous_source()) to minimize call-site churn on a field-to-enum migration"
    - "specta(transparent) on validating newtypes (DirectoryName/ContentHash) so TS bindings render as string (Pitfall 6)"

key-files:
  created: []
  modified:
    - "crates/tome/src/manifest.rs (SkillOwnership enum, SkillEntryRepr, accessors, 5 migration tests)"
    - "crates/tome/Cargo.toml (bindings feature + optional specta dep)"
    - "crates/tome/src/status.rs (gated specta on StatusReport/DirectoryStatus/CountOrError)"
    - "crates/tome/src/remove.rs (RemovePlan pub+Serialize, io::Error→String, gated specta)"
    - "crates/tome/src/config/types.rs (DirectoryRole + DirectoryName specta gating)"
    - "crates/tome/src/validation.rs (ContentHash specta gating)"
    - "crates/tome/src/summary.rs (SkillSummary specta gating)"

key-decisions:
  - "New enum named SkillOwnership (not SkillProvenance) to avoid collision with discover.rs::SkillProvenance (D-08 / Pitfall 3)"
  - "SkillEntryRepr reads both old flat fields AND the new ownership enum object — required for serialize→deserialize round-trip, not just backward-compat"
  - "RemoveFailure/RemoveSkillFailure error: std::io::Error → String (deliberate boundary field-shape sub-decision; GUI cannot use a live io::Error)"
  - "LockEntry kept its flat source_name/previous_source fields — it does NOT cross the Phase-25 IPC boundary and is a separate on-disk format (tome.lock); mirroring it to an enum was out of scope"
  - "Accessor-method migration over rewriting every read site as an enum match — smaller diff, preserves call-site readability"

patterns-established:
  - "bindings-feature specta gating: every IPC-boundary type carries cfg_attr(feature=bindings, derive(specta::Type)); CLI default build proven specta-free via cargo tree -e normal"
  - "migration-on-read repr that accepts old+new shapes keeps asymmetric serde round-trip-safe"

requirements-completed: [CORE-01]

# Metrics
duration: 38min
completed: 2026-05-25
---

# Phase 25 Plan 01: SkillOwnership enum + specta bindings feature Summary

**Replaced SkillEntry's flat source_name/previous_source pair with a TS-discriminated-union `SkillOwnership` enum (migrate-on-read, round-trip-safe) and added a `bindings` cargo feature that derives `specta::Type` on every cross-IPC-boundary type while keeping the default CLI build specta-free.**

## Performance

- **Duration:** ~38 min
- **Started:** 2026-05-25 (worktree spawn)
- **Completed:** 2026-05-25
- **Tasks:** 3
- **Files modified:** 18 (16 src + Cargo.toml/Cargo.lock + 2 test files)

## Accomplishments
- `SkillOwnership { Owned { source }, Unowned { last_owner } }` enum on `SkillEntry`, serializing as `{kind, source}` / `{kind, last_owner}` tagged union (#542 / D-08).
- Migration-on-read via `SkillEntryRepr` + `#[serde(from)]`: old manifests (`source_name` string/null/absent + optional `previous_source`) still deserialize; the repr also reads the new enum shape so serialize→deserialize round-trips.
- `bindings` cargo feature (`["dep:specta"]`) + optional `specta =2.0.0-rc.25`; `cargo build -p tome --features bindings` compiles every reachable boundary type with `specta::Type`, and `cargo tree -e normal` confirms zero specta under default features.
- `RemovePlan` promoted to `pub` + `Serialize`; `RemoveFailure`/`RemoveSkillFailure` `io::Error` fields converted to `String` (boundary-serializable), resolving Pitfall 2.
- Full default-feature test suite (863 unit + all integration suites) green, clippy `-D warnings` clean on both default and `bindings` features, zero `insta` snapshot drift.

## Task Commits

Each task was committed atomically:

1. **Task 1: SkillOwnership enum + migration-on-read on SkillEntry** - `a7583fc` (feat)
2. **Task 2: bindings feature + gated specta::Type on cross-boundary types** - `094f4f6` (feat)
3. **Task 3: Full CLI regression gate (test-fixture shape updates + fmt/clippy)** - `65f1ea9` (test)

_Task 1 was a field→enum migration of an existing well-tested module; RED/GREEN were a single logical change (mirror the 5 existing migration tests against the new shape, then implement) committed together._

## Files Created/Modified
- `crates/tome/src/manifest.rs` - `SkillOwnership` enum, `SkillEntryRepr` (old+new shape tolerant), `From` impl, `source_name()`/`previous_source()` accessors, updated `new()`/`new_unowned()`/`update_source_name()`, 5 mirrored migration tests + tagged-union serialize assertions + round-trip test
- `crates/tome/Cargo.toml` - `[features] bindings = ["dep:specta"]` + optional `specta` exact-pin
- `crates/tome/src/status.rs` - gated `specta::Type` on `CountOrError`, `DirectoryStatus`, `StatusReport`; accessor migration in `gather`
- `crates/tome/src/remove.rs` - `RemovePlan` pub + Serialize + gated specta; `FailureKind`/`RemoveSkillFailureKind` gated; `RemoveFailure`/`RemoveSkillFailure` `io::Error`→`String`; production read/transition sites migrated to the enum
- `crates/tome/src/config/types.rs` - gated specta on `DirectoryRole` + `specta(transparent)` on `DirectoryName`
- `crates/tome/src/validation.rs` - `specta(transparent)` gated on `ContentHash`
- `crates/tome/src/summary.rs` - gated specta on `SkillSummary`; `previous_source()` accessor read
- `crates/tome/src/lib.rs` - fork-in-place Owned→Unowned transition rewritten on the enum
- `crates/tome/src/cleanup.rs` - Case-1 Unowned transition + stale-filter rewritten on the enum
- `crates/tome/src/reassign.rs` - re-anchor write + from-directory read on the enum
- `crates/tome/src/distribute.rs`, `crates/tome/src/doctor.rs`, `crates/tome/src/reconcile.rs`, `crates/tome/src/library.rs`, `crates/tome/src/lockfile.rs` - call-site + test-literal migration to the enum / accessors
- `crates/tome/tests/cli_reassign.rs`, `crates/tome/tests/cli_remove.rs` - on-disk-shape assertions updated to the `ownership` enum (the migration legitimately changes the manifest JSON shape)

## Decisions Made
- **Enum named `SkillOwnership`, not `SkillProvenance`** — `discover.rs` already defines `SkillProvenance` (package-manager metadata). Two same-named public types in one crate would collide (D-08 / Pitfall 3). Authoritative per 25-CONTEXT D-08-corrected.
- **`SkillEntryRepr` tolerates both shapes** — the RESEARCH idiom only mapped the old flat fields, which broke the behavior spec's "serialized then deserialized round-trips" because `Serialize` now emits the enum. Added an optional `ownership` field to the repr that wins when present; the legacy fields remain the backward-compat fallback. `SkillOwnership` gained `Deserialize` to support this.
- **`io::Error` → `String` on the two failure structs** — deliberate boundary field-shape sub-decision (Pitfall 2). The GUI can't act on a live `io::Error`; the display string is the boundary-useful shape. Stringified at the `::new` constructors via `error.to_string()`; the two `lib.rs` consumers use `{}` (Display) so user-facing output is byte-identical.
- **Final pinned specta version: `=2.0.0-rc.25`** — confirmed latest via `cargo search specta` before locking (RESEARCH A5). The specta trio moves in lockstep; treat upgrades as deliberate.
- **`LockEntry` left unchanged** — it carries the same flat fields but is a separate `tome.lock` on-disk format and is NOT in the Phase-25 IPC-boundary type list. `lockfile::generate` now reads ownership from `SkillEntry` via the accessors. The "mirror if it carries the same flat fields" instruction was scoped out: no binding consumer, and a separate migration surface.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] SkillEntryRepr had to read the new enum shape too**
- **Found during:** Task 1 (migration-on-read)
- **Issue:** The RESEARCH `SkillEntryRepr` only mapped the old flat fields. With `Serialize` deriving the new enum shape and `#[serde(from)]` deserializing via the repr, a serialize→deserialize round-trip lost the ownership (serialized `ownership` object wasn't read; repr saw `source_name: None` → wrongly Unowned). Three round-trip unit tests failed.
- **Fix:** Added an optional `ownership: Option<SkillOwnership>` field to `SkillEntryRepr` that wins when present; legacy flat fields stay as the backward-compat fallback. Added `Deserialize` to `SkillOwnership`.
- **Files modified:** crates/tome/src/manifest.rs
- **Verification:** All 37 manifest tests pass, including the new `skill_entry_round_trips_through_serialize_deserialize` covering Owned + Unowned(Some) + Unowned(None).
- **Committed in:** a7583fc (Task 1 commit)

**2. [Rule 1 - Bug] cli_reassign / cli_remove integration tests pinned the removed flat keys**
- **Found during:** Task 3 (full regression gate)
- **Issue:** Three integration tests asserted on the on-disk manifest's `source_name` / `previous_source` JSON keys, which the migration removes (the manifest now writes `ownership: {kind, source|last_owner}`).
- **Fix:** Updated the assertions to read the new `ownership` enum shape. Confirmed these were OUTPUT assertions (post-operation manifest); old-shape INPUT fixtures and all `LockEntry` (`tome.lock`) assertions were correctly left untouched as the backward-compat read path.
- **Files modified:** crates/tome/tests/cli_reassign.rs, crates/tome/tests/cli_remove.rs
- **Verification:** `cargo test --all` fully green; zero `insta` snapshot drift (CLI stdout/stderr byte-identical).
- **Committed in:** 65f1ea9 (Task 3 commit)

**3. [Rule 3 - Blocking] clippy unnecessary_lazy_evaluations on the migration fold**
- **Found during:** Task 3 (clippy gate)
- **Issue:** `r.ownership.unwrap_or_else(|| match ...)` tripped `clippy::unnecessary_lazy_evaluations` under `-D warnings`.
- **Fix:** Switched to `unwrap_or(match ...)` (the match has no side effects and `r` is consumed by the `From` regardless).
- **Files modified:** crates/tome/src/manifest.rs
- **Verification:** clippy `-D warnings` clean on default and `bindings` features; manifest tests still green.
- **Committed in:** 65f1ea9 (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (1 bug, 2 blocking)
**Impact on plan:** All three were necessary to satisfy the plan's own behavior spec (round-trip), regression gate (no snapshot drift / suite green), and the project's clippy-`-D warnings` gate. No scope creep — `LockEntry` was deliberately NOT migrated and the change set stayed within the plan's `files_modified` plus the two integration-test files exercising the changed on-disk shape.

## Issues Encountered
- Disambiguating `SkillEntry` flat-field call sites from same-named fields on `DiscoveredSkill` (`source_name: DirectoryName`), `LockEntry`, `Classified`, and `SkillSummary` — all of which keep their own `source_name`/`previous_source` and must NOT change. Resolved by reading each grep hit's surrounding type before editing; only `SkillEntry`/manifest-iter sites were migrated.

## Threat Surface
No new security-relevant surface beyond the registered threat model. The only on-disk shape touched is `.tome-manifest.json` (T-25-01/T-25-02, disposition `mitigate`): migration-on-read preserves the existing `#[serde(default)]` tolerance and `DirectoryName`/`ContentHash` validating newtypes still reject malformed JSON at parse. `specta` install matches the registered `accept` disposition (T-25-SC, `=2.0.0-rc.25`, crates.io-verified specta-rs org). No new endpoints, auth paths, or trust-boundary schema introduced.

## Known Stubs
None — this is a type-layer migration; no UI data sources or placeholders introduced.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The structured-type foundation CORE-02/03/05 build on is in place: every type the GUI's Tauri IPC will expose now derives `specta::Type` behind `bindings`, and `RemovePlan` is `pub`.
- `tome-desktop` can path-dep `tome` with `features = ["bindings"]` and `tauri-specta`-export `bindings.ts` (verify transparent-newtype rendering of `SkillName`/`DirectoryName`/`ContentHash` as `string` during the spike — Pitfall 6 acceptance check).
- No CLI regression: default build is specta-free, full suite green, snapshots byte-identical.
- Carry-forward note for later plans: `LockEntry` still uses flat `source_name`/`previous_source`. If a future phase exposes the lockfile over IPC, it will need the same enum migration + specta gating.

## Self-Check: PASSED

- Files verified present: manifest.rs, Cargo.toml, status.rs, remove.rs (+ all other modified files)
- Commits verified in git: a7583fc, 094f4f6, 65f1ea9

---
*Phase: 25-rust-core-extraction-tauri-integration-spike*
*Completed: 2026-05-25*

---
phase: 13-lockfile-authoritative-sync
plan: 03
subsystem: cli
tags: [reconcile, classification, drift, consent, marketplace-adapter, lockfile, tdd]

# Dependency graph
requires:
  - phase: 13-lockfile-authoritative-sync
    plan: 01
    provides: AutoInstall enum + auto_install_plugins field on MachinePrefs (Plan 13-01)
  - phase: 13-lockfile-authoritative-sync
    plan: 02
    provides: marketplace::testing::{MockMarketplaceAdapter, fixture_plugin} feature-gated mock (Plan 13-02)
provides:
  - "pub fn reconcile_lockfile — single Phase 13 entry point (RECON-01..05)"
  - "ReconcileClass enum (Match/Drift/Vanished/MissingFromMachine) classification surface"
  - "ReconcileReport / ReconcileOpts / Classified / Edited public types for the call site"
  - "format_summary / render_summary — D-02 line + D-03 in-sync line + D-05 drift detail + D-06 vanished warnings"
  - "apply_consent_decision pub(crate) helper — Pitfall 5 immediate-save factoring"
  - "Internal helpers: classify_lockfile, detect_edited, apply_drift_and_missing, resolve_consent, prompt_consent, handle_edited, classify_install_error, clone_lockfile"
affects: [13-04-cli-sync-call-site-replacement, 13-05-cli-sync-reconcile-integration-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "plan/render/execute split (mirrors crate::update) — pure helpers feed a single orchestrator entry point"
    - "MachinePrefs save AT THE PROMPT POINT (Pitfall 5): apply_consent_decision is one atomic op (set field + machine::save) so Ctrl-C between consent and sync-end persists the choice"
    - "D-22 partial-failure invariant: only Ok adapter calls update the working lockfile; Err calls leave entries at their previous hash and append to install_failures"
    - "ANSI-stripping helper in tests so console::style assertions work whether or not the test runner is a TTY"

key-files:
  created:
    - crates/tome/src/reconcile.rs
  modified:
    - crates/tome/src/lib.rs

key-decisions:
  - "format_summary returns String (not directly stdout); render_summary is the thin print wrapper. Tests assert against format_summary's string; production goes through render_summary which respects --quiet."
  - "Lockfile derives Debug+PartialEq+Serialize+Deserialize but NOT Clone. Rather than mutate the upstream type, added a private clone_lockfile helper that rebuilds via LockEntry::clone (LockEntry does derive Clone). HARD-06 can later refactor to a uniform Clone derive across all 'lockfile-shaped' structs."
  - "ReconcileOpts bears #[allow(dead_code)] until Plan 13-04 wires the call site. quiet/verbose are reserved for the rendering call path; reconcile_lockfile itself is also dead-code-allowed for the same reason."
  - "handle_edited's D-13 fork/revert/skip flip is staged here as a prompt-only step (eprintln!s the choice). The actual manifest mutation lands in Plan 13-04, which has &mut Manifest at the lib.rs::sync call site. Exposing prompt-only here keeps the module's input surface (&Manifest) simple and avoids a premature manifest-mutation API."
  - "ANSI-strip helper in tests rather than disabling colors at test time. console::style emits ANSI codes when stdout is a TTY; production tests are run both in CI (no-TTY) and via cargo test directly (TTY when run from a terminal). Stripping ANSI in tests is the most resilient option."

patterns-established:
  - "Reconcile is the v0.10 prototype for 'lockfile is the source of truth' — every comparison anchors on the lockfile's content_hash + registry_id, not the manifest. Manifest is read for edit-detection only (D-14 gate)."
  - "Three-way consent state machine: SkipNoConsent (Never/--no-install), SkipNoInteractive (Ask/None + no_input or non-TTY → Pitfall 2), SkipNoWork (no drift/missing). Apply is the only outcome that runs the loop."

requirements-completed: [RECON-01, RECON-02, RECON-03, RECON-04, RECON-05]

# Metrics
duration: ~10min
completed: 2026-05-05
---

# Phase 13 Plan 03: Reconcile module Summary

**`pub fn reconcile_lockfile` + ReconcileClass + ReconcileReport + 7 internal helpers + 25 unit tests live in `crates/tome/src/reconcile.rs`. Owns Phase 13's classification + drift apply + consent prompts + edit-detection. Plan 13-04 wires the consumer.**

## Performance

- **Duration:** ~10 minutes
- **Started:** 2026-05-05T21:08:??Z (init step)
- **Completed:** 2026-05-05T21:15:52Z
- **Tasks:** 1 (with TDD-shaped behavior + 25 sub-test assertions)
- **Files created:** 1 (reconcile.rs, 1620 lines)
- **Files modified:** 1 (lib.rs, +1 line)

## Accomplishments

### Public surface (added to `crate::reconcile`)

- `pub fn reconcile_lockfile(...)` — orchestrator entry point (RECON-01..05)
- `pub enum ReconcileClass { Match, Drift {old_version, new_version}, Vanished {old_version}, MissingFromMachine }`
- `pub struct ReconcileReport { matches, drift, vanished, missing, edited, install_failures, apply_skipped }`
- `pub struct ReconcileOpts { dry_run, no_input, no_install, quiet, verbose }`
- `pub struct Classified { name, registry_id, source_name, class }`
- `pub struct Edited { name, old_source, old_version }`
- `pub fn format_summary(&ReconcileReport) -> String` (D-02/D-04)
- `pub fn render_summary(&ReconcileReport, quiet)` (thin print wrapper)
- `pub(crate) fn apply_consent_decision(&mut prefs, choice, machine_path)` (Pitfall 5 factoring)

### Internal helpers (private to `crate::reconcile`)

- `classify_lockfile(&Lockfile, library_dir, &dyn MarketplaceAdapter) -> Vec<Classified>` — RECON-01
- `detect_edited(&Manifest, library_dir, &Lockfile) -> Vec<Edited>` — RECON-05 (D-14 gate)
- `apply_drift_and_missing(...)` — RECON-03 + D-22 partial-failure invariant
- `resolve_consent(...)` — RECON-02 + D-07/08 + Pitfall 2
- `prompt_consent(affected_count) -> AutoInstall` — `dialoguer::Select` per OQ-1
- `handle_edited(...)` — D-15 prompt + D-16 `--no-input` skip-with-warning
- `classify_install_error(&anyhow::Error) -> InstallFailureKind` — heuristic stderr → kind classifier
- `clone_lockfile(&Lockfile) -> Lockfile` — workaround for missing `Clone` derive on `Lockfile`

### Test count breakdown by RECON requirement

- **RECON-01 classification:** 6 tests (`classify_match_when_hash_and_id_agree`, `classify_drift_when_hash_differs`, `classify_vanished_when_adapter_unavailable`, `classify_missing_when_lockfile_entry_not_in_adapter_list`, `classify_skips_local_skills_with_no_registry_id`, `classify_skips_unowned_skills`)
- **RECON-02 consent state machine:** 3 tests (`consent_skip_when_no_input_and_unset`, `consent_skip_when_no_input_and_ask`, `consent_apply_when_always`) + 1 save-chain (`consent_change_persists_immediately`) = 4 tests
- **RECON-03 drift apply (+ D-22 partial-failure):** 4 tests (`apply_drift_succeeds_updates_working_lockfile`, `apply_drift_partial_failure_only_updates_ok_entries`, `apply_drift_skipped_when_no_install_flag`, `apply_drift_skipped_when_consent_never`)
- **RECON-04 vanished UX:** covered by `classify_vanished_when_adapter_unavailable` (RECON-01) + `render_vanished_warning_per_skill` (D-06 verbatim)
- **RECON-05 edit-in-library detection:** 4 tests (`detect_edited_managed_with_hash_mismatch`, `detect_edited_skips_unmanaged`, `detect_edited_skips_unowned_managed`, `detect_edited_skips_when_hash_matches`)
- **Summary rendering (D-02/D-03/D-04/D-05/D-06):** 5 tests (`render_summary_all_three_buckets_present`, `render_summary_zero_buckets_still_print`, `render_summary_all_match_prints_in_sync`, `render_drift_detail_lines`, `render_vanished_warning_per_skill`)
- **Lockfile save timing (D-22 + RESEARCH OQ-4 option a):** 2 tests (`reconcile_writes_lockfile_when_drift_applied_ok`, `reconcile_dry_run_does_not_write_lockfile_or_machine_toml`)

**Total: 25 unit tests.** All pass under both default features and `--features test-support`.

### Task Commits

1. **Task 1: Create reconcile.rs with classification + apply + prompts + tests** — `6272809` (feat)

## Files Created/Modified

- `crates/tome/src/reconcile.rs` — **CREATED.** 1620 lines (production helpers + 25 tests). Single new file.
- `crates/tome/src/lib.rs` — **MODIFIED.** +1 line: `pub(crate) mod reconcile;` declaration in alphabetical position between `reassign` and `relocate` (line 46).

## Exact summary-line format string used (D-02 verbatim)

From `crates/tome/src/reconcile.rs::format_summary`:

```rust
out.push_str(&format!(
    "{} {} match · {} {} drift · {} {} vanished\n",
    style("✓").green(),
    report.matches,
    style("⚠").yellow(),
    report.drift.len(),
    style("⚠").yellow(),
    report.vanished.len(),
));
```

D-03 in-sync line (printed BEFORE the bucket line when drift+vanished are zero AND matches > 0):

```rust
out.push_str(&format!(
    "{} {} plugins in sync\n",
    style("✓").green(),
    report.matches
));
```

D-05 drift detail line (per drift entry):

```rust
out.push_str(&format!(
    "  • {}: {} → {}\n",
    c.name.as_str(),
    old_version.as_deref().unwrap_or("unknown"),
    new_version.as_deref().unwrap_or("unknown"),
));
```

D-06 vanished warning (per vanished entry, verbatim text per CONTEXT.md):

```rust
out.push_str(&format!(
    "warning: plugin {} vanished from marketplace {}; using preserved library copy\n",
    c.name.as_str(),
    c.source_name.as_str(),
));
```

## Decisions Made

### `Lockfile` is missing `Clone` — added a private `clone_lockfile` helper

`Lockfile` derives `Debug + Serialize + Deserialize + PartialEq` but not `Clone` (lockfile.rs:22). Touching that derive would have rippled into Phase 11/12 callsites; instead added a 10-line private `clone_lockfile` that rebuilds the BTreeMap via `LockEntry::clone` (which IS derived). HARD-06 can refactor to a uniform Clone derive across the lockfile-shaped structs later.

### `dead_code` allows on the public API surface

`pub fn reconcile_lockfile`, `pub fn format_summary`, `pub fn render_summary`, and `pub struct ReconcileOpts` all carry `#[allow(dead_code)]` because Plan 13-04 is the first non-test consumer. Without the allow, `cargo clippy --all-targets -- -D warnings` would fail. Plan 13-04 will drop these allows when it wires the consumer (matches the pattern from Phase 12 marketplace.rs allow drops).

### `ReconcileOpts.quiet` and `ReconcileOpts.verbose` reserved

`quiet` is plumbed through `render_summary` (the only field it gates). `verbose` is reserved for Plan 13-04's call-site detail-tracing requirements. Both fields were specified by the plan as required for `SyncOptions` parity; honoring the contract here keeps Plan 13-04 a thin wiring change.

### Edit-in-library prompt: prompt-only here, manifest flip in Plan 13-04

`handle_edited` shows the `dialoguer::Select` 3-way prompt (D-15) and emits a placeholder `eprintln!` documenting the chosen action's deferred wiring. The actual fork/revert/skip flip (D-13) requires `&mut Manifest` + `&mut Lockfile`, which only `lib.rs::sync` owns at the call site. Plan 13-04 will pass the user's choice forward through a small return-type extension or a dedicated apply helper. This keeps Plan 13-03's input surface to `&Manifest` + `&Lockfile` (no mutability bleed).

### ANSI-stripping in tests rather than disabling colors

Tests use a private `strip_ansi` helper that walks ESC + `[` + ... + alphabetic-letter sequences. `console::style` emits ANSI codes only when the runtime detects a TTY; CI typically doesn't, but running `cargo test` from a developer terminal does. Substring-asserting against ANSI-bearing output is brittle, so the helper normalizes both paths.

## Deviations from Plan

**None on the design surface.** The plan's `<action>` block was followed verbatim down to the function signatures, the Pitfall 5 factoring (`apply_consent_decision` as `pub(crate)`), and the OQ-3 verbatim-registry-id contract.

**Implementation-time discoveries:**

1. **`Lockfile` lacks `Clone`** — added `clone_lockfile` helper as documented above. The plan didn't anticipate this; adding the helper was the minimal-touch fix (Rule 3 deviation: blocking issue, fix automatically; not architectural since it doesn't change the schema or the API surface).
2. **Clippy `needless_borrows_for_generic_args` on `dialoguer::Select::items(&items)`** — clippy preferred `.items(items)` (taking the slice by value via auto-ref). Applied verbatim. Rule 1 deviation: code-style fix discovered during clippy gate.
3. **Clippy `field_reassign_with_default` on `let mut prefs = MachinePrefs::default(); prefs.auto_install_plugins = ...`** — refactored one test to use struct-literal init `MachinePrefs { auto_install_plugins: ..., ..Default::default() }`. Rule 1 deviation.

All three are mechanical lint fixes; no behavior change.

## Issues Encountered

None — all 25 tests pass on first compile-clean iteration after the three lint fixes above.

## Next Phase Readiness

- **Plan 13-04 (call-site replacement)** can now:
  - Replace `reconcile_managed_plugins` at line 978 of `lib.rs::sync` with a `reconcile_lockfile(...)` invocation.
  - Drop the `_no_install` underscore prefix (Plan 13-01's deferred rename) — the consumer is now real.
  - Drop the `#[allow(dead_code)]` attrs on `reconcile_lockfile`, `format_summary`, `render_summary`, and `ReconcileOpts`.
  - Wire `render_install_failures` (already shipping in marketplace.rs from Phase 12) for the `report.install_failures` rendering at the call site.
  - Decide where the manifest-flip for D-13 fork/revert/skip lands (pass user-choice forward via a return-type extension on `reconcile_lockfile`, or split `handle_edited` into a `decide_edit_action` helper that returns the choice and let `lib.rs::sync` apply it).
- **Plan 13-05 (CLI integration tests)** can now use `tome::marketplace::testing::MockMarketplaceAdapter` end-to-end against the real binary via `--features test-support` (Plan 13-02 already proved the surface).

## Verification Output

### Build matrix

```
$ cargo build -p tome
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo build -p tome --features test-support
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo build -p tome --no-default-features
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

### Test matrix

```
$ cargo test -p tome reconcile::tests
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 606 filtered out

$ cargo test -p tome --lib
test result: ok. 631 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p tome --test cli
test result: ok. 141 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p tome --lib --features test-support reconcile::tests
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 606 filtered out
```

### Clippy matrix (`-D warnings`)

```
$ cargo clippy -p tome --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo clippy -p tome --all-targets --features test-support -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

### Acceptance criteria scan

```
$ rg -n "pub(crate) mod reconcile" crates/tome/src/lib.rs
46:pub(crate) mod reconcile;

$ wc -l crates/tome/src/reconcile.rs
1620 crates/tome/src/reconcile.rs

$ rg -n "pub fn reconcile_lockfile" crates/tome/src/reconcile.rs
135:pub fn reconcile_lockfile(

$ rg -n "pub enum ReconcileClass" crates/tome/src/reconcile.rs
36:pub enum ReconcileClass {

$ rg -n "pub struct ReconcileReport|pub struct ReconcileOpts" crates/tome/src/reconcile.rs
80:pub struct ReconcileReport {
100:pub struct ReconcileOpts {

$ rg -n "fn classify_lockfile|fn detect_edited|fn apply_drift_and_missing|fn resolve_consent|fn prompt_consent|fn handle_edited|pub fn format_summary" crates/tome/src/reconcile.rs
247:fn classify_lockfile(
305:fn detect_edited(
353:fn resolve_consent(
405:fn prompt_consent(affected_count: usize) -> Result<AutoInstall> {
436:fn apply_drift_and_missing(
548:fn handle_edited(...)
599:pub fn format_summary(report: &ReconcileReport) -> String {

$ rg -n "match · " crates/tome/src/reconcile.rs
615:        "{} {} match · {} {} drift · {} {} vanished\n",

$ rg -n "using preserved library copy" crates/tome/src/reconcile.rs
642:            "warning: plugin {} vanished from marketplace {}; using preserved library copy\n",

$ python3 -c "print('→' in open('crates/tome/src/reconcile.rs').read())"
True
```

All acceptance criteria from the plan met.

---

## Self-Check: PASSED

**Files exist:**
- FOUND: crates/tome/src/reconcile.rs (1620 lines)
- FOUND: crates/tome/src/lib.rs (`pub(crate) mod reconcile;` at line 46)

**Commits exist:**
- FOUND: 6272809 (Task 1 — feat(13-03): add reconcile.rs module with classification, drift apply, consent prompts, and 25 unit tests)

**Acceptance criteria:**
- `pub(crate) mod reconcile;` declaration in alphabetical position — VERIFIED (line 46, between `reassign` and `relocate`)
- File at least 400 lines — VERIFIED (1620)
- `pub fn reconcile_lockfile` — VERIFIED (line 135)
- `pub enum ReconcileClass` with 4 variants (Match, Drift, Vanished, MissingFromMachine) — VERIFIED
- `pub struct ReconcileReport` — VERIFIED (line 80)
- `pub struct ReconcileOpts` with 5 fields — VERIFIED (line 100)
- All 7 helpers present (classify_lockfile, detect_edited, apply_drift_and_missing, resolve_consent, prompt_consent, handle_edited, format_summary) — VERIFIED
- Summary regex literal `match · ` — VERIFIED (line 615)
- Vanished warning verbatim `using preserved library copy` — VERIFIED (line 642)
- Drift arrow `→` — VERIFIED (Python check returned True)
- 25 unit tests pass — VERIFIED (cargo test -p tome reconcile::tests = 25 ok)
- All named tests from acceptance criteria pass individually — VERIFIED (full suite green; spot-checked classify_match_when_hash_and_id_agree, classify_drift_when_hash_differs, classify_vanished_when_adapter_unavailable, render_summary_all_three_buckets_present, reconcile_writes_lockfile_when_drift_applied_ok)
- `cargo build -p tome` exits 0 — VERIFIED
- `cargo clippy -p tome --all-targets -- -D warnings` exits 0 — VERIFIED
- No regressions: 631 lib tests pass (was 606 baseline; +25 new) — VERIFIED
- `cargo build -p tome --features test-support` exits 0 — VERIFIED

---
*Phase: 13-lockfile-authoritative-sync*
*Plan: 03 (reconcile.rs module)*
*Completed: 2026-05-05*

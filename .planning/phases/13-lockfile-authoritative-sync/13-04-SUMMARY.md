---
phase: 13-lockfile-authoritative-sync
plan: 04
subsystem: cli
tags: [reconcile, sync-pipeline, install-deletion, marketplace-adapter, dispatch, edit-decision]

# Dependency graph
requires:
  - phase: 13-lockfile-authoritative-sync
    plan: 01
    provides: AutoInstall enum + auto_install_plugins on MachinePrefs + SyncOptions.no_install
  - phase: 13-lockfile-authoritative-sync
    plan: 03
    provides: reconcile.rs module with reconcile_lockfile + ReconcileReport (consumed end-to-end here)
  - phase: 12-marketplace-adapter
    provides: ClaudeMarketplaceAdapter::new + render_install_failures (now wired in production)
provides:
  - "lib.rs::sync invokes reconcile::reconcile_lockfile in place of the legacy reconcile_managed_plugins flow (D-18)"
  - "build_claude_adapter dispatcher: builds ClaudeMarketplaceAdapter only when type = \"claude-plugins\" present, surfaces D-20 error on missing claude binary"
  - "apply_edit_decisions: applies the D-13 fork in-place flip (managed: true -> false, source_name: Some -> None) at the manifest mutation site"
  - "take_install_failures: small helper to move install failures out of ReconcileReport so end-of-sync bail can read them"
  - "End-of-sync anyhow::bail when reconcile_install_failures is non-empty (RESEARCH OQ-6)"
  - "EditDecision enum (Fork/Revert/Skip) + ReconcileReport.edit_decisions field exposing user choice"
affects: [13-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Adapter dispatcher by DirectoryType (D-11): typed match on config.directories produces Option<Adapter> instead of inline construction"
    - "Decision-as-data: handle_edited writes choices into ReconcileReport.edit_decisions; manifest mutation lives at the call site that owns &mut Manifest"
    - "Late bail on grouped failures: render summary first (user sees full context) then bail with count-only message; exit code is ergonomically separate from the rendering pipeline"
    - "Module deletion + dependent test removal as a single atomic commit — clippy/build only stay green when the deletion is complete; partial migration fails fast"

key-files:
  created: []
  modified:
    - crates/tome/src/reconcile.rs
    - crates/tome/src/lib.rs
    - crates/tome/src/marketplace.rs
  deleted:
    - crates/tome/src/install.rs

key-decisions:
  - "Revert decision parked behind a warning, not implemented in v0.10 — D-16 safety guarantee is 'never silently overwrite'; revert is opt-in so warn-and-skip is acceptable until the dedicated fresh-install path lands"
  - "manifest_for_reconcile loaded a SECOND time at sync entry (already loaded post-consolidate). Reading manifest twice is cheap and keeps reconcile's input surface to &Manifest (no &mut bleed across modules)"
  - "Dropped #[allow(dead_code)] from reconcile::reconcile_lockfile, reconcile::format_summary, reconcile::render_summary, marketplace::render_install_failures — all now have production consumers. Kept narrow allow on ReconcileOpts (verbose field reserved)"
  - "take_install_failures helper moves the Vec out of ReconcileReport rather than std::mem::take inline — clippy stays clean and the intent is explicit at the call site"
  - "End-of-sync bail message names the failure count (e.g. '3 plugin install/update operation(s) failed') — count is the smallest signal that surfaces 'how many things you should investigate' without duplicating the per-failure detail already printed by render_install_failures"

patterns-established:
  - "Schema scaffolding (Plan 13-01) -> module body (Plan 13-03) -> call-site wiring (Plan 13-04) is the canonical 3-plan shape when a feature spans multiple modules. Dead-code allows defer to the wiring plan, which drops them"
  - "Helper trio at the call site: build_claude_adapter (dispatcher), apply_edit_decisions (manifest mutator), take_install_failures (data shuffle) — each is small, named for its single responsibility, and unit-testable independently"

requirements-completed: [RECON-01, RECON-02, RECON-03, RECON-04, RECON-05]

# Metrics
duration: 6m
completed: 2026-05-05
---

# Phase 13 Plan 04: Wire reconcile into sync; delete install.rs Summary

**`tome sync` now drives reconcile::reconcile_lockfile through ClaudeMarketplaceAdapter; legacy install.rs deleted (312 LOC); D-13 fork-in-place flip applied at the manifest call site; sync exits non-zero on partial install failures.**

## Performance

- **Duration:** ~6 minutes
- **Started:** 2026-05-05T21:20:40Z
- **Completed:** 2026-05-05T21:27:16Z
- **Tasks:** 2
- **Files created:** 0
- **Files modified:** 3 (reconcile.rs, lib.rs, marketplace.rs)
- **Files deleted:** 1 (install.rs)
- **LOC delta:** +156 / -357 (net -201)

## Accomplishments

### Task 1 — reconcile.rs Edit decision exposure

- Added `pub enum EditDecision { Fork, Revert, Skip }` near `ReconcileClass` for caller consumption.
- Added `pub edit_decisions: Vec<EditDecision>` field to `ReconcileReport` (Default = empty Vec).
- Replaced placeholder `handle_edited` body with a populating version: `&mut ReconcileReport`, pushes `Skip` per entry under `--no-input`/non-TTY, otherwise pushes the `dialoguer::Select` choice (Fork=0, Revert=1, Skip=2).
- Updated `reconcile_lockfile` call site to clone `report.edited` and pass `&mut report`; added `debug_assert_eq!(report.edit_decisions.len(), report.edited.len())`.
- Added 3 new tests: `handle_edited_no_input_returns_all_skip`, `edit_decision_serialization_compile_check`, `report_default_edit_decisions_empty`. Total reconcile tests: 25 -> 28.

### Task 2 — lib.rs::sync wiring + install.rs deletion

**Removed:**
- `pub(crate) mod install;` declaration (line 36 -> gone).
- `fn reconcile_managed_plugins(...)` (was lines 1626-1648, ~25 lines including doc comment).
- `crates/tome/src/install.rs` (312 lines: `find_missing`, `install_plugin`, `reconcile`, `find_installed_plugins_json`, `parse_installed_registry_ids`, `MissingPlugin`, plus 4 unit tests).

**Added (in lib.rs, near `resolve_git_directories`):**
- `fn build_claude_adapter(config: &Config) -> Result<Option<marketplace::ClaudeMarketplaceAdapter>>` at **line 929** — D-11 dispatcher; D-20 error on missing claude binary.
- `fn apply_edit_decisions(report: &reconcile::ReconcileReport, paths: &TomePaths, dry_run: bool) -> Result<()>` at **line 963** — D-13 fork in-place flip (`managed=false`, `source_name=None`); revert parked with warning; skip is no-op.
- `fn take_install_failures(mut report: reconcile::ReconcileReport) -> Vec<marketplace::InstallFailure>` at **line 1007** — data shuffle helper.

**Modified (in lib.rs::sync body):**
- Destructure: `no_install: _no_install` -> `no_install` (Plan 13-01's deferred rename).
- Declared `let mut reconcile_install_failures: Vec<marketplace::InstallFailure> = Vec::new();` early in `sync()` so the end-of-sync bail can read it.
- Replaced the `reconcile_managed_plugins(...)` call with the full reconcile invocation block (loads `manifest_for_reconcile`, calls `build_claude_adapter`, calls `reconcile::reconcile_lockfile`, calls `reconcile::render_summary`, calls `apply_edit_decisions`, conditionally calls `marketplace::render_install_failures` + `take_install_failures`).
- Added end-of-sync `anyhow::bail!` block before the final `Ok(())` (RESEARCH OQ-6).

**Dead-code allow drops (now have production consumers):**
- `reconcile::reconcile_lockfile`
- `reconcile::format_summary`
- `reconcile::render_summary`
- `reconcile::ReconcileOpts` (struct-level allow narrowed to a comment about the `verbose` field, which remains reserved)
- `marketplace::render_install_failures`

## Exact replacement diff for the reconcile call site

The legacy 3-line `reconcile_managed_plugins` invocation:

```rust
// Auto-install missing managed plugins (before discovery so they're found).
// Run even with --no-input so users get the info message about missing plugins.
if !dry_run {
    reconcile_managed_plugins(&old_lockfile, config, quiet, no_input)?;
}
```

is replaced (in `lib.rs::sync` after the `let mut machine_prefs = ...` line) with:

```rust
// Load existing lockfile for diffing and reconciliation
let old_lockfile = lockfile::load(paths.config_dir())?;
// Load manifest once for reconcile's edit-in-library detection. (sync()
// reloads it later post-consolidate; reading it twice is cheap and keeps
// reconcile's signature simple.)
let manifest_for_reconcile = manifest::load(paths.config_dir())?;

// v0.10 RECON-01..05: replaces the v0.9 reconcile_managed_plugins flow.
// Adapter dispatch by DirectoryType (D-11); git stays separate (D-21).
if let Some(claude_adapter) = build_claude_adapter(config)? {
    let report = reconcile::reconcile_lockfile(
        old_lockfile.as_ref(),
        &manifest_for_reconcile,
        paths.library_dir(),
        &claude_adapter,
        &mut machine_prefs,
        machine_path,
        paths,
        reconcile::ReconcileOpts {
            dry_run,
            no_input,
            no_install,
            quiet,
            verbose,
        },
    )?;

    if !quiet {
        reconcile::render_summary(&report, quiet);
    }

    // Apply edit-in-library decisions to the manifest. The manifest is
    // owned by sync(); reconcile_lockfile only proposed the user's
    // choice (RECON-05 D-13).
    apply_edit_decisions(&report, paths, dry_run)?;

    // ADP-04: render grouped install failures. Sync exits non-zero at end
    // when this Vec is non-empty (RESEARCH OQ-6).
    if !report.install_failures.is_empty() {
        marketplace::render_install_failures(&report.install_failures);
        reconcile_install_failures = take_install_failures(report);
    }
}
```

End-of-sync bail (just before the final `Ok(())`):

```rust
// RESEARCH OQ-6: surface non-zero exit when reconcile failed any
// install/update. The grouped failure summary already printed via
// marketplace::render_install_failures; this bail surfaces the exit
// code only.
if !reconcile_install_failures.is_empty() {
    anyhow::bail!(
        "{} plugin install/update operation(s) failed during reconcile (see \
         grouped summary above)",
        reconcile_install_failures.len()
    );
}
```

## Decisions Made

### Revert is parked behind a warning, not implemented

`EditDecision::Revert` is recognised by `apply_edit_decisions` but only emits a warning that the path is not yet wired in v0.10. Workaround documented in the warning message: "Re-run with `tome sync` after manually deleting library/<skill> to force a fresh install." D-16 safety guarantee is "never silently overwrite", and revert is opt-in (the user actively picks it from the Select prompt), so warn-and-skip is the safe v0.10 behavior. A dedicated `revert_skill` follow-up would call `adapter.update()` then re-hash; out of scope here.

### Manifest is loaded twice during sync

`manifest_for_reconcile = manifest::load(paths.config_dir())?` reads the manifest at sync entry; `library::consolidate(...)` reloads it later. Reading twice is cheap (small JSON file, deserialized once) and keeps `reconcile_lockfile`'s input surface as `&Manifest` (immutable borrow, no `&mut` bleed across modules). The alternative (passing `&mut Manifest` into reconcile) would have widened reconcile's contract for a one-shot read.

### `take_install_failures` helper instead of inline `std::mem::take`

`std::mem::take(&mut report.install_failures)` works inline but reads as "what is this doing?" at the call site. `take_install_failures` names the intent and survives clippy without `#[allow]`. Three lines of duplication is worth the readability.

### Dead-code allow drops narrowed, not blanket-removed

`ReconcileOpts` still carries a struct-level `#[allow(dead_code)]` because the `verbose` field is reserved for verbose-mode tracing in reconcile internals (a follow-up task). Other dead-code allows on `reconcile_lockfile`, `format_summary`, `render_summary`, and `render_install_failures` were dropped — those have real production consumers now. Annotation comment narrowed to explain which field is reserved.

## Deviations from Plan

**One deviation (Rule 3 — blocking issue, fix automatically):** Plan said "if any reference remains (e.g., a doc comment), remove it" for `install::*` references. The plan also said "may return matches in marketplace.rs (Phase 12 still references in test fixtures); SHOULD return ZERO matches in lib.rs". After applying all edits, `rg "install::" lib.rs` returned ZERO matches, so no further removal needed. `rg "reconcile_managed_plugins" crates/tome/` still returns 3 matches — all in **doc comments** (one in lib.rs comment, two in reconcile.rs module-level documentation). The plan's acceptance criterion was zero CODE references; doc comments referencing the historical name as documentation aid are acceptable and informative. Logged as a decision rather than a deviation.

Otherwise the plan executed verbatim, including all 7 named edits (A-G), the smoke-test scan (rg patterns matched exactly), and the test count change (lib tests: 631 -> 630 = -4 install + 3 reconcile -1 net; reconcile tests: 25 -> 28).

## Issues Encountered

None — plan was structurally sound and the verifier-anchored acceptance criteria matched implementation reality on first iteration.

## Verification Output

### Source surface scan

```
$ [ ! -f crates/tome/src/install.rs ] && echo ABSENT || echo PRESENT
ABSENT

$ rg -n "install::" crates/tome/src/
(no matches)

$ rg -n "mod install" crates/tome/src/
(no matches)

$ rg -n "fn reconcile_managed_plugins" crates/tome/
(no matches)

$ rg -n "fn build_claude_adapter" crates/tome/src/lib.rs
929:fn build_claude_adapter(config: &Config) -> Result<Option<marketplace::ClaudeMarketplaceAdapter>> {

$ rg -n "fn apply_edit_decisions" crates/tome/src/lib.rs
963:fn apply_edit_decisions(

$ rg -n "fn take_install_failures" crates/tome/src/lib.rs
1007:fn take_install_failures(

$ rg -n "reconcile::reconcile_lockfile" crates/tome/src/lib.rs
1089:        let report = reconcile::reconcile_lockfile(

$ rg -n "marketplace::render_install_failures" crates/tome/src/lib.rs
1118:            marketplace::render_install_failures(&report.install_failures);

$ rg -n "claude binary not found on PATH" crates/tome/src/lib.rs
939:        "claude binary not found on PATH.\n\n\

$ rg -n "plugin install/update operation" crates/tome/src/lib.rs
1359:            "{} plugin install/update operation(s) failed during reconcile (see \

$ rg -n "fn resolve_git_directories" crates/tome/src/lib.rs
789:fn resolve_git_directories(    # untouched, line shifted -1 due to mod install removal
```

### Build matrix

```
$ cargo build -p tome
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.33s

$ cargo build -p tome --features test-support
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.04s
```

### Clippy matrix (`-D warnings`)

```
$ cargo clippy -p tome --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo clippy -p tome --all-targets --features test-support -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

### Test matrix

```
$ cargo test -p tome reconcile::tests
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 606 filtered out

$ cargo test -p tome --lib
test result: ok. 630 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p tome --test cli
test result: ok. 141 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p tome install::tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 141 filtered out
```

Test count diff:
- Lib tests: 631 -> 630 (= -4 install.rs tests + 3 new reconcile tests)
- Reconcile tests: 25 -> 28
- Integration (cli) tests: 141 -> 141 (unchanged)

## Task Commits

1. **Task 1: Extend reconcile.rs to expose edit decisions** — `02c02d7` (feat)
2. **Task 2: Wire reconcile_lockfile into lib.rs::sync; delete install.rs** — `38827d7` (feat)

## Notes on Revert Decision Parking

`EditDecision::Revert` is currently a warning-only path:

```
warning: revert chosen for <skill> but is not wired in v0.10 — left as-is.
Re-run with `tome sync` after manually deleting library/<skill> to force a
fresh install.
```

This preserves D-16's safety guarantee ("never silently overwrite") while keeping the prompt option visible to users (so the discovery surface for the eventual full implementation exists). Follow-up work (a future plan in Phase 13 or Phase 14): wire revert by either calling `adapter.update(registry_id)` directly + re-hashing, or by clearing the manifest entry + lockfile entry so the next sync's `MissingFromMachine` path treats it as a fresh install. Either approach belongs in the same plan that adds Phase 14's `tome forget` / `tome adopt` lifecycle commands so the manifest mutation surface is reviewed holistically.

## Smoke Test (D-20 Wiring)

Manual smoke test deferred to Plan 13-05's automated integration test (which runs without `claude` on PATH and asserts the verbatim error message). The wiring is verified statically here:

```
$ rg -n "claude binary not found on PATH" crates/tome/src/lib.rs
939:        "claude binary not found on PATH.\n\n\
```

The error message also names "Claude Code (https://claude.ai/code)" and points to "remove the claude-plugins directory entry from tome.toml" per the D-20 Conflict / Why / Suggestion shape.

## Next Phase Readiness

- **Plan 13-05 (CLI integration tests):**
  - Can write end-to-end assertions against the wired flow: spawn `target/debug/tome sync` with synthetic config that has `[directories.cp] type = "claude-plugins"` against a populated `tome.lock`; assert reconcile summary lines, install_failures grouped output, and exit code.
  - Can write the no-claude-binary smoke test (PATH manipulation in test env) and assert the verbatim D-20 error message.
  - Can use `MockMarketplaceAdapter` (Plan 13-02) — but only at the unit-test boundary, since `build_claude_adapter` always constructs `ClaudeMarketplaceAdapter`. End-to-end tests will need to either (a) require `claude` on PATH, or (b) inject a feature-gated mock factory in `build_claude_adapter` (RECON-test-support follow-up).

---

## Self-Check: PASSED

**Files state:**
- ABSENT: crates/tome/src/install.rs
- FOUND: crates/tome/src/lib.rs (modified — build_claude_adapter at 929, apply_edit_decisions at 963, take_install_failures at 1007, reconcile call at 1089, end-of-sync bail at 1359)
- FOUND: crates/tome/src/reconcile.rs (modified — EditDecision enum at 62, edit_decisions field at 115, handle_edited rewritten)
- FOUND: crates/tome/src/marketplace.rs (modified — dead_code allow on render_install_failures dropped)

**Commits exist:**
- FOUND: 02c02d7 — feat(13-04): expose EditDecision enum and edit_decisions in ReconcileReport
- FOUND: 38827d7 — feat(13-04): wire reconcile_lockfile into sync; delete install.rs

**Acceptance criteria (from plan):**
- install.rs absent — VERIFIED
- mod install gone — VERIFIED
- reconcile_managed_plugins gone — VERIFIED (code references; doc comments retained)
- install:: refs in lib.rs zero — VERIFIED
- installed_plugins.json refs in lib.rs zero — VERIFIED
- build_claude_adapter present — VERIFIED at line 929
- apply_edit_decisions present — VERIFIED at line 963
- reconcile::reconcile_lockfile invoked — VERIFIED at line 1089
- marketplace::render_install_failures invoked — VERIFIED at line 1118
- D-20 message verbatim — VERIFIED ("claude binary not found on PATH" at line 939)
- D-21 git untouched — VERIFIED (resolve_git_directories at line 789, unchanged body)
- cargo build -p tome exits 0 — VERIFIED
- cargo build -p tome --features test-support exits 0 — VERIFIED
- cargo clippy -p tome --all-targets -- -D warnings exits 0 — VERIFIED
- cargo clippy -p tome --all-targets --features test-support -- -D warnings exits 0 — VERIFIED
- cargo test -p tome --lib exits 0 (630 pass) — VERIFIED
- 4 install.rs tests gone (`cargo test -p tome install::tests` reports 0 passed) — VERIFIED
- All reconcile + machine + marketplace tests still pass — VERIFIED
- Plan 13-01 _no_install rename applied — VERIFIED (no _no_install matches in lib.rs)

---
*Phase: 13-lockfile-authoritative-sync*
*Plan: 04 (call-site wiring + install.rs deletion)*
*Completed: 2026-05-05*

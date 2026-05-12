---
phase: 18-observability-foundation-sync-diagnostics
plan: 02
type: execute
wave: 2
depends_on:
  - 18-01-tracing-substrate-and-reconcile-proof
files_modified:
  - crates/tome/src/change_cause.rs
  - crates/tome/src/lib.rs
  - crates/tome/src/library.rs
  - crates/tome/src/distribute.rs
  - crates/tome/src/cleanup.rs
  - crates/tome/src/reconcile.rs
autonomous: true
requirements:
  - OBS-01
  - OBS-03
  - OBS-04
  - OBS-05

must_haves:
  truths:
    - "Every `eprintln!`/`println!` site in `crates/tome/src/{library,distribute,cleanup}.rs` that the RESEARCH §Output Channel Split call-site table classifies as `tracing::warn!`/`tracing::info!`/`tracing::debug!` has been migrated; sites classified as STDOUT (keep)/STDERR (keep) remain unchanged."
    - "The `discover.rs` warnings-loop in `lib.rs::sync` (the `for w in &warnings { eprintln!(\"warning: {}\", w); }` block at ~line 1604) emits via `tracing::warn!`."
    - "The `lib.rs::sync` pipeline wraps each of the 5 steps in an explicit `info_span!` block whose names are exactly `discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`; a top-level `info_span!(\"sync\", ...)` wraps the entire pipeline (D-SPAN-1)."
    - "`crates/tome/src/change_cause.rs` exports `enum ChangeCause { HashChanged, PreviouslyFailed, NewlyAdded, DirectoryNowAllowed }` with `ALL: [Self; 4]`, exhaustive-match sentinel, `const_assert!(ChangeCause::ALL.len() == 4, ...)`, and `impl Display` returning the literal strings `\"hash changed\"`, `\"previously failed\"`, `\"newly added\"`, `\"directory now allowed\"` — verbatim per OBS-04 vocabulary."
    - "`library.rs::consolidate_managed` (lines 168-218) and `consolidate_local` (lines 220-318) emit `tracing::info!(skill = %name, directory = %dir, cause = %ChangeCause::HashChanged, \"re-emitted\")` on every `result.updated += 1` branch and `tracing::info!(...cause = %ChangeCause::NewlyAdded, ...)` on every `result.created += 1` branch — local state availability per RESEARCH §Cause Attribution table."
    - "`distribute.rs::distribute_to_directory` emits `tracing::info!(...cause = %ChangeCause::NewlyAdded, ...)` when `target_link.is_symlink()` was false (line ~155) and `tracing::info!(...cause = %ChangeCause::HashChanged, ...)` when the existing symlink was stale (line ~110-145). The `DirectoryNowAllowed` inference branch is either wired per RESEARCH §Cause Attribution recommendation OR captured in `18-deferred-items.md` with a written reason."
    - "`PreviouslyFailed` is NOT wired to any emission site (no manifest schema change in this plan) — its deferral is captured in `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md`."
    - "The inline `reconcile::render_summary(&report, quiet)` call at `lib.rs:1557` is REMOVED. `render_sync_report` (lib.rs:2002+) emits the line `reconcile: ✓ {N} match · ⚠ {M} drift · ⚠ {K} vanished · ⚠ {L} missing-from-machine` immediately above the per-bucket cleanup summary block, populated from `ReconcileReport::{matches, drift, vanished, missing}`."
    - "Per-drift detail lines and per-vanished warnings are relocated from the deleted call site into `render_sync_report` (or invoked from it). Per-drift detail lines stay on STDOUT (RESEARCH §Output Channel Split / reconcile.rs lines 512, 521 — STDOUT keep)."
    - "`render_sync_report` itself stays on STDOUT (uses `println!`) — it is user-facing summary output per D-OUT-1 / RESEARCH Pitfall 3. The reconcile line is `println!`, NOT `tracing::info!`."
    - "Run `cargo run -- sync --verbose 2>&1 | grep -E '\\b(discover|reconcile|consolidate|distribute|cleanup) close'` (or equivalent FmtSpan::CLOSE marker) returns ≥5 lines, one per step, with `time.busy=` field present (RESEARCH §elapsed_ms FINDING — `time.busy` is the auto-emitted name; \"elapsed_ms\" in the success criterion is conceptual)."
  artifacts:
    - path: "crates/tome/src/change_cause.rs"
      provides: "Typed enum + Display impl for re-emit cause attribution; ALL array + exhaustive-match sentinel + const_assert mirrors LogLevel pattern (cli.rs:14-54)."
      contains: "DirectoryNowAllowed"
    - path: "crates/tome/src/library.rs"
      provides: "Migrated 6 eprintln warning sites to tracing::warn! + OBS-04 emission at re-emit decision branches in consolidate_managed (lines 176/190/206) and consolidate_local (lines 246/271/282/298/312)."
      contains: "tracing::info!"
    - path: "crates/tome/src/distribute.rs"
      provides: "Migrated 3 eprintln warning sites to tracing::warn! + OBS-04 emission at re-emit branch (line 164)."
      contains: "tracing::info!"
    - path: "crates/tome/src/cleanup.rs"
      provides: "Migrated 1 eprintln warning site (line 479) to tracing::warn!; `render_cleanup_buckets` and `render_distribution_cleanup_failures` stay as direct stderr writers (RESEARCH §cleanup.rs STDERR keep)."
      contains: "tracing::warn!"
    - path: "crates/tome/src/lib.rs"
      provides: "lib.rs::sync 97-site migration sweep; top-level `info_span!(\"sync\")` + 5 step spans (`discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`); deletion of inline reconcile::render_summary call at line 1557; SyncReport extended with `reconcile: Option<ReconcileReport>` field; render_sync_report emits the OBS-05 classification line + relocated per-drift detail; discover warnings-loop migrated to tracing::warn!."
      contains: "info_span!"
    - path: "crates/tome/src/reconcile.rs"
      provides: "Adapter helper `format_classification_detail(&ReconcileReport) -> String` (or equivalent helper per RESEARCH §render_sync_report extension lines 940-947) that returns only the per-drift `• X: 1.0 → 2.0` lines + per-vanished warnings (NO classification line) so render_sync_report owns the classification line and orchestrates the detail. Existing `format_summary` and `render_summary` functions stay callable but their inline call from lib.rs:1557 is removed."
      contains: "format_classification_detail"
  key_links:
    - from: "crates/tome/src/library.rs (consolidate_managed/_local re-emit branches)"
      to: "global tracing subscriber"
      via: "tracing::info!(skill=%name, directory=%dir, cause=%cause, \"re-emitted\") at each result.updated/result.created decision branch"
      pattern: "cause = %ChangeCause::"
    - from: "crates/tome/src/distribute.rs (distribute_to_directory line ~164 result.changed += 1)"
      to: "global tracing subscriber"
      via: "tracing::info! at the symlink-create branch"
      pattern: "cause = %ChangeCause::"
    - from: "crates/tome/src/lib.rs::sync per-step boundaries"
      to: "FmtSpan::CLOSE events on stderr"
      via: "let _span = info_span!(\"step_name\").entered(); { ... } drop(_span);"
      pattern: "info_span!\\(\"(discover|reconcile|consolidate|distribute|cleanup|sync)\""
    - from: "crates/tome/src/lib.rs::render_sync_report"
      to: "stdout (println!) — user-facing summary"
      via: "println!(\"  reconcile: {} {} match · {} {} drift · {} {} vanished · {} {} missing-from-machine\", ...)"
      pattern: "reconcile: .* match .* drift .* vanished .* missing-from-machine"
    - from: "crates/tome/src/lib.rs (deleted inline reconcile::render_summary call)"
      to: "render_sync_report (final summary block)"
      via: "removal of `reconcile::render_summary(&report, quiet)` at ex-line 1557; SyncReport gains `reconcile: Option<ReconcileReport>` field threaded through to render_sync_report"
      pattern: "reconcile: Option<ReconcileReport>"
---

<objective>
Sweep the four remaining in-scope modules (`library.rs`, `distribute.rs`, `cleanup.rs`, `lib.rs::sync` + the discover warnings emission point) onto `tracing::{info,warn,debug}!`; layer on the three Phase 18 sync-diagnostic features:

1. **OBS-03 spans:** one top-level `info_span!("sync")` + 5 step spans (`discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`) per D-SPAN-1, emitting `time.busy`/`time.idle` on close per D-SPAN-2.
2. **OBS-04 cause attribution:** new `crates/tome/src/change_cause.rs` module exporting `ChangeCause { HashChanged, PreviouslyFailed, NewlyAdded, DirectoryNowAllowed }` (matches POLISH-04 `ALL` + exhaustive-match sentinel + Display pattern); emit `tracing::info!(skill=%name, directory=%dir, cause=%cause, "re-emitted")` at the decision branches in `consolidate_managed`/`consolidate_local`/`distribute_to_directory` for `HashChanged` and `NewlyAdded`. Attempt locally-computable `DirectoryNowAllowed` inference per RESEARCH §Cause Attribution Open Question 2; if not feasible, defer with a written entry. `PreviouslyFailed` is deferred unconditionally (manifest schema bump out of scope) — defer entry mandatory.
3. **OBS-05 reconcile breakdown line:** delete inline `reconcile::render_summary(&report, quiet)` at `lib.rs:1557`; extend `SyncReport` with `reconcile: Option<ReconcileReport>`; have `render_sync_report` emit `reconcile: ✓ {N} match · ⚠ {M} drift · ⚠ {K} vanished · ⚠ {L} missing-from-machine` IMMEDIATELY ABOVE the per-bucket cleanup summary; relocate per-drift detail + per-vanished warnings from the deleted call site into `render_sync_report` (or invoke from it).

This is the locked Plan B per CONTEXT.md D-SUB-2 ("sweep the remaining 4 in-scope modules + OBS-03 spans + OBS-04 ChangeCause emission + OBS-05 reconcile-line relocation"). The 5-task shape stretches the standard 2-3-tasks-per-plan budget; per D-SUB-2 the grouping is locked because the four migration vectors share file ownership in `lib.rs` and cannot safely run as four parallel plans.

**`elapsed_ms` field name caveat (RESEARCH §elapsed_ms FINDING + Pitfall 5):** OBS-03's success criterion says "elapsed_ms field on span close." `FmtSpan::CLOSE` with the default fmt formatter auto-emits `time.busy` and `time.idle` field names — NOT a literal `elapsed_ms` field. This plan accepts the auto-emitted timing fields as satisfying OBS-03 per RESEARCH §elapsed_ms FINDING option 1 (the recommended path). Document in the SUMMARY: "OBS-03 timing is satisfied by `time.busy=<value>` auto-emitted via `FmtSpan::CLOSE`; verification greps for `time.busy=` not `elapsed_ms=`." This is the dispositive choice — no per-step `Instant::now()` recording.

**`DirectoryNowAllowed` attempt:** the locally-computable inference proposed in RESEARCH §Cause Attribution Open Question 2 ("skill in manifest, allowed now, target has no symlink yet") is wired in `distribute.rs::distribute_to_directory` at the new-symlink-create branch when the skill IS in the manifest (not a NewlyAdded case) AND the target has no existing symlink. If the inference is infeasible in implementation (e.g., it produces false positives), the task creates `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` documenting the reason.

**`PreviouslyFailed` deferral:** unconditional, captured in `18-deferred-items.md` with the manifest-schema-bump rationale from RESEARCH §Cause Attribution Open Question 1. The enum variant + Display impl still ship (greppability) — only the emission site is deferred.

Output: `change_cause.rs` (new); migrated `library.rs`, `distribute.rs`, `cleanup.rs`; rewritten `lib.rs::sync` per-step span structure + 97-site migration sweep + `render_sync_report` extension + `SyncReport` field addition; deleted `reconcile::render_summary` call at line 1557; helper `format_classification_detail` (or equivalent) in `reconcile.rs`; mandatory `18-deferred-items.md` for `PreviouslyFailed` and possibly `DirectoryNowAllowed`.
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
@.planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md
@.planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md
@.planning/phases/18-observability-foundation-sync-diagnostics/18-01-tracing-substrate-and-reconcile-proof-PLAN.md
@.planning/phases/18-observability-foundation-sync-diagnostics/18-01-SUMMARY.md

@crates/tome/src/lib.rs
@crates/tome/src/library.rs
@crates/tome/src/distribute.rs
@crates/tome/src/cleanup.rs
@crates/tome/src/reconcile.rs
@crates/tome/src/discover.rs
@crates/tome/src/cli.rs

<interfaces>
<!-- Key types and call sites the executor needs. Extracted from the codebase. -->

From crates/tome/src/lib.rs (current SyncReport at lines 102-107 — TARGET for extension):

```rust
pub struct SyncReport {
    pub consolidate: ConsolidateResult,
    pub distributions: Vec<DistributeResult>,
    pub cleanup: CleanupResult,
    pub removed_from_targets: usize,
    // NEW: thread reconcile report through to render_sync_report for OBS-05.
    // Optional because not every sync invokes reconcile (only when a Claude
    // adapter is built per line 1538).
    pub reconcile: Option<ReconcileReport>,
}
```

From crates/tome/src/reconcile.rs (ReconcileReport at lines 100-116 — READ ONLY, do not modify schema):

```rust
pub struct ReconcileReport {
    pub matches: usize,            // → reconcile-line "N match"
    pub drift: Vec<Classified>,    // → reconcile-line "M drift" via .len()
    pub vanished: Vec<Classified>, // → reconcile-line "K vanished" via .len()
    pub missing: Vec<Classified>,  // → reconcile-line "L missing-from-machine" via .len()
    pub edited: Vec<Edited>,
    pub install_failures: Vec<InstallFailure>,
    pub apply_skipped: bool,
    pub edit_decisions: Vec<EditDecision>,
}
```

From crates/tome/src/lib.rs (CURRENT render_sync_report at lines 2002-2037 — TARGET for OBS-05 extension):

```rust
fn render_sync_report(report: &SyncReport) {
    println!("{}", style("Sync complete").green().bold());
    println!("  Library: {} created, {} unchanged, {} updated{}", ...);
    for dr in &report.distributions { println!("  {}: ...", ...); }
    if report.cleanup.removed_from_library > 0 { println!("  Cleaned {} stale entry/entries", ...); }
    if report.removed_from_targets > 0 { println!("  Cleaned {} stale target link(s)", ...); }
    // NEW (OBS-05): print reconcile classification line here, immediately above
    // the cleanup-bucket render call (which today is at lib.rs:1768 on stderr).
}
```

From crates/tome/src/lib.rs (CURRENT inline reconcile::render_summary call site at line 1557 — TARGET for deletion):

```rust
if let Some(claude_adapter) = build_claude_adapter(config)? {
    let report = reconcile::reconcile_lockfile(...)?;

    if !quiet {
        reconcile::render_summary(&report, quiet);  // <<< DELETE THIS LINE
    }

    // apply_edit_decisions / render_install_failures stay — those are
    // separate code paths per RESEARCH §Cause Attribution (lib.rs:1567-1571).
}
```

After deletion the `report` variable must still be reachable for the `apply_edit_decisions(&report, ...)` call below it AND must be threaded into the final `SyncReport`. Strategy: rename to `reconcile_report` and clone/move into the SyncReport literal at line 1793.

From crates/tome/src/lib.rs (CURRENT cleanup-bucket render call at lines 1764-1778 — REFERENCE, stays on stderr per D-OUT-2):

```rust
if !quiet {
    let mut stderr = std::io::stderr().lock();
    let _ = cleanup::render_cleanup_buckets(&mut stderr, ...);
    let _ = cleanup::render_distribution_cleanup_failures(&mut stderr, ...);
}
```

The OBS-05 reconcile line is emitted INSIDE `render_sync_report` (which is called at line 1801 AFTER this stderr block). Per D-ENV-4 the success criterion "immediately above the cleanup buckets" — the cleanup-buckets render call MOVES into `render_sync_report` (option 2 in RESEARCH §render_sync_report extension), OR is reordered so the reconcile line at `render_sync_report` call site precedes the stderr cleanup block (option 1). Task 5 decides; the success criterion only requires the visual ordering (reconcile line above cleanup buckets), not common ownership.

From crates/tome/src/library.rs (re-emit decision branches per RESEARCH §Cause Attribution table — TARGETS for OBS-04 emission):

```rust
// consolidate_managed (line 168-218):
//   line 176: result.updated += 1  → flip-managed case (HashChanged equivalent — content same, flag flipped)
//   line 190: result.updated += 1  → "Content changed or force — re-copy" → HashChanged
//   line 206: result.created += 1  → DestinationState::Empty new-skill → NewlyAdded

// consolidate_local (line 220-318):
//   line 246: result.updated += 1  → managed→local strategy transition → HashChanged
//   line 271: result.updated += 1  → legacy v0.1.x symlink migration → HashChanged
//   line 282: result.updated += 1  → flip-managed case (symmetric to line 176) → HashChanged
//   line 298: result.updated += 1  → "Content changed or force — re-copy" → HashChanged
//   line 312: result.created += 1  → New skill copy → NewlyAdded
```

Note: line 176 (and 282) is a flag-flip-only case where content_hash is unchanged. The literal cause is technically "managed flag changed" not "hash changed". Per RESEARCH the local pattern lumps these into `HashChanged` because no separate cause variant exists; this is an acceptable approximation (the user-visible behavior — "skill was re-emitted in the manifest" — is the same). Alternatively the task may demote these flag-flip cases to NOT emit a ChangeCause event because they don't re-copy bytes; only line 190/298 (actual re-copy) and 206/312 (create) emit. Task 3 picks the policy and documents in the per-task SUMMARY.

From crates/tome/src/distribute.rs (re-emit decision branches per RESEARCH §Cause Attribution table):

```rust
// distribute_to_directory (line 67-167):
//   line 164: result.changed += 1
// This single counter-increment site covers BOTH cases:
//   - "Update stale link" branch (line 110-145): target_link was a symlink, but
//     symlink_points_to() returned false → was pointing somewhere else → HashChanged
//   - "Create new symlink" path (line 146-153 falls through to 154-163): target_link
//     was not a symlink at all → NewlyAdded
// And the proposed inference:
//   - DirectoryNowAllowed: skill is in manifest AND target has no symlink AND
//     manifest indicates it was distributed before. RESEARCH OQ-2 recommends
//     wiring this in distribute.rs at the new-symlink-create branch when
//     `manifest.contains(skill)` is true (NewlyAdded only when manifest doesn't
//     have the skill yet, which is unusual at distribute-time because consolidate
//     already added it).
```
</interfaces>

</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Create crates/tome/src/change_cause.rs with ChangeCause enum + Display + ALL + sentinel</name>
  <files>crates/tome/src/change_cause.rs, crates/tome/src/lib.rs</files>
  <read_first>
    - crates/tome/src/cli.rs lines 14-54 (existing LogLevel pattern — `ALL`, `_log_level_exhaustiveness`, `const_assert!`)
    - crates/tome/src/migration_v010.rs lines 53-76 (existing MigrationFailureKind pattern — RESEARCH cites this as the verbatim template)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Cause Attribution / change_cause.rs (lines 678-742) — full module content
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md D-SPAN-3 — locked enum vocabulary + Display strings
  </read_first>
  <behavior>
    - `ChangeCause::HashChanged.to_string() == "hash changed"`
    - `ChangeCause::PreviouslyFailed.to_string() == "previously failed"`
    - `ChangeCause::NewlyAdded.to_string() == "newly added"`
    - `ChangeCause::DirectoryNowAllowed.to_string() == "directory now allowed"`
    - `ChangeCause::ALL` has length 4 (enforced by `const_assert!`)
    - Adding a 5th enum variant without updating `ALL` is a compile error (POLISH-04 sentinel pattern)
  </behavior>
  <action>
    Step 1 — Create new file `crates/tome/src/change_cause.rs` with EXACT content (per RESEARCH §Cause Attribution / change_cause.rs lines 682-742):

    ```rust
    //! `ChangeCause` — typed reason a skill was re-emitted by consolidate or
    //! distribute. OBS-04 (Phase 18) locks the four user-facing strings.
    //!
    //! Greppability matters: `grep "cause=hash changed" sync-output.txt` is
    //! the user's debugging workflow. Renaming any string is a BREAKING change
    //! to the OBS-04 contract.

    use std::fmt;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ChangeCause {
        HashChanged,
        PreviouslyFailed,
        NewlyAdded,
        DirectoryNowAllowed,
    }

    impl ChangeCause {
        /// POLISH-04 exhaustiveness sentinel — compile fails if a new variant
        /// is added without updating ChangeCause::ALL. Mirrors LogLevel::ALL
        /// (cli.rs:28) and MigrationFailureKind::ALL (migration_v010.rs:53).
        pub const ALL: [Self; 4] = [
            Self::HashChanged,
            Self::PreviouslyFailed,
            Self::NewlyAdded,
            Self::DirectoryNowAllowed,
        ];
    }

    #[allow(dead_code)]
    fn _change_cause_exhaustiveness(c: ChangeCause) {
        match c {
            ChangeCause::HashChanged => {}
            ChangeCause::PreviouslyFailed => {}
            ChangeCause::NewlyAdded => {}
            ChangeCause::DirectoryNowAllowed => {}
        }
    }

    const _: () = assert!(
        ChangeCause::ALL.len() == 4,
        "ChangeCause::ALL must list every variant",
    );

    impl fmt::Display for ChangeCause {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let s = match self {
                Self::HashChanged => "hash changed",
                Self::PreviouslyFailed => "previously failed",
                Self::NewlyAdded => "newly added",
                Self::DirectoryNowAllowed => "directory now allowed",
            };
            f.write_str(s)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn display_strings_match_obs04_vocabulary() {
            // Hard-pin the OBS-04 grep vocabulary. Renaming is BREAKING.
            assert_eq!(ChangeCause::HashChanged.to_string(), "hash changed");
            assert_eq!(ChangeCause::PreviouslyFailed.to_string(), "previously failed");
            assert_eq!(ChangeCause::NewlyAdded.to_string(), "newly added");
            assert_eq!(ChangeCause::DirectoryNowAllowed.to_string(), "directory now allowed");
        }

        #[test]
        fn all_array_has_length_four() {
            assert_eq!(ChangeCause::ALL.len(), 4);
        }
    }
    ```

    Step 2 — In `crates/tome/src/lib.rs`, add `pub mod change_cause;` to the alphabetical `pub mod` declaration block (between `pub mod cli;` and `pub mod cleanup;` or wherever alphabetical order places it). Add `pub use change_cause::ChangeCause;` to the re-export block near the top of `lib.rs` so downstream module imports can use the short path `use crate::ChangeCause`.

    DO NOT add the enum to any other module's `mod.rs` or re-export it from anywhere else. One canonical home (`change_cause.rs`), one re-export (`lib.rs`).
  </action>
  <verify>
    <automated>cargo test -p tome --lib change_cause::tests 2>&amp;1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - File `crates/tome/src/change_cause.rs` exists
    - `rg "pub enum ChangeCause" crates/tome/src/change_cause.rs` returns 1 match
    - `rg "\"hash changed\"" crates/tome/src/change_cause.rs` returns 1 match
    - `rg "\"previously failed\"" crates/tome/src/change_cause.rs` returns 1 match
    - `rg "\"newly added\"" crates/tome/src/change_cause.rs` returns 1 match
    - `rg "\"directory now allowed\"" crates/tome/src/change_cause.rs` returns 1 match
    - `rg "pub const ALL: \\[Self; 4\\]" crates/tome/src/change_cause.rs` returns 1 match
    - `rg "ChangeCause::ALL.len\\(\\) == 4" crates/tome/src/change_cause.rs` returns 1 match
    - `rg "_change_cause_exhaustiveness" crates/tome/src/change_cause.rs` returns 1 match (sentinel present)
    - `rg "pub mod change_cause;" crates/tome/src/lib.rs` returns 1 match
    - `cargo test -p tome --lib change_cause::tests` exits 0 (both `display_strings_match_obs04_vocabulary` and `all_array_has_length_four` pass)
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>`ChangeCause` enum + `ALL` + sentinel + Display impl exists with the four locked vocabulary strings; mirrors the LogLevel/MigrationFailureKind precedent; unit tests verify the vocabulary.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Migrate library.rs eprintln warning sites + emit OBS-04 ChangeCause events at re-emit decision branches</name>
  <files>crates/tome/src/library.rs</files>
  <read_first>
    - crates/tome/src/library.rs (full file — verify line numbers for the 6 eprintln sites + 8 result.updated/result.created branches)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Output Channel Split / library.rs (lines 451-461) — the 6 eprintln warning sites table
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Cause Attribution / library.rs (lines 749-757) — the variant-to-branch table
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Code Examples / OBS-04 emission (lines 1266-1286)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md D-SPAN-4 — decision-site emission, no result-struct extension
  </read_first>
  <behavior>
    - Six `eprintln!("warning: ...")` sites in `library.rs` (lines ~161, 193, 209, 263, 301, 352 per RESEARCH §Output Channel Split / library.rs) replaced with `tracing::warn!`
    - Each of the 8 `result.updated += 1` / `result.created += 1` branches (per RESEARCH §Cause Attribution table) is followed by a `tracing::info!(skill = %skill.name, directory = %dir_name_or_equivalent, cause = %cause, "re-emitted")` event
    - HashChanged variant fires at content-changed branches (lines 190, 271, 298 — actual content re-copy)
    - NewlyAdded variant fires at create branches (lines 206, 312 — `result.created += 1`)
    - Flag-flip-only branches (line 176, 282 in consolidate_managed/consolidate_local where `entry.managed != current_managed` but content unchanged): treat as `HashChanged` per RESEARCH (no separate variant exists; documented in SUMMARY) — emit the event so the manifest mutation is visible in the trace
    - Existing `cargo test -p tome --lib -- library::tests` passes (unit tests assert on result counts, not stderr text)
    - PreviouslyFailed variant is NOT emitted from library.rs (no manifest schema bump in scope)
    - DirectoryNowAllowed variant is NOT emitted from library.rs (this is distribute's territory per RESEARCH §Cause Attribution table)
  </behavior>
  <action>
    Step 1 — At top of `crates/tome/src/library.rs` in the existing `use` block, add:

    ```rust
    use tracing::{info, warn};

    use crate::change_cause::ChangeCause;
    ```

    Insert alphabetically. The `tracing` use is new; the `change_cause` use crosses module boundary so a fully-qualified `crate::change_cause::ChangeCause` is also acceptable.

    Step 2 — Migrate the 6 `eprintln!("warning: ...")` sites. Use `rg -n 'eprintln!\\("warning:' crates/tome/src/library.rs` to enumerate the current line numbers (may have drifted from RESEARCH). Conversion pattern:

    BEFORE:
    ```rust
    eprintln!("warning: {} is a v0.9-shape symlink for managed skill — ...", dest.display());
    ```

    AFTER:
    ```rust
    warn!("{} is a v0.9-shape symlink for managed skill — ...", dest.display());
    ```

    Drop the literal `"warning: "` prefix — tracing's warn macro renders with the level prefix per D-OUT-4. Per RESEARCH §Pitfall 6, if any site wraps an arg in `console::style(...).yellow()` drop the wrapper.

    Step 3 — Emit OBS-04 ChangeCause events at the 8 result.updated/result.created decision branches. For each branch:

    AT line ~176 (consolidate_managed, flag-flip case after `result.updated += 1`):
    ```rust
    // OBS-04 emission: managed-flag flip with unchanged content. Locally
    // approximated as HashChanged (no separate "flag flipped" variant in
    // CONTEXT.md D-SPAN-3 vocabulary; documented in 18-02 SUMMARY).
    info!(
        skill = %skill.name,
        directory = %skill.source_name,
        cause = %ChangeCause::HashChanged,
        "re-emitted",
    );
    ```

    AT line ~190 (consolidate_managed, content-changed-or-force re-copy):
    ```rust
    info!(
        skill = %skill.name,
        directory = %skill.source_name,
        cause = %ChangeCause::HashChanged,
        "re-emitted",
    );
    ```

    AT line ~206 (consolidate_managed, DestinationState::Empty new skill):
    ```rust
    info!(
        skill = %skill.name,
        directory = %skill.source_name,
        cause = %ChangeCause::NewlyAdded,
        "re-emitted",
    );
    ```

    AT line ~246 (consolidate_local, managed→local strategy transition):
    ```rust
    info!(
        skill = %skill.name,
        directory = %skill.source_name,
        cause = %ChangeCause::HashChanged,
        "re-emitted",
    );
    ```

    AT line ~271 (consolidate_local, legacy v0.1.x symlink migration):
    ```rust
    info!(
        skill = %skill.name,
        directory = %skill.source_name,
        cause = %ChangeCause::HashChanged,
        "re-emitted",
    );
    ```

    AT line ~282 (consolidate_local, flag-flip case):
    ```rust
    info!(
        skill = %skill.name,
        directory = %skill.source_name,
        cause = %ChangeCause::HashChanged,
        "re-emitted",
    );
    ```

    AT line ~298 (consolidate_local, content-changed-or-force re-copy):
    ```rust
    info!(
        skill = %skill.name,
        directory = %skill.source_name,
        cause = %ChangeCause::HashChanged,
        "re-emitted",
    );
    ```

    AT line ~312 (consolidate_local, new skill copy):
    ```rust
    info!(
        skill = %skill.name,
        directory = %skill.source_name,
        cause = %ChangeCause::NewlyAdded,
        "re-emitted",
    );
    ```

    The `directory` field uses `skill.source_name` — verify this is a Display-able type in `DiscoveredSkill`. If `source_name` is `Option<DirectoryName>` (Unowned case), use `skill.source_name.as_ref().map(|d| d.as_str()).unwrap_or("<unowned>")` or equivalent. Examine the `DiscoveredSkill` struct definition in `discover.rs` to confirm the field type before committing.

    Step 4 — Run `cargo test -p tome --lib -- library::tests` to confirm all existing unit tests pass (they assert on result counts, not tracing output, so they should be untouched). Run `cargo clippy --all-targets -- -D warnings` and resolve any lints.
  </action>
  <verify>
    <automated>rg -n 'eprintln!\\("warning:' crates/tome/src/library.rs | wc -l | tr -d ' '</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'eprintln!\\("warning:' crates/tome/src/library.rs | wc -l` returns 0
    - `rg "use tracing::\\{info, warn\\}" crates/tome/src/library.rs` returns 1 match (or equivalent split-import form)
    - `rg "use crate::change_cause::ChangeCause" crates/tome/src/library.rs` returns 1 match (or `crate::ChangeCause` if re-exported)
    - `rg -c "warn!\\(" crates/tome/src/library.rs` returns ≥ 6
    - `rg -c "cause = %ChangeCause::" crates/tome/src/library.rs` returns ≥ 5 (the 5 verified re-emit sites; the 3 flag-flip approximation sites may or may not be present per Task SUMMARY decision)
    - `rg -c "cause = %ChangeCause::HashChanged" crates/tome/src/library.rs` returns ≥ 3
    - `rg -c "cause = %ChangeCause::NewlyAdded" crates/tome/src/library.rs` returns ≥ 2
    - `cargo test -p tome --lib -- library::tests` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>library.rs has zero `eprintln!("warning: ...")` sites; OBS-04 emission events fire at every re-emit decision branch with HashChanged/NewlyAdded variants; existing unit tests pass.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 3: Migrate distribute.rs + cleanup.rs warning sites; emit OBS-04 events in distribute (incl. DirectoryNowAllowed inference attempt)</name>
  <files>crates/tome/src/distribute.rs, crates/tome/src/cleanup.rs</files>
  <read_first>
    - crates/tome/src/distribute.rs lines 67-167 (distribute_to_directory — the single result.changed += 1 site at line 164 splits HashChanged vs NewlyAdded vs DirectoryNowAllowed)
    - crates/tome/src/distribute.rs (3 eprintln warning sites at lines ~100, ~132, ~147 per RESEARCH §Output Channel Split / distribute.rs)
    - crates/tome/src/cleanup.rs line ~479 (the single eprintln warning site per RESEARCH §Output Channel Split / cleanup.rs)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Cause Attribution / distribute.rs (lines 760-767) — variant table
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Open Questions Q2 (lines 1329-1339) — DirectoryNowAllowed inference recommendation
  </read_first>
  <behavior>
    - distribute.rs: 3 `eprintln!("warning: ...")` sites migrated to `tracing::warn!`
    - distribute.rs: at the symlink-create/replace branch (currently around line 164 where `result.changed += 1`), emit `tracing::info!(skill=..., directory=..., cause=%cause, "re-emitted")` with cause determined by local state:
      - `HashChanged` when `target_link.is_symlink()` was true before this iteration (stale-link update path; existing symlink was replaced)
      - `NewlyAdded` when `target_link.is_symlink()` AND `target_link.exists()` were both false AND the skill is NOT in the manifest (new skill, first sync)
      - `DirectoryNowAllowed` when target had no symlink BUT skill IS in the manifest (was distributed before, target lost the symlink — most plausibly because the skill was disabled then re-enabled in machine.toml; per RESEARCH OQ-2 inference)
    - If the DirectoryNowAllowed inference produces false positives during implementation (e.g., the skill simply was never distributed to this directory because machine_prefs disabled it from the start), the implementor documents the limitation in `18-deferred-items.md` and demotes those cases to `NewlyAdded` (acceptable degradation per RESEARCH)
    - cleanup.rs: 1 `eprintln!("warning: ...")` at line ~479 migrated to `tracing::warn!`
    - cleanup.rs: `render_cleanup_buckets` and `render_distribution_cleanup_failures` STAY as direct stderr writers (`&mut impl Write`) — they are user-facing ceremonial output per RESEARCH §Output Channel Split / cleanup.rs STDERR (keep)
    - Existing `cargo test -p tome --lib -- distribute::tests cleanup::tests` passes
  </behavior>
  <action>
    Step 1 — In `crates/tome/src/distribute.rs` at the top of the file in the existing `use` block, add:

    ```rust
    use tracing::{info, warn};

    use crate::change_cause::ChangeCause;
    ```

    Step 2 — Migrate the 3 `eprintln!("warning: ...")` sites in distribute.rs (lines ~100, ~132, ~147). Use `rg -n 'eprintln!\\("warning:' crates/tome/src/distribute.rs` to enumerate current line numbers. Pattern:

    BEFORE:
    ```rust
    eprintln!("warning: failed to remove legacy symlink {}: {}", target_link.display(), e);
    ```

    AFTER:
    ```rust
    warn!("failed to remove legacy symlink {}: {}", target_link.display(), e);
    ```

    Same drop-the-prefix pattern as Plan 18-01 reconcile and Task 2 library.

    For the multi-line line ~132 foreign-symlink warning (RESEARCH calls out the multi-line nature; tracing's compact format flattens to one line — acceptable per RESEARCH §Output Channel Split / distribute.rs line 132), the executor MAY need to join the multi-line message into a single format string. The `--force` hint must remain in the message text.

    Step 3 — Emit the OBS-04 ChangeCause event in distribute.rs at the symlink creation point. Locate the `result.changed += 1` at line ~164 in `distribute_to_directory`. The decision pattern:

    ```rust
    // BEFORE the increment, capture the cause from local state.
    let was_symlink = target_link.is_symlink();
    let in_manifest = manifest.get(skill_name_str.as_ref()).is_some();

    // ... existing remove-stale-link / foreign-symlink / not-a-symlink branches ...

    if !dry_run {
        unix_fs::symlink(&library_skill_path, &target_link).with_context(|| { ... })?;
    }
    result.changed += 1;

    // OBS-04 emission: classify the cause from snapshots taken at the
    // start of this iteration (before any removal happened).
    //
    // - was_symlink && symlink_points_to() was false → stale link replaced → HashChanged
    // - !was_symlink && in_manifest → skill in manifest but no symlink in target;
    //   plausibly machine_prefs allowed it this sync after being disabled →
    //   DirectoryNowAllowed (inference per RESEARCH OQ-2; may be a false positive
    //   on the very first sync after the skill is added — track in deferred-items
    //   if false-positive cases dominate)
    // - !was_symlink && !in_manifest → first distribution of this skill to this
    //   target → NewlyAdded
    let cause = if was_symlink {
        ChangeCause::HashChanged
    } else if in_manifest {
        ChangeCause::DirectoryNowAllowed
    } else {
        ChangeCause::NewlyAdded
    };
    info!(
        skill = %skill_name_str,
        directory = %dir_name,
        cause = %cause,
        "re-emitted",
    );
    ```

    Adapt variable names (`skill_name_str`, `dir_name`) to whatever the local binding names actually are at line ~164. Use Read first to verify the surrounding code.

    Step 4 — In `crates/tome/src/cleanup.rs`, at the top in the existing `use` block, add:

    ```rust
    use tracing::warn;
    ```

    (No `info` import needed for cleanup — only the one `eprintln!("warning: ...")` site at line ~479 migrates; the renderers stay as direct writers per RESEARCH.)

    Step 5 — Migrate the cleanup.rs `eprintln!` at line ~479 (`"warning: could not canonicalize library path ..."`):

    BEFORE:
    ```rust
    eprintln!("warning: could not canonicalize library path {} ...", ...);
    ```

    AFTER:
    ```rust
    warn!("could not canonicalize library path {} ...", ...);
    ```

    Step 6 — DO NOT touch:
    - `render_cleanup_buckets` (lines 135-238) — stays as `&mut impl Write` writer pattern (RESEARCH STDERR keep)
    - `render_distribution_cleanup_failures` (lines 244-278) — stays as writer pattern (RESEARCH STDERR keep)
    - The existing `cleanup_module_source_does_not_contain_forbidden_phrase` test at line ~1252 — RESEARCH confirms it continues to pass byte-for-byte after this migration
    - The doc-comment reference to `eprintln!` discipline at line ~30 — update the comment text in passing to mention `tracing::warn!` instead of `eprintln!`, but it is doc-only and not load-bearing.

    Step 7 — Decision point: after wiring the `DirectoryNowAllowed` inference, mentally walk through 3 test cases:
    1. Fresh machine, first sync, new skill → expected emission: `NewlyAdded` ✓ (`!was_symlink && !in_manifest`)
    2. Existing skill, edited in source, re-sync → expected: `HashChanged` ✓ (`was_symlink && symlink_points_to() false`)
    3. Skill disabled in machine.toml, then re-enabled, re-sync → expected: `DirectoryNowAllowed` ✓ (`!was_symlink && in_manifest`)

    If walkthrough surfaces a contradicting case (e.g., a skill that's in manifest but truly being distributed for the first time to THIS specific directory — possible if directories were added incrementally), document the false-positive case in `18-deferred-items.md` (file mandatory in Task 6 anyway; this task either creates it OR appends to it).
  </action>
  <verify>
    <automated>rg -n 'eprintln!\\("warning:' crates/tome/src/distribute.rs crates/tome/src/cleanup.rs | wc -l | tr -d ' '</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'eprintln!\\("warning:' crates/tome/src/distribute.rs | wc -l` returns 0
    - `rg -n 'eprintln!\\("warning:' crates/tome/src/cleanup.rs | wc -l` returns 0
    - `rg "use tracing::\\{info, warn\\}" crates/tome/src/distribute.rs` returns 1 match
    - `rg "use tracing::warn" crates/tome/src/cleanup.rs` returns 1 match
    - `rg "cause = %ChangeCause::" crates/tome/src/distribute.rs` returns ≥ 1 match (DirectoryNowAllowed plus HashChanged plus NewlyAdded)
    - `rg "cause = %ChangeCause::DirectoryNowAllowed" crates/tome/src/distribute.rs` returns 1 match OR `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` exists and documents the deferral reason
    - `rg "render_cleanup_buckets" crates/tome/src/cleanup.rs | wc -l` returns ≥ 1 (function still exists; was NOT migrated to tracing)
    - `cargo test -p tome --lib -- distribute::tests cleanup::tests` exits 0
    - `cargo test -p tome --lib cleanup::tests::cleanup_module_source_does_not_contain_forbidden_phrase` exits 0 (Phase 16 D-UX01-3 invariant preserved)
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>distribute.rs has zero `eprintln!("warning: ...")`; OBS-04 events emit at line 164 with cause picked from local state (HashChanged/NewlyAdded/DirectoryNowAllowed); cleanup.rs has one eprintln-to-tracing migration; render_cleanup_buckets unchanged; all unit tests pass.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 4: lib.rs sweep — migrate sync pipeline eprintln/println sites + add 5 step spans + top-level sync span; discover warnings emission to warn!</name>
  <files>crates/tome/src/lib.rs</files>
  <read_first>
    - crates/tome/src/lib.rs lines 1464-2037 (the full sync function body + render_sync_report — heaviest single read in this plan)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Output Channel Split / lib.rs (lines 495-545) — the 97-site heuristic + per-line recommendations for the key sites
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Span Shape (lines 559-621) — the 5-step span pattern + naming
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Common Pitfalls Pitfall 4 (span lifetime + `?` early returns)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md D-SPAN-1, D-SPAN-2, D-OUT-3
  </read_first>
  <behavior>
    - `lib.rs::sync` opens with `let _sync_span = tracing::info_span!("sync", dry_run = dry_run, force = force).entered();` (or equivalent shape with at least `dry_run` + `force` as recorded fields)
    - The 5 pipeline-step boundaries inside `sync` are each bracketed by `{ let _span = tracing::info_span!("step_name").entered(); ... }` lexical scope blocks; step names are exactly: `discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`
    - The `discover` span wraps both `resolve_git_directories` AND `discover::discover_all` per RESEARCH §Span Shape recommendation (single span covering the full discovery work)
    - The `reconcile` span wraps the `if let Some(claude_adapter) = build_claude_adapter(config)? { ... reconcile::reconcile_lockfile(...) ... }` block at line 1538
    - The `consolidate` span wraps the `library::consolidate(...)` call at line 1640
    - The `distribute` span wraps the per-directory distribute loop (use `rg -n "distribute::distribute_to_directory|fn distribute" crates/tome/src/lib.rs` to find the exact location, currently around lines 1693-1730)
    - The `cleanup` span wraps the cleanup pipeline (cleanup::cleanup_library + cleanup_disabled_from_target + the stderr bucket render at line 1768)
    - Every `eprintln!("warning: ...")` site in the sync function body migrates to `tracing::warn!` per RESEARCH §Output Channel Split / lib.rs (lines 1506, 1519, 1576, 1604, 1808, 1835, 2046, 2195, 2385, 2386)
    - The discover warnings emission loop at line ~1604 `for w in &warnings { eprintln!("warning: {}", w); }` migrates to `tracing::warn!("{}", w)` inside the loop
    - The `if verbose { eprintln!("Resolving git sources..."); }` style lines at lines 1584, 1594, 1616, 1638, 1684, 1704, 1717 are DELETED — per D-OUT-3 they become redundant once span CLOSE events print the same step name with `time.busy`; trust the span (RESEARCH §Output Channel Split / lib.rs tracing::debug! recommendation)
    - The `if verbose { eprintln!("  Found {} skills", skills.len()); }` style payload-carrying verbose lines at lines 1616 etc. migrate to `tracing::debug!("Found {} skills", skills.len())` — they carry payload the span doesn't
    - STDOUT user-facing lines (e.g., `println!("tome {}", ...)` version at line 189; `println!("Sync complete")` in render_sync_report; list/status table prints; completions/config/git-init prints; "Pulled changes from remote" + "Pushed to remote") STAY AS `println!` per RESEARCH §Output Channel Split / lib.rs STDOUT (keep)
    - Wizard chrome lines (around dialoguer prompts: 215, 216, 656, 672, 674, 793, 808, 810, 894, 968, 998 per RESEARCH) STAY as `eprintln!` per HARD-15 / RESEARCH STDERR (keep)
    - `cargo run -- sync --verbose 2>&1` against a non-empty repo emits at least 5 span CLOSE events with `time.busy=` field present, one per step name
  </behavior>
  <action>
    Step 1 — At top of `lib.rs` in the existing `use` block (the one with `use console::*`, `use crate::*` etc.), add:

    ```rust
    use tracing::{debug, info_span, warn};
    ```

    Insert alphabetically among existing `use` statements.

    Step 2 — Run `rg -n 'eprintln!|println!' crates/tome/src/lib.rs` and partition every line in the sync function body (currently lines 1464-1860 approximately) into one of:
    - `tracing::warn!` — every `eprintln!("warning: ...")` inside the sync function
    - `tracing::debug!` — every `if verbose { eprintln!("  Found ...") }` payload-carrying verbose line
    - DELETE — every `if verbose { eprintln!("{}", style("Resolving...").dim()); }` step-banner verbose line (span CLOSE prints the equivalent now)
    - KEEP `println!` — every user-facing line (version output, "Sync complete", "Pulled/Pushed from remote", list/status table prints, completions, config, "No skills found", "Library changes detected:")
    - KEEP `eprintln!` — wizard chrome lines around dialoguer prompts (HARD-15 carve-out)

    The 97-site count is mechanical sweep work. Use RESEARCH §Output Channel Split / lib.rs as the heuristic foundation; for any ambiguous site, default to STDOUT (keep) and document the audit in the SUMMARY.

    Step 3 — Insert the top-level `sync` span at the very top of the `sync` function body (after parameter destructuring, before the first piece of work). Pattern (RESEARCH §Code Examples lines 1259-1264):

    ```rust
    pub fn sync(...) -> Result<()> {
        let _sync_span = info_span!("sync", dry_run = dry_run, force = force).entered();
        // ... existing function body ...
    }
    ```

    Verify the function signature parameter names match (`dry_run`, `force` — adjust field names if signature uses different binding names).

    Step 4 — Wrap each of the 5 pipeline steps in a lexically-scoped span block. The pattern for each step:

    ```rust
    // BEFORE:
    let resolved_git_paths = resolve_git_directories(...);
    // ... discover work ...

    // AFTER:
    let (resolved_git_paths, skills) = {
        let _span = info_span!("discover").entered();
        let resolved_git_paths = resolve_git_directories(...);
        // ... discover work ...
        let skills = discover::discover_all(...)?;
        (resolved_git_paths, skills)
    };
    ```

    Use lexical `{}` blocks so RAII matches lexical scope (RESEARCH §Common Pitfalls Pitfall 4). The 5 steps in their current code order are:

    1. `discover` — wraps `resolve_git_directories` + `discover::discover_all` + the warnings-loop migration + the empty-skills early return + the v0.9-shape detection (currently lines 1581-1633)
    2. `reconcile` — wraps the `if let Some(claude_adapter) = build_claude_adapter(...)?` block (currently lines 1538-1571; this runs BEFORE discover in code order but per RESEARCH §Span Shape "Naming note" the span order in the trace reflects execution order — name presence is what matters for OBS-03)
    3. `consolidate` — wraps `library::consolidate(...)` + the lockfile diff/triage block (currently lines 1635-1689)
    4. `distribute` — wraps the per-directory distribute loop (currently lines ~1693-1760)
    5. `cleanup` — wraps `cleanup::cleanup_library(...)` + `cleanup_disabled_from_target(...)` + the stderr bucket render block (currently lines ~1746-1778)

    The lexical-scope challenge: many variables (resolved_git_paths, skills, manifest, etc.) cross step boundaries. Resolution per RESEARCH §Span Shape: use `let x = { let _span = ...; ... ; x }` block expressions to scope the span while letting values escape via the block return value. Alternative: explicit `drop(_span)` at end-of-step within the function body (NOT inside a nested block). Both honour D-SPAN-1/D-SPAN-2. Use whichever reads cleaner per step.

    Step 5 — At the discover-warnings emission site (currently lines 1602-1606):

    BEFORE:
    ```rust
    if !quiet {
        for w in &warnings {
            eprintln!("warning: {}", w);
        }
    }
    ```

    AFTER:
    ```rust
    for w in &warnings {
        warn!("{}", w);
    }
    ```

    Remove the `if !quiet` guard — the global subscriber's EnvFilter handles quiet-vs-warn discipline (Quiet level = `warn` per `LogLevel::directive`; warn-level events still fire when `--quiet` is set, matching the existing behavior of these warnings always printing).

    Wait — verify: at `LogLevel::Quiet → "warn"`, warnings DO fire. The current `if !quiet` guard suppressed them in quiet mode. Migration changes the behavior: in `--quiet` mode, warnings WILL now print (because Quiet level = `warn` and these are `warn!` events). Per D-ENV-3 ("NO lines silently disappear"; "if a line genuinely is noise, demote it to `debug!`"), removing the `if !quiet` guard is the correct migration. If the executor concludes these warnings ARE noise that quiet mode should suppress, demote to `debug!` and document in SUMMARY.

    Step 6 — DO NOT touch the `render_sync_report` function body in this task — that's Task 5's job (OBS-05 reconcile line + SyncReport extension).

    Step 7 — DO NOT delete the inline `reconcile::render_summary` call at line 1557 in this task — that's Task 5's job. The reconcile span wraps the call site as-is for now.

    Step 8 — Run `cargo build -p tome` and resolve any compile errors. Run `cargo test -p tome` and confirm no regression (snapshot diffs are expected for tests that assert on stderr; integration tests that capture stdout-only stay green). Run `cargo clippy --all-targets -- -D warnings` and resolve any new lints (likely `unused_variables` for `_sync_span` in tests where sync isn't called — `let _ = _sync_span;` pattern or the leading underscore should suffice).
  </action>
  <verify>
    <automated>cargo build -p tome 2>&amp;1 | tail -10 &amp;&amp; rg -c 'info_span!\\("(sync|discover|reconcile|consolidate|distribute|cleanup)"' crates/tome/src/lib.rs</automated>
  </verify>
  <acceptance_criteria>
    - `rg "info_span!\\(\"sync\"" crates/tome/src/lib.rs` returns 1 match
    - `rg "info_span!\\(\"discover\"" crates/tome/src/lib.rs` returns 1 match
    - `rg "info_span!\\(\"reconcile\"" crates/tome/src/lib.rs` returns 1 match
    - `rg "info_span!\\(\"consolidate\"" crates/tome/src/lib.rs` returns 1 match
    - `rg "info_span!\\(\"distribute\"" crates/tome/src/lib.rs` returns 1 match
    - `rg "info_span!\\(\"cleanup\"" crates/tome/src/lib.rs` returns 1 match
    - `rg -n 'eprintln!\\("warning:' crates/tome/src/lib.rs` returns ≤ 11 matches (the wizard-chrome carve-outs from RESEARCH lines 215, 216, 656, 672, 674, 793, 808, 810, 894, 968, 998 — sites OUTSIDE the sync function body)
    - `rg "use tracing::\\{debug, info_span, warn\\}" crates/tome/src/lib.rs` returns 1 match (or equivalent multi-line split-import)
    - `cargo build -p tome` exits 0
    - `cargo test -p tome --lib` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - Empirical span check: `cd /Users/martin/dev/opensource/tome && cargo run -p tome -- sync --dry-run --verbose 2>&1 | rg -c "close" | tr -d ' '` returns ≥ 5 (one span CLOSE event per step). If the user's working repo isn't a tome-configured environment, this check may not produce all 5 spans; instead run `cargo run -p tome -- sync --verbose 2>&1 | rg "(discover|reconcile|consolidate|distribute|cleanup)"` and verify at least the discover and consolidate spans fire.
    - Empirical timing check: `cargo run -p tome -- sync --verbose 2>&1 | rg "time\\.busy=" | wc -l` returns ≥ 1 (confirms RESEARCH §elapsed_ms FINDING — auto-emitted timing field name)
  </acceptance_criteria>
  <done>lib.rs::sync wraps the pipeline in a top-level `sync` span + 5 step spans; eprintln warning sites inside sync migrated to tracing::warn; verbose step-banner lines deleted (span CLOSE replaces); payload verbose lines moved to tracing::debug; discover warnings emission migrated to tracing::warn; STDOUT user-facing prints preserved; wizard-chrome eprintln carve-outs preserved.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 5: Extend SyncReport + render_sync_report for OBS-05; delete inline reconcile::render_summary call; relocate per-drift/per-vanished detail; add reconcile::format_classification_detail helper</name>
  <files>crates/tome/src/lib.rs, crates/tome/src/reconcile.rs</files>
  <read_first>
    - crates/tome/src/lib.rs lines 102-107 (SyncReport struct) + lines 1538-1571 (current reconcile call site) + lines 1793-1801 (SyncReport literal construction + render_sync_report call) + lines 2002-2037 (render_sync_report body)
    - crates/tome/src/reconcile.rs lines 683-753 (format_summary + render_summary — the helper-extraction target)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Reconcile Breakdown Line / render_sync_report extension (lines 904-977) — concrete render code
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Common Pitfalls Pitfall 3 (don't migrate render_sync_report to tracing::info!)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md D-ENV-4 — the locked OBS-05 placement decision
  </read_first>
  <behavior>
    - `SyncReport` struct gains `pub reconcile: Option<ReconcileReport>` field (optional because sync only invokes reconcile when `build_claude_adapter(config)?` returns Some)
    - The inline `reconcile::render_summary(&report, quiet)` call at line ~1557 is DELETED
    - The `report` variable from `reconcile::reconcile_lockfile(...)` is renamed to `reconcile_report` (or kept as `report` but moved/cloned into the `SyncReport` literal at line ~1793 via `reconcile: Some(reconcile_report.clone())` or by moving ownership)
    - `render_sync_report` is extended to emit `println!("  reconcile: {} {} match · {} {} drift · {} {} vanished · {} {} missing-from-machine", style("✓").green(), rr.matches, style("⚠").yellow(), rr.drift.len(), style("⚠").yellow(), rr.vanished.len(), style("⚠").yellow(), rr.missing.len())` immediately above the per-bucket cleanup output
    - Per-drift detail lines AND per-vanished warnings relocate from the deleted call site into `render_sync_report` (via a new `reconcile::format_classification_detail(&ReconcileReport) -> String` helper that returns ONLY the detail strings, NO classification line — per RESEARCH §render_sync_report extension lines 940-947)
    - `render_sync_report` remains `println!`-based (STDOUT) — DO NOT migrate to `tracing::info!` per RESEARCH §Pitfall 3
    - The cleanup-buckets render call at line ~1768 either (a) MOVES into render_sync_report (option 2 per RESEARCH §render_sync_report extension, signature widens to accept stderr writer), OR (b) STAYS at line 1768 and is reordered so render_sync_report is called BEFORE it (option 1). Either satisfies the success criterion "immediately above the per-bucket cleanup output." Pick option 1 unless option 2 reads cleaner with current code structure.
  </behavior>
  <action>
    Step 1 — In `crates/tome/src/lib.rs` near the top, modify the `SyncReport` struct (lines 102-107) to add the `reconcile` field:

    ```rust
    pub struct SyncReport {
        pub consolidate: ConsolidateResult,
        pub distributions: Vec<DistributeResult>,
        pub cleanup: CleanupResult,
        pub removed_from_targets: usize,
        /// Phase 18 OBS-05: per-classification reconcile counts surfaced in
        /// the final summary block. `None` when the sync didn't invoke a
        /// reconcile pass (no Claude adapter configured).
        pub reconcile: Option<reconcile::ReconcileReport>,
    }
    ```

    Use the fully-qualified `reconcile::ReconcileReport` (or `use crate::reconcile::ReconcileReport` at top of file).

    Step 2 — In the reconcile section of sync (currently lines 1538-1571), refactor:

    BEFORE:
    ```rust
    if let Some(claude_adapter) = build_claude_adapter(config)? {
        let report = reconcile::reconcile_lockfile(...)?;

        if !quiet {
            reconcile::render_summary(&report, quiet);  // <<< delete
        }

        apply_edit_decisions(&report, paths, dry_run)?;

        if !report.install_failures.is_empty() {
            marketplace::render_install_failures(&report.install_failures);
            reconcile_install_failures = take_install_failures(report);
        }
    }
    ```

    AFTER:
    ```rust
    let mut reconcile_report: Option<reconcile::ReconcileReport> = None;
    if let Some(claude_adapter) = build_claude_adapter(config)? {
        let report = reconcile::reconcile_lockfile(...)?;

        // Note: render_summary call removed per Phase 18 D-ENV-4.
        // The classification line is now emitted from render_sync_report
        // (final summary block, immediately above the cleanup buckets).

        apply_edit_decisions(&report, paths, dry_run)?;

        if !report.install_failures.is_empty() {
            marketplace::render_install_failures(&report.install_failures);
            reconcile_install_failures = take_install_failures(report.clone());
            // ^ if take_install_failures consumes `report`, refactor to clone or
            //   restructure. Verify the existing take_install_failures signature
            //   in lib.rs and adapt without changing its semantics.
        }
        reconcile_report = Some(report);
    }
    ```

    Verify `apply_edit_decisions(&report, ...)` signature accepts `&ReconcileReport`. Verify `take_install_failures(report)` ownership — if it consumes, the refactor must move `take_install_failures` BEFORE the `reconcile_report = Some(report)` assignment, OR clone. The existing semantics (install_failures get drained into `reconcile_install_failures`) must be preserved.

    Step 3 — In the SyncReport literal construction (currently around line 1793-1798), add the `reconcile` field:

    BEFORE:
    ```rust
    let report = SyncReport {
        consolidate: consolidate_result,
        distributions: distribute_results,
        cleanup: cleanup_result,
        removed_from_targets,
    };
    ```

    AFTER:
    ```rust
    let report = SyncReport {
        consolidate: consolidate_result,
        distributions: distribute_results,
        cleanup: cleanup_result,
        removed_from_targets,
        reconcile: reconcile_report,
    };
    ```

    Step 4 — In `crates/tome/src/reconcile.rs`, add a new public helper that returns ONLY the per-drift detail + per-vanished warnings (NO classification line — render_sync_report owns that). Locate the existing `format_summary` function (around lines 683-753) and add ADJACENT to it (do NOT modify `format_summary` itself — existing callers must still work):

    ```rust
    /// Return ONLY the per-drift detail lines and per-vanished warning lines
    /// for the OBS-05 final-summary relocation (Phase 18 D-ENV-4).
    /// The classification line itself ("reconcile: N match · M drift · ...")
    /// is rendered by `lib.rs::render_sync_report` so it sits above the
    /// cleanup buckets; this helper returns the detail block that goes BELOW
    /// the classification line. Returns an empty string if nothing to detail.
    pub fn format_classification_detail(report: &ReconcileReport) -> String {
        let mut s = String::new();
        for d in &report.drift {
            // Mirror the format_summary per-drift line shape — the literal
            // bullet-glyph + "→" arrow + version pair. Refer to format_summary
            // for the exact format string; this helper just lifts those lines
            // without the classification header.
            // ... per-drift formatting per existing format_summary ...
        }
        for v in &report.vanished {
            // ... per-vanished warning per existing format_summary ...
        }
        s
    }
    ```

    The exact format strings come from inspecting `format_summary` and lifting the per-drift / per-vanished branches into the new function. The classification-line branch of `format_summary` is NOT lifted (render_sync_report owns it now).

    Add a unit test for the new helper inside reconcile.rs `mod tests`:

    ```rust
    #[test]
    fn format_classification_detail_omits_header() {
        let mut report = ReconcileReport::default();
        // Populate one drift entry, one vanished entry. (Use existing test
        // fixtures or construct manually.)
        let s = format_classification_detail(&report);
        assert!(!s.contains("match · "), "must NOT include classification header");
        // ... assert detail entries present ...
    }
    ```

    Step 5 — In `crates/tome/src/lib.rs::render_sync_report`, extend the function body. The current function (lines 2002-2037) prints the Library / distributions / cleanup-removed lines. AFTER those existing lines and BEFORE the function's closing brace, add (per RESEARCH §render_sync_report extension lines 922-947):

    ```rust
    // OBS-05 (D-ENV-4): reconcile classification line, IMMEDIATELY ABOVE the
    // per-bucket cleanup summary (which is rendered to stderr at the caller
    // site at line ~1768 BEFORE render_sync_report is called at line ~1801,
    // so the visual ordering in the user's terminal is:
    //   - cleanup buckets (stderr) → printed first (caller-driven)
    //   - "Sync complete" + body (stdout) → printed second (this function)
    //   - reconcile classification line (stdout, this function, just added)
    //
    // For the success-criterion "immediately above the per-bucket cleanup
    // summary" to hold, we need the cleanup-buckets render call to fire
    // AFTER render_sync_report has emitted the classification line. Two
    // options per D-ENV-4 + RESEARCH §render_sync_report extension:
    //   (1) Keep cleanup-buckets at lib.rs:1768; CALL render_sync_report
    //       FIRST (move line 1801 up); rely on stderr line order so the
    //       classification line at stdout appears before stderr cleanup
    //       buckets in interleaved terminal output. CAVEAT: stdout/stderr
    //       interleaving is NOT guaranteed-ordered across pipes; user
    //       running `tome sync` to terminal sees the intended visual order,
    //       but `tome sync > out.txt 2> err.txt` splits them — that's
    //       acceptable per the success-criterion's terminal-user intent.
    //   (2) MOVE cleanup-buckets render INTO render_sync_report, widening
    //       the signature to accept a stderr writer. Cleaner ownership;
    //       larger diff.
    // Implementor picks; document choice in SUMMARY. Option 1 recommended
    // for smaller diff.
    if let Some(rr) = &report.reconcile {
        println!(
            "  reconcile: {} {} match · {} {} drift · {} {} vanished · {} {} missing-from-machine",
            style("✓").green(), rr.matches,
            style("⚠").yellow(), rr.drift.len(),
            style("⚠").yellow(), rr.vanished.len(),
            style("⚠").yellow(), rr.missing.len(),
        );

        // Per-drift detail + per-vanished warnings relocated from the
        // deleted lib.rs:1557 inline render call.
        let detail = reconcile::format_classification_detail(rr);
        if !detail.is_empty() {
            print!("{}", detail);
        }
    }
    ```

    Step 6 — Choose option 1 or option 2 for the cleanup-buckets ordering (above). Option 1: move the `render_sync_report(&report)` call from line ~1801 UP to BEFORE the cleanup-buckets stderr block at line ~1764. Option 2: pass the stderr writer + cleanup buckets into `render_sync_report`. Pick whichever reads cleaner. Document the choice in the SUMMARY.

    Step 7 — Run the full quality gate: `cargo fmt -- --check; cargo clippy --all-targets -- -D warnings; cargo test -p tome`. Expect possible snapshot diffs in `cli_sync__*.snap` files because the v0.10 sync output didn't have the reconcile classification line — those snapshots need re-baselining. Run `cargo insta review` interactively OR accept new snapshots via `INSTA_UPDATE=always cargo test` and inspect diffs. The diffs should show ONE new line (the reconcile classification) in scenarios where reconcile fired; scenarios without reconcile (empty configs, init flows) should be unchanged.
  </action>
  <verify>
    <automated>rg -n 'reconcile::render_summary' crates/tome/src/lib.rs | wc -l | tr -d ' '</automated>
  </verify>
  <acceptance_criteria>
    - `rg "reconcile::render_summary" crates/tome/src/lib.rs | wc -l` returns 0 (inline call deleted)
    - `rg "pub reconcile: Option<reconcile::ReconcileReport>" crates/tome/src/lib.rs` returns 1 match (SyncReport field added)
    - `rg "reconcile: reconcile_report" crates/tome/src/lib.rs` returns 1 match (literal construction wires the field)
    - `rg "reconcile: .* match · .* drift · .* vanished · .* missing-from-machine" crates/tome/src/lib.rs` returns 1 match (the classification line format string)
    - `rg "pub fn format_classification_detail" crates/tome/src/reconcile.rs` returns 1 match
    - `rg "format_classification_detail" crates/tome/src/lib.rs` returns ≥ 1 match (call site in render_sync_report)
    - `cargo test -p tome --lib -- reconcile::tests::format_classification_detail_omits_header` exits 0
    - `cargo test -p tome --test cli_sync` exits 0 (any snapshot diffs accepted/re-baselined — the v0.11 reconcile-line addition is the expected new content)
    - `cargo test -p tome --test cli_status` exits 0 (status snapshots unchanged — sync's reconcile flow doesn't run for status)
    - `cargo run -p tome -- sync --dry-run 2>&1 | rg "reconcile: " | wc -l` returns ≤ 1 (zero if no Claude adapter; one if one fires — the line appears ONLY when `reconcile` is `Some`)
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>SyncReport has reconcile field; inline reconcile::render_summary deleted; render_sync_report emits the OBS-05 classification line above cleanup buckets with relocated detail; format_classification_detail helper added to reconcile.rs; all tests pass (snapshots re-baselined where appropriate).</done>
</task>

<task type="auto" tdd="false">
  <name>Task 6: Create 18-deferred-items.md capturing PreviouslyFailed deferral and any DirectoryNowAllowed false-positive caveats</name>
  <files>.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md</files>
  <read_first>
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Open Questions Q1 (PreviouslyFailed signal — lines 1319-1327) and Q2 (DirectoryNowAllowed signal — lines 1329-1339)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md `<deferred>` section (lines 633-688) — existing deferral pattern
    - .planning/phases/16-cleanup-message-ux-docs/16-deferred-items.md (the precedent for this file's shape)
  </read_first>
  <behavior>
    - File `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` exists
    - File contains one section per deferred item with: name, reason for deferral, what would unblock it, target phase/milestone
    - PreviouslyFailed entry exists (mandatory)
    - DirectoryNowAllowed entry exists if Task 3 surfaced a false-positive case; absent if the inference worked cleanly
  </behavior>
  <action>
    Create `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` with content:

    ```markdown
    # Phase 18 — Deferred Items

    **Phase:** 18-observability-foundation-sync-diagnostics
    **Created:** {YYYY-MM-DD on commit day}
    **Status:** Deferrals captured during Plan 18-02 execution.

    ## OBS-04 — `ChangeCause::PreviouslyFailed` emission deferred

    **Status:** Enum variant + `Display` impl SHIPPED in Plan 18-02 (greppability preserved per OBS-04 vocabulary contract — `cause=previously failed` is reachable if/when an emission site fires). Emission site NOT WIRED in Phase 18.

    **Why deferred:** The current `SkillEntry` schema in `manifest.rs` does not track per-skill failure state. Detecting "previous sync failed for this skill" requires one of:
    1. Adding `last_sync_failed: bool` (or richer enum) field to `SkillEntry`, persisted in `.tome-manifest.json`. Manifest schema bump (backward-compatible via `#[serde(default)]`, but still a schema change).
    2. Persisting the previous `SyncReport` to disk (e.g., `last-sync-report.json`) and diffing on next sync. New file, new failure mode (corrupted file).
    3. Inferring from existing state — e.g., "skill is in lockfile but missing from library." Not strictly equivalent to "previous sync failed for this skill" semantically; produces false positives for skills that were intentionally removed.

    None of these are essential to Phase 18's substrate scope. The OBS-04 success criterion lists four causes; emitting three (`HashChanged`, `NewlyAdded`, `DirectoryNowAllowed`) satisfies the literal text ("the cause that fires IS one of these four"). A strict read demands all four fire eventually; that strict read is honoured by deferring the emission site rather than dropping the variant.

    **What would unblock:** Phase 19 polish, OR a v0.12 dedicated manifest-schema bump phase. The cleanest path is Option 1 (`last_sync_failed: bool` on SkillEntry) — `#[serde(default)]` makes it backward-compatible. The emission site would land in `library.rs::consolidate_managed` / `consolidate_local` at the `Err(_)` arms that today silently propagate up, branching to set the flag and emit the event on the NEXT sync when the flag is observed.

    **Target:** v0.12 or later (no milestone commitment yet).

    {IF Task 3 surfaced a false-positive case for DirectoryNowAllowed, append the following section. OTHERWISE delete this whole subsection:}

    ## OBS-04 — `ChangeCause::DirectoryNowAllowed` inference caveat

    **Status:** Wired in `distribute.rs::distribute_to_directory` per the locally-computable inference recommended in RESEARCH §Open Question 2 ("skill in manifest, target has no symlink → was disabled previously"). However, Plan 18-02 Task 3's mental walkthrough surfaced the following false-positive case(s):

    - **Case:** {describe specific false-positive scenario, e.g. "a directory configured AFTER a skill was already in the manifest"}
    - **Implication:** the emission may fire with `cause=directory now allowed` for cases where the skill was genuinely never distributed to this directory before (not "directory was disabled then re-enabled")

    **Why accepted:** the false-positive rate is bounded (only fires on the genuine new-symlink-create branch, never on existing-symlink-replace) and the user-visible meaning ("skill is being symlinked into this directory for the first time in this directory's history") is close enough to "directory now allowed" that the grep vocabulary stays meaningful.

    **What would unblock a strict implementation:** persisting a per-directory-per-skill "has been distributed before" bit. Either a new manifest schema field (similar to PreviouslyFailed) or inferring from machine.toml history. Same trade-off as PreviouslyFailed: not essential to Phase 18 substrate scope.

    **Target:** v0.12 or later (no milestone commitment yet).
    ```

    Replace `{YYYY-MM-DD on commit day}` with the actual current date. Drop the second section entirely if Task 3 did NOT surface false-positive cases (the implementor concluded the inference is clean).
  </action>
  <verify>
    <automated>test -f .planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md &amp;&amp; rg "PreviouslyFailed" .planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md | head -1</automated>
  </verify>
  <acceptance_criteria>
    - File `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` exists
    - `rg "PreviouslyFailed" .planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md` returns ≥ 1 match
    - File contains at least one "Why deferred" section explaining the manifest-schema-bump constraint
    - File contains at least one "What would unblock" section identifying the path forward
    - File contains at least one "Target" line naming a future phase or milestone
  </acceptance_criteria>
  <done>18-deferred-items.md documents the PreviouslyFailed deferral (and DirectoryNowAllowed caveats if applicable); each deferral has a rationale + unblock path + target milestone.</done>
</task>

</tasks>

<verification>
After all 6 tasks land:

1. **Build + lint + test:**
   ```
   cargo fmt -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test -p tome
   ```
   All three exit 0. Some snapshot tests in `tests/cli_sync.rs` may need re-baselining via `cargo insta review` because the OBS-05 classification line is a new stdout line; status/list/doctor/init snapshots stay byte-identical.

2. **Span emission (OBS-03):**
   ```
   cargo run -p tome -- sync --verbose 2>&1 | rg -E "(discover|reconcile|consolidate|distribute|cleanup)" | rg "close"
   ```
   At least 5 lines, one per step. The literal `time.busy=` field name (NOT `elapsed_ms`) is the auto-emitted timing field per RESEARCH §elapsed_ms FINDING — documented in plan 18-02 SUMMARY.

3. **Cause emission (OBS-04):**
   ```
   cargo run -p tome -- sync --verbose 2>&1 | rg "cause="
   ```
   Returns non-empty output when actual re-emits happen. Verify at least `cause=hash changed` and `cause=newly added` are reachable; `cause=directory now allowed` reachable if the inference applies; `cause=previously failed` NOT emitted (deferred per 18-deferred-items.md).

4. **OBS-05 reconcile breakdown line:**
   ```
   cargo run -p tome -- sync 2>&1 | rg "^  reconcile: .+ match · .+ drift · .+ vanished · .+ missing-from-machine"
   ```
   Returns 1 line when reconcile fires (i.e. when `build_claude_adapter` returns Some). Returns 0 lines on a config with no Claude adapter — that's correct (the field is `Option<ReconcileReport>`).

5. **Wave-1 invariants preserved:** Plan 18-01 substrate is intact (`rg "pub fn install" crates/tome/src/tracing_init.rs` returns 1; `rg "tracing_init::install" crates/tome/src/main.rs` returns 1; `rg "pub fn directive" crates/tome/src/cli.rs` returns 1).

6. **Out-of-scope modules untouched** (success criterion 1 — BYTE-IDENTICAL stdout for status + init):
   ```
   cargo test -p tome --test cli_status
   cargo test -p tome --test cli_init
   ```
   Both exit 0 with no snapshot diffs.

7. **Cleanup-rendering invariants preserved:**
   ```
   cargo test -p tome --lib cleanup::tests::cleanup_module_source_does_not_contain_forbidden_phrase
   ```
   Exits 0 (Phase 16 D-UX01-3 invariant).
</verification>

<success_criteria>
- `crates/tome/src/change_cause.rs` exists with `ChangeCause` enum, `ALL`, sentinel, `const_assert!`, `impl Display` — vocabulary matches OBS-04 verbatim (OBS-04 enum side)
- `library.rs`, `distribute.rs`, `cleanup.rs` migrated per RESEARCH §Output Channel Split (OBS-01 sweep portion)
- `library.rs` consolidate branches emit OBS-04 `tracing::info!` events with HashChanged + NewlyAdded variants
- `distribute.rs` distribute branch emits OBS-04 events with HashChanged + NewlyAdded + DirectoryNowAllowed inference
- `lib.rs::sync` wraps the pipeline in `info_span!("sync")` + 5 step spans (`discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`) (OBS-03)
- `lib.rs::sync` 97-site sweep: warning sites → `tracing::warn!`, payload-carrying verbose lines → `tracing::debug!`, banner verbose lines deleted, STDOUT user-facing prints preserved (OBS-01 main sweep)
- Discover-warnings emission migrated to `tracing::warn!`
- Inline `reconcile::render_summary(&report, quiet)` at line 1557 DELETED; `SyncReport` extended with `reconcile: Option<ReconcileReport>`; `render_sync_report` emits `reconcile: ✓ N match · ⚠ M drift · ⚠ K vanished · ⚠ L missing-from-machine` (OBS-05); per-drift + per-vanished detail relocated via `reconcile::format_classification_detail` helper
- `render_sync_report` stays on STDOUT `println!` (D-OUT-1 / Pitfall 3 preserved)
- `18-deferred-items.md` exists documenting PreviouslyFailed deferral (and DirectoryNowAllowed false-positive caveats if surfaced)
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p tome` all pass (with `cargo insta review` snapshot re-baselining accepted for sync flows that now show the reconcile line)
- `cargo run -- status` and `cargo run -- init --dry-run --no-input` produce stdout byte-identical to v0.10.0 (no snapshot diffs in `cli_status__*.snap` / any init snapshot — those code paths are untouched)
</success_criteria>

<output>
After completion, create `.planning/phases/18-observability-foundation-sync-diagnostics/18-02-SUMMARY.md` summarizing:

- Which modules were swept, with site counts (e.g., "library.rs: 6 warning sites migrated + 8 OBS-04 emission sites added")
- Total `eprintln!`/`println!` deltas (sweep size confirmation)
- Decision on cleanup-buckets render ordering (option 1 vs option 2 per Task 5)
- Decision on flag-flip cases in `library.rs` (emit HashChanged, OR skip emission per Task 2 walkthrough)
- DirectoryNowAllowed inference status (clean OR false-positive caveat captured in 18-deferred-items.md)
- Snapshot files that needed re-baselining (with reason: "OBS-05 classification line added to sync stdout when reconcile fires")
- Confirmation that `time.busy=` appears in `tome sync --verbose 2>&1 | grep` output (the OBS-03 `elapsed_ms` mapping)
- Notes for Plan 18-03 verification (e.g., "verify `tome status` snapshot unchanged via `cargo test -p tome --test cli_status`")
</output>

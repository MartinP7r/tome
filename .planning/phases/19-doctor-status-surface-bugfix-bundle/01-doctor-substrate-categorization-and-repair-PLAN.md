---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/doctor.rs
  - crates/tome/tests/cli_doctor.rs
autonomous: true
requirements: [OBS-06, FIX-01, FIX-03]
requirements_addressed: [OBS-06, FIX-01, FIX-03]

must_haves:
  truths:
    - "tome doctor text output groups issues by category (Library / Directory / Config / Foreign-symlink) with per-category counts in the summary line"
    - "tome doctor --json adds a category string field per DiagnosticIssue (serialized snake_case) and a per-category counts map in the summary object"
    - "tome doctor never prints the 'N auto-fixable issues' line followed by '(no auto-repair available)' — when auto_fixable_count == 0 the global prompt and that line are skipped"
    - "tome doctor on a clean v0.10-shape library produces zero 'tracked in git' warnings (FIX-03 D-FIX03-2)"
    - "Sum of per-category counts equals report.total_issues() — ForeignSymlink issues count ONLY in the ForeignSymlink bucket"
    - "Adding a RepairKind variant without a dispatcher handler arm fails to compile (POLISH-04 enum-exhaustiveness sentinel + exhaustive match in dispatcher)"
  artifacts:
    - path: "crates/tome/src/doctor.rs"
      provides: "IssueCategory enum, RepairKind enum, DiagnosticIssue.category + repair_kind fields, repair dispatcher matching on RepairKind, deleted tracked_managed_symlinks helper + interactive git-tracked block"
      contains: "enum IssueCategory"
    - path: "crates/tome/tests/cli_doctor.rs"
      provides: "Integration test asserting clean v0.10-shape library emits no 'tracked in git' warning (D-FIX03-2)"
      contains: "doctor_clean_v010_library_emits_no_tracked_in_git_warning"
  key_links:
    - from: "crates/tome/src/doctor.rs DiagnosticIssue::* constructors"
      to: "IssueCategory + Option<RepairKind> fields"
      via: "Per-emit-site constructors hardcode category; repair_kind is Some(kind) iff a dispatcher handler exists"
      pattern: "DiagnosticIssue::library_repairable|DiagnosticIssue::directory|DiagnosticIssue::config"
    - from: "crates/tome/src/doctor.rs diagnose() repair dispatcher"
      to: "RepairKind enum"
      via: "exhaustive match on Option<RepairKind>"
      pattern: "match issue\\.repair_kind"
---

<objective>
Bundle the entangled doctor.rs work — OBS-06 categorization, FIX-01 auto-fixable contradiction fix, FIX-03 stale "tracked in git" deletion — into a single plan because they all touch overlapping line ranges in doctor.rs. Substring-matching is replaced with typed RepairKind discrimination (D-REPAIR-3). The stale "managed symlinks tracked in git" check is deleted wholesale (D-FIX03-1) since v0.10 made managed skills real directory copies.

Purpose: Close GitHub #530 (auto-fixable contradiction), #532 (stale tracked-in-git check), and deliver OBS-06 (categorized doctor surface). Single change that eliminates the entire substring-matching anti-pattern in doctor.rs:267-285.
Output: Two new POLISH-04 enums (IssueCategory, RepairKind), updated DiagnosticIssue with category + repair_kind fields, deleted tracked_managed_symlinks helper + interactive git-tracked render block, regression tests for #530/#532, and category-aware summary rendering.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/STATE.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md
@crates/tome/src/doctor.rs

<interfaces>
<!-- Key existing types/constructors that the executor needs from doctor.rs.
     Extracted from RESEARCH.md code anchors. Use these directly — no
     codebase exploration needed. -->

From `crates/tome/src/doctor.rs`:
```rust
// Existing (DO NOT redesign):
pub struct DiagnosticIssue { /* ... */ }
pub enum DiagnosticIssueKind { ForeignSymlink }  // single variant with POLISH-04 sentinel at line 60
pub struct DoctorReport {
    pub library_issues: Vec<DiagnosticIssue>,
    pub directory_issues: Vec<DirectoryDiagnostic>,  // each has .issues: Vec<DiagnosticIssue>
    pub config_issues: Vec<DiagnosticIssue>,
    pub unowned_skills: Vec<SkillSummary>,  // per UNOWN-03; informational
}
impl DoctorReport {
    pub fn total_issues(&self) -> usize  // around doctor.rs:140
}

// Existing emit sites (8 total — research-confirmed):
DiagnosticIssue::untyped(severity, message)  // most call sites
DiagnosticIssue::typed(severity, message, kind)  // ForeignSymlink emit site only

// Existing repair handlers (the inventory for RepairKind):
//   - "manifest entry 'X' has no directory on disk" / "managed skill 'X' has a broken symlink"
//     at check_library emit sites around doctor.rs:599-626  → RepairKind::RemoveStaleManifestEntry
//     (covers both because the action is identical: m.remove(name) + remove_file if symlink)
//   - "broken legacy symlink: X -> Y" at doctor.rs:647-661 → RepairKind::RemoveBrokenLibrarySymlink
//   - "stale symlink X" in check_distribution_dir → RepairKind::RemoveStaleTargetSymlink
//     (currently substring-matched at doctor.rs:297-307 via cleanup::cleanup_target)

// DELETED by FIX-03:
//   - doctor.rs:665-682 (check_library "tracked in git" emit block)
//   - doctor.rs:383-448 (diagnose() interactive has_git_tracked block, including
//     substring detection at :387 + render+Confirm at :392-447)
//   - doctor.rs:687-730 (tracked_managed_symlinks helper function)
```

POLISH-04 template (clone at doctor.rs:60 — `_diagnostic_issue_kind_exhaustiveness_sentinel`):
```rust
impl E {
    pub const ALL: [Self; N] = [ /* all variants */ ];
}
#[allow(dead_code)]
const fn _e_exhaustiveness_sentinel(x: E) {
    match x {
        E::Variant1 => {}
        E::Variant2 => {}
        // ... every variant
    }
}
const _: () = { assert!(E::ALL.len() == N); };
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add IssueCategory + RepairKind enums with POLISH-04 sentinels</name>
  <files>crates/tome/src/doctor.rs</files>
  <read_first>
    - crates/tome/src/doctor.rs (full file — need exact line context for the sentinel template at line 60, and the existing `DiagnosticIssueKind` enum 39-68)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (D-CAT-1/2/3, D-REPAIR-1/2/3 locked decisions)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md sections "RepairKind Variant Inventory" + "IssueCategory Decisions" (lines 106-307)
  </read_first>
  <behavior>
    - Test 1: `IssueCategory::ALL` has length 4; all 4 variants present (Library, Directory, Config, ForeignSymlink)
    - Test 2: `IssueCategory` serializes to snake_case ("library", "directory", "config", "foreign_symlink") via serde_json::to_string
    - Test 3: `RepairKind::ALL` has length 3; all 3 variants present (RemoveStaleManifestEntry, RemoveBrokenLibrarySymlink, RemoveStaleTargetSymlink)
    - Test 4: `RepairKind` serializes to snake_case ("remove_stale_manifest_entry", "remove_broken_library_symlink", "remove_stale_target_symlink")
    - Test 5: Adding a new variant to either enum without updating `ALL` is a compile-time failure (validated by the `const _: () = { assert!(E::ALL.len() == N); }` sentinel)
  </behavior>
  <action>
    Add two new enums to `crates/tome/src/doctor.rs`, placed immediately after the existing `DiagnosticIssueKind` block (currently ending around line 68). Use the POLISH-04 pattern verbatim from `_diagnostic_issue_kind_exhaustiveness_sentinel` at line 60.

    1. **IssueCategory enum** — pub, Debug+Clone+Copy+PartialEq+Eq+serde::Serialize, `#[serde(rename_all = "snake_case")]`:
       ```rust
       /// Category of a `DiagnosticIssue`. Derived at construction from the
       /// DoctorReport field the issue lives in, with `ForeignSymlink`
       /// promoted regardless of source field (D-CAT-1).
       #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
       #[serde(rename_all = "snake_case")]
       pub enum IssueCategory {
           Library,
           Directory,
           Config,
           ForeignSymlink,
       }

       impl IssueCategory {
           pub const ALL: [Self; 4] = [
               Self::Library,
               Self::Directory,
               Self::Config,
               Self::ForeignSymlink,
           ];
       }

       #[allow(dead_code)]
       const fn _issue_category_exhaustiveness_sentinel(c: IssueCategory) {
           match c {
               IssueCategory::Library => {}
               IssueCategory::Directory => {}
               IssueCategory::Config => {}
               IssueCategory::ForeignSymlink => {}
           }
       }
       const _: () = { assert!(IssueCategory::ALL.len() == 4); };
       ```

    2. **RepairKind enum** — same derive set, same serde attr:
       ```rust
       /// Categorizes the auto-repair available for a `DiagnosticIssue`.
       ///
       /// `Some(kind)` on `DiagnosticIssue::repair_kind` ↔ the issue is
       /// auto-fixable. `None` ↔ interactive-only (orphan dirs) or
       /// informational. The repair dispatcher in `diagnose()` matches
       /// exhaustively on `Option<RepairKind>` — adding a variant without
       /// a handler arm fails to compile.
       #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
       #[serde(rename_all = "snake_case")]
       pub enum RepairKind {
           /// Remove a manifest entry whose library directory is missing on
           /// disk OR whose managed symlink is broken. Emit sites in
           /// `check_library` (the two related cases share an action:
           /// `Manifest::remove(name)` + remove_file if symlink branch).
           RemoveStaleManifestEntry,
           /// Remove a broken legacy symlink in the library directory
           /// (not referenced by manifest). Emit site: `check_library`
           /// "broken legacy symlink: X -> Y".
           RemoveBrokenLibrarySymlink,
           /// Remove a stale symlink from a distribution directory. Emit
           /// site: `check_distribution_dir`. Action: `cleanup::cleanup_target`.
           RemoveStaleTargetSymlink,
       }

       impl RepairKind {
           pub const ALL: [Self; 3] = [
               Self::RemoveStaleManifestEntry,
               Self::RemoveBrokenLibrarySymlink,
               Self::RemoveStaleTargetSymlink,
           ];
       }

       #[allow(dead_code)]
       const fn _repair_kind_exhaustiveness_sentinel(k: RepairKind) {
           match k {
               RepairKind::RemoveStaleManifestEntry => {}
               RepairKind::RemoveBrokenLibrarySymlink => {}
               RepairKind::RemoveStaleTargetSymlink => {}
           }
       }
       const _: () = { assert!(RepairKind::ALL.len() == 3); };
       ```

    3. Add tests in the existing `#[cfg(test)] mod tests` block at the bottom of doctor.rs:
       - `issue_category_all_len_4`
       - `issue_category_serializes_snake_case` — assert `serde_json::to_string(&IssueCategory::ForeignSymlink).unwrap() == "\"foreign_symlink\""`
       - `repair_kind_all_len_3`
       - `repair_kind_serializes_snake_case` — assert all three variants serialize correctly

    Do NOT modify `DiagnosticIssue` struct yet — that's Task 2.
  </action>
  <verify>
    <automated>cargo test -p tome --lib doctor::tests::issue_category doctor::tests::repair_kind</automated>
  </verify>
  <acceptance_criteria>
    - `rg "^pub enum IssueCategory" crates/tome/src/doctor.rs` returns 1 match
    - `rg "^pub enum RepairKind" crates/tome/src/doctor.rs` returns 1 match
    - `rg "IssueCategory::ALL: \[Self; 4\]" crates/tome/src/doctor.rs` returns 1 match
    - `rg "RepairKind::ALL: \[Self; 3\]" crates/tome/src/doctor.rs` returns 1 match
    - `rg "_issue_category_exhaustiveness_sentinel" crates/tome/src/doctor.rs` returns 1 match
    - `rg "_repair_kind_exhaustiveness_sentinel" crates/tome/src/doctor.rs` returns 1 match
    - `cargo test -p tome --lib doctor::tests::issue_category doctor::tests::repair_kind` exits 0 with 4 tests passing
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>Both enums exist with POLISH-04 sentinels; four unit tests pass; clippy clean.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Wire category + repair_kind onto DiagnosticIssue; update 8 emit sites; rewrite dispatcher; add D-CAT-3 summary breakdown</name>
  <files>crates/tome/src/doctor.rs</files>
  <read_first>
    - crates/tome/src/doctor.rs (full file — emit sites at :297-307, :383-448, :459-497, :599-626, :647-661, :822-882; dispatcher at :267-285)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (D-CAT-1/2/3, D-REPAIR-1/2/3)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "Dispatcher shape" + "Note on issue-emit-site changes" (lines 180-223)
  </read_first>
  <behavior>
    - Test 1 (D-CAT-2 invariant): A synthetic DoctorReport with one issue per category (Library + Directory + Config + ForeignSymlink-promoted-from-directory) — sum of per-category counts equals report.total_issues(); the ForeignSymlink-kind issue counts ONLY in the ForeignSymlink bucket
    - Test 2 (FIX-01 D-REPAIR-2 — zero-prompt skip): When `auto_fixable_count(report) == 0` (a report with one non-repairable issue), the dispatcher path that would print "Apply N auto-fixable repairs?" is not entered
    - Test 3 (D-CAT-3 breakdown rendering): When auto_fixable_count > 0 spanning multiple categories, the summary line contains "(N auto-fixable: Library M, Foreign-symlink K)" with only non-zero categories listed
    - Test 4 (substring matching gone): grep of doctor.rs confirms no `i.message.contains("orphan directory")`, `i.message.contains("tracked in git")`, `i.message.contains("stale symlink")`, or `i.message.contains("no directory on disk")` checks remain in the dispatcher path (substring matches in the `kind == None` interactive Select for orphan dirs MAY remain — orphan dirs are interactive-only per inventory)
  </behavior>
  <action>
    Modify `DiagnosticIssue` to carry the two new fields. Refactor emit sites to use category-specific constructors. Rewrite the dispatcher to match on `Option<RepairKind>` instead of substring. Add the D-CAT-3 summary breakdown.

    1. **Extend `DiagnosticIssue` struct** (existing definition around doctor.rs:75):
       ```rust
       pub struct DiagnosticIssue {
           pub severity: IssueSeverity,
           pub message: String,
           pub kind: Option<DiagnosticIssueKind>,
           pub category: IssueCategory,           // NEW (D-CAT-1)
           pub repair_kind: Option<RepairKind>,   // NEW (D-REPAIR-1)
       }
       ```
       Update the JSON `Serialize` impl (or `#[derive(Serialize)]` attrs) so `category` always emits and `repair_kind` uses `#[serde(skip_serializing_if = "Option::is_none")]`.

    2. **Add per-category constructors** on `impl DiagnosticIssue` (recommended option 1 from RESEARCH.md "Note on issue-emit-site changes"):
       ```rust
       impl DiagnosticIssue {
           pub fn library(severity: IssueSeverity, message: impl Into<String>) -> Self { /* category: Library, kind: None, repair_kind: None */ }
           pub fn library_repairable(severity: IssueSeverity, message: impl Into<String>, repair_kind: RepairKind) -> Self { /* category: Library, repair_kind: Some(...) */ }
           pub fn directory(severity: IssueSeverity, message: impl Into<String>) -> Self { /* category: Directory */ }
           pub fn directory_repairable(severity: IssueSeverity, message: impl Into<String>, repair_kind: RepairKind) -> Self { /* category: Directory */ }
           pub fn directory_foreign_symlink(severity: IssueSeverity, message: impl Into<String>) -> Self { /* category: ForeignSymlink (promoted), kind: Some(ForeignSymlink) */ }
           pub fn config(severity: IssueSeverity, message: impl Into<String>) -> Self { /* category: Config */ }
       }
       ```
       Keep `DiagnosticIssue::untyped` and `DiagnosticIssue::typed` as deprecated shims that delegate (or delete them entirely if zero remaining call sites — researcher inventory says ~8 emit sites all migrate).

    3. **Retrofit 8 emit sites** (per RESEARCH.md inventory):
       - `check_library` lines :599-605 (broken managed symlink): `DiagnosticIssue::library_repairable(IssueSeverity::Error, "managed skill '{name}' has a broken symlink (source may have been uninstalled)", RepairKind::RemoveStaleManifestEntry)`
       - `check_library` lines :614-619 / :621-626 (stale manifest entry): `DiagnosticIssue::library_repairable(IssueSeverity::Error, "manifest entry '{name}' has no directory on disk", RepairKind::RemoveStaleManifestEntry)`
       - `check_library` lines :647-661 (broken legacy symlink): `DiagnosticIssue::library_repairable(IssueSeverity::Error, "broken legacy symlink: {path} -> {target}", RepairKind::RemoveBrokenLibrarySymlink)`
       - `check_distribution_dir` (stale symlink emit, currently substring "stale symlink"): `DiagnosticIssue::directory_repairable(IssueSeverity::Error, "stale symlink {path}", RepairKind::RemoveStaleTargetSymlink)`
       - `check_distribution_dir` (orphan directory emit): `DiagnosticIssue::directory(IssueSeverity::Warning, "orphan directory: {path}")` — `repair_kind: None` (interactive-only per inventory)
       - `check_distribution_dir` (foreign symlink emit, `kind: Some(ForeignSymlink)`): `DiagnosticIssue::directory_foreign_symlink(...)` — promotes to category: ForeignSymlink
       - `check_config` (validation emits): `DiagnosticIssue::config(IssueSeverity::Error, ...)` — category: Config

    4. **Rewrite the dispatcher** (current substring math at :267-285):
       Replace the `i.message.contains("orphan directory") || i.message.contains("tracked in git")` substring chain with:
       ```rust
       fn auto_fixable_count(report: &DoctorReport) -> usize {
           report.all_issues().filter(|i| i.repair_kind.is_some()).count()
       }
       ```
       Add a `fn all_issues(&self) -> impl Iterator<Item=&DiagnosticIssue>` method on `DoctorReport` that flattens `library_issues + directory_issues[*].issues + config_issues`.

       In the `diagnose()` repair flow (currently around :283-309), gate the global prompt on `auto_fixable_count(&report) > 0` (D-REPAIR-2):
       ```rust
       let fixable = auto_fixable_count(&report);
       if fixable > 0 {
           // ... existing "Apply N auto-fixable repairs? [Y/n]" prompt
           if user_confirmed {
               for issue in report.all_issues() {
                   match issue.repair_kind {
                       Some(RepairKind::RemoveStaleManifestEntry) => repair_stale_manifest_entry(/* args */)?,
                       Some(RepairKind::RemoveBrokenLibrarySymlink) => repair_broken_library_symlink(/* args */)?,
                       Some(RepairKind::RemoveStaleTargetSymlink) => repair_stale_target_symlink(/* args */)?,
                       None => continue,
                   }
               }
           }
       }
       // The "(no auto-repair available)" follow-up line MUST be deleted — the prompt was already skipped at zero.
       ```

       The orphan-directory interactive Select path (around :312-381) stays as-is — it's keyed off `repair_kind: None` + `category: Directory` (or a category check). It can keep using a non-substring discriminator like `category == Directory && message starts with "orphan directory"` if needed, OR (preferred) add a new sentinel field — but to keep diff size bounded, keeping the message-prefix check inside the orphan flow ONLY is acceptable. The D-REPAIR-3 contract removes substring-matching from the DISPATCHER level; per-handler internal checks for orphan dirs are not the bug class #530 was about.

    5. **D-CAT-3 summary breakdown** — In the summary line renderer (currently the `format!("{N} issue(s) total ({M} auto-fixable)")` builder around :250-260), compute per-category breakdown:
       ```rust
       let by_category = IssueCategory::ALL.iter()
           .filter_map(|c| {
               let n = report.all_issues().filter(|i| i.repair_kind.is_some() && i.category == *c).count();
               if n > 0 { Some(format!("{} {}", display_name(*c), n)) } else { None }
           })
           .collect::<Vec<_>>()
           .join(", ");
       // Example: "(3 auto-fixable: Library 2, Foreign-symlink 1)"
       ```
       Display names: `Library`, `Directory`, `Config`, `Foreign-symlink` (note the hyphen in the human-readable form; serialization is still `foreign_symlink`).

    6. **Add `summary` JSON object** in DoctorReport's serialization OR a top-level summary helper that exposes per-category counts and `auto_fixable_count`. Per OQ-1 in RESEARCH (recommended): include `auto_fixable_by_category` map in the JSON `summary` object alongside `by_category` and `auto_fixable_count`. Concretely:
       ```json
       "summary": {
         "total_issues": 5,
         "by_category": { "library": 2, "directory": 1, "config": 1, "foreign_symlink": 1 },
         "auto_fixable_count": 3,
         "auto_fixable_by_category": { "library": 2, "directory": 1 }
       }
       ```

    7. **Add D-CAT-2 invariant test** in `#[cfg(test)] mod tests`:
       ```rust
       #[test]
       fn category_counts_sum_to_total_issues() {
           // Build a DoctorReport with one library issue, one directory issue,
           // one config issue, one ForeignSymlink-kind issue under directory_issues.
           // Assert sum-of-per-category-counts == total_issues() == 4.
           // Assert the ForeignSymlink issue counts ONLY in the ForeignSymlink bucket.
       }
       ```

    8. **Add D-REPAIR-2 prompt-skip test** — a report with one non-repairable issue (e.g. orphan dir) MUST NOT enter the global-prompt code path. Easiest verification: call `auto_fixable_count(&report)` and assert `== 0`.

    9. **Emit `tracing::debug!(target: "doctor::repair", ?kind, ?reason, "skipped repair")`** when the dispatcher skips a repair (user declined or no handler). This is per RESEARCH OQ-2 + CONTEXT.md `<specifics>`. Aligns with Phase 18 OBS-01 in-scope contract — diagnostic output routes through `tracing`, NOT `eprintln!`.
  </action>
  <verify>
    <automated>cargo test -p tome --lib doctor:: && cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `rg "pub category: IssueCategory" crates/tome/src/doctor.rs` returns 1 match
    - `rg "pub repair_kind: Option<RepairKind>" crates/tome/src/doctor.rs` returns 1 match
    - `rg "fn library_repairable" crates/tome/src/doctor.rs` returns 1 match
    - `rg "fn directory_foreign_symlink" crates/tome/src/doctor.rs` returns 1 match
    - `rg "fn auto_fixable_count" crates/tome/src/doctor.rs` returns 1 match
    - `rg "match issue\.repair_kind" crates/tome/src/doctor.rs` returns 1 match
    - `rg "i\.message\.contains" crates/tome/src/doctor.rs` returns 0 matches inside `diagnose()` (allow at most 1 match inside the orphan-dir handler, no others)
    - `rg "no auto-repair available" crates/tome/src/doctor.rs` returns 0 matches (the literal contradiction string from #530 is deleted)
    - `rg "category_counts_sum_to_total_issues" crates/tome/src/doctor.rs` returns 1 match (the D-CAT-2 invariant test)
    - `rg "auto_fixable_by_category" crates/tome/src/doctor.rs` returns at least 1 match (JSON shape)
    - `cargo test -p tome --lib doctor::` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>DiagnosticIssue carries category + repair_kind; 8 emit sites use new constructors; dispatcher matches on Option<RepairKind>; D-CAT-2 invariant test passes; D-CAT-3 summary breakdown renders; #530 contradiction line is gone.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Delete the stale "tracked in git" doctor check + helper + interactive block (FIX-03); add D-FIX03-2 regression test</name>
  <files>crates/tome/src/doctor.rs, crates/tome/tests/cli_doctor.rs</files>
  <read_first>
    - crates/tome/src/doctor.rs lines 383-448 (interactive has_git_tracked block), 665-682 (check_library emit), 687-730 (tracked_managed_symlinks helper)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (D-FIX03-1, D-FIX03-2)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "FIX-03 (stale 'tracked in git' check — closes #532)" section (lines 531-563)
    - crates/tome/tests/cli.rs (for the existing CLI test harness patterns — `Command::cargo_bin`, `TempDir`)
  </read_first>
  <behavior>
    - Test (D-FIX03-2): A fresh v0.10-shape library (real directory copy, not a symlink) seeded under TempDir + a managed-true manifest entry produces zero occurrences of "tracked in git" in `tome doctor` stdout+stderr combined output.
  </behavior>
  <action>
    1. **Delete three blocks in `crates/tome/src/doctor.rs`** (line numbers from RESEARCH.md; executor must re-locate them after Task 2 modifications shift line numbers — anchor by content, not line number):
       - The `check_library` emit block that produces the "N managed symlink(s) tracked in git" issue (search anchor: `tracked in git` substring or `tracked_managed_symlinks` call site, was at :665-682 pre-Task-2)
       - The `tracked_managed_symlinks` helper function (search anchor: `fn tracked_managed_symlinks`, was at :687-730 pre-Task-2)
       - The interactive `has_git_tracked` block inside `diagnose()` (search anchor: `i.message.contains("tracked in git")` or the surrounding Confirm prompt at `:392-447`, was at :383-448 pre-Task-2). Note: after Task 2's substring-removal pass, the substring detection should already be gone — but the interactive render+Confirm block downstream of it may still exist. Delete the entire render+Confirm block.

    2. **Verify no orphan imports**: after deletion, check that `std::process::Command` is still used elsewhere in `doctor.rs`. If the deleted helper was the ONLY user, remove the `use` line. Run `cargo build` after deletion to surface any orphan-import warnings (clippy will catch them with `-D warnings`).

    3. **Add D-FIX03-2 regression test in `crates/tome/tests/cli_doctor.rs`** (CREATE this file if it does not exist — researcher noted no current `cli_doctor.rs` integration test file; verify with `fd cli_doctor crates/tome/tests`):
       ```rust
       // crates/tome/tests/cli_doctor.rs
       use assert_cmd::Command;
       use std::fs;
       use tempfile::TempDir;

       #[test]
       fn doctor_clean_v010_library_emits_no_tracked_in_git_warning() {
           let tmp = TempDir::new().unwrap();
           let tome_home = tmp.path();
           let lib = tome_home.join("skills");
           fs::create_dir_all(lib.join("my-managed-skill")).unwrap();
           // Seed a real SKILL.md inside the skill dir so consolidate recognizes it
           fs::write(
               lib.join("my-managed-skill/SKILL.md"),
               "---\nname: my-managed-skill\ndescription: test\n---\n# Test\n",
           ).unwrap();
           // Seed a minimal .tome-manifest.json with a managed entry
           let manifest = serde_json::json!({
               "skills": {
                   "my-managed-skill": {
                       "source_name": "test-marketplace",
                       "source_path": "/dev/null",
                       "content_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                       "synced_at": "2026-05-13T00:00:00Z",
                       "managed": true
                   }
               }
           });
           fs::write(
               tome_home.join(".tome-manifest.json"),
               serde_json::to_string_pretty(&manifest).unwrap(),
           ).unwrap();
           // Minimal config (empty directories table is fine)
           fs::write(tome_home.join("tome.toml"), "[directories]\n").unwrap();

           let output = Command::cargo_bin("tome").unwrap()
               .env("TOME_HOME", tome_home)
               .arg("doctor")
               .output()
               .unwrap();
           let combined = format!(
               "{}{}",
               String::from_utf8_lossy(&output.stderr),
               String::from_utf8_lossy(&output.stdout)
           );
           assert!(
               !combined.contains("tracked in git"),
               "v0.10-shape library must not emit stale 'tracked in git' warning. Output:\n{combined}"
           );
       }
       ```

       IMPORTANT: The manifest JSON shape MUST match the current `Manifest` struct serialization. If the executor finds the test fails due to manifest schema mismatch, read `crates/tome/src/manifest.rs` to align the seeded JSON shape with the actual `SkillEntry` fields. The test's purpose is the `!contains("tracked in git")` assertion — any seed-file plumbing required to get there is acceptable.
  </action>
  <verify>
    <automated>cargo test -p tome --test cli_doctor doctor_clean_v010_library_emits_no_tracked_in_git_warning && rg "tracked in git" crates/tome/src/doctor.rs</automated>
  </verify>
  <acceptance_criteria>
    - `rg "tracked in git" crates/tome/src/doctor.rs` returns 0 matches
    - `rg "tracked_managed_symlinks" crates/tome/src/doctor.rs` returns 0 matches (the helper is deleted)
    - `rg "managed symlink" crates/tome/src/doctor.rs` returns 0 matches (the entire concept is gone post-v0.10)
    - `crates/tome/tests/cli_doctor.rs` exists
    - `rg "doctor_clean_v010_library_emits_no_tracked_in_git_warning" crates/tome/tests/cli_doctor.rs` returns 1 match
    - `cargo test -p tome --test cli_doctor` exits 0
    - `cargo build -p tome` exits 0 (no orphan-import warnings)
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>Three code blocks deleted from doctor.rs; D-FIX03-2 integration test passes; clippy clean.</done>
</task>

</tasks>

<verification>
- `cargo test -p tome --lib doctor::` — all unit tests pass
- `cargo test -p tome --test cli_doctor` — integration test passes
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt -- --check` — clean
- `rg "no auto-repair available" crates/tome/src/doctor.rs` — 0 matches (#530 fix verified)
- `rg "tracked in git" crates/tome/src/doctor.rs` — 0 matches (FIX-03 verified)
- `rg "i\.message\.contains" crates/tome/src/doctor.rs` — at most 1 match (orphan-dir internal check is acceptable per D-REPAIR-3 scope)
- Manual smoke: `cargo run -p tome -- doctor --json | jq '.library_issues[0].category'` returns a snake_case string when issues exist
</verification>

<success_criteria>
- OBS-06: `tome doctor` text output includes per-category counts; JSON has `category` field per issue + `auto_fixable_by_category` map in summary
- FIX-01: `auto_fixable_count == 0` skips the global prompt entirely; the "(no auto-repair available)" contradiction line is deleted
- FIX-03: `tome doctor` on a clean v0.10-shape library produces zero "tracked in git" warnings; helper + interactive block + emit site all deleted
- Sum-of-per-category-counts invariant test (D-CAT-2) passes
- RepairKind dispatcher fails to compile if a new variant is added without a handler arm (POLISH-04 sentinel + exhaustive match)
</success_criteria>

<output>
After completion, create `.planning/phases/19-doctor-status-surface-bugfix-bundle/19-01-SUMMARY.md` documenting:
- Final RepairKind variants shipped (should be the 3 in the inventory)
- Number of emit sites retrofitted (target: 8)
- Any deviation from RESEARCH.md's recommended approach (e.g. if the executor found a 4th repair handler)
- LOC deleted vs added (FIX-03 should net negative)
- Test count delta in `doctor::` module
</output>

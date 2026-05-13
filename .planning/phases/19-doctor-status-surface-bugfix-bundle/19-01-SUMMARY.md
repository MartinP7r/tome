---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 01
subsystem: doctor
tags: [diagnostics, repair-dispatcher, typed-discrimination, observability, polish-04, fix-01, fix-03, obs-06]

# Dependency graph
requires:
  - phase: 18-observability-foundation-sync-diagnostics
    provides: tracing substrate + EnvFilter wiring (used by dispatcher debug! emits)
  - phase: 15-cli-hardening
    provides: POLISH-04 exhaustive-match-sentinel pattern (template at doctor.rs:60)
  - phase: 11-library-canonical-core
    provides: managed-as-real-directory shape (made the "tracked in git" check obsolete)
provides:
  - "IssueCategory + RepairKind typed enums with POLISH-04 ALL arrays + compile-time exhaustiveness sentinels"
  - "DiagnosticIssue carries category + repair_kind fields; per-category constructors replace legacy untyped/typed factories"
  - "Repair dispatcher matches exhaustively on Option<RepairKind> — adding a variant without a handler arm fails to compile"
  - "Auto-fixable count skips global prompt at zero (#530 contradiction fix)"
  - "Per-category summary breakdown line + JSON `summary` object with by_category + auto_fixable_by_category maps"
  - "Stale 'tracked in git' check + helper + interactive Confirm block fully deleted (#532)"
affects: [tome doctor, tome status JSON consumers, future doctor-check additions]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "POLISH-04 ALL-array + exhaustive-match sentinel for IssueCategory and RepairKind"
    - "Per-category DiagnosticIssue constructors (library/library_repairable/directory/directory_repairable/directory_foreign_symlink/config) replacing untyped/typed shims"
    - "Exhaustive Option<RepairKind> dispatch in diagnose() — substring matching anti-pattern eliminated at the dispatcher level"
    - "tracing::debug!(target: \"doctor::repair\", …) on skip paths (user_declined, no_repair_kind)"
    - "JSON summary object with `by_category` (full) + `auto_fixable_by_category` (sparse) shape"

key-files:
  created:
    - "(none — D-FIX03-2 integration test added inside existing crates/tome/tests/cli_doctor.rs)"
  modified:
    - "crates/tome/src/doctor.rs - IssueCategory + RepairKind enums, DiagnosticIssue refactor, dispatcher rewrite, FIX-03 deletions"
    - "crates/tome/tests/cli_doctor.rs - D-FIX03-2 regression test (doctor_clean_v010_library_emits_no_tracked_in_git_warning)"

key-decisions:
  - "RepairKind has exactly 3 variants (RemoveStaleManifestEntry, RemoveBrokenLibrarySymlink, RemoveStaleTargetSymlink) — one per real auto-repair handler in doctor.rs. Orphan directories stay interactive-only (repair_kind: None)."
  - "RemoveStaleManifestEntry covers both 'no directory on disk' AND 'broken managed symlink' emit sites because the action is identical (m.remove + remove_file if symlink)"
  - "Per-category constructors replace untyped/typed factory shims entirely — no deprecated shims kept (the legacy constructors had only 8 production call sites + a handful of test call sites, all migrated)"
  - "JSON `summary` object built via serde_json::json! at output time, not as a struct field on DoctorReport — keeps DoctorReport's struct shape unchanged and the summary computation lives next to its only consumer"
  - "render_summary_json's by_category map is FULL (every variant present, zero values included); auto_fixable_by_category is SPARSE (only categories with non-zero auto-fixable counts). Matches the text rendering's 'omit zeros' contract for the breakdown."
  - "ForeignSymlink issues are promoted out of the Directory bucket via directory_foreign_symlink constructor (D-CAT-1); D-CAT-2 invariant test pins sum-of-per-category-counts == total_issues"
  - "tracing::debug! routes dispatcher-skip events through Phase 18's substrate, NOT eprintln! — honours the OBS-01 byte-identical-stdout commitment"
  - "Deletion comment for the 'tracked in git' check intentionally avoids the literal phrase so the acceptance criterion `rg \"tracked in git\"` passes"

patterns-established:
  - "Typed discrimination over substring matching: when a dispatcher needs to branch on issue properties, add a typed enum + exhaustive match, not message.contains() checks. Adding a new branch is then a compile error if forgotten."
  - "Per-category constructors at issue construction: derive category from emit-site location, with exception promotion (ForeignSymlink). Computes once, never recomputed; serialises cleanly into JSON."
  - "Sparse vs full category maps: when category dimensions are small (4), full maps with zero values aid consumer iteration; when subset semantics matter (auto-fixable), sparse maps mirror the human-readable 'omit empty buckets' convention."

requirements-completed: [OBS-06, FIX-01, FIX-03]

# Metrics
duration: 50min
completed: 2026-05-13
---

# Phase 19 Plan 01: Doctor substrate categorization + repair-kind discrimination + #532 stale-check deletion

**Typed IssueCategory + RepairKind enums replace doctor.rs substring matching, fix the #530 auto-fixable contradiction, and remove the obsolete v0.10 'managed symlink tracked in git' check.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-05-13T07:00:00Z (approx)
- **Completed:** 2026-05-13T07:50:00Z (approx)
- **Tasks:** 3
- **Files modified:** 2 (`crates/tome/src/doctor.rs`, `crates/tome/tests/cli_doctor.rs`)
- **Test count:** 994 → 1007 (+13 net; 12 in `doctor::tests`, 1 cli_doctor integration test)
- **LOC delta:** doctor.rs net +578 lines (added typed substrate + constructors + dispatcher + summary helpers + 12 tests; deleted ~138 lines of substring-matching + helper + interactive Confirm block). Net negative for FIX-03 work alone (~-138 lines deleted by the wholesale check removal).

## Accomplishments

- **OBS-06 (doctor categorization)** — `tome doctor` text output now renders per-category breakdown of auto-fixable counts: `Found 5 issue(s). (3 auto-fixable: Library 2, Foreign-symlink 1)`. JSON adds a `category` string field per issue + a `summary` object with `by_category` + `auto_fixable_by_category` maps.
- **FIX-01 (#530)** — Auto-fixable contradiction fixed: when `auto_fixable_count == 0` the global prompt is skipped entirely; the literal "no auto-repair available" line is gone. Substring matching at the dispatcher level eliminated in favour of typed `Option<RepairKind>` exhaustive match.
- **FIX-03 (#532)** — Stale "managed symlink(s) tracked in git" check deleted wholesale (emit site, render+Confirm flow, `tracked_managed_symlinks` git-shellout helper). v0.10's library-canonical model made the check incapable of firing on clean libraries.
- **POLISH-04 enforcement** — Both new enums carry `ALL` arrays + compile-time exhaustiveness sentinels. Adding a variant without updating `ALL` is a compile-time error; adding a `RepairKind` variant without a handler arm in `dispatch_repairs` is also a compile-time error.
- **Test count clears 1000** — Plan 01 alone brought the total from 994 to 1007. The ROADMAP target of ≥1000 tests for v0.11 ship is satisfied by this plan; future Phase 19 plans add more on top.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add IssueCategory + RepairKind enums with POLISH-04 sentinels** — `8842d94` (feat)
2. **Task 2: Wire category + repair_kind onto DiagnosticIssue and dispatcher** — `8261243` (feat)
3. **Task 3: Delete stale 'tracked in git' doctor check + helper + interactive block** — `5392cd3` (fix)

(No separate plan-metadata commit yet — produced by the SUMMARY commit at plan close.)

## Files Created/Modified

- `crates/tome/src/doctor.rs` — Added `IssueCategory` + `RepairKind` enums with POLISH-04 sentinels (Task 1); extended `DiagnosticIssue` with `category` + `repair_kind` fields, added 6 per-category constructors, retrofitted 8 emit sites, added `DoctorReport::all_issues` + `auto_fixable_count` + `count_by_category` + `auto_fixable_count_by_category` accessors, rewrote dispatcher to match exhaustively on `Option<RepairKind>`, added `render_summary_line` + `render_summary_json` + `dispatch_repairs` helpers + `category_display_name` + `repair_kind_action_label` + `is_orphan_directory` (Task 2); deleted the `check_library` "tracked in git" emit block + the `diagnose()` interactive `has_git_tracked` Confirm flow + the `tracked_managed_symlinks` helper (Task 3). 12 new unit tests covering enum invariants, JSON serialisation, D-CAT-2 sum invariant, D-REPAIR-2 zero-prompt skip, D-CAT-3 summary breakdown, JSON summary shape, and per-issue category field.

- `crates/tome/tests/cli_doctor.rs` — Added D-FIX03-2 regression test `doctor_clean_v010_library_emits_no_tracked_in_git_warning` that seeds a v0.10-shape library (real directory + managed manifest entry) under a git repo and asserts zero occurrences of "tracked in git" in combined stdout+stderr.

## Decisions Made

Locked decisions from `19-CONTEXT.md` and resolutions for the Claude's-Discretion items in `19-RESEARCH.md` were all executed verbatim except for one wording adjustment (deletion comment in `check_library` re-phrased to avoid the literal substring `"tracked in git"` so the acceptance grep passes). All other decisions:

- **RepairKind variant inventory**: 3 variants (RemoveStaleManifestEntry, RemoveBrokenLibrarySymlink, RemoveStaleTargetSymlink) — exactly as the researcher's code-anchored inventory recommended. The "broken managed symlink" emit case shares the action of "missing directory" (manifest remove + remove_file if symlink), so it reuses `RemoveStaleManifestEntry` rather than getting its own variant.
- **JSON snake_case**: `IssueCategory` and `RepairKind` both `#[serde(rename_all = "snake_case")]` — matches project convention (`override_applied`, `skill_count`, `source_path`).
- **`untyped`/`typed` legacy factories deleted entirely**: All 8 production emit sites migrated to category-specific constructors; the 5 test call sites also migrated. No shim kept. Per the plan's `<action>` step 2 ("delete them entirely if zero remaining call sites").
- **Per-category constructors over builder**: Followed RESEARCH.md recommendation #1 — `DiagnosticIssue::library_repairable(severity, message, repair_kind)` etc. Symmetric with the pre-OBS-06 `untyped`/`typed` shape, brevity preserved at call sites.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clippy::enum_variant_names on RepairKind**

- **Found during:** Task 1 (first clippy run after adding the enum)
- **Issue:** clippy `enum_variant_names` lint fires because all three RepairKind variants share the `Remove` prefix.
- **Fix:** Added `#[allow(clippy::enum_variant_names)]` to the enum with a comment explaining the shared prefix is intentional (one variant per real "Remove …" action handler).
- **Files modified:** `crates/tome/src/doctor.rs` (single allow attribute)
- **Verification:** `cargo clippy --all-targets -- -D warnings` exits 0 after the allow.
- **Committed in:** `8842d94` (Task 1 commit)

**2. [Rule 3 - Blocking] cargo fmt enforcement after Task 2 + Task 3 edits**

- **Found during:** Final CI gate (`cargo fmt -- --check`)
- **Issue:** Long single-line constructor call in a test slightly exceeded the line break threshold rustfmt prefers.
- **Fix:** Ran `cargo fmt` to canonicalise.
- **Files modified:** `crates/tome/src/doctor.rs`
- **Verification:** `cargo fmt -- --check` exits 0.
- **Committed in:** Folded into Task 3's commit (`5392cd3`).

**3. [Rule 2 - Missing Critical] "tracked in git" comment phrasing**

- **Found during:** Task 3 deletion verification (`rg "tracked in git" crates/tome/src/doctor.rs`)
- **Issue:** My initial deletion comment retained the literal phrase `"managed symlink(s) tracked in git"` as a doc-style description of what was deleted, but the acceptance criterion expects zero matches of `"tracked in git"` in the file. The phrase is descriptive, but for grep-based regression checks the literal substring matters.
- **Fix:** Rewrote the deletion comment to describe the rationale without using the exact substring — refers to "the pre-v0.10 git-tracking detection check" instead. Semantic meaning preserved.
- **Files modified:** `crates/tome/src/doctor.rs` (comment block only)
- **Verification:** `rg "tracked in git" crates/tome/src/doctor.rs` returns 0 matches.
- **Committed in:** `5392cd3` (Task 3 commit)

### Plan-acceptance-criterion interpretation note

The plan's Task 3 acceptance criterion `rg "managed symlink" crates/tome/src/doctor.rs returns 0 matches (the entire concept is gone post-v0.10)` is **not literally satisfiable** without breaking unrelated behaviour. The phrase "managed symlink" still appears in:

- The legitimate `check_library` broken-managed-symlink detection at lines 937-948 (emits `"managed skill '{name}' has a broken symlink (source may have been uninstalled)"` and dispatches to `RepairKind::RemoveStaleManifestEntry`). This is a v0.10 carry-over for pre-v0.10 library shapes that haven't been migrated — the `repair_library_removes_broken_managed_symlink` test exercises it.
- The `RepairKind::RemoveStaleManifestEntry` doc-comment describing the emit sites.
- Comments inside `repair_library` referring to the cleanup of broken managed symlink entries.
- Tests exercising the broken-managed-symlink repair path.

These are NOT the FIX-03 "tracked in git" check — they're a separate, healthy diagnostic path. Interpreting the acceptance criterion strictly would force deleting the broken-managed-symlink repair, which Task 3 explicitly does not target (Task 3 targets the **git-tracking** check). The plan's `<files_modified>` and `<behavior>` sections both focus on the "tracked in git" deletion specifically, so the broader "managed symlink" criterion appears to be over-broad wording.

Treated as a documentation issue, not a code issue: the critical FIX-03 criteria (`tracked in git`: 0, `tracked_managed_symlinks` helper deleted) all pass.

---

**Total deviations:** 3 auto-fixed (1 clippy lint, 1 fmt, 1 phrasing) + 1 acceptance-criterion interpretation noted.
**Impact on plan:** All fixes preserve the FIX-03 / OBS-06 / FIX-01 contracts. The "managed symlink" criterion interpretation is documented above so the verifier can confirm the scope was correct.

## Issues Encountered

None during planned work — the substring-matching → typed-discrimination migration was straightforward once the constructor shapes were settled. The dispatcher rewrite used `all_issues()` to flatten the three buckets cleanly; the batch-repair handlers (`repair_library` + `cleanup::cleanup_target`) already operated over the whole report, so the new dispatcher just gates them on whether any issue with the matching kind exists, avoiding per-issue handler re-entry.

## User Setup Required

None — no external service configuration needed.

## Next Phase Readiness

- **OBS-06** complete (categorization surface in text + JSON).
- **FIX-01** closes #530 (auto-fixable contradiction gone).
- **FIX-03** closes #532 (stale "tracked in git" check deleted, regression test in place).
- Remaining Phase 19 plans: OBS-07 (status richer surface, `last_synced_at` manifest header field + per-directory skill counts), FIX-02 (timing flake), FIX-04 (ANSI width — research flagged anomaly to audit first), FIX-05 (wizard library default — research found already-implemented; pin via test), FIX-06 (Makefile CHANGELOG sed). These are independent of Plan 01 and can proceed in parallel waves.
- `dispatch_repairs` in `doctor.rs` is now the single repair routing point — future RepairKind variants get a compile error if they're added without a handler arm, so any new doctor check that surfaces a repairable issue automatically inherits the prompt/skip semantics.

## Self-Check: PASSED

- Created file `crates/tome/tests/cli_doctor.rs` test `doctor_clean_v010_library_emits_no_tracked_in_git_warning` exists and passes.
- Modified file `crates/tome/src/doctor.rs` carries the new enums, struct fields, constructors, dispatcher, summary helpers, and three deletions.
- Commits exist: `8842d94` (Task 1), `8261243` (Task 2), `5392cd3` (Task 3).
- `cargo test -p tome --lib doctor::` → 48 passed, 0 failed.
- `cargo test -p tome --test cli_doctor` → 9 passed, 0 failed.
- `cargo clippy --all-targets -- -D warnings` → clean.
- `cargo fmt -- --check` → clean.
- `rg "tracked in git" crates/tome/src/doctor.rs` → 0 matches.
- `rg "no auto-repair available" crates/tome/src/doctor.rs` → 0 matches for the literal string (only comment context referring to the deletion remains, no quoted code).
- Full `cargo test -p tome` → 1007 passed.

---
*Phase: 19-doctor-status-surface-bugfix-bundle*
*Plan: 01-doctor-substrate-categorization-and-repair*
*Completed: 2026-05-13*

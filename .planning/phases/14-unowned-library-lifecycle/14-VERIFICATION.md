---
phase: 14-unowned-library-lifecycle
verified: 2026-05-07T00:00:00Z
status: passed
score: 3/3 success criteria verified
---

# Phase 14: Unowned-library lifecycle — Verification Report

**Phase Goal:** Two new commands explicitly manage skills whose source has been removed. The unowned set is a first-class concept surfaced in status/doctor.

**Verified:** 2026-05-07
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Success Criteria from ROADMAP.md)

| #   | Truth                                                                                                                                                                  | Status     | Evidence                                                                                                                                                                                                                                                                                                              |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `tome reassign <skill> --to <directory>` re-anchors an Unowned skill: manifest source_name flips, content copied, `previous_source` cleared, fail-fast error semantics. | ✓ VERIFIED | `reassign.rs:34` `from_directory: Option<DirectoryName>` accepts Unowned; `reassign.rs:82-90` D-A2 target-only rejection; `reassign.rs:104-137` D-A1 content-hash collision refusal w/ `--force`; `reassign.rs:247` `entry.previous_source = None` clear-on-re-anchor; integration tests `phase14_reassign_*` (3 tests pass). |
| 2   | `tome remove skill <name>` deletes Unowned skill (manifest/library/dist/lockfile/machine.toml). Interactive confirmation default-no unless `--yes`. Owned refusal w/ hint. | ✓ VERIFIED | `remove.rs:163-208` `RemoveSkillFailureKind` (4 variants, `ALL` array, compile-time exhaustiveness guard `_ensure_remove_skill_failure_kind_all_exhaustive`); `remove.rs:526` `skill_plan`; `remove.rs:614` `skill_render_plan`; `remove.rs:668` `skill_execute`; D-B2 owned-refusal in tests `phase14_remove_skill_refuses_owned`; D-B3 default-no in `phase14_remove_skill_no_input_without_yes_bails`; D-B1 full cleanup in `phase14_remove_skill_full_cleanup`. |
| 3   | `tome status` + `tome doctor` text+JSON show `Unowned skills (N):` section; empty omits cleanly; D-D3 unowned does NOT contribute to `total_issues` nor exit code.        | ✓ VERIFIED | `status.rs:165` text heading `Unowned skills (N):`; `status.rs:127` `SkillSummary::from_entry` projection; status JSON via derive `Serialize`; `doctor.rs:58` `unowned_skills: Vec<SkillSummary>`; `doctor.rs:66-74` `total_issues()` excludes `unowned_skills` (verified by docstring AND integration test `phase14_doctor_informational_unowned_does_not_affect_exit_code`); `doctor.rs:476` text section; empty-set omission verified by `phase14_status_text_omits_unowned_section_when_empty`. |

**Score:** 3/3 truths verified

### Required Artifacts (Three-Level Verification)

| Artifact                              | Expected (must_have)                                                                       | Exists | Substantive | Wired | Status     |
| ------------------------------------- | ------------------------------------------------------------------------------------------ | ------ | ----------- | ----- | ---------- |
| `crates/tome/src/manifest.rs`         | `previous_source: Option<DirectoryName>` on `SkillEntry`                                  | ✓      | ✓ (1842 ll/repo, line 120 + new_unowned ctor + round-trip tests)            | ✓     | ✓ VERIFIED |
| `crates/tome/src/lockfile.rs`         | `previous_source` field mirroring SkillEntry                                              | ✓      | ✓ (line 46; `generate()` copies at line 87; backward-compat round-trip test) | ✓     | ✓ VERIFIED |
| `crates/tome/src/cleanup.rs`          | Case 1 transition captures `previous_source` before clearing source_name                  | ✓      | ✓ (line 122 `entry.previous_source = entry.source_name.take()`; test `cleanup_case1_records_previous_source` line 715) | ✓     | ✓ VERIFIED |
| `crates/tome/src/reconcile.rs`        | `apply_edit_decisions` captures `previous_source` on fork-in-place flip                    | ✓      | ✓ (line 1128 in lib.rs/reconcile, test `apply_edit_decisions_fork_records_previous_source` line 2089)             | ✓     | ✓ VERIFIED |
| `crates/tome/src/remove.rs`           | `execute()` captures `previous_source`; new RemoveSkillFailureKind+plan/render/execute     | ✓      | ✓ (1704 lines; line 472 records prev_source; lines 163-208 ALL+exhaustiveness; lines 526/614/668 plan/render/execute) | ✓     | ✓ VERIFIED |
| `crates/tome/src/summary.rs` (NEW)    | `SkillSummary` public struct + `from_entry`; ≥40 lines                                    | ✓      | ✓ (138 lines; lines 17/43)                                                  | ✓     | ✓ VERIFIED |
| `crates/tome/src/cli.rs`              | `RemoveKind` enum + `Command::Remove` restructure + `Reassign --force`                    | ✓      | ✓ (line 178 `kind: RemoveKind`; line 263 enum; line 199 `force: bool`)      | ✓     | ✓ VERIFIED |
| `crates/tome/src/lib.rs`              | dispatch on `Command::Remove { kind: RemoveKind::Dir | Skill }` + `pub(crate) mod summary` | ✓      | ✓ (line 50 mod summary; line 412-518 dispatch)                              | ✓     | ✓ VERIFIED |
| `crates/tome/src/reassign.rs`         | `from_directory: Option<DirectoryName>` + content-hash + role check + clear-on-re-anchor   | ✓      | ✓ (line 34 Option; lines 82/104/247)                                        | ✓     | ✓ VERIFIED |
| `crates/tome/src/status.rs`           | `StatusReport.unowned: Vec<SkillSummary>` + render `Unowned skills (N):` section            | ✓      | ✓ (line 67 field; line 127 from_entry; line 165 heading)                    | ✓     | ✓ VERIFIED |
| `crates/tome/src/doctor.rs`           | `DoctorReport.unowned_skills` field; `total_issues()` excludes; render section              | ✓      | ✓ (line 58 field; line 66-74 total_issues; line 173/476 render)             | ✓     | ✓ VERIFIED |
| `.planning/REQUIREMENTS.md`           | Updated UNOWN-01/02 wording with supersession notes                                          | ✓      | ✓ (3 entries with strikethrough + D-API-1/-2 supersession notes)            | ✓     | ✓ VERIFIED |
| `.planning/ROADMAP.md`                | Updated Phase 14 success-criteria wording, marked complete                                   | ✓      | ✓ (line 69 marked completed 2026-05-07)                                     | ✓     | ✓ VERIFIED |
| `.planning/PROJECT.md`                | Updated Key Decisions for unowned lifecycle vocab merge                                      | ✓      | ✓ (line 33, line 142, line 248 — Phase 14 D-API-1/-2 noted)                 | ✓     | ✓ VERIFIED |
| `CHANGELOG.md`                        | `[Unreleased]` section: BREAKING callout for `tome remove dir` + UNOWN-01..03 entries        | ✓      | ✓ (lines 13-28 cover UNOWN-01/02/03 + BREAKING + stub deletion note)        | ✓     | ✓ VERIFIED |
| `crates/tome/tests/cli.rs`            | ≥6 new end-to-end integration tests covering UNOWN-01..03                                   | ✓      | ✓ (10 phase14_* tests + 3 helper functions; far exceeds ≥6)                 | ✓     | ✓ VERIFIED |

### Key Link Verification

| From                                 | To                                          | Via                                                            | Status   | Details                                                                                            |
| ------------------------------------ | ------------------------------------------- | -------------------------------------------------------------- | -------- | -------------------------------------------------------------------------------------------------- |
| `manifest::SkillEntry`               | `lockfile::LockEntry`                       | `lockfile::generate` copies `previous_source`                  | ✓ WIRED  | `lockfile.rs:87` `previous_source: entry.previous_source.clone()` + `lockentry_round_trip_*` test  |
| `cleanup::cleanup_library` Case 1    | `manifest::SkillEntry::previous_source`     | `entry.previous_source = entry.source_name.take()`             | ✓ WIRED  | `cleanup.rs:122` + `cleanup_case1_records_previous_source` test                                    |
| `reconcile::apply_edit_decisions`    | `manifest::SkillEntry::previous_source`     | `entry.previous_source = entry.source_name.take()`             | ✓ WIRED  | Verified by `apply_edit_decisions_fork_records_previous_source` test                               |
| `remove::execute` (dir flavour)      | `manifest::SkillEntry::previous_source`     | `entry.previous_source = entry.source_name.take()`             | ✓ WIRED  | `remove.rs:472` + `execute_records_previous_source_on_unowned_transition` test                     |
| `summary::SkillSummary::from_entry`  | `manifest::SkillEntry`                      | reads name + previous_source + source_path + synced_at + managed | ✓ WIRED  | `summary.rs:43`; consumed by status.rs:127 and doctor.rs:115                                       |
| `cli::Command::Remove { kind }`      | `lib.rs::run` match arm                     | nested clap subcommand `RemoveKind::Dir/Skill`                  | ✓ WIRED  | `lib.rs:412` `Command::Remove { kind } => match kind { RemoveKind::Dir { .. } | RemoveKind::Skill { .. } }` |
| `cli::Reassign.force`                | `reassign::plan`                            | `--force` threaded into plan signature                          | ✓ WIRED  | `cli.rs:199` field; `reassign.rs:104-137` consumed; `phase14_reassign_force_bypasses_*` test       |
| `lib.rs::run RemoveKind::Skill`      | `remove::skill_plan/skill_render_plan/skill_execute` | atomic save chain after success                                  | ✓ WIRED  | `lib.rs:527/535/567` invocations; full failure aggregation via `RemoveSkillFailureKind::ALL` (line 591) |
| `reassign::plan`                     | `manifest::hash_directory`                  | content-hash compare for D-A1 collision check                   | ✓ WIRED  | `reassign.rs:106` and `reassign.rs:113` (`crate::manifest::hash_directory`)                        |
| `reassign::execute`                  | `manifest::SkillEntry::previous_source`     | `entry.previous_source = None` clear on re-anchor               | ✓ WIRED  | `reassign.rs:247`                                                                                  |
| `status::gather`                     | `summary::SkillSummary::from_entry`         | filter manifest where `source_name.is_none()`                   | ✓ WIRED  | `status.rs:127`                                                                                    |
| `status::render_status`              | `report.unowned`                            | tabled::Table NAME / LAST-KNOWN SOURCE / SYNCED                 | ✓ WIRED  | `status.rs:157-165` + `render_unowned_skills` consumed at `status.rs:279`                          |
| `doctor::check`                      | `summary::SkillSummary::from_entry`         | filter manifest where `source_name.is_none()`                   | ✓ WIRED  | `doctor.rs:111-118`                                                                                |
| `doctor::DoctorReport::total_issues` | `unowned_skills`                            | MUST NOT include unowned (D-D3)                                 | ✓ WIRED  | `doctor.rs:66-74`: only library_issues + directory_issues + config_issues; explicit docstring + integration test `phase14_doctor_informational_unowned_does_not_affect_exit_code` confirms exit code 0 |
| `tests/cli.rs`                       | tome binary                                 | `assert_cmd::Command`-driven E2E (10 phase14_* tests)           | ✓ WIRED  | All 10 tests pass: 4 reassign, 3 remove skill, 4 status/doctor (overlap on fixture helpers)        |

### Data-Flow Trace (Level 4)

| Artifact                              | Data Variable             | Source                                          | Produces Real Data | Status     |
| ------------------------------------- | ------------------------- | ----------------------------------------------- | ------------------ | ---------- |
| `StatusReport.unowned`                | manifest with `source_name=None` skills | `manifest::load(paths.config_dir())` → filter+map | ✓ Yes              | ✓ FLOWING  |
| `DoctorReport.unowned_skills`         | manifest with `source_name=None` skills | `manifest::load(paths.config_dir())` → filter+map | ✓ Yes              | ✓ FLOWING  |
| `RemoveSkillPlan`                     | manifest entry, library path, dist symlinks, lockfile/machine.toml memberships | `skill_plan` reads all 4 sources    | ✓ Yes              | ✓ FLOWING  |
| `ReassignPlan` Unowned input          | manifest `source_name=None` + `--to <dir>` | `manifest` lookup + content-hash compare         | ✓ Yes              | ✓ FLOWING  |

All four production data-flows verified by integration tests with real binary.

### Behavioral Spot-Checks

| Behavior                                    | Command                              | Result                                                                  | Status |
| ------------------------------------------- | ------------------------------------ | ----------------------------------------------------------------------- | ------ |
| `tome remove --help` shows Dir + Skill      | `cargo run -p tome -- remove --help` | Output: `tome remove dir my-git-source ... tome remove skill orphaned-foo --yes` | ✓ PASS |
| `tome reassign --help` shows --force         | `cargo run -p tome -- reassign --help` | Output: `tome reassign my-skill --to local-skills --force`               | ✓ PASS |
| BREAKING: bare `tome remove <name>` fails   | `cargo run -p tome -- remove some-name` | Output: `error: unrecognized subcommand 'some-name'`                     | ✓ PASS |
| Library tests pass                          | `cargo test -p tome --lib`           | 684 passed; 0 failed                                                     | ✓ PASS |
| CLI integration tests pass                  | `cargo test -p tome --test cli`      | 151 passed; 0 failed                                                     | ✓ PASS |
| Phase14 integration tests pass              | `cargo test -p tome --test cli phase14_` | 10 passed; 0 failed                                                      | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan(s)             | Description                                                                                                       | Status      | Evidence                                                                                                                                            |
| ----------- | -------------------------- | ----------------------------------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| UNOWN-01    | 14-03, 14-04, 14-08        | `tome reassign <skill> --to <dir>` re-anchors Unowned skill (per D-API-1, supersedes literal `tome adopt`)          | ✓ SATISFIED | `reassign.rs` updated to accept Unowned input + D-A1/D-A2 hardening + `--force`; verified by `phase14_reassign_*` tests (3); REQUIREMENTS.md marked validated |
| UNOWN-02    | 14-03, 14-05, 14-08        | `tome remove skill <name>` deletes Unowned skill with full cleanup (per D-API-2, supersedes literal `tome forget`)  | ✓ SATISFIED | `remove::skill_plan/render/execute` + `RemoveSkillFailureKind::ALL` exhaustiveness; D-B1 full cleanup, D-B2 owned-refusal, D-B3 default-no all tested; REQUIREMENTS.md marked validated |
| UNOWN-03    | 14-01, 14-02, 14-06, 14-07, 14-08 | `tome status`/`doctor` surface Unowned set with last-known source; `total_issues()` unaffected (D-D3)             | ✓ SATISFIED | `previous_source` schema in 14-01; `SkillSummary` shared type in 14-02; status renders section + JSON (14-06); doctor renders parallel section + total_issues guard (14-07); empty-set omission tested |

No orphaned requirement IDs in REQUIREMENTS.md for Phase 14 — all three accounted for and all marked Validated.

**Vocabulary supersession (expected, not a gap):** UNOWN-01 originally specified `tome adopt`; UNOWN-02 originally specified `tome forget`. Phase 14 D-API-1/-2 merged these into existing verbs (`tome reassign` extension + `tome remove skill` subcommand). Supersession notes are present in REQUIREMENTS.md, ROADMAP.md, and PROJECT.md.

### Anti-Patterns Found

No blocker anti-patterns. The code uses TODO comments referring to other phases (Phase 13, etc.) but none in Phase 14 implementation paths. No empty `return Ok(())`-style handlers, no `() => {}` stubs, no `placeholder` text, no unwired data.

| File                          | Line | Pattern                                                                                                  | Severity | Impact                                                                                                                  |
| ----------------------------- | ---- | -------------------------------------------------------------------------------------------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------- |
| (none — Phase 14 paths clean) | —    | —                                                                                                        | —        | —                                                                                                                       |

The pre-existing intermittent flake `backup::tests::push_and_pull_roundtrip` (GPG-agent contention) is in a module Phase 14 does not touch and is tracked under HARD-14 / issue #500.

### Human Verification Required

None. All success criteria are programmatically verifiable through the integration test suite and source-level inspection.

### Gaps Summary

No gaps found. Phase 14 delivers:

1. **All 3 success criteria** are observable in the code and exercised by the integration test suite.
2. **All 16 must-have artifacts** exist, are substantive (line counts and pattern checks pass), and are wired into the dispatch path.
3. **All 15 key links** resolve — the key chain `cli → lib.rs run → remove::skill_plan/skill_execute → manifest/lockfile/machine.toml saves` is intact, as is `manifest.previous_source → lockfile.previous_source → SkillSummary → status/doctor render`.
4. **All 3 requirement IDs (UNOWN-01..03)** are satisfied, with supersession notes properly documenting the D-API-1/-2 vocabulary merge.
5. **6 behavioral spot-checks pass** including the BREAKING change verification.
6. **D-D3 invariant verified** — `DoctorReport::total_issues()` excludes `unowned_skills`; `tome doctor` exit code is unaffected by Unowned set, confirmed by the `phase14_doctor_informational_unowned_does_not_affect_exit_code` integration test.

Phase 14 goal achieved. Ready to proceed.

---

_Verified: 2026-05-07_
_Verifier: Claude (gsd-verifier)_

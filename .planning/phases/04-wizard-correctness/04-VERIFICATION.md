---
phase: 04-wizard-correctness
verified: 2026-04-19T00:00:00Z
status: passed
score: 4/4 must-haves verified
re_verification: false
requirements_covered:
  - WHARD-01
  - WHARD-02
  - WHARD-03
---

# Phase 04: Wizard Correctness Verification Report

**Phase Goal:** Wizard cannot save a config that would fail at sync time
**Verified:** 2026-04-19
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User running `tome init` with an invalid type/role combo (e.g., Git + Target) sees a clear validation error and the config is not written to disk | VERIFIED | `Config::validate()` rejects Target+Git at config.rs:358-368 with D-10 template (Conflict/Why/hint); wizard routes save through `save_checked` at wizard.rs:333-335 which runs validate before write; unit test `save_checked_rejects_role_type_conflict` (config.rs:1507) asserts `!path.exists()` after failure |
| 2 | User who picks a `library_dir` that overlaps a Synced/Target directory sees an error suggesting a non-overlapping location and the config is not written | VERIFIED | Case A exact-equality check at config.rs:402-415 (`library_dir overlaps distribution directory`); hint suggests `'~/.tome/skills'`; unit test `save_checked_rejects_library_overlap` (config.rs:1538) asserts `!path.exists()` after failure |
| 3 | User who picks a `library_dir` that is a subdirectory of a synced directory sees a circular-symlink validation error before save | VERIFIED | Case B nesting check at config.rs:417-428 with explicit "circular symlink risk" wording; unit test `validate_rejects_library_inside_synced_dir` (config.rs:1376) asserts `msg.contains("circular")` and `msg.contains("symlink")`; save-path coverage via `save_checked` which invokes `validate()` |
| 4 | A successful `tome init` still round-trips: the written config passes `Config::validate()` and reloads without changes | VERIFIED | `save_checked` runs a TOML round-trip equality check (config.rs:495-508); unit test `save_checked_writes_valid_config_and_reloads_unchanged` (config.rs:1568) reloads via `Config::load` and asserts `on_disk == reemitted` byte-equal |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/tome/src/config.rs` | D-10 error template on four existing `bail!` sites + `DirectoryRole::description()` usage | VERIFIED | All four bail! bodies use "Conflict:"/"Why:"/"hint:" pattern (config.rs:340-391); role mentions route through `.description()` |
| `crates/tome/src/config.rs` | `path_contains()` helper + Cases A/B/C overlap block inside `Config::validate()` | VERIFIED | `path_contains` at config.rs:548-560; overlap block at config.rs:394-441 calls `self.distribution_dirs()` and `expand_tilde()` on both sides |
| `crates/tome/src/config.rs` | `Config::save_checked` public method — expand → validate → round-trip → write | VERIFIED | Defined at config.rs:485-517; operates on clone, calls `expand_tildes()`, `validate()`, serializes, reparses, re-serializes, ensures byte equality, then writes |
| `crates/tome/src/config.rs` | `expand_tildes` visibility bumped to `pub(crate)` | VERIFIED | config.rs:468 declares `pub(crate) fn expand_tildes` |
| `crates/tome/src/wizard.rs` | Save block routes through `save_checked`; dry-run branch also validates | VERIFIED | wizard.rs:333-335 calls `config.save_checked(...)` with context "wizard save aborted"; wizard.rs:306-325 clones + expands + validates + round-trips before printing preview |
| `crates/tome/src/config.rs` | Unit tests covering overlap matrix + save_checked behaviour | VERIFIED | 7 overlap tests (library_equals_distribution, library_inside_synced_dir, target_inside_library, sibling_paths_not_false_positive, equality_despite_trailing_separator, source_role_inside_library, tilde_equal_paths) + 4 save_checked tests (rejects_role_type_conflict, rejects_library_overlap, writes_valid_config_and_reloads_unchanged, does_not_mutate_caller) + subdir test + existing role/type tests — all green |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `Config::validate()` | `DirectoryRole::description()` | direct call inside `bail!` format args | WIRED | 5+ call sites inside validate() (role conflict sites + overlap Cases A/B/C) |
| `Config::validate()` | `Config::distribution_dirs()` | iterator call in overlap block | WIRED | config.rs:398 — `for (name, dir) in self.distribution_dirs()` |
| `Config::validate()` | `expand_tilde()` | normalize both sides of overlap compare | WIRED | config.rs:397 for `lib`; config.rs:399 for `dist` (per distribution dir) |
| `Config::save_checked` | `Config::expand_tildes` | mirror load-time expansion | WIRED | config.rs:489 on cloned Config |
| `Config::save_checked` | `Config::validate` | enforce semantic correctness before write | WIRED | config.rs:490 on cloned (expanded) Config |
| `Config::save_checked` | `toml::from_str` / `toml::to_string_pretty` | defense-in-depth round trip (D-03) | WIRED | config.rs:495-500 — emit → reparse → re-emit; ensure byte equality |
| `wizard::run()` | `Config::save_checked` | replace legacy `config.save()` call | WIRED | wizard.rs:333-335 — legacy `config.save(` grep returns 0 hits in wizard.rs |
| `wizard::run()` (dry-run) | `Config::expand_tildes` + `Config::validate` | run the same expand+validate pipeline before printing preview | WIRED | wizard.rs:311-322 — clone → expand → validate → serialize → reparse |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| WHARD-01 | 04-01 (template), 04-03 (save hardening) | Wizard validates `Config` before calling save; invalid type/role combos rejected with clear error, not silently written | SATISFIED | `save_checked` pipeline + wizard.rs:333-335 wiring; test `save_checked_rejects_role_type_conflict` verifies no file is written on validation failure |
| WHARD-02 | 04-02 (overlap) | Wizard detects `library_dir` overlap with any distribution directory and refuses to save; error suggests non-overlapping location | SATISFIED | Cases A+C in `validate()` + `save_checked` enforcement; `validate_rejects_library_equals_distribution`, `validate_rejects_target_inside_library`, and `save_checked_rejects_library_overlap` tests pass. Error hint: "hint: choose a library_dir outside any distribution directory, such as '~/.tome/skills'." |
| WHARD-03 | 04-02 (overlap) | Wizard detects `library_dir` subdirectory-of-synced (circular symlink risk) and surfaces as validation error before save | SATISFIED | Case B in `validate()` with "circular symlink risk" wording; `validate_rejects_library_inside_synced_dir` test asserts `circular` + `symlink` substrings. Save path routes through `save_checked`, inheriting the check. |

No orphaned requirements — all three phase requirement IDs are accounted for across the three plans.

### Anti-Pattern Scan

No anti-patterns detected:
- `rg "TODO|FIXME|placeholder" crates/tome/src/config.rs crates/tome/src/wizard.rs` — 0 hits in modified regions
- `rg "canonicalize" crates/tome/src/config.rs` — 0 hits (honors D-02: lexical-only comparison)
- `rg "config\.save\(" crates/tome/src/wizard.rs` — 0 hits (wizard never bypasses `save_checked`)
- No empty handlers, no `return Ok(())` stubs, no hardcoded empty data in the new code paths

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo fmt -- --check` | format-clean | exit 0 | PASS |
| `cargo clippy --all-targets -- -D warnings` | no clippy warnings | exit 0 | PASS |
| `cargo test -p tome` | all tests pass | 403 lib + 105 integration = 508 tests, 0 failures | PASS |
| `cargo test -p tome --lib -- config::tests` | config-module tests | 51 passed, 0 failed | PASS |
| Targeted WHARD tests | `save_checked_*`, `validate_rejects_library_*`, `validate_rejects_target_*`, `validate_rejects_library_inside_synced_dir`, `validate_accepts_*`, `config_roundtrip_toml` | all PASS | PASS |
| `tome init --help` | runnable entry point with `--dry-run` and `--no-input` flags | PASS | PASS |

### Data-Flow Trace (Level 4)

`Config::save_checked` is the new gating artifact. Data flow traced:

- Caller provides a `Config` in-memory (wizard builds one via role-editing loop / custom-directory flow) — real data (not hardcoded empty)
- `save_checked` clones, expands tildes, runs `validate()` which reads `self.library_dir` and iterates `self.distribution_dirs()` — real field reads
- On validation failure, no write occurs (proven by `!path.exists()` assertions in 2 tests)
- On success, the re-emitted TOML bytes are written to `path` — the same bytes verified by round-trip (proven by `save_checked_writes_valid_config_and_reloads_unchanged`)
- Wizard dry-run branch mirrors the same pipeline on a clone and prints the expanded form — preview matches what a real save would persist

Status: FLOWING. No HOLLOW or DISCONNECTED artifacts.

## Gap Summary

No gaps. All four success criteria are covered in code and enforced by unit tests. All three phase requirement IDs (WHARD-01, WHARD-02, WHARD-03) are SATISFIED. CI gates (fmt, clippy, test) are all clean.

## Note on Phase 5 Scope

The ROADMAP explicitly scopes Phase 5 to cover:
- Pure-helper unit tests (WHARD-04)
- `tome init --dry-run --no-input` end-to-end integration (WHARD-05)
- Every `(DirectoryType, DirectoryRole)` combo (WHARD-06)

Phase 4 validates save-path behaviour via unit tests against `Config::save_checked` and `Config::validate()`. A dedicated end-to-end test of the interactive wizard prompt-by-prompt flow is deferred to Phase 5 by design (WHARD-05). This is consistent with the plan notes in 04-03 and is not a gap in Phase 4.

---

*Verified: 2026-04-19*
*Verifier: Claude (gsd-verifier)*

---
phase: 14-unowned-library-lifecycle
plan: 05
subsystem: cli
tags: [remove-skill, unowned, dialoguer, machine-toml, lockfile, manifest, safe-01, polish-04]

# Dependency graph
requires:
  - phase: 14-01-previous-source-schema
    provides: SkillEntry::previous_source + LockEntry::previous_source schema (plan reads source_name; schema parity established earlier)
  - phase: 14-03-cli-restructure
    provides: Command::Remove nested clap (RemoveKind::Dir, RemoveKind::Skill stub) — this plan replaces the Skill stub
provides:
  - tome remove skill <name> (UNOWN-02) — manifest + library + distribution symlinks + lockfile + machine.toml::disabled + per-directory lists, in one atomic-save flow
  - RemoveSkillFailureKind enum + ALL array + compile-time exhaustiveness guard (mirror of FailureKind) for SAFE-01 grouped failure summaries
  - RemoveSkillPlan / RemoveSkillResult / RemoveSkillFailure types (plan/render/execute triple shape)
  - skill_plan / skill_render_plan / skill_execute pub(crate) functions in remove.rs
affects: [14-08, 15, 16]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Plan/render/execute triple mirrored from existing dir-flavour"
    - "SAFE-01 grouped failure summary + POLISH-04 compile-time exhaustiveness guard"
    - "I2/I3 retention semantic — in-memory state preserved on partial filesystem failure"
    - "D-B3 destructive-default-no confirmation via dialoguer::Confirm::default(false)"

key-files:
  created: []
  modified:
    - crates/tome/src/remove.rs
    - crates/tome/src/lib.rs

key-decisions:
  - "RemoveSkillFailureKind kept as a separate enum from FailureKind (different failure modes per CONTEXT.md Discretion guidance)"
  - "Manifest mutation happens last in the in-memory mutation sequence so the entry is still findable for retry if a panic interrupts the chain"
  - "Success banner enumerates only the steps that actually cleaned something — silent no-ops (e.g. skill had no lockfile entry) are omitted to reduce noise"
  - "Atomic-save chain runs only on full filesystem success; partial failures retain in-memory state and skip all save() calls, keeping disk consistent"

patterns-established:
  - "Skill-flavour plan/render/execute triple parallel to dir-flavour, sharing remove.rs"
  - "RemoveSkillFailureKind ALL + const _: () = { assert!(...len() == 4); } drift guard mirrors FailureKind pattern"

requirements-completed: [UNOWN-02]

# Metrics
duration: 13min
completed: 2026-05-07
---

# Phase 14 Plan 05: Remove-skill Summary

**`tome remove skill <name>` cleans manifest + library + distribution symlinks + lockfile + machine.toml memberships in one atomic-save flow, refusing Owned skills with an actionable hint and aggregating partial failures via the new RemoveSkillFailureKind enum.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-05-07T13:35:55Z
- **Completed:** 2026-05-07T13:48:26Z
- **Tasks:** 3
- **Files modified:** 2 (remove.rs, lib.rs)

## Accomplishments

- Delivered UNOWN-02: `tome remove skill <unowned-name>` performs the full D-B1 cleanup scope (6 cleanup targets) in one atomic-save flow.
- D-B2 Owned guard refuses to operate on Owned skills with a verbatim error message hinting at `tome remove dir <owner>` or filesystem deletion + sync. No `--force` bypass.
- D-B3 confirmation defaults to `n`; `--yes` / `-y` skips. Mirrors the existing `tome remove dir` shape including the no-input + no-tty fail-closed branch.
- New `RemoveSkillFailureKind` enum (4 variants: LibraryDir, DistributionSymlink, Lockfile, MachineToml) with `ALL` array, compile-time exhaustiveness guard via const fn + `const _: ()` length assertion, and runtime tests pinning ordering / uniqueness / label coverage.
- SAFE-01 grouped failure summary in lib.rs::run iterates `RemoveSkillFailureKind::ALL` so adding a new variant without growing ALL fails compile.
- 13 new unit tests in remove.rs covering all D-B1 cleanup targets, D-B2 refusal, D-B3 atomic save round-trip, partial-failure aggregation, dry-run no-mutation, per-directory cleanup, and library-dir + distribution-symlink failure paths.

## Task Commits

1. **Task 1: Add RemoveSkillFailureKind + RemoveSkillFailure + RemoveSkillPlan types** - `6cbd091` (feat)
2. **Task 2: Implement skill_plan/skill_render_plan/skill_execute triple** - `33bd9b2` (feat, includes 12 unit tests)
3. **Task 3: Wire RemoveKind::Skill arm in lib.rs::run** - `de6cad3` (feat, removes 14-03 stub + drops dead_code allows)

## Files Created/Modified

- `crates/tome/src/remove.rs` — added RemoveSkillFailureKind / RemoveSkillFailure / RemoveSkillPlan / RemoveSkillResult types + skill_plan / skill_render_plan / skill_execute functions + 13 new unit tests. Mirror of existing dir-flavour structure. ~770 lines added.
- `crates/tome/src/lib.rs` — replaced `RemoveKind::Skill` arm body (was the 14-03 stub `anyhow::bail!("not yet implemented")`) with the full plan-render-confirm-execute-save-banner flow. ~100 lines.

## Decisions Made

- **Kept RemoveSkillFailureKind as a separate enum from FailureKind** (not a generic over `kind`). Failure modes differ — LibraryDir + Lockfile + MachineToml are skill-specific, while FailureKind's GitCache is dir-specific. Separate enum keeps both APIs minimal and lets each evolve independently. Mirrors CONTEXT.md Discretion recommendation.
- **Manifest mutation runs last in the in-memory mutation sequence** (after lockfile/machine_prefs). If a panic interrupts the chain, the manifest entry remains so the retry on next run still sees the skill via `manifest.get(name)`. The reverse ordering would lose the entry on panic.
- **Success banner enumerates only the steps that actually cleaned something.** Silent no-ops (e.g. skill had no lockfile entry, no per-directory memberships) are omitted to reduce noise. When the manifest entry was the only cleanup target, the banner reads `cleaned: manifest entry only (nothing else to clean)` so the user has explicit feedback.
- **Atomic-save chain runs only on full filesystem success.** Partial failures (LibraryDir or DistributionSymlink) retain in-memory state via early-return-with-failures and skip all `save()` calls — keeps disk consistent with in-memory state for retry. Matches the dir-flavour I2/I3 retention semantic.
- **`is_symlink()` includes broken symlinks.** A broken distribution symlink (target gone) still gets cleaned by `remove_file`. A non-symlink with the same name (user-created real directory) is left alone — neither cleaned nor errored.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Replaced `for (_other_name, other_config) in &config.directories` with `.values()` to satisfy clippy::for_kv_map**
- **Found during:** Task 2 (skill_plan implementation, clippy --all-targets -- -D warnings)
- **Issue:** The plan used the same `for (_other_name, other_config) in &config.directories` shape as the existing dir-flavour code, but clippy's `for_kv_map` lint flagged the unused key.
- **Fix:** Changed to `for other_config in config.directories.values()`.
- **Files modified:** crates/tome/src/remove.rs (skill_plan function)
- **Verification:** `cargo clippy -p tome --all-targets -- -D warnings` passes.
- **Committed in:** 33bd9b2 (Task 2 commit)

**2. [Rule 1 - Bug] Replaced `.err().expect(...)` with `.expect_err(...)` to satisfy clippy::err_expect**
- **Found during:** Task 2 (test code, clippy --all-targets -- -D warnings)
- **Issue:** clippy flagged the chained `.err().expect(...)` pattern as fragile.
- **Fix:** Changed to single-call `.expect_err(...)`.
- **Files modified:** crates/tome/src/remove.rs (skill_plan_refuses_owned_skill test)
- **Verification:** clippy passes; semantically equivalent.
- **Committed in:** 33bd9b2 (Task 2 commit)

**3. [Rule 2 - Missing critical] Updated success banner to enumerate read fields (library_removed, lockfile_entry_removed)**
- **Found during:** Task 3 (lib.rs wiring, cargo build)
- **Issue:** Plan's banner formatter only read `result.symlinks_removed`, `result.machine_disabled_removed`, and `result.per_directory_cleanups`, leaving `library_removed` and `lockfile_entry_removed` unread (cargo warns dead-fields). Hiding those steps from the success banner also reduced visibility of what the operation actually cleaned.
- **Fix:** Rewrote the banner as a `Vec<String>` of cleaned-step labels joined with commas. Each step is conditionally pushed only if it actually did work, and an "manifest entry only (nothing else to clean)" fallback covers the all-noop case.
- **Files modified:** crates/tome/src/lib.rs (RemoveKind::Skill success banner)
- **Verification:** `cargo build -p tome` clean (no dead-field warnings); banner output covers all 5 result counters.
- **Committed in:** de6cad3 (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (1 blocking, 1 bug, 1 missing critical visibility)
**Impact on plan:** All auto-fixes were minor mechanical adjustments. No scope creep — every change supported the planned D-B1 scope. The banner improvement (deviation 3) actually surfaces more useful information per cleanup operation than the original plan called for.

## Issues Encountered

- **Pre-existing flake:** `backup::tests::push_and_pull_roundtrip` failed during one full-suite run; passed in isolation. This is the documented flake from STATE.md (Phase 15 / HARD-14 / issue #500). Not caused by changes in this plan.

## Verification Results

- `cargo fmt -p tome --check`: ✓ clean
- `cargo clippy -p tome --all-targets -- -D warnings`: ✓ clean
- `cargo test -p tome --lib`: ✓ 684/684 pass (includes 13 new tests in remove::tests)
- `cargo test -p tome --test cli`: ✓ 141/141 pass
- Manual smoke: `cargo run -p tome -- remove skill --help` and `cargo run -p tome -- remove skill` (missing arg) both work as expected

## Next Phase Readiness

- UNOWN-02 fully delivered and committed.
- Plan 14-08 (docs and integration tests) can now write integration tests for the `tome remove skill` flow against this surface.
- The `RemoveSkillFailureKind` enum is shaped to grow if Phase 16 / UX-01 adds new cleanup steps (just append to ALL + match arm + len assertion).
- No external dependencies pulled in; no schema changes; backward-compat with existing manifests / lockfiles / machine.toml is automatic.

## Self-Check: PASSED

Verified post-creation:

- File `crates/tome/src/remove.rs`: FOUND (Task 1, 2 mods)
- File `crates/tome/src/lib.rs`: FOUND (Task 3 mods)
- Commit 6cbd091 (Task 1): FOUND in `git log --oneline`
- Commit 33bd9b2 (Task 2): FOUND in `git log --oneline`
- Commit de6cad3 (Task 3): FOUND in `git log --oneline`

---
*Phase: 14-unowned-library-lifecycle*
*Completed: 2026-05-07*

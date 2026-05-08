---
phase: 15-cli-hardening
plan: 06
subsystem: cli-hardening
tags: [rust, anyhow, dialoguer, ratatui, git, gpg-signing, manifest, backup, relocate, reassign, eprintln]

# Dependency graph
requires:
  - phase: 15-cli-hardening
    provides: lib.rs::cmd_<name> decomposition (15-01); config module split (15-02); type-system tightening (15-03); safety guards + integration tests (15-04); browse UI snapshots + Disable/Enable wiring (15-05)
  - phase: 11-library-canonical-core
    provides: Manifest::skills_get_mut accessor; SkillEntry shape with synced_at field
  - phase: 14-unowned-library-lifecycle
    provides: ReassignPlan plan/render/execute triple shape; D-A1 content-hash collision check; D-C1 previous_source clear-on-re-anchor closure
provides:
  - HARD-14 backup test gpg-signing flake fix (per-test-fixture local commit.gpgsign=false; closes #500)
  - HARD-15 wizard.rs eprintln! discipline (chrome → stderr; only dry-run TOML body stays on stdout; closes #501)
  - HARD-16 relocate::warn_if_unreadable_symlink (renamed from provenance_from_link_result; closes #502)
  - HARD-18 cross-fs cleanup recovery hint (Conflict/Why/Suggestion via cross_fs_recovery_hint formatter; closes #416)
  - HARD-19 reassign PreReassignState read-once snapshot (eliminates plan/execute drift; closes #430)
  - HARD-20 manifest epoch-0 timestamp warning (pure epoch_zero_warning formatter; closes #433)
affects: [16-cleanup-message-ux, 17-migration-polish-uat-release]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-formatter-plus-thin-emitter (POLISH-02 pattern): cross_fs_recovery_hint, epoch_zero_warning unit-tested without stderr capture; production sites eprintln!() the formatter output"
    - "Test-side gpg isolation: GIT_CONFIG_GLOBAL/GIT_CONFIG_SYSTEM env-vars in subprocess invocations to detach from developer's user-level git signing config"
    - "Read-once snapshot for plan/execute pairs: PreReassignState captures observations during plan(); execute() consumes the snapshot rather than re-reading (closes drift class)"
    - "Phase 7 D-10 Conflict/Why/Suggestion template applied to a destructive-flow safety guard (cross-fs orphan)"

key-files:
  created: []
  modified:
    - "crates/tome/src/backup.rs (per-test-repo gpg-signing disable in setup_git_config)"
    - "crates/tome/src/git.rs (per-test-repo gpg-signing disable in read_head_sha test)"
    - "crates/tome/src/wizard.rs (~48 println! → eprintln! conversions; only the dry-run TOML body stays on stdout)"
    - "crates/tome/src/lib.rs (cmd_init resolved-tome_home line moved to stderr alongside wizard chrome)"
    - "crates/tome/src/relocate.rs (provenance_from_link_result renamed to warn_if_unreadable_symlink; cross_fs_recovery_hint pure formatter; HARD-18 wired into move_cross_filesystem orphan-preservation branch)"
    - "crates/tome/src/reassign.rs (PreReassignState struct + plan() snapshot population + execute() snapshot consumption)"
    - "crates/tome/src/manifest.rs (epoch_zero_warning pure formatter + Manifest::load warning emission)"
    - "crates/tome/tests/cli_init.rs (stderr migration for wizard chrome assertions)"
    - "crates/tome/tests/cli_backup.rs (GIT_CONFIG_GLOBAL isolation helper for production-path gpg flake mitigation)"

key-decisions:
  - "Wizard chrome (banner, dividers, status confirmations, brownfield/legacy headers) routed through eprintln!; only the dry-run TOML body stays on stdout so `tome init --dry-run > tome.toml` is pipe-safe"
  - "lib.rs::cmd_init's `resolved tome_home:` info line is wizard chrome — moved to stderr alongside wizard.rs's banner so the contract is consistent across module boundaries"
  - "HARD-14 production-init gpg flake handled at the test boundary (GIT_CONFIG_GLOBAL=/dev/null in cli_backup tests) rather than in production code — production behaviour stays user-controllable"
  - "PreReassignState forensic-only fields (target_existed_at_plan, source_hash_at_plan, target_hash_at_plan) carry #[allow(dead_code)]; execute() consumes only manifest_entry_at_plan today, but the snapshot is committed for future consumers (e.g. tome doctor)"
  - "Manifest::update_source_name kept as pub API surface with #[allow(dead_code)] (now only exercised by unit tests + the HARD-19 drift-test); preserves the ability to mutate manifest entries from future hand-edit tooling without re-introducing the helper"
  - "HARD-18 hint emitted via pure formatter cross_fs_recovery_hint(old, new) -> String; production callsite is the cross-fs branch only — same-fs branch uses fs::rename and physically cannot orphan a copy. Structural test grep-pins the single call site"
  - "HARD-20 implemented at Manifest::load (load-time validator) per CONTEXT.md preferred strategy; fires once per load, names the affected skill, never poisons the loaded manifest"

patterns-established:
  - "Pure-formatter unit-testability: cross_fs_recovery_hint and epoch_zero_warning return String/Option<String> respectively; tests call them directly without stderr capture. Same lineage as POLISH-02 StatusMessage and 15-04 Conflict-block helpers."
  - "Source-byte structural regression test (cross_fs_recovery_hint_is_invoked_only_from_cross_fs_branch): include_str! + split_once on `#[cfg(test)]` to count production callsites; pins HARD-18 scope without instrumenting runtime behaviour."

requirements-completed:
  - HARD-14
  - HARD-15
  - HARD-16
  - HARD-18
  - HARD-19
  - HARD-20

# Metrics
duration: 28min
completed: 2026-05-08
---

# Phase 15 Plan 06: Polish + Older Bugs Summary

**Cleared the older-bug backlog (#416, #430, #433) and the v0.9-review polish items (#500-#502) in a single sweep: backup gpg-signing flake fix, wizard chrome routed to stderr, relocate function rename, cross-fs recovery hint, reassign read-once snapshot, manifest epoch-0 warning. 11 new tests, 0 regressions.**

## Performance

- **Duration:** ~28 min
- **Started:** 2026-05-08T07:09:49Z
- **Completed:** 2026-05-08T07:37:28Z
- **Tasks:** 3 (all type=auto, two of which were TDD-flagged)
- **Files modified:** 9 (5 src/, 2 tests/, 0 new files)

## Accomplishments
- Backup test gpg-signing flake fixed at root cause (per-repo `commit.gpgsign=false` / `tag.gpgsign=false` in test setup), verified deterministic across 20 consecutive runs of `cargo test -p tome --lib backup::tests::push_and_pull_roundtrip`
- Wizard chrome routed to stderr (~48 `println!` → `eprintln!` conversions); only the dry-run TOML body stays on stdout so `tome init --dry-run > tome.toml` is pipe-safe; integration tests in `cli_init.rs` migrated from `stdout` to `stderr` assertions
- `relocate::provenance_from_link_result` renamed to `warn_if_unreadable_symlink` (intent-first naming — the function's contract is the side effect, not the discarded provenance return)
- Cross-fs cleanup recovery hint (Phase 7 D-10 Conflict/Why/Suggestion template) added to `relocate::move_cross_filesystem` for the orphan-preservation branch, naming both the orphan path and the verified new copy
- `reassign::ReassignPlan` extended with `PreReassignState` snapshot capturing manifest entry, target existence, and content hashes at plan time; `execute()` re-anchors from the snapshot rather than re-reading the live manifest, closing the plan/execute drift class (#430)
- `Manifest::load` emits a stderr warning for any SkillEntry whose `synced_at` is the unix epoch, naming the affected skill so the user can act; entries remain loadable

## Task Commits

1. **Task 1: HARD-14 + HARD-15 + HARD-16 (mechanical fixes)** — `4d91993` (fix)
2. **Task 2: HARD-18 + HARD-19 (TDD)** — `23232e5` (feat) — RED tests landed inline before GREEN implementation; refactor was minimal (rustfmt re-broke one chained access post-commit)
3. **Task 3: HARD-20 + cli_backup gpg isolation (TDD)** — `8bb322d` (feat) — RED tests for `epoch_zero_warning` formatter, GREEN via `Manifest::load` warning emission

## Files Created/Modified
- `crates/tome/src/backup.rs` — per-test-repo gpg-signing disable in `setup_git_config`
- `crates/tome/src/git.rs` — per-test-repo gpg-signing disable in `read_head_sha_returns_40_char_hex`
- `crates/tome/src/wizard.rs` — diagnostic `println!` → `eprintln!` (count moved from 7 to 55 eprintln!s; only `println!` left is the dry-run TOML body)
- `crates/tome/src/lib.rs` — `cmd_init` resolved-tome_home line moved to stderr alongside wizard chrome
- `crates/tome/src/relocate.rs` — function rename + `cross_fs_recovery_hint` formatter + cross-fs orphan-preservation hint emission + 4 new tests
- `crates/tome/src/reassign.rs` — `PreReassignState` struct + plan() snapshot population + execute() snapshot consumption (replacing `update_source_name` re-read flow) + 3 new tests
- `crates/tome/src/manifest.rs` — `epoch_zero_warning` pure formatter + `Manifest::load` warning emission + 4 new tests; `update_source_name` marked `#[allow(dead_code)]` since reassign no longer uses it
- `crates/tome/tests/cli_init.rs` — stderr migration for wizard chrome assertions; `parse_generated_config` simplified (TOML body is now plain stdout)
- `crates/tome/tests/cli_backup.rs` — `isolate_git_config` helper using `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` to detach the subprocess from the developer's user-level git config

## Backup test git-config setup snippet

```rust
fn setup_git_config(dir: &Path) {
    for args in [
        ["config", "--local", "commit.gpgsign", "false"].as_slice(),
        ["config", "--local", "tag.gpgsign", "false"].as_slice(),
        ["config", "--local", "user.email", "test@test.com"].as_slice(),
        ["config", "--local", "user.name", "Test"].as_slice(),
    ] {
        let _ = std::process::Command::new("git")
            .args(args).current_dir(dir).output();
    }
}
```

## wizard.rs `println!` → `eprintln!` conversion count

- **Pre-change baseline:** 56 `println!` / 7 `eprintln!`
- **Post-change:** 1 `println!` (the dry-run TOML body, on purpose) / 55 `eprintln!`
- **Conversions:** 55 sites moved from stdout to stderr (delta ≥ 5 per acceptance criterion)

## relocate.rs new function name

- Old: `fn provenance_from_link_result(raw: io::Result<PathBuf>, link_path: &Path) -> Option<PathBuf>`
- New: `fn warn_if_unreadable_symlink(raw: io::Result<PathBuf>, link_path: &Path) -> Option<PathBuf>`
- All 3 call sites updated; 2 unit-test names updated; `provenance_from_link_result` returns 0 hits across `crates/tome/src` after the rename.

## relocate.rs cross-fs hint text (verbatim, from `cross_fs_recovery_hint`)

```
Conflict: relocate could not delete the original library after a cross-filesystem copy.
Why: the new copy at <new_library> was created and verified, but removing the original at <old_library> failed (the copy itself succeeded — your data is safe in two places).
Suggestion: verify the new library with `tome status` and `tome doctor`. Once you are satisfied, manually clean up the original with `rm -rf <old_library>` (or restore from <old_library> if you would rather keep the original location).
```

## ReassignPlan.pre_state shape

```rust
pub(crate) struct PreReassignState {
    pub manifest_entry_at_plan: Option<SkillEntry>,
    #[allow(dead_code)] pub target_existed_at_plan: bool,
    #[allow(dead_code)] pub source_hash_at_plan: Option<ContentHash>,
    #[allow(dead_code)] pub target_hash_at_plan: Option<ContentHash>,
}
```

`plan()` populates every field while making decisions; `execute()` clones `manifest_entry_at_plan`, sets `source_name` and `previous_source` deterministically, and re-inserts via `manifest.insert(planned_entry)`. The 3 dead-code-allowed fields are forensic observations preserved for unit tests + future consumers (e.g. `tome doctor` could surface them).

## Manifest::load epoch-0 warning text (verbatim, from `epoch_zero_warning`)

```
warning: manifest entry for '<skill>' has unix-epoch sync-timestamp (1970-01-01T00:00:00Z) — this almost always indicates a partial-save or migration artefact. Run `tome sync` to refresh the entry, or `tome doctor` for full diagnosis.
```

## Issues closed

- **#500 / HARD-14** — backup test flake fix (gpg signing disabled per-test-repo + production-path tests use `GIT_CONFIG_GLOBAL` isolation)
- **#501 / HARD-15** — wizard.rs `eprintln!` discipline
- **#502 / HARD-16** — `provenance_from_link_result` → `warn_if_unreadable_symlink` rename
- **#416 / HARD-18** — cross-fs cleanup recovery hint
- **#430 / HARD-19** — reassign read-once filesystem state via `PreReassignState`
- **#433 / HARD-20** — manifest epoch-0 timestamp warning

## Decisions Made

See frontmatter `key-decisions`. Notable:

- **HARD-14 production-vs-test scoping:** the `backup_list_shows_history` integration test exercises the production `backup::init()` path, which performs real `git commit` subprocesses. Rather than disable signing in production (a behaviour change), we isolate the subprocess from the user's global git config via `GIT_CONFIG_GLOBAL=/dev/null`. Production users who add a remote and push still control signing on their own subsequent operations.
- **HARD-15 stdout/stderr split:** the wizard's interactive chrome moves to stderr, but the dry-run TOML body stays on stdout. This preserves `tome init --dry-run > tome.toml` as a documented pipe-safe usage. The `lib.rs::cmd_init` `resolved tome_home:` info line moved alongside (HARD-15 by traceability, even though the line lives outside `wizard.rs`).
- **HARD-19 implementation depth:** the snapshot only consumes `manifest_entry_at_plan` in `execute()` today (single dotted-access; rustfmt breaks it across lines). The other three fields (`target_existed_at_plan`, `source_hash_at_plan`, `target_hash_at_plan`) are forensic captures kept as `#[allow(dead_code)]` for future consumers and unit tests. `manifest.insert(planned_entry)` replaces the previous `update_source_name` + `skills_get_mut` chain — atomic from the caller's perspective.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Production-init gpg flake (CLI integration test)**
- **Found during:** Task 3 verification (a single full-suite run flaked on `backup_list_shows_history`)
- **Issue:** the cli integration tests invoke the production `backup::init` path, which performs real `git commit` subprocesses. On developer machines with global `commit.gpgsign=true`, those commits inherit the signing requirement and fail intermittently when the gpg agent refuses. HARD-14's plan scope was `backup::tests` only, but the same root cause affected the cli tests because they shell out to the production binary.
- **Fix:** added an `isolate_git_config(cmd, tmp)` helper to `cli_backup.rs` that sets `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` to non-existent paths in the temp dir, detaching the subprocess from the user's global git config. No production code change.
- **Files modified:** `crates/tome/tests/cli_backup.rs`
- **Verification:** `cargo test -p tome --test cli_backup` × 5 → all pass; `cargo test -p tome` × 3 full-suite runs → all 955 tests pass each time.
- **Committed in:** `8bb322d` (Task 3 commit)

**2. [Rule 3 - Blocking] cli_init.rs integration tests broke from HARD-15 stdout→stderr conversion**
- **Found during:** Task 1 verification (8/18 cli_init tests failed because they grep'd `stdout` for chrome strings that are now on stderr)
- **Issue:** the wizard chrome used to live on stdout, and the integration tests asserted on `stdout.contains("Generated config:")` etc. After HARD-15, those strings are on stderr.
- **Fix:** systematically migrated 12 `stdout` → `stderr` checks in `cli_init.rs`; rewrote `parse_generated_config` to treat all of stdout as TOML body (since chrome no longer surrounds it on stdout). Refactored two intermediate-style assertions to terse one-liners (rustfmt later collapsed them further).
- **Files modified:** `crates/tome/tests/cli_init.rs`
- **Verification:** `cargo test -p tome --test cli_init` → 18/18 pass.
- **Committed in:** `4d91993` (Task 1 commit, atomically with the wizard.rs change)

**3. [Rule 3 - Blocking] `Manifest::update_source_name` became dead code after HARD-19**
- **Found during:** Task 2 (HARD-19 GREEN phase)
- **Issue:** with `execute()` now re-anchoring via `manifest.insert(planned_entry)` directly, the only callers of `update_source_name` are unit tests + the new HARD-19 drift-test. `cargo clippy --all-targets -- -D warnings` errored with "method `update_source_name` is never used".
- **Fix:** added `#[allow(dead_code)]` to the method with an explanatory doc comment noting it's preserved as public API for future hand-edit tooling.
- **Files modified:** `crates/tome/src/manifest.rs`
- **Verification:** `cargo clippy -p tome --all-targets -- -D warnings` exits 0.
- **Committed in:** `23232e5` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 bug, 2 blocking)
**Impact on plan:** All three were necessary downstream consequences of HARD-14, HARD-15, and HARD-19. No scope creep — each is tightly coupled to its parent requirement.

## Issues Encountered

- One transient flake on `backup_list_shows_history` during a single full-suite run prompted Deviation #1 above. After applying the GIT_CONFIG_GLOBAL isolation, 3 consecutive full-suite runs pass cleanly.
- `make ci` includes a `typos` step that requires the `typos-cli` binary; the developer machine doesn't have it installed (`make: typos: No such file or directory`). All other CI steps (`fmt-check`, `lint`, `test`) pass clean. Not introduced by this plan.

## Phase 15 final status

This is the **final plan of Phase 15** (the v0.10 beta cut). After this plan:

- All 22 HARD-* requirements (HARD-01..22) are validated.
- Test count: **955** total (774 unit + 181 integration). +11 new tests this plan.
- `make fmt-check && make lint && make test` all pass clean.
- The phase is ready for `/gsd:verify-work 15-cli-hardening`.

## Next Phase Readiness

- **Phase 16 (Cleanup-message UX + docs)** — UX-01 (3-bucket partition for stale skills) intersects with HARD-09 territory but is independent. DOC-01..03 (CHANGELOG.md v0.10 release notes, architecture docs, cross-machine sync docs) consume the BREAKING refactor history from Phase 15.
- **Phase 17 (Migration polish + UAT + release)** — REL-01..05 will fold in the carry-over Linux UAT items and the cargo-dist release.
- No blockers introduced by Phase 15.

## Self-Check: PASSED

All 10 modified files exist on disk; all 3 task commits are present in git log:

- 4d91993 — Task 1 (HARD-14 + HARD-15 + HARD-16)
- 23232e5 — Task 2 (HARD-18 + HARD-19, TDD)
- 8bb322d — Task 3 (HARD-20, TDD + HARD-14 cli-test follow-up)

---
*Phase: 15-cli-hardening*
*Completed: 2026-05-08*

---
phase: 15-cli-hardening
plan: 04
type: execute
wave: 2
depends_on:
  - 15-01
  - 15-02
  - 15-03
files_modified:
  - crates/tome/src/lint.rs
  - crates/tome/src/main.rs
  - crates/tome/src/lib.rs
  - crates/tome/src/distribute.rs
  - crates/tome/src/doctor.rs
  - crates/tome/src/manifest.rs
  - crates/tome/src/lockfile.rs
  - crates/tome/src/machine.rs
  - crates/tome/src/config/mod.rs
  - crates/tome/tests/cli_remove.rs
  - crates/tome/tests/cli_overrides.rs
  - crates/tome/tests/cli_sync.rs
  - crates/tome/tests/common/mod.rs
autonomous: true
requirements:
  - HARD-04
  - HARD-08
  - HARD-09
  - HARD-10
  - HARD-11
must_haves:
  truths:
    - "tome lint failures bubble through anyhow as a downcastable LintFailed error and main.rs maps it to exit code 1; no process::exit(1) inside lib.rs"
    - "Manifest::save, Lockfile::save, MachinePrefs::save, Config::save_checked all preserve the previous on-disk file contents on rename failure"
    - "distribute warns-and-skips foreign symlinks pointing outside the current library_dir; --force bypasses; doctor surfaces ForeignSymlink as a Warning"
    - "Hostile-input integration tests cover .. traversal, symlink loop, and same-target overrides for [directory_overrides.<name>]"
    - "tome remove dir <git-dir> and tome remove dir <claude-plugins-dir> end-to-end integration tests pass"
  artifacts:
    - path: "crates/tome/src/lint.rs"
      provides: "LintFailed error type"
      contains: "LintFailed"
    - path: "crates/tome/src/main.rs"
      provides: "downcast-to-LintFailed exit-code-1 mapping"
      contains: "downcast"
    - path: "crates/tome/src/distribute.rs"
      provides: "Foreign-symlink warn-and-skip logic in lines 110-128 block"
      contains: "foreign symlink"
    - path: "crates/tome/src/doctor.rs"
      provides: "DiagnosticIssue::ForeignSymlink variant + ALL-array entry + sentinel"
      contains: "ForeignSymlink"
    - path: "crates/tome/tests/cli_remove.rs"
      provides: "tome remove dir <git-dir> + <claude-plugins-dir> e2e tests"
    - path: "crates/tome/tests/cli_overrides.rs"
      provides: "Hostile-input tests for [directory_overrides.<name>]"
  key_links:
    - from: "crates/tome/src/main.rs"
      to: "crates/tome/src/lint.rs::LintFailed"
      via: "anyhow::Error::downcast_ref"
      pattern: "downcast_ref::<LintFailed>"
    - from: "crates/tome/src/distribute.rs"
      to: "ForeignSymlink doctor variant"
      via: "warn-and-skip path; doctor surfaces persistently"
      pattern: "ForeignSymlink"
---

<objective>
Land the safety + integration-test cluster: `LintFailed` error type replacing `process::exit(1)` (HARD-04, closes #488); atomic-save preservation regression (HARD-08, closes #494); `distribute` foreign-symlink refuse-or-force-clobber per D-DIST-1/-2 (HARD-09, closes #495); hostile-input tests for `[directory_overrides]` (HARD-10, closes #496); `tome remove dir` e2e tests (HARD-11, closes #497).

Purpose: Ship the Phase 15 safety bar — process::exit removed, save-rename-failure preserves data, foreign symlinks don't get clobbered, override paths can't escape sandboxing, remove-dir flow is regression-tested for both git and claude-plugins.
Output: New `LintFailed` error + `main.rs` downcast handler; foreign-symlink detection in `distribute.rs:110-128`; `DiagnosticIssue::ForeignSymlink` variant; `tests/cli_overrides.rs` (new); HARD-11 tests in `tests/cli_remove.rs`; atomic-save regression tests across all four save() callsites.
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
@.planning/phases/15-cli-hardening/15-CONTEXT.md
@.planning/phases/15-cli-hardening/15-01-cli-decomposition-PLAN.md

@crates/tome/src/lint.rs
@crates/tome/src/main.rs
@crates/tome/src/lib.rs
@crates/tome/src/distribute.rs
@crates/tome/src/doctor.rs
@crates/tome/src/manifest.rs
@crates/tome/src/lockfile.rs
@crates/tome/src/machine.rs
@crates/tome/src/remove.rs

<interfaces>
Existing surfaces this plan extends:

From crates/tome/src/lib.rs (line 394, current shape — inside Command::Lint dispatch arm or post-15-01 cmd_lint helper):
  if !lint_passes { process::exit(1); }   // HARD-04 target

From crates/tome/src/distribute.rs (lines 110-128, current symlink handling):
  Existing block handles already-symlinked targets — extend it for foreign-library
  symlinks. force: bool at line 111 is REUSED, no new flag.

From crates/tome/src/doctor.rs:
  pub enum DiagnosticIssue { OrphanLibraryEntry { ... }, BrokenSymlink { ... }, ... }
  pub enum IssueSeverity { Warning, Error, ... }
  // Extend with ForeignSymlink { target_path: PathBuf, actual_target: PathBuf }
  // Update DiagnosticIssue::ALL array + POLISH-04 exhaustiveness sentinel.

From manifest.rs / lockfile.rs / machine.rs / config/mod.rs:
  Each has save() / save_checked() using atomic temp+rename:
    1. write to PATH.tmp.RAND
    2. fs::rename(tmp, PATH)
  HARD-08 regression: if step 2 fails, the original file content must remain unchanged.

D-DIST-1 warning text (verbatim — copy into Task 3 action):
  warning: ~/.claude/skills/foo is a foreign symlink
           (→ /Users/martin/other-tome/library/foo); skipping.
           Pass --force to overwrite, or remove manually.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: HARD-04 LintFailed error + main.rs exit code mapping</name>
  <files>crates/tome/src/lint.rs, crates/tome/src/lib.rs, crates/tome/src/main.rs</files>
  <read_first>
    - crates/tome/src/lint.rs (existing error types — sibling shape for LintFailed)
    - crates/tome/src/lib.rs (Command::Lint dispatch — process::exit(1) lives at ~line 394; post-15-01 may be in a cmd_lint helper)
    - crates/tome/src/main.rs (top-level error handler — extend the existing downcast logic)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-04 LintFailed placement" (Claude's Discretion: inline in lint.rs)
    - .planning/REQUIREMENTS.md section "HARD-04"
  </read_first>
  <behavior>
    LintFailed error + exit-code-1 mapping:
    - Test (unit): LintFailed { violations: vec![...] } constructs cleanly; Display impl includes the violation count.
    - Test (unit): anyhow::Error::from(LintFailed { ... }) is downcast-able back to &LintFailed via downcast_ref::<LintFailed>().
    - Test (integration): tome lint against a fixture with a known frontmatter violation exits with code 1 (NOT via process::exit, but via main.rs's anyhow handler downcasting).
    - Test (integration): tome lint on a clean fixture exits 0.
    - Test (regression): grep -rE "process::exit\(1\)" crates/tome/src/lib.rs returns NOTHING.
  </behavior>
  <action>
    Per CONTEXT.md "Claude's Discretion": **inline LintFailed in lint.rs** as a sibling error type — no new errors.rs module.

    **Step A: Add LintFailed to crates/tome/src/lint.rs.**

    ```rust
    /// Lint command bubbles this error when validation produces violations.
    /// main.rs downcasts and maps to exit code 1.
    #[derive(Debug)]
    pub struct LintFailed {
        pub violations: usize,
    }

    impl std::fmt::Display for LintFailed {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "lint failed: {} violation(s)", self.violations)
        }
    }

    impl std::error::Error for LintFailed {}
    ```

    **Step B: Replace process::exit(1) in lib.rs (~line 394, post-15-01 in cmd_lint).**

    Read the current Lint dispatch to confirm the violation-count source. Replace:

    ```rust
    if !lint_passes { process::exit(1); }
    ```

    with:

    ```rust
    if violations > 0 {
        anyhow::bail!(LintFailed { violations });
    }
    ```

    The bail produces an anyhow::Error carrying the LintFailed underlying type, which main.rs will downcast.

    **Step C: Extend crates/tome/src/main.rs top-level error handler.**

    Current shape (read first):

    ```rust
    fn main() {
        match tome::run(...) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("error: {e:#}");
                process::exit(1);
            }
        }
    }
    ```

    Extend to:

    ```rust
    fn main() {
        match tome::run(...) {
            Ok(()) => {}
            Err(e) => {
                if let Some(lint_failed) = e.downcast_ref::<tome::lint::LintFailed>() {
                    eprintln!("error: {}", lint_failed);
                    process::exit(1);
                }
                eprintln!("error: {e:#}");
                process::exit(1);
            }
        }
    }
    ```

    Both branches return exit code 1, so the downcast is mostly defensive — but it gives Phase 16/17 a hook point to differentiate exit codes by error type if needed (matches HARD-04 spec literally).

    **Step D: Add unit tests in lint.rs::tests** covering construction and downcast (per behavior section). Add an integration test in tests/cli_init.rs (or wherever lint tests live post-15-01) asserting `tome lint BAD-FIXTURE` exits 1.
  </action>
  <verify>
    <automated>cargo test -p tome lint::tests; cargo build -p tome; cargo clippy --all-targets -- -D warnings; if grep -E "process::exit\(1\)" crates/tome/src/lib.rs; then exit 1; fi</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E "pub struct LintFailed" crates/tome/src/lint.rs` returns one match.
    - `grep -E "impl std::error::Error for LintFailed" crates/tome/src/lint.rs` returns one match.
    - `grep -E "process::exit\\(1\\)" crates/tome/src/lib.rs` returns NOTHING.
    - `grep -E "anyhow::bail!\\(LintFailed" crates/tome/src/lib.rs` returns at least one match.
    - `grep -E "downcast_ref::<.*LintFailed>" crates/tome/src/main.rs` returns one match.
    - At least one new unit test verifies anyhow downcast: `cargo test -p tome lint::tests::lint_failed_downcast` (or analogous test name) passes.
    - At least one new integration test verifies `tome lint BAD-FIXTURE` exits 1 without process::exit (binary-level check via assert_cmd).
    - `cargo build -p tome` exits 0; `cargo clippy --all-targets -- -D warnings` exits 0.
  </acceptance_criteria>
  <done>
    LintFailed exists as a downcastable error in lint.rs; lib.rs uses anyhow::bail!(LintFailed { ... }) instead of process::exit(1); main.rs downcasts and exits 1. Tests verify construction, downcast, and binary-level exit code.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: HARD-08 atomic-save preservation regression tests</name>
  <files>crates/tome/src/manifest.rs, crates/tome/src/lockfile.rs, crates/tome/src/machine.rs, crates/tome/src/config/mod.rs (or types.rs)</files>
  <read_first>
    - crates/tome/src/manifest.rs (Manifest::save — temp+rename atomic write)
    - crates/tome/src/lockfile.rs (Lockfile::save — same pattern)
    - crates/tome/src/machine.rs (MachinePrefs::save — same pattern)
    - crates/tome/src/config/mod.rs or types.rs post-15-02 (Config::save_checked — same pattern)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-08 atomic-save test mechanism" (Claude's Discretion: real fs)
    - .planning/REQUIREMENTS.md section "HARD-08"
    - .planning/phases/13-lockfile-authoritative-sync/13-CONTEXT.md (Phase 13 D-22 lockfile per-skill in-memory updates with atomic end-of-loop save — round-trip invariant this test verifies)
  </read_first>
  <behavior>
    Each of the four save() implementations preserves previous on-disk content when fs::rename fails:
    - Test: write a Manifest with content A. Trigger Manifest::save with content B that fails the rename step. Re-read the file from disk. Assert content == A (NOT B, NOT empty, NOT corrupt).
    - Test: same for Lockfile::save (verifies Phase 13 D-22 atomic end-of-loop save invariant).
    - Test: same for MachinePrefs::save.
    - Test: same for Config::save_checked.
    - Test (idempotency / forward-compat): Phase 14 added previous_source: Option<DirectoryName>; round-trip a manifest containing Some(name) and None; assert serde-default skip_serializing_if behaves correctly.
  </behavior>
  <action>
    Per CONTEXT.md "Claude's Discretion": **use real fs (matches existing test style; mock layers add infrastructure for one test).**

    Mechanism: induce fs::rename failure via permission-denied target directory, OR via a target path that is itself a directory (rename file -> dir fails on most platforms), OR via a target on a different filesystem (rename across filesystems fails on Linux). Pick whichever yields a portable test (macOS + Linux). The most reliable cross-platform trick is:
    1. Create a tempdir.
    2. Write file A to PATH.
    3. Create a subdirectory with the same name as the temp file Manifest::save would write to during temp+rename. Rename(file, dir-with-same-name) fails on POSIX.
    4. OR: chmod 0o500 (read+execute, no write) on the parent directory after writing A; rename will fail with EACCES.

    Recommended: chmod approach — simplest, single fixture, clean teardown.

    **Step A: write 4 regression tests** — one per save() impl. Co-locate each test with its module:

    - crates/tome/src/manifest.rs::tests::save_preserves_previous_on_rename_failure
    - crates/tome/src/lockfile.rs::tests::save_preserves_previous_on_rename_failure
    - crates/tome/src/machine.rs::tests::save_preserves_previous_on_rename_failure
    - crates/tome/src/config/mod.rs::tests::save_checked_preserves_previous_on_rename_failure
      (or wherever Config::save_checked landed in 15-02 — likely mod.rs or types.rs)

    Each test:
    1. tempdir with restored permissions on Drop (use tempfile::TempDir).
    2. Write content A to the target file (use the *::save / *::save_checked happy path to construct content A).
    3. Make the parent directory non-writable (chmod 0o500 on Unix; std::fs::set_permissions).
    4. Attempt save with content B; assert .is_err() OR exits non-zero.
    5. Restore permissions; re-read the target file.
    6. Assert read content == A. Hash both via crate::manifest::hash_directory or .as_bytes() comparison.

    Phase 14 D-C1 round-trip pin: ALSO add a test for each save() variant that round-trips an instance carrying previous_source = Some(name). Not strictly part of HARD-08 wording but explicitly called out in CONTEXT.md "Carried forward from prior phases" as the round-trip invariant HARD-08 should pin.

    **Step B: ensure existing save() impls actually preserve content on rename failure.**

    Read each save() impl. The temp+rename pattern is:
    ```rust
    let tmp = path.with_extension("tmp.RAND");
    fs::write(&tmp, serialised)?;
    fs::rename(&tmp, path)?;  // <-- if this fails, original `path` is untouched (good)
    Ok(())
    ```

    The pattern naturally preserves original content on rename failure (the original was never overwritten; only the .tmp file was written, and then the rename failed before swapping). The regression test pins this invariant against future regressions (e.g. a refactor that switches to fs::write directly and loses atomicity).

    If reading the existing impl reveals NON-atomic behaviour (e.g. fs::write over the target without temp+rename), this is a bug — fix it to use temp+rename FIRST, then add the regression test. This unlikely scenario is what HARD-08 is guarding against.
  </action>
  <verify>
    <automated>cargo test -p tome -- save_preserves_previous_on_rename_failure save_checked_preserves_previous_on_rename_failure</automated>
  </verify>
  <acceptance_criteria>
    - 4 new tests exist (one per save() implementation), each named `*save*preserves_previous_on_rename_failure*` or close variant.
    - `rg "save_preserves_previous_on_rename_failure" crates/tome/src` returns ≥4 matches across manifest.rs, lockfile.rs, machine.rs, config/.
    - Each test uses real fs (tempfile::TempDir + permission-flip), not mock fs (`grep -rE "MockFs|FaultInjection" crates/tome/src` returns NOTHING in test code).
    - All 4 tests pass: `cargo test -p tome -- save_preserves_previous_on_rename_failure` exits 0.
    - At least 4 round-trip tests pin Phase 14 previous_source field through serialisation: `grep -rE "previous_source.*round_trip" crates/tome/src` returns ≥4 matches (manifest, lockfile, save, load).
    - `cargo build -p tome` exits 0; `cargo clippy --all-targets -- -D warnings` exits 0.
  </acceptance_criteria>
  <done>
    All four save() impls have a regression test asserting on-disk content preservation across rename failure. Phase 14 previous_source round-trips through every save/load. Tests use real fs, not mocks.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: HARD-09 distribute foreign-symlink refuse + doctor surface (D-DIST-1, D-DIST-2)</name>
  <files>crates/tome/src/distribute.rs, crates/tome/src/doctor.rs, crates/tome/tests/cli_sync.rs</files>
  <read_first>
    - crates/tome/src/distribute.rs lines 110-128 (current symlink-handling block); also line 111 (existing `force: bool` parameter — REUSE it, do NOT add a new flag)
    - crates/tome/src/distribute.rs (existing `symlink_points_to` helper — the foreign check is its negation)
    - crates/tome/src/doctor.rs (DiagnosticIssue enum + IssueSeverity + ALL array + exhaustiveness sentinel)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-09 distribute clobber policy" "D-DIST-1 (warn-and-skip foreign symlinks)" "D-DIST-2 (doctor surfaces too)"
    - .planning/REQUIREMENTS.md section "HARD-09"
    - .planning/phases/10-phase-8-review-tail/10-CONTEXT.md (POLISH-04 ALL-array sentinel pattern)
  </read_first>
  <behavior>
    distribute foreign-symlink detection:
    - Test: target path is a symlink pointing INTO the current library_dir → existing behaviour (recreate or no-op based on existing logic, regression-pin).
    - Test: target path is a symlink pointing OUTSIDE library_dir (e.g. /Users/martin/other-tome/library/foo) → warn-and-skip; result.skipped is incremented; symlink is unchanged on disk.
    - Test: same as above but with force=true → clobber; new symlink replaces the foreign one.
    - Test: target path is a regular file (not a symlink) → existing warn-and-skip behaviour preserved (regression-pin).
    - Test: target path doesn't exist → existing happy-path (create symlink) preserved.

    doctor::DiagnosticIssue::ForeignSymlink:
    - Test: doctor against a tome install with a foreign symlink in a target directory surfaces ForeignSymlink in the report.
    - Test: ForeignSymlink renders with Warning severity (NOT Error).
    - Test: ForeignSymlink contributes to total_issues() count (per CONTEXT.md D-DIST-2).
    - Test: DiagnosticIssue::ALL array includes ForeignSymlink; POLISH-04 exhaustiveness sentinel matches.
    - Test (JSON shape): `tome doctor --format json` emits the new variant in valid JSON shape.
  </behavior>
  <action>
    **Step A: Extend crates/tome/src/distribute.rs lines 110-128 symlink-handling block (D-DIST-1).**

    Current block handles already-symlinked targets. Extend it to detect foreign library symlinks:

    ```rust
    // Pseudo-Rust extending the existing block. Adapt to the actual code shape.
    if target.is_symlink() {
        let actual_target = fs::read_link(&target_path)?;
        let canonical_actual = fs::canonicalize(&actual_target).ok();
        let canonical_library = fs::canonicalize(&paths.library_dir).ok();

        let points_to_current_library = match (&canonical_actual, &canonical_library) {
            (Some(actual), Some(lib)) => actual.starts_with(lib),
            _ => false,
        };

        if !points_to_current_library && !force {
            // D-DIST-1: foreign symlink, warn-and-skip
            eprintln!(
                "warning: {} is a foreign symlink\n         (→ {}); skipping.\n         Pass --force to overwrite, or remove manually.",
                target_path.display(),
                actual_target.display(),
            );
            result.skipped += 1;
            continue;
        }
        // else: force=true OR points-to-current-library → existing behaviour
        //       (recreate stale symlink if needed)
    }
    ```

    Warning text VERBATIM from CONTEXT.md D-DIST-1:

    ```
    warning: ~/.claude/skills/foo is a foreign symlink
             (→ /Users/martin/other-tome/library/foo); skipping.
             Pass --force to overwrite, or remove manually.
    ```

    (Path strings differ per case; the SHAPE of the message must match: `<target> is a foreign symlink (→ <actual>); skipping. Pass --force to overwrite, or remove manually.`)

    Per CONTEXT.md "Claude's Discretion": **use std::fs::canonicalize** to handle symlinks-in-the-middle correctly (NOT lexical normalisation).

    Reuse the existing `force: bool` parameter at distribute.rs:111 — do NOT introduce a new CLI flag. force=true bypasses the foreign-symlink check (opt-in clobber). Semantically consistent with force's existing "overwrite stale symlinks" meaning.

    **Step B: Add DiagnosticIssue::ForeignSymlink to crates/tome/src/doctor.rs (D-DIST-2).**

    ```rust
    pub enum DiagnosticIssue {
        // ... existing variants
        ForeignSymlink {
            target_path: PathBuf,
            actual_target: PathBuf,
        },
    }
    ```

    Update IssueSeverity dispatch — ForeignSymlink renders as IssueSeverity::Warning (per CONTEXT.md "Renders as IssueSeverity::Warning; contributes to total_issues").

    Update DiagnosticIssue::ALL array (POLISH-04 pattern) to include ForeignSymlink. Update the compile-time exhaustiveness sentinel match.

    Doctor scanning: extend the directory-issue detection pass to detect foreign symlinks via the same canonicalize-and-compare logic from Step A. Place in `directory_issues` (per CONTEXT.md "likely in directory_issues since it's a per-directory health concern, but planner can place it elsewhere if the data shape doesn't fit").

    Render: per the existing tabled output convention, ForeignSymlink renders one row per occurrence with the target_path and actual_target columns. JSON shape extends the existing DiagnosticIssue serde representation.

    **Step C: Tests.**

    - distribute.rs unit tests covering the 5 cases in <behavior>.
    - doctor.rs unit tests covering the 5 cases in <behavior> for ForeignSymlink.
    - One integration test in tests/cli_sync.rs (post-15-01) that:
      1. Creates a tempdir with a target dir containing a pre-existing symlink pointing OUTSIDE the library.
      2. Runs `tome sync` → asserts stderr contains the foreign-symlink warning text and result.skipped is incremented.
      3. Re-runs with `--force` → asserts the symlink is clobbered.
  </action>
  <verify>
    <automated>cargo test -p tome distribute::tests; cargo test -p tome doctor::tests; cargo test -p tome --test cli_sync; cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E "is a foreign symlink" crates/tome/src/distribute.rs` returns one match (the warning text).
    - `grep -E "fs::canonicalize" crates/tome/src/distribute.rs` returns at least one match.
    - `grep -E "ForeignSymlink" crates/tome/src/doctor.rs` returns at least 3 matches (variant decl, ALL-array entry, sentinel match arm).
    - DiagnosticIssue::ALL array includes ForeignSymlink: `grep -A20 "DiagnosticIssue::ALL" crates/tome/src/doctor.rs | grep -E "ForeignSymlink"` returns at least one match.
    - At least 5 new distribute tests covering all branches of `<behavior>`.
    - At least 4 new doctor tests covering the new variant.
    - At least one integration test in tests/cli_sync.rs runs end-to-end with foreign-symlink fixture; assert_cmd verifies stderr contains "is a foreign symlink".
    - `tome doctor --format json` JSON output includes the new variant when triggered (manual or scripted check).
    - No new CLI flag added: `git diff crates/tome/src/cli.rs` shows zero new flags for HARD-09.
    - `cargo build -p tome` exits 0; `cargo clippy --all-targets -- -D warnings` exits 0.
  </acceptance_criteria>
  <done>
    distribute warns-and-skips foreign symlinks pointing outside library_dir; --force bypasses; doctor surfaces ForeignSymlink as Warning with target/actual paths and contributes to total_issues. POLISH-04 ALL-array + sentinel updated.
  </done>
</task>

<task type="auto">
  <name>Task 4: HARD-10 directory_overrides hostile-input integration tests</name>
  <files>crates/tome/tests/cli_overrides.rs, crates/tome/tests/common/mod.rs</files>
  <read_first>
    - crates/tome/src/config.rs (post-15-02: config/overrides.rs — apply_machine_overrides logic)
    - crates/tome/src/machine.rs (DirectoryOverride struct, MachinePrefs schema)
    - .planning/phases/09-cross-machine-path-overrides/09-CONTEXT.md (PORT-01..05 schema)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-10"
    - .planning/REQUIREMENTS.md section "HARD-10"
    - crates/tome/tests/cli_sync_reconcile.rs (already-split file — match its style for new cli_overrides.rs)
  </read_first>
  <action>
    Create new integration test file crates/tome/tests/cli_overrides.rs covering hostile-input cases for `[directory_overrides.<name>]`. The file is created by Plan 15-04 (NOT Plan 15-01, because the hostile-input scenarios are Phase 15 content — Plan 15-01 only redistributes existing tests).

    Test cases (each as a separate #[test] fn):

    1. **Override path with `..` traversal**: machine.toml contains `[directory_overrides.foo] path = "../../../etc"`. Run `tome sync`. Expected: clear error message naming the offending directory; non-zero exit; library is NOT touched. The Phase 9 PORT-04 typo-warning + named-machine.toml error class applies — error message specifies `machine.toml` (NOT `tome.toml`) per Phase 9 PORT-04.

    2. **Symlink loop in override path**: create a tempdir with two symlinks (a -> b, b -> a). Override `[directory_overrides.foo] path = ".../a"`. Run `tome sync`. Expected: clear error (likely from canonicalize failing or a max-depth check); non-zero exit; library NOT touched.

    3. **Two directories overriding to the same path**: machine.toml contains both `[directory_overrides.foo] path = "/tmp/shared"` AND `[directory_overrides.bar] path = "/tmp/shared"`. Run `tome sync`. Expected: clear error naming the duplicate path; both directory names enumerated; non-zero exit.

    Each test uses assert_cmd::Command::cargo_bin("tome"), tempfile::TempDir for filesystem isolation, and the shared common::* helpers from tests/common/mod.rs (created in 15-01).

    Test naming: prefix all 3 with `cli_overrides_hostile_*` so they group cleanly under `cargo test cli_overrides_hostile_`:
    - `cli_overrides_hostile_dotdot_traversal_rejected`
    - `cli_overrides_hostile_symlink_loop_rejected`
    - `cli_overrides_hostile_duplicate_target_rejected`

    Each test asserts on stderr text — pin the specific error wording so future refactors can't silently change it (the Phase 7 D-10 Conflict / Why / Suggestion template should be present).
  </action>
  <verify>
    <automated>cargo test -p tome --test cli_overrides</automated>
  </verify>
  <acceptance_criteria>
    - File `crates/tome/tests/cli_overrides.rs` exists.
    - 3 #[test] fns exist matching pattern `cli_overrides_hostile_*`: `grep -cE "^#\\[test\\]" crates/tome/tests/cli_overrides.rs` returns ≥3.
    - All 3 tests pass: `cargo test -p tome --test cli_overrides` exits 0.
    - Each test asserts non-zero exit code via assert_cmd's .failure() or .code(...).
    - Each test asserts stderr contains the offending directory name (verify via `grep -A5 "stderr"` in test source).
    - Tests use tempfile::TempDir (no /tmp pollution): `grep -E "TempDir" crates/tome/tests/cli_overrides.rs` returns ≥3 matches (one per test).
    - Tests reference `mod common;`: `grep -E "^mod common" crates/tome/tests/cli_overrides.rs` returns one match.
  </acceptance_criteria>
  <done>
    tests/cli_overrides.rs has 3 hostile-input integration tests covering `..` traversal, symlink loops, and duplicate target paths in `[directory_overrides.<name>]`. All pass; non-zero exits asserted; error messages pinned.
  </done>
</task>

<task type="auto">
  <name>Task 5: HARD-11 tome remove dir e2e integration tests (git + claude-plugins)</name>
  <files>crates/tome/tests/cli_remove.rs, crates/tome/tests/common/mod.rs</files>
  <read_first>
    - crates/tome/src/remove.rs (RemovePlan, RemoveFailureKind — existing infrastructure)
    - crates/tome/src/git.rs (git directory clone path: ~/.tome/repos/SHA/)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-API-2: post-merge `tome remove dir <name>` shape — this is the tested shape)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-11"
    - .planning/REQUIREMENTS.md section "HARD-11"
    - crates/tome/tests/cli_remove.rs (created in 15-01 with redistributed Phase 14 tests; HARD-11 ADDS to it, doesn't replace)
  </read_first>
  <action>
    **S7 PREFLIGHT — Phase 14 Owned→Unowned transition dependency check.**

    Both HARD-11 tests assert that manifest entries from the removed directory transition to `source_name = None` (Unowned) per Phase 14 D-API-2 / Phase 11 LIB-04. Confirm that transition exists in the codebase BEFORE writing the tests:

    ```
    rg -n 'source_name.*= ?None' crates/tome/src/remove.rs
    ```

    - **If ≥1 match:** Phase 14 shipped the Owned→Unowned transition; proceed with the tests below.
    - **If 0 matches:** HARD-11 is BLOCKED on a Phase 14 follow-up (the transition is not implemented). DO NOT write speculative tests against an absent transition. Instead:
      1. Surface this in `.planning/phases/15-cli-hardening/15-deferred-items.md` per D-PLAN-2 (strict beta-cut scope; new issues go to deferred-items, not into this phase).
      2. Record the deferred-items entry with: gap = "remove.rs lacks Owned→Unowned source_name=None transition required by HARD-11 acceptance criteria"; suggested triage = Phase 16 / Phase 17 / backlog.
      3. STOP Task 5. Continue with the rest of the plan; HARD-11 closes when the Phase 14 transition is added.

    Do not pad acceptance criteria around an absent transition — the test would fail or give false-positive coverage.

    ---

    Add two end-to-end integration tests for `tome remove dir <name>` covering the two directory types that have non-trivial cleanup behaviour: git directories (cleans `~/.tome/repos/<sha>/` cache) and claude-plugins directories (cleans installed-plugins-side symlinks).

    **Test 1: tome_remove_dir_cleans_git_cache**

    1. Create a tempdir as the test's $HOME-equivalent (override TOME_HOME env or use --tome-home flag).
    2. Set up a synthetic git directory in tome.toml: `[directories.test-git] type = "git", url = "<file-protocol-bare-repo-url>"`. Use a local bare repo created via `git init --bare` inside the tempdir to avoid network dependency.
    3. Run `tome sync` to populate the library + git cache (`~/.tome/repos/<sha>/`).
    4. Assert the git cache directory exists.
    5. Run `tome remove dir test-git` (the Phase 14 D-API-2 post-merge shape).
    6. Assert: tome.toml no longer has `[directories.test-git]`; git cache directory is removed; library entries originally sourced from test-git transition to Unowned (per Phase 11 LIB-04 / D-10 hybrid trigger); manifest no longer has source_name = Some("test-git") for those entries (it's now None per Phase 11 D-10).
    7. Assert exit code 0.

    **Test 2: tome_remove_dir_cleans_claude_plugins**

    1. Create a tempdir; populate a synthetic ~/.config/claude/installed_plugins.json with one plugin entry.
    2. Set up tome.toml with `[directories.test-cp] type = "claude-plugins"`.
    3. Run `tome sync` — populates library entries via the ClaudePlugins discovery path (Phase 11 made these real-dir copies).
    4. Run `tome remove dir test-cp`.
    5. Assert: tome.toml no longer has `[directories.test-cp]`; downstream distribution symlinks pointing at library entries from test-cp are removed; library entries transition to Unowned.
    6. Assert exit code 0.

    Both tests use the existing `RemovePlan` plan/render/execute infrastructure end-to-end via assert_cmd. Use tests/common/mod.rs helpers for the synthetic-fixture setup (introduced in 15-01).

    Per CONTEXT.md `<canonical_refs>` "Patterns to follow": existing `RemovePlan` / `RemoveFailureKind` infrastructure is reused — no production code changes are required for HARD-11; this is integration-test-only.

    Note: HARD-11 tests land in `tests/cli_remove.rs` (created in 15-01 — confirm it exists before this task starts). If 15-01 split also moved Phase 14 remove tests to cli_remove.rs, HARD-11 tests sit alongside them.
  </action>
  <verify>
    <automated>cargo test -p tome --test cli_remove tome_remove_dir_cleans</automated>
  </verify>
  <acceptance_criteria>
    - **S7 preflight passed before tests written:** `rg -n 'source_name.*= ?None' crates/tome/src/remove.rs` returned ≥1 match (Phase 14 D-API-2 Owned→Unowned transition implemented). If 0 matches, this task is recorded in `15-deferred-items.md` and skipped per D-PLAN-2.
    - `crates/tome/tests/cli_remove.rs` contains at least 2 new test fns matching `tome_remove_dir_cleans_*`: `grep -cE "fn tome_remove_dir_cleans" crates/tome/tests/cli_remove.rs` returns ≥2.
    - Both tests pass: `cargo test -p tome --test cli_remove tome_remove_dir_cleans` exits 0.
    - Each test exercises the binary via assert_cmd::Command::cargo_bin("tome") (NOT a unit-level RemovePlan call): `grep -E "cargo_bin" crates/tome/tests/cli_remove.rs` returns ≥2 matches in the new tests.
    - Each test verifies the post-Phase-14 D-API-2 shape: `grep -E "remove dir" crates/tome/tests/cli_remove.rs` returns ≥2 (NOT bare `tome remove <name>`).
    - Git test verifies cache cleanup: `grep -E "tome/repos/" crates/tome/tests/cli_remove.rs` returns ≥1.
    - Claude-plugins test verifies discovery-side cleanup: `grep -E "installed_plugins" crates/tome/tests/cli_remove.rs` returns ≥1.
    - Both tests verify Unowned transition (Phase 11 LIB-04): manifest entries transition to source_name = None.
  </acceptance_criteria>
  <done>
    Two new e2e integration tests in tests/cli_remove.rs cover `tome remove dir <git-dir>` and `tome remove dir <claude-plugins-dir>`. Both verify config + cache + library + manifest state post-removal. Both pass.
  </done>
</task>

</tasks>

<verification>
- `cargo build -p tome` exits 0
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo test -p tome` passes; new tests added (target: ≥18 new tests across all 5 sub-tasks)
- `grep -rE "process::exit\(1\)" crates/tome/src/lib.rs` returns 0 results (HARD-04 done)
- All four save() impls have a *_preserves_previous_on_rename_failure regression test (HARD-08 done)
- distribute warns-and-skips foreign symlinks; doctor surfaces ForeignSymlink (HARD-09 done)
- tests/cli_overrides.rs has 3 hostile-input tests (HARD-10 done)
- tests/cli_remove.rs has 2 new e2e tests for git + claude-plugins (HARD-11 done)
</verification>

<success_criteria>
- HARD-04: `process::exit(1)` removed from `lib.rs`; LintFailed downcasts in main.rs; exit code 1 preserved (closes #488)
- HARD-08: Atomic-save preservation regression test exists for manifest, lockfile, machine.toml, tome.toml; previous_source field round-trips through serialization (closes #494)
- HARD-09: distribute warns-and-skips foreign symlinks per D-DIST-1; --force bypasses; doctor surfaces ForeignSymlink per D-DIST-2 with Warning severity contributing to total_issues; POLISH-04 ALL-array + sentinel updated (closes #495)
- HARD-10: tests/cli_overrides.rs has 3 hostile-input tests covering `..` traversal, symlink loops, duplicate target paths (closes #496)
- HARD-11: tests/cli_remove.rs has 2 e2e tests for `tome remove dir <git-dir>` and `tome remove dir <claude-plugins-dir>` (closes #497)
- Test count grows by ≥18 (4 atomic-save + 4 round-trip + 5 distribute + 4 doctor + 3 overrides + 2 remove + 1 lint integration)
</success_criteria>

<output>
After completion, create `.planning/phases/15-cli-hardening/15-04-SUMMARY.md` recording:
- LintFailed type signature + main.rs downcast pattern
- Atomic-save mechanism used (chmod permission flip vs mock)
- Foreign-symlink warning text (verbatim)
- DiagnosticIssue::ForeignSymlink JSON shape
- HARD-10 + HARD-11 test counts in tests/cli_overrides.rs and tests/cli_remove.rs
- Issues closed: #488 (HARD-04), #494 (HARD-08), #495 (HARD-09), #496 (HARD-10), #497 (HARD-11)
</output>

---
phase: 08-safety-refactors-partial-failure-visibility-cross-platform
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/remove.rs
  - crates/tome/src/lib.rs
  - crates/tome/tests/cli.rs
  - CHANGELOG.md
autonomous: true
requirements:
  - SAFE-01
must_haves:
  truths:
    - "User running `tome remove <name>` in a state where some symlinks/dirs cannot be cleaned (permissions, missing files) sees a distinct `⚠ N operations failed` summary with per-item detail and the command exits non-zero — the clean success path remains quiet as before"
    - "`cargo test` covers the new `RemoveResult` aggregation (including a partial-failure case)"
  artifacts:
    - path: "crates/tome/src/remove.rs"
      provides: "FailureKind enum, RemoveFailure struct, RemoveResult.failures field, per-loop push-on-error (no in-loop eprintln)"
      contains: "pub enum FailureKind"
    - path: "crates/tome/src/lib.rs"
      provides: "Command::Remove handler emits grouped `⚠ K operations failed` summary and returns Err on non-empty failures"
      contains: "operations failed"
    - path: "crates/tome/tests/cli.rs"
      provides: "integration test: tome remove under chmod 0o000 produces non-zero exit + ⚠ stderr marker"
      contains: "remove_partial_failure"
  key_links:
    - from: "crates/tome/src/remove.rs execute() four failure loops"
      to: "result.failures.push(RemoveFailure { ... })"
      via: "match arm replacing former eprintln!"
      pattern: "failures\\.push\\(RemoveFailure"
    - from: "crates/tome/src/lib.rs Command::Remove"
      to: "stderr grouped summary + anyhow::anyhow!(\"remove completed with ... failures\")"
      via: "if !result.failures.is_empty() { ... return Err(...) }"
      pattern: "remove completed with .* failures"
---

<objective>
Aggregate partial-cleanup failures in `remove::execute` into a typed `Vec<RemoveFailure>` on `RemoveResult`, drop the in-loop `eprintln!` warnings, and wire `Command::Remove` in `lib.rs` to surface a grouped `⚠ K operations failed` summary and return a non-zero exit. Covers SAFE-01 (#413).

Purpose: Today `remove::execute` continues past per-loop failures and prints warnings inline, but the command still exits 0 — shell scripts see "success" while filesystem artifacts leaked. This plan makes partial failure loud: typed failure records, grouped summary, exit ≠ 0. The plan/render/execute pattern stays intact; this is purely additive on the result struct plus caller-side surfacing.

Output: `FailureKind` enum + `RemoveFailure` struct + extended `RemoveResult` in `remove.rs`; rewritten four failure loops; new caller branch in `lib.rs::Command::Remove`; one unit test (failure injection) in `remove.rs`; one integration test (`chmod 0o000` fixture) in `tests/cli.rs`; CHANGELOG bullet under v0.8 unreleased.
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
@.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md
@.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md
@crates/tome/src/remove.rs
@crates/tome/src/lib.rs
@crates/tome/tests/cli.rs

**CONTEXT.md locks all 20 decisions (D-01..D-20) for this phase — do NOT revisit.** RESEARCH.md confirms file:line refs are accurate ±1 line. The integration-test fixture clones `edge_permission_denied_on_target` at `tests/cli.rs:2230-2256`; remember `chmod 0o755` before `TempDir::drop` or cleanup panics (Pitfall 2). Per D-17 do NOT introduce test abstraction traits. Per D-03 drop existing in-loop `eprintln!` lines — caller is single source of user-facing output. Per D-06 state-save ordering stays: save config → save manifest → regen lockfile → print summary → return Err on failures.
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add FailureKind enum, RemoveFailure struct, extend RemoveResult (D-01, D-02)</name>
  <files>crates/tome/src/remove.rs</files>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-01, D-02)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Pattern 1: Aggregated Partial-Failure Struct)
    - crates/tome/src/remove.rs (verify exact line ranges for RemoveResult and the 4 loops; RESEARCH.md reports drift is ±1)
  </read_first>
  <action>
    In `crates/tome/src/remove.rs`, above the existing `RemoveResult` struct (around lines 40-51), add:

    ```rust
    #[derive(Debug, PartialEq, Eq)]
    pub enum FailureKind {
        Symlink,        // distribution-dir symlinks (step 1)
        LibraryDir,     // local library directories (step 2a)
        LibrarySymlink, // managed-skill library symlinks (step 2b)
        GitCache,       // git repo cache (step 4)
    }

    #[derive(Debug)]
    pub struct RemoveFailure {
        pub path: std::path::PathBuf,
        pub op: FailureKind,
        pub error: std::io::Error,
    }
    ```

    Extend the existing `RemoveResult` struct:
    - Add `pub failures: Vec<RemoveFailure>` field (append at end of struct).
    - Initialize `failures: Vec::new()` in the `RemoveResult::new()` / default constructor (wherever `RemoveResult` is built — mirror the existing zeroed-counter init pattern).
    - **Drop the `#[allow(dead_code)]` attribute on `git_cache_removed`** (D-02) — the caller will now consume it.

    Visibility: keep `pub(crate)` consistent with `RemoveResult` itself per Phase 5 D-09 crate-boundary rule. If `RemoveResult` is currently `pub`, keep the new types `pub` too.

    Do not modify the four failure loops yet — that's Task 2.
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'pub enum FailureKind' crates/tome/src/remove.rs`
    - `grep -q 'Symlink,' crates/tome/src/remove.rs`
    - `grep -q 'LibraryDir,' crates/tome/src/remove.rs`
    - `grep -q 'LibrarySymlink,' crates/tome/src/remove.rs`
    - `grep -q 'GitCache,' crates/tome/src/remove.rs`
    - `grep -q 'pub struct RemoveFailure' crates/tome/src/remove.rs`
    - `grep -q 'pub failures: Vec<RemoveFailure>' crates/tome/src/remove.rs`
    - `! grep -q '#\[allow(dead_code)\]\s*$\|allow(dead_code).*git_cache_removed' crates/tome/src/remove.rs` (the allow on git_cache_removed is gone)
    - `cargo build -p tome 2>&1 | grep -q 'error' && exit 1 || exit 0`
  </acceptance_criteria>
  <done>FailureKind + RemoveFailure types exist, RemoveResult.failures field exists and initializes empty, `#[allow(dead_code)]` on git_cache_removed removed, crate still compiles.</done>
</task>

<task type="auto">
  <name>Task 2: Rewrite the 4 partial-failure loops to push into result.failures (D-03)</name>
  <files>crates/tome/src/remove.rs</files>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-03)
    - crates/tome/src/remove.rs (re-read the 4 loops before editing — line ranges are approximate ±1 per RESEARCH.md drift report)
  </read_first>
  <action>
    In `crates/tome/src/remove.rs::execute()` (body at lines ~181-265), locate each of the four partial-failure loops and replace the in-loop `eprintln!("warning: ...")` with a `result.failures.push(RemoveFailure { ... })`. Loop locations (verify at edit time — RESEARCH.md says ±1 drift):

    **Loop 1 — distribution symlinks (~remove.rs:192-204, op = FailureKind::Symlink)**
    Replace the `Err(e) => eprintln!(...)` arm with:
    ```rust
    Err(e) => result.failures.push(RemoveFailure {
        path: symlink.clone(),
        op: FailureKind::Symlink,
        error: e,
    }),
    ```
    Preserve any `continue` control flow — do not skip subsequent loops.

    **Loop 2 — local library directories (~remove.rs:207-231 main branch, op = FailureKind::LibraryDir)**
    Same shape — push with `op: FailureKind::LibraryDir` and `path` = the library directory being removed.

    **Loop 3 — managed-skill library symlinks (~remove.rs:207-231 sub-branch that handles symlinks, op = FailureKind::LibrarySymlink)**
    Same shape — push with `op: FailureKind::LibrarySymlink`.

    **Loop 4 — git cache (~remove.rs:241-253, op = FailureKind::GitCache)**
    Same shape — push with `op: FailureKind::GitCache` and `path` = the cache directory (`cache_dir`).

    **Critical:** remove every `eprintln!("warning: failed to remove ...")` inside `execute()`. The caller is the single source of warning output per D-03. Do not leave any of the four in-loop warnings behind.

    Do NOT change the success-count incrementing (`symlinks_removed += 1`, etc.) — counts still fire on `Ok(_)` arms.
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -10 && cargo test -p tome --lib remove:: 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `grep -cE 'failures\.push\(RemoveFailure' crates/tome/src/remove.rs` returns 4 or more (one per loop)
    - `grep -q 'op: FailureKind::Symlink' crates/tome/src/remove.rs`
    - `grep -q 'op: FailureKind::LibraryDir' crates/tome/src/remove.rs`
    - `grep -q 'op: FailureKind::LibrarySymlink' crates/tome/src/remove.rs`
    - `grep -q 'op: FailureKind::GitCache' crates/tome/src/remove.rs`
    - `! grep -nE 'eprintln!\("warning: failed to remove' crates/tome/src/remove.rs` (no in-loop warnings remain)
    - `cargo build -p tome 2>&1 | grep -q 'error\[' && exit 1 || exit 0`
    - Existing `remove::tests` unit tests still pass (`cargo test -p tome --lib remove:: 2>&1 | grep -q 'test result: ok'`)
  </acceptance_criteria>
  <done>All four loops push typed RemoveFailure records on Err; zero in-loop `eprintln!("warning:")` remain in `execute`; existing tests still pass.</done>
</task>

<task type="auto">
  <name>Task 3: Wire Command::Remove to surface grouped summary + exit ≠ 0 (D-04, D-05)</name>
  <files>crates/tome/src/lib.rs</files>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-04, D-05, D-06)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Caller pattern block under Pattern 1)
    - crates/tome/src/lib.rs (Command::Remove handler around lines 362-414)
    - crates/tome/src/status.rs (around line 181 — reference the existing use of `paths::collapse_home()`)
  </read_first>
  <action>
    In `crates/tome/src/lib.rs`, the `Command::Remove` handler (~lines 362-414): keep the existing save-config → save-manifest → regen-lockfile → print `✓ Removed directory '{name}'` summary flow unchanged (per D-06). After the existing success summary println, append the failure-surfacing block:

    ```rust
    if !result.failures.is_empty() {
        let k = result.failures.len();
        eprintln!(
            "{} {} operations failed — run {}:",
            console::style("⚠").yellow(),
            k,
            console::style("`tome doctor`").bold(),
        );

        // Group by FailureKind, print per-group header + per-path entries.
        use crate::remove::FailureKind;
        let label = |op: &FailureKind| match op {
            FailureKind::Symlink => "Distribution symlinks",
            FailureKind::LibraryDir => "Library entries",
            FailureKind::LibrarySymlink => "Library symlinks",
            FailureKind::GitCache => "Git cache",
        };

        for kind in [FailureKind::Symlink, FailureKind::LibraryDir, FailureKind::LibrarySymlink, FailureKind::GitCache] {
            let group: Vec<&crate::remove::RemoveFailure> = result
                .failures
                .iter()
                .filter(|f| f.op == kind)
                .collect();
            if group.is_empty() { continue; }
            eprintln!("  {} ({}):", label(&kind), group.len());
            for f in group {
                eprintln!(
                    "    {}: {}",
                    paths::collapse_home(&f.path).display(),
                    f.error,
                );
            }
        }

        return Err(anyhow::anyhow!("remove completed with {k} failures"));
    }
    ```

    Paths must render via `paths::collapse_home()` so users see `~/.tome/…` rather than `/Users/martin/.tome/…` (D-05). `⚠` uses `console::style().yellow()`; `tome doctor` is `.bold()` (matches repo color vocabulary per Phase 6 D-01..D-06).

    On empty failures, behavior is unchanged (existing success path falls through and returns `Ok(())`).

    Import adjustments: if `FailureKind` / `RemoveFailure` need explicit imports at the top of `lib.rs`, add them alphabetically inside the existing `use crate::remove::{ ... }` block (or local `use` inside the branch, as shown above — both acceptable).
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -10 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -10</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'operations failed' crates/tome/src/lib.rs`
    - `grep -q 'run.*tome doctor' crates/tome/src/lib.rs`
    - `grep -q 'remove completed with' crates/tome/src/lib.rs`
    - `grep -q 'paths::collapse_home' crates/tome/src/lib.rs` (path rendering uses the existing helper)
    - `grep -q 'console::style("⚠")' crates/tome/src/lib.rs` (preserves repo glyph + color vocabulary)
    - `grep -q 'Distribution symlinks' crates/tome/src/lib.rs`
    - `grep -q 'Library entries' crates/tome/src/lib.rs`
    - `grep -q 'Library symlinks' crates/tome/src/lib.rs`
    - `grep -q 'Git cache' crates/tome/src/lib.rs`
    - `cargo build -p tome 2>&1 | grep -q 'error\[' && exit 1 || exit 0`
    - `cargo clippy -p tome --all-targets -- -D warnings 2>&1 | grep -q 'error' && exit 1 || exit 0`
  </acceptance_criteria>
  <done>Command::Remove prints grouped `⚠ K operations failed` summary on non-empty `result.failures`, uses `paths::collapse_home`, returns `Err(anyhow!("remove completed with {k} failures"))` so exit code ≠ 0; clippy clean.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 4: Unit test — pre-delete symlink triggers FailureKind::Symlink (D-18 Unit)</name>
  <files>crates/tome/src/remove.rs</files>
  <behavior>
    - With a valid remove-test setup where a dist-dir symlink gets pre-deleted before `execute()` runs, the returned `RemoveResult.failures` MUST contain at least one entry with `op == FailureKind::Symlink` and `path` equal to the symlink path that was pre-deleted.
    - The success counters still fire for artifacts that cleaned up successfully — this is a partial-failure test, not an all-fail test.
  </behavior>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-18)
    - crates/tome/src/remove.rs (existing `#[cfg(test)] mod tests` block — extend it; do NOT invent new abstractions per D-17)
  </read_first>
  <action>
    In the existing `#[cfg(test)] mod tests { ... }` block at the end of `remove.rs`, add a new test `partial_failure_aggregates_symlink_error`:

    1. Use the module's existing `make_test_setup()` / fixture helper (the same pattern the existing tests use — extend if needed, but do NOT introduce new traits).
    2. Build a setup that has at least one distribution symlink registered in the plan.
    3. Before calling `execute()`, pre-delete the dist-dir symlink file (so `remove_file` inside the first loop hits `ENOENT`). Use `std::fs::remove_file(&symlink_path).ok()` to delete without panicking if already absent.
    4. Call `execute()`, capture `result`.
    5. Assert: `result.failures.iter().any(|f| f.op == FailureKind::Symlink)`.
    6. Assert: the matching `RemoveFailure` has `path` equal to the pre-deleted symlink path (use `result.failures.iter().find(|f| f.op == FailureKind::Symlink).unwrap().path` and compare).
    7. The test must NOT rely on any platform-specific trick beyond `std::fs::remove_file` — this is a portable ENOENT injection.

    If the existing fixture helper does not expose the symlink path, extend it minimally to return the path(s) it creates (single focused addition, no abstraction layer).
  </action>
  <verify>
    <automated>cargo test -p tome --lib remove::tests::partial_failure_aggregates_symlink_error 2>&1 | tail -15</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'partial_failure_aggregates_symlink_error' crates/tome/src/remove.rs`
    - `grep -q 'FailureKind::Symlink' crates/tome/src/remove.rs` (test asserts on this variant)
    - `cargo test -p tome --lib remove::tests::partial_failure_aggregates_symlink_error 2>&1 | grep -q 'test result: ok. 1 passed'`
    - No new trait or mock struct introduced (grep for `trait ` additions in the test block returns nothing new — compare pre/post diff in PR)
  </acceptance_criteria>
  <done>Unit test asserts FailureKind::Symlink propagates through RemoveResult.failures under pre-deleted-symlink fixture; test passes in isolation.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 5: Integration test — `tome remove` under chmod 0o000 exits non-zero with ⚠ marker (D-18 Integration)</name>
  <files>crates/tome/tests/cli.rs</files>
  <behavior>
    - Running `tome remove <name> --force` against a fixture where a target directory has been `chmod 0o000`'d causes at least one symlink removal to fail.
    - Exit code is non-zero.
    - Stderr contains the literal `⚠` glyph AND the substring `operations failed`.
    - Stderr contains the substring `remove completed with` (from the anyhow error).
    - Permissions are restored to `0o755` BEFORE assertions so `TempDir::drop` succeeds.
  </behavior>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-18 Integration)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Pitfall 2 — chmod 0o000 fixture leaks; existing fixture block at cli.rs:2230-2256)
    - crates/tome/tests/cli.rs (lines 2230-2256 for the fixture template; lines 3171-3240+ for existing remove_test_env helper and current remove tests)
  </read_first>
  <action>
    Add a new integration test `remove_partial_failure_exits_nonzero_with_warning_marker` to `crates/tome/tests/cli.rs`. Use `#[cfg(unix)]` gating (the chmod trick is Unix-only).

    Structure (mirror `edge_permission_denied_on_target` at lines 2230-2256 + existing `remove_test_env` at 3171-3188):

    1. `use std::os::unix::fs::PermissionsExt;`
    2. Set up a `TempDir`, create a local skills source directory with one skill, create a distribution target directory via `remove_test_env` (or clone its body inline).
    3. Run `tome sync` first so the distribution symlink is populated (mirror existing `test_remove_local_directory` at ~cli.rs:3218+).
    4. `chmod 0o000` the **target** directory (not `TempDir` itself — the target containing the symlink), identical to line 2242 of the existing test.
    5. Run `tome remove <name> --force` via `Command::cargo_bin("tome")` with the tome_home arg.
    6. Capture `Output`.
    7. **Restore permissions FIRST**: `std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755)).unwrap();` — BEFORE any assertion so `TempDir::drop` works (Pitfall 2 from RESEARCH.md).
    8. Assert (order matters — restore first, assert second):
       - `assert!(!output.status.success(), "remove should fail on chmod 0o000");`
       - `let stderr = String::from_utf8_lossy(&output.stderr);`
       - `assert!(stderr.contains("⚠"), "stderr missing ⚠ marker: {stderr}");`
       - `assert!(stderr.contains("operations failed"), "stderr missing 'operations failed': {stderr}");`
       - `assert!(stderr.contains("remove completed with"), "stderr missing anyhow error: {stderr}");`

    Do not introduce any new helper abstraction — clone or inline the fixture setup (D-17 no trait abstraction).
  </action>
  <verify>
    <automated>cargo test -p tome --test cli remove_partial_failure_exits_nonzero_with_warning_marker 2>&1 | tail -15</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'remove_partial_failure_exits_nonzero_with_warning_marker' crates/tome/tests/cli.rs`
    - `grep -q 'Permissions::from_mode(0o000)' crates/tome/tests/cli.rs` (fixture uses the chmod trick)
    - `grep -q 'Permissions::from_mode(0o755)' crates/tome/tests/cli.rs` (permissions restored before assertions)
    - `grep -q 'operations failed' crates/tome/tests/cli.rs` (assertion on stderr)
    - `grep -q 'remove completed with' crates/tome/tests/cli.rs` (assertion on anyhow error)
    - `cargo test -p tome --test cli remove_partial_failure_exits_nonzero_with_warning_marker 2>&1 | grep -q 'test result: ok. 1 passed'`
  </acceptance_criteria>
  <done>Integration test asserts non-zero exit, `⚠` in stderr, `operations failed` in stderr, and restores permissions before assertion to keep TempDir cleanup happy.</done>
</task>

<task type="auto">
  <name>Task 6: CHANGELOG entry under v0.8 unreleased (SAFE-01 / #413)</name>
  <files>CHANGELOG.md</files>
  <read_first>
    - CHANGELOG.md (top ~50 lines — verify the v0.8 unreleased section exists and see the v0.7.0 style for reference)
  </read_first>
  <action>
    In `CHANGELOG.md`, under the existing v0.8 unreleased section (or add one at the top if missing, matching v0.7.0's format with `### Fixed` / `### Changed` subsections), add this bullet under `### Fixed`:

    ```
    - `tome remove` now aggregates partial-cleanup failures and exits non-zero with a distinct `⚠ N operations failed` summary grouped by failure kind (distribution symlinks, library entries, library symlinks, git cache). Previously the command reported success while filesystem artifacts leaked. ([#413](https://github.com/MartinP7r/tome/issues/413))
    ```

    Do NOT bump the version number in `Cargo.toml` — `make release` handles that (global rule).
  </action>
  <verify>
    <automated>grep -q 'tome remove.*aggregates partial-cleanup' CHANGELOG.md && grep -q '#413' CHANGELOG.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'operations failed' CHANGELOG.md`
    - `grep -q '#413' CHANGELOG.md`
    - `grep -q 'aggregates partial-cleanup' CHANGELOG.md`
    - `git diff Cargo.toml 2>&1 | grep -q '^+version' && exit 1 || exit 0` (no version bump in Cargo.toml)
  </acceptance_criteria>
  <done>CHANGELOG.md has a bullet for SAFE-01 under v0.8 unreleased `### Fixed`, referencing #413; no Cargo.toml version bump.</done>
</task>

</tasks>

<verification>
- `cargo fmt -- --check` passes (repo style preserved)
- `cargo clippy --all-targets -- -D warnings` passes (no new warnings from FailureKind enum or caller branch)
- `cargo test` passes (both the new unit test `remove::tests::partial_failure_aggregates_symlink_error` and the integration test `remove_partial_failure_exits_nonzero_with_warning_marker` green)
- Running `tome remove` on a clean setup still exits 0 and prints the existing `✓ Removed directory 'X': N library entries, M symlinks` line with no extra noise (success path unchanged per D-06)
- Running `tome remove` against a `chmod 0o000` target prints the `⚠ K operations failed — run `tome doctor`:` header with grouped per-path entries AND exits non-zero (evidenced by the integration test)
</verification>

<success_criteria>
- SAFE-01 requirement satisfied: partial-failure visibility (`⚠ N operations failed` summary, exit ≠ 0) is observable on `tome remove`.
- `RemoveResult.failures` is the single source of partial-failure state; in-loop `eprintln!` warnings in `execute()` are gone (D-03).
- New test count: +1 unit test + 1 integration test = 2 new tests (matches D-18 expectation).
- CHANGELOG entry present; Cargo.toml version untouched.
- Four untouched files per RESEARCH.md blast radius: `config.rs`, `discover.rs`, `library.rs`, `distribute.rs`, `manifest.rs`, `cleanup.rs`, `doctor.rs`, `lockfile.rs`, `wizard.rs`, `machine.rs`, `git.rs`, `theme.rs`, `browse/*`, `relocate.rs`.
</success_criteria>

<output>
After completion, create `.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-01-safe-01-remove-partial-failure-aggregation-SUMMARY.md` capturing:
- Artifacts created (FailureKind enum, RemoveFailure struct, failures field, caller branch, two tests, CHANGELOG bullet)
- Files modified with exact line ranges
- Test names added + pass status
- Any deviations from CONTEXT.md D-01..D-06 or D-18 (expected: none)
- Next: SAFE-02 + SAFE-03 are wave-1 parallel plans with no ordering dependency
</output>

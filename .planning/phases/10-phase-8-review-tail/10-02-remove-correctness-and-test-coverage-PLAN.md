---
phase: 10-phase-8-review-tail
plan: 02
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/remove.rs
  - crates/tome/src/lib.rs
  - crates/tome/tests/cli.rs
autonomous: true
requirements: [POLISH-04, POLISH-05, TEST-01, TEST-02, TEST-04]
issue: "https://github.com/MartinP7r/tome/issues/463 + https://github.com/MartinP7r/tome/issues/462"

must_haves:
  truths:
    - "`FailureKind::ALL` cannot drift from the enum: a compile-time guard (exhaustive-match sentinel function `_ensure_failure_kind_all_exhaustive`) fails to compile if a new variant is added without also being appended to `ALL`. No runtime drift detection — the check happens at `cargo build` time."
    - "`RemoveFailure::new` carries a real invariant: `debug_assert!(path.is_absolute(), \"RemoveFailure::path must be absolute, got: {}\", path.display())` fires in debug builds when constructed with a relative path. (POLISH-05 option (a) — keep the constructor + add the invariant. Removing the constructor (option b) would touch 4 production call sites; option a is smaller blast-radius and gives the absolute-path invariant we want for collapse_home rendering.)"
    - "`remove_partial_failure_exits_nonzero_with_warning_marker` asserts the success banner `\"✓ Removed directory\"` is ABSENT from BOTH stdout AND stderr when the partial-failure path fires."
    - "An end-to-end retry-after-fix test (`remove_retry_succeeds_after_failure_resolved`) drives the full I2/I3 contract: chmod 0o500 → `tome remove` → expect partial failure (config + manifest preserved) → chmod 0o755 → `tome remove` again → succeeds with empty `failures`, config entry gone, manifest empty for that source, library dir for the skill gone."
    - "On the happy `tome remove` path, the success banner `\"✓ Removed directory\"` is the LAST output line — `regen_warnings` printed via `eprintln!(\"warning: ...\")` either fire BEFORE the success banner is printed (option a, deferred) or are scoped with a `[lockfile regen]` prefix (option b). We pick option (a): defer the warning prints until after the success banner — the banner is the user's anchor and warnings should be a subordinate footnote."
    - "The TEST-04 source-order regression test ANCHORS its `String::find()` searches to the `Command::Remove` handler region in `lib.rs` so a future reorder of unrelated regen-warnings handlers (Reassign / Fork) cannot create a false-positive failure unrelated to the Remove ordering contract."
  artifacts:
    - path: "crates/tome/src/remove.rs"
      provides: "Compile-enforced `FailureKind::ALL` via `_ensure_failure_kind_all_exhaustive` const-eval sentinel that exhaustive-matches every variant. `RemoveFailure::new` gains `debug_assert!(path.is_absolute(), ...)`."
      contains: "_ensure_failure_kind_all_exhaustive"
    - path: "crates/tome/src/lib.rs"
      provides: "`Command::Remove` happy-path: success banner `println!` runs BEFORE the `eprintln!(\"warning: ...\")` loop over `regen_warnings`. Code comment explains the deferred-warnings choice (option a)."
      contains: "Removed directory"
    - path: "crates/tome/tests/cli.rs"
      provides: "Updated `remove_partial_failure_exits_nonzero_with_warning_marker` with success-banner-absence assertion. New `remove_retry_succeeds_after_failure_resolved` end-to-end retry test. New `lib_rs_remove_handler_prints_success_banner_before_regen_warnings` ordering regression test (anchored to the `Command::Remove` region in lib.rs)."
      contains: "remove_retry_succeeds_after_failure_resolved"
  key_links:
    - from: "crates/tome/src/remove.rs::FailureKind::ALL"
      to: "_ensure_failure_kind_all_exhaustive"
      via: "compile-time exhaustive match — adding a variant requires updating both"
      pattern: "_ensure_failure_kind_all_exhaustive"
    - from: "crates/tome/src/remove.rs::RemoveFailure::new"
      to: "debug_assert! on path.is_absolute()"
      via: "constructor invariant"
      pattern: "debug_assert!\\(path\\.is_absolute"
    - from: "crates/tome/src/lib.rs::Command::Remove handler"
      to: "success banner println before regen_warnings eprintln loop"
      via: "deferred-warnings ordering (TEST-04 option a)"
      pattern: "Removed directory.*\\n.*for w in.*regen_warnings"
    - from: "crates/tome/tests/cli.rs::remove_partial_failure_exits_nonzero_with_warning_marker"
      to: "stdout/stderr success-banner-absence assertion"
      via: "predicate that ✓ Removed directory does NOT appear"
      pattern: "Removed directory"
---

<objective>
Close the Remove-module review tail: compile-enforce `FailureKind::ALL` so it cannot drift from the enum, give `RemoveFailure::new` a real `debug_assert!` invariant, fix the `regen_warnings`-before-success-banner ordering on the happy path, expand the existing partial-failure CLI test to assert the success banner is absent, and add an end-to-end retry-after-fix integration test that pins the I2/I3 retention contract.

This plan owns 5 of the 11 review-tail items, all centered on `crates/tome/src/remove.rs` + `lib.rs::Command::Remove` + `tests/cli.rs`.

**Closes:** POLISH-04 (D4, FailureKind::ALL drift-proofing), POLISH-05 (D5, RemoveFailure::new invariant), TEST-01 (P1, success-banner-absence), TEST-02 (P2, retry-after-fix e2e), TEST-04 (P4, regen-warnings-after-banner).

**Decisions pinned:**
- POLISH-04 option: **(c) exhaustive-match sentinel** (compile-enforced, no `strum` dependency). Smaller blast-radius than (a) `strum::EnumIter`.
- POLISH-05 option: **(a) keep `new()` + add `debug_assert!`** invariant. Smaller blast-radius than (b) replacing 4 call sites.
- TEST-04 option: **(a) defer warnings until after the success banner**. The banner is the user's anchor; warnings as a footnote feel more natural than the `[lockfile regen]` prefix (option b) which adds visual noise even on the happy path.

Purpose: harden the partial-failure contract that v0.8 + v0.8.1 shipped, eliminate the manual-sync drift hazard on `FailureKind::ALL`, and lock the success-banner-last invariant so a future refactor cannot regress it silently.

Output: 1 compile-time sentinel function in `remove.rs`, 1 debug_assert in `RemoveFailure::new`, ~10-line reorder in `lib.rs::Command::Remove`, success-banner-absence assertion added to existing test, 2 new integration tests.
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

@crates/tome/src/remove.rs
@crates/tome/src/lib.rs
@crates/tome/tests/cli.rs

<interfaces>
<!-- Key types and contracts the executor needs. Extracted from codebase. -->

Current `FailureKind` (`crates/tome/src/remove.rs` lines 50–88):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureKind {
    DistributionSymlink,
    LibraryDir,
    LibrarySymlink,
    GitCache,
}

impl FailureKind {
    pub(crate) const ALL: [FailureKind; 4] = [
        FailureKind::DistributionSymlink,
        FailureKind::LibraryDir,
        FailureKind::LibrarySymlink,
        FailureKind::GitCache,
    ];
    pub(crate) fn label(self) -> &'static str { /* match */ }
}
```

After this plan:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureKind {
    DistributionSymlink,
    LibraryDir,
    LibrarySymlink,
    GitCache,
}

impl FailureKind {
    pub(crate) const ALL: [FailureKind; 4] = [
        FailureKind::DistributionSymlink,
        FailureKind::LibraryDir,
        FailureKind::LibrarySymlink,
        FailureKind::GitCache,
    ];
    pub(crate) fn label(self) -> &'static str { /* unchanged */ }
}

/// Compile-time drift guard for `FailureKind::ALL`. If a new variant is
/// added to `FailureKind` without being appended to `ALL`, this function
/// fails to compile (the exhaustive match misses the new variant).
///
/// The function is dead code at runtime — `#[allow(dead_code)]` documents
/// that it exists purely as a compile-time check. Symmetric to the
/// 12-combo `(DirectoryType, DirectoryRole)` matrix test (WHARD-06)
/// which compile-enforces config-shape invariants.
#[allow(dead_code)]
const fn _ensure_failure_kind_all_exhaustive(k: FailureKind) -> usize {
    // Match every variant — the compiler bails if a new variant is added
    // without being added here. Each arm returns the variant's index in
    // `ALL`, so a const_assert on FailureKind::ALL[index] == variant
    // could be added later if desired (currently unnecessary; the
    // ordering is read-only — it only affects user-visible label
    // grouping).
    match k {
        FailureKind::DistributionSymlink => 0,
        FailureKind::LibraryDir => 1,
        FailureKind::LibrarySymlink => 2,
        FailureKind::GitCache => 3,
    }
}

// Optionally: a const-time length assertion that pins ALL.len() == number of arms above.
const _: () = {
    // If this fails, FailureKind::ALL has the wrong length. The arms in
    // _ensure_failure_kind_all_exhaustive are the source of truth for
    // "what variants exist"; ALL must contain exactly that many entries.
    assert!(FailureKind::ALL.len() == 4);
};
```

Current `RemoveFailure::new` (lines 98–104):
```rust
pub(crate) fn new(kind: FailureKind, path: PathBuf, error: std::io::Error) -> Self {
    RemoveFailure { kind, path, error }
}
```

After this plan:
```rust
/// Construct a `RemoveFailure`. Centralizes the absolute-path invariant
/// so downstream rendering (`paths::collapse_home(&f.path)` in lib.rs)
/// is well-defined: collapse_home expects an absolute path; relative
/// paths would render unmodified, leaking working-directory-relative
/// shapes into user-facing error output. The four call sites in
/// `execute()` all pass paths derived from config-resolved directories
/// (always absolute), so this debug_assert never fires in normal use —
/// it's a forward guard against a future refactor that adds a fifth
/// call site with a relative path.
pub(crate) fn new(kind: FailureKind, path: PathBuf, error: std::io::Error) -> Self {
    debug_assert!(
        path.is_absolute(),
        "RemoveFailure::path must be absolute, got: {}",
        path.display()
    );
    RemoveFailure { kind, path, error }
}
```

Current `Command::Remove` happy-path (`crates/tome/src/lib.rs` lines 449–478):
```rust
// Save updated config
config.save(&paths.config_path())?;
// Save updated manifest
manifest::save(&manifest, paths.config_dir())?;
// Regenerate lockfile.
let (resolved_paths, mut regen_warnings) =
    lockfile::resolved_paths_from_lockfile_cache(&config, &paths);
let skills = discover::discover_all(&config, &resolved_paths, &mut regen_warnings)?;
for w in &regen_warnings {
    eprintln!("warning: {}", w);            // <-- prints BEFORE success banner
}
let lockfile = lockfile::generate(&manifest, &skills);
lockfile::save(&lockfile, paths.config_dir())?;

// Success path — full cleanup completed with no failures.
println!(
    "\n{} Removed directory '{}': {} library entries, {} symlinks{}",
    style("✓").green(), name, ...
);
```

After this plan (TEST-04 option a — defer warnings until after the banner):
```rust
// Save updated config
config.save(&paths.config_path())?;
// Save updated manifest
manifest::save(&manifest, paths.config_dir())?;
// Regenerate lockfile. Recover git-skill provenance offline ...
let (resolved_paths, mut regen_warnings) =
    lockfile::resolved_paths_from_lockfile_cache(&config, &paths);
let skills = discover::discover_all(&config, &resolved_paths, &mut regen_warnings)?;
let lockfile = lockfile::generate(&manifest, &skills);
lockfile::save(&lockfile, paths.config_dir())?;

// Success banner FIRST — it's the user's anchor for "what just happened".
// Warnings come after as a footnote (TEST-04 option a — deferred ordering).
// Without this ordering, the success banner gets buried under stderr
// warnings on multi-warning regen and the user has to scroll up to find
// the green ✓ confirmation.
println!(
    "\n{} Removed directory '{}': {} library entries, {} symlinks{}",
    style("✓").green(),
    name,
    result.library_entries_removed,
    result.symlinks_removed,
    if result.git_cache_removed { ", git cache" } else { "" },
);
for w in &regen_warnings {
    eprintln!("warning: {}", w);
}
```

Current `remove_partial_failure_exits_nonzero_with_warning_marker` test (`crates/tome/tests/cli.rs` lines 3375–3459) asserts:
- exit non-success
- stderr contains `⚠`
- stderr contains `"operations failed"`
- stderr contains `"remove completed with"`
- stderr contains `"retained"` OR `"retry"`

After this plan, ADD:
- stdout does NOT contain `"✓ Removed directory"`
- stderr does NOT contain `"✓ Removed directory"` (defense-in-depth)

The reasoning: on partial failure, `lib.rs::Command::Remove` returns Err *before* reaching the success banner, so the banner never prints. The current test only asserts the warning marker IS present; a future refactor that prints the banner unconditionally (e.g., moving it before the partial-failure block) would not be caught. P1 closes this gap.

`remove_test_env` and `create_skill` are existing test helpers in `tests/cli.rs` — reuse them.

**`lib.rs` contains THREE `for w in &regen_warnings` loops**: one each in the `Command::Remove`, `Command::Reassign`, and `Command::Fork` handlers. Currently the Remove handler is the FIRST occurrence in source order, so a naïve `lib_rs.find("for w in &regen_warnings")` would find the right loop by coincidence. To prevent a future-refactor false-positive (e.g., reordering Reassign above Remove, or inserting a new handler with its own regen-warnings loop above Remove), the source-order test in Task 4 / Step 2 anchors all `find()` calls to the `Command::Remove` handler region first, then searches FORWARD from there for both the success banner and the warnings loop. This guarantees the test fails ONLY when the Remove handler itself regresses.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Compile-enforce `FailureKind::ALL` + add `debug_assert!` to `RemoveFailure::new` (POLISH-04, POLISH-05)</name>
  <files>crates/tome/src/remove.rs</files>
  <read_first>
    - crates/tome/src/remove.rs lines 45–104 (FailureKind enum + ALL constant + RemoveFailure::new)
    - crates/tome/src/remove.rs lines 252–321 (the four call sites of `RemoveFailure::new` in `execute`)
    - crates/tome/src/remove.rs lines 350–625 (existing test module — extend, do not break)
  </read_first>
  <behavior>
    - Test 1 (`failure_kind_all_length_matches_variant_count`): asserts `FailureKind::ALL.len() == 4` and that ALL contains every variant exactly once (using a `BTreeSet`). Pins the runtime check that complements the compile-time sentinel.
    - Test 2 (`failure_kind_all_ordering_pinned`): asserts the order of ALL is exactly `[DistributionSymlink, LibraryDir, LibrarySymlink, GitCache]`. The order is meaningful — it's the order the `lib.rs` consumer iterates for grouped user-facing output. A reorder is a UI change and should require an explicit code edit.
    - Test 3 (`remove_failure_new_relative_path_panics_in_debug`): drives `RemoveFailure::new(FailureKind::DistributionSymlink, PathBuf::from("relative/path"), io::Error::other("e"))` and asserts a panic via `std::panic::catch_unwind`. Gated on `cfg!(debug_assertions)` — release builds compile out the assert, so in release the test asserts construction succeeds without panicking. Use:
      ```rust
      #[test]
      fn remove_failure_new_relative_path_panics_in_debug() {
          let result = std::panic::catch_unwind(|| {
              RemoveFailure::new(
                  FailureKind::DistributionSymlink,
                  PathBuf::from("relative/path"),
                  std::io::Error::other("test"),
              )
          });
          if cfg!(debug_assertions) {
              assert!(result.is_err(), "debug build must panic on relative path");
          } else {
              assert!(result.is_ok(), "release build must allow construction");
          }
      }
      ```
    - Test 4 (`remove_failure_new_absolute_path_succeeds`): drives `RemoveFailure::new(...)` with `PathBuf::from("/abs/path")` and asserts construction succeeds in both debug and release. Reuses the existing `test_hash` helper if needed for io::Error.
  </behavior>
  <action>
**Step 1 — Add the compile-time sentinel** (place immediately after the `impl FailureKind` block in `crates/tome/src/remove.rs`, around line 89):

```rust
/// Compile-time drift guard for `FailureKind::ALL`. If a new variant is
/// added to `FailureKind`, this `const fn` fails to compile because the
/// match below is exhaustive. The fix is to (a) add an arm here AND
/// (b) append the new variant to `ALL`. Symmetric to the 12-combo
/// `(DirectoryType, DirectoryRole)` matrix test that compile-enforces
/// config-shape invariants (WHARD-06).
///
/// The function is dead-code at runtime — its sole purpose is the
/// exhaustiveness check. The `const _: () = ...` block below additionally
/// pins `ALL.len() == 4` at compile time.
#[allow(dead_code)]
const fn _ensure_failure_kind_all_exhaustive(k: FailureKind) -> usize {
    match k {
        FailureKind::DistributionSymlink => 0,
        FailureKind::LibraryDir => 1,
        FailureKind::LibrarySymlink => 2,
        FailureKind::GitCache => 3,
    }
}

const _: () = {
    // If this fails: FailureKind::ALL is missing or has extra variants.
    // The match arms in _ensure_failure_kind_all_exhaustive are the source
    // of truth — ALL must contain exactly one entry per arm.
    assert!(FailureKind::ALL.len() == 4);
};
```

**Step 2 — Add `debug_assert!` to `RemoveFailure::new`** (lines 98–104):

```rust
impl RemoveFailure {
    /// Construct a `RemoveFailure`. The path MUST be absolute — downstream
    /// rendering uses `paths::collapse_home(&f.path)` in lib.rs, which
    /// expects an absolute path. The four `execute()` call sites all pass
    /// config-resolved directory paths (always absolute), so this guard
    /// never fires in normal use; it's a forward guard against a future
    /// refactor that adds a relative-path call site.
    ///
    /// Debug-only via `debug_assert!` to keep release builds zero-cost.
    pub(crate) fn new(kind: FailureKind, path: PathBuf, error: std::io::Error) -> Self {
        debug_assert!(
            path.is_absolute(),
            "RemoveFailure::path must be absolute, got: {}",
            path.display()
        );
        RemoveFailure { kind, path, error }
    }
}
```

**Step 3 — Verify the four existing `execute()` call sites pass absolute paths.** Read `execute()` (lines 252–321) and confirm:
- `plan.symlinks_to_remove` originates from config-directory `path` joined with `entry.path()` — both absolute. ✓
- `plan.library_paths` originates from `paths.library_dir().join(skill)` — `library_dir` is absolute. ✓
- `plan.git_cache_path` originates from `git::repo_cache_dir(&paths.repos_dir(), url_str)` — `repos_dir` is absolute. ✓

If any site passes a relative path, the existing test `partial_failure_aggregates_symlink_error` would now panic in debug. Run it after the change to confirm.

**Step 4 — Add the 4 new tests** to `mod tests` in `remove.rs` (after `partial_failure_aggregates_multiple_kinds`, ~line 624):

```rust
#[test]
fn failure_kind_all_length_matches_variant_count() {
    use std::collections::BTreeSet;
    let set: BTreeSet<FailureKind> = FailureKind::ALL.iter().copied().collect();
    assert_eq!(
        set.len(),
        4,
        "FailureKind::ALL must contain every variant exactly once"
    );
    assert!(set.contains(&FailureKind::DistributionSymlink));
    assert!(set.contains(&FailureKind::LibraryDir));
    assert!(set.contains(&FailureKind::LibrarySymlink));
    assert!(set.contains(&FailureKind::GitCache));
}

#[test]
fn failure_kind_all_ordering_pinned() {
    // The grouped failure-summary output in lib.rs::Command::Remove iterates
    // FailureKind::ALL in declaration order. The user-visible grouping
    // therefore depends on this exact order. A reorder is a UI change and
    // must require an explicit code edit (this test fails on reorder).
    assert_eq!(
        FailureKind::ALL,
        [
            FailureKind::DistributionSymlink,
            FailureKind::LibraryDir,
            FailureKind::LibrarySymlink,
            FailureKind::GitCache,
        ],
        "FailureKind::ALL ordering is part of the user-visible grouping contract"
    );
}

#[test]
fn remove_failure_new_relative_path_panics_in_debug() {
    let result = std::panic::catch_unwind(|| {
        RemoveFailure::new(
            FailureKind::DistributionSymlink,
            PathBuf::from("relative/path"),
            std::io::Error::other("test"),
        )
    });
    if cfg!(debug_assertions) {
        assert!(result.is_err(), "debug build must panic on relative path");
    } else {
        assert!(result.is_ok(), "release build must allow construction (debug_assert compiled out)");
    }
}

#[test]
fn remove_failure_new_absolute_path_succeeds() {
    let f = RemoveFailure::new(
        FailureKind::DistributionSymlink,
        PathBuf::from("/abs/path"),
        std::io::Error::other("test"),
    );
    assert_eq!(f.kind, FailureKind::DistributionSymlink);
    assert_eq!(f.path, PathBuf::from("/abs/path"));
}
```

`std::io::Error::other` is stable since 1.74 (we're on 1.85+). Replace with `std::io::Error::new(std::io::ErrorKind::Other, "test")` if clippy complains.

**Step 5 — Verify drift-guard works.** Add a temporary `FailureKind::Bogus` variant locally, run `cargo build -p tome`, observe the error pointing to `_ensure_failure_kind_all_exhaustive`, then revert. (This is a manual verification step — the executor should NOT commit the temporary variant; it's a confidence check that the sentinel actually catches drift. Document this in the SUMMARY as a one-line "drift-guard manually verified.")

Run: `cargo test -p tome remove::tests::failure_kind_all remove::tests::remove_failure_new`
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo build -p tome 2>&1 | tail -5 && cargo test -p tome remove::tests 2>&1 | tail -15 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "_ensure_failure_kind_all_exhaustive" crates/tome/src/remove.rs` returns at least 1 match.
    - `rg -n "const _: \(\) = \{" crates/tome/src/remove.rs` returns at least 1 match (the const-assert block).
    - `rg -n "FailureKind::ALL.len\(\) == 4" crates/tome/src/remove.rs` returns at least 1 match.
    - `rg -n "debug_assert!\(path\.is_absolute" crates/tome/src/remove.rs` returns exactly 1 match.
    - `rg -n "RemoveFailure::path must be absolute" crates/tome/src/remove.rs` returns exactly 1 match.
    - `cargo test -p tome remove::tests::failure_kind_all_length_matches_variant_count` passes.
    - `cargo test -p tome remove::tests::failure_kind_all_ordering_pinned` passes.
    - `cargo test -p tome remove::tests::remove_failure_new_relative_path_panics_in_debug` passes.
    - `cargo test -p tome remove::tests::remove_failure_new_absolute_path_succeeds` passes.
    - `cargo test -p tome remove::tests::partial_failure_aggregates_symlink_error` still passes (existing test must not regress; would panic if any execute() call site passes a relative path).
    - `cargo test -p tome remove::tests::partial_failure_aggregates_multiple_kinds` still passes (regression).
    - `cargo build -p tome` is clean.
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
  </acceptance_criteria>
  <done>
    `_ensure_failure_kind_all_exhaustive` const fn + `const _: () = { assert!(...) };` block compile-enforce that `FailureKind::ALL` cannot drift from the enum. `RemoveFailure::new` carries `debug_assert!(path.is_absolute(), ...)`. 4 new unit tests pin the contract. Existing tests still pass.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Reorder `regen_warnings` after the success banner on the happy path (TEST-04)</name>
  <files>crates/tome/src/lib.rs</files>
  <read_first>
    - crates/tome/src/lib.rs lines 449–478 (Command::Remove happy-path block)
  </read_first>
  <behavior>
    Behavior is verified end-to-end via the `lib_rs_remove_handler_prints_success_banner_before_regen_warnings` source-order regression test added in Task 4. No unit test here — the call site is bare `println!`/`eprintln!` ordering which is meaningless without process-level stdout/stderr capture.
  </behavior>
  <action>
**Reorder** the happy-path block in `Command::Remove` (`crates/tome/src/lib.rs` lines 449–478). Move the `for w in &regen_warnings { eprintln!(...) }` loop to AFTER the success-banner `println!`.

Current:
```rust
// Save updated config
config.save(&paths.config_path())?;
manifest::save(&manifest, paths.config_dir())?;

let (resolved_paths, mut regen_warnings) =
    lockfile::resolved_paths_from_lockfile_cache(&config, &paths);
let skills = discover::discover_all(&config, &resolved_paths, &mut regen_warnings)?;
for w in &regen_warnings {
    eprintln!("warning: {}", w);
}
let lockfile = lockfile::generate(&manifest, &skills);
lockfile::save(&lockfile, paths.config_dir())?;

// Success path — full cleanup completed with no failures.
println!(
    "\n{} Removed directory '{}': {} library entries, {} symlinks{}",
    style("✓").green(), name, ...
);
```

After (TEST-04 option a — deferred warnings):
```rust
// Save updated config
config.save(&paths.config_path())?;
manifest::save(&manifest, paths.config_dir())?;

// Regenerate lockfile. Recover git-skill provenance offline from the
// previous lockfile + on-disk cache so git-type directories are not
// silently dropped during regen (#461 H1). Warnings collected here
// are deferred until AFTER the success banner — see comment below.
let (resolved_paths, mut regen_warnings) =
    lockfile::resolved_paths_from_lockfile_cache(&config, &paths);
let skills = discover::discover_all(&config, &resolved_paths, &mut regen_warnings)?;
let lockfile = lockfile::generate(&manifest, &skills);
lockfile::save(&lockfile, paths.config_dir())?;

// Success banner FIRST (TEST-04 option a — deferred regen-warnings).
// The banner is the user's anchor for "what just happened"; warnings
// come after as a footnote. Without this ordering, multi-warning
// regen output buries the green ✓ confirmation and the user has to
// scroll up to find it. The deferred ordering is regression-tested by
// `lib_rs_remove_handler_prints_success_banner_before_regen_warnings`
// in tests/cli.rs.
println!(
    "\n{} Removed directory '{}': {} library entries, {} symlinks{}",
    style("✓").green(),
    name,
    result.library_entries_removed,
    result.symlinks_removed,
    if result.git_cache_removed {
        ", git cache"
    } else {
        ""
    },
);
for w in &regen_warnings {
    eprintln!("warning: {}", w);
}
```

The reorder is purely textual — no logic change. The `regen_warnings` Vec is populated by `discover_all` BEFORE the banner prints, so the warning loop has all warnings ready when it runs.

Run: `cargo build -p tome && cargo test -p tome --test cli`
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo build -p tome 2>&1 | tail -5 && cargo test -p tome --test cli 2>&1 | tail -10</automated>
  </verify>
  <acceptance_criteria>
    - In `crates/tome/src/lib.rs::Command::Remove` happy-path, the `println!("✓ Removed directory ...")` line MUST appear BEFORE the `for w in &regen_warnings { eprintln!("warning: {}", w); }` loop. Verify with this multi-line grep:
      ```bash
      rg -nU "Removed directory.*\n(.*\n){0,8}for w in &regen_warnings" crates/tome/src/lib.rs
      ```
      MUST return at least 1 match.
    - `rg -nU "for w in &regen_warnings.*\n(.*\n){0,8}Removed directory" crates/tome/src/lib.rs` MUST return 0 matches (no instance of warnings BEFORE banner).
    - `rg -n "TEST-04 option a" crates/tome/src/lib.rs` returns at least 1 match (decision documented in code).
    - `cargo build -p tome` is clean.
    - All existing `remove_*` integration tests in `tests/cli.rs` still pass.
  </acceptance_criteria>
  <done>
    Happy-path success banner prints BEFORE regen_warnings on `tome remove`. Code comment in lib.rs cites TEST-04 option a. Existing test suite still green.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Update `remove_partial_failure_exits_nonzero_with_warning_marker` to assert success-banner-absence (TEST-01)</name>
  <files>crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/tests/cli.rs lines 3375–3459 (existing test)
  </read_first>
  <behavior>
    The existing test exercises the partial-failure path (chmod 0o500 on the target dir → DistributionSymlink failure during `tome remove`). After the chmod-restore, the test currently asserts:
    1. exit non-success
    2. stderr contains `⚠`
    3. stderr contains `"operations failed"`
    4. stderr contains `"remove completed with"`
    5. stderr contains `"retained"` OR `"retry"`

    Add (TEST-01 / P1):
    6. stdout does NOT contain `"✓ Removed directory"` (the success banner string)
    7. stderr does NOT contain `"✓ Removed directory"` (defense-in-depth)

    The assertion uses the literal substring `"Removed directory"` (without the `✓` glyph) since console color codes are stripped by `NO_COLOR=1` and the styled `✓` may render as the bare character. Asserting on `"Removed directory"` (no glyph) is robust against both styled and unstyled output.
  </behavior>
  <action>
In `crates/tome/tests/cli.rs`, locate `remove_partial_failure_exits_nonzero_with_warning_marker` (line 3377) and add two assertions immediately after the existing assertion block (after line 3458, before the closing brace at 3459):

```rust
let stdout = String::from_utf8_lossy(&output.stdout);
// TEST-01 / P1: success banner MUST NOT appear on partial failure.
// The banner string is "✓ Removed directory" but the leading glyph may
// be styled with ANSI codes; we assert on "Removed directory" (no glyph)
// for robustness against console color rendering.
assert!(
    !stdout.contains("Removed directory"),
    "stdout must NOT contain success banner on partial failure; got: {stdout}",
);
assert!(
    !stderr.contains("Removed directory"),
    "stderr must NOT contain success banner on partial failure (defense-in-depth); got: {stderr}",
);
```

Place these AFTER the existing `assert!(stderr.contains(...))` block but BEFORE the closing brace. The `stderr` variable is already in scope from line 3443 (`let stderr = String::from_utf8_lossy(&output.stderr);`); add a parallel `stdout` extraction.

Run: `cargo test -p tome --test cli remove_partial_failure_exits_nonzero_with_warning_marker`
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo test -p tome --test cli remove_partial_failure_exits_nonzero_with_warning_marker 2>&1 | tail -10</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "stdout must NOT contain success banner|stderr must NOT contain success banner" crates/tome/tests/cli.rs` returns at least 2 matches (one stdout, one stderr).
    - `rg -n "!stdout\.contains\(\"Removed directory\"\)|!stderr\.contains\(\"Removed directory\"\)" crates/tome/tests/cli.rs` returns at least 2 matches.
    - `cargo test -p tome --test cli remove_partial_failure_exits_nonzero_with_warning_marker` passes.
    - `cargo test -p tome --test cli remove_partial_failure_does_not_save_disk_state` still passes (sibling test, regression).
  </acceptance_criteria>
  <done>
    `remove_partial_failure_exits_nonzero_with_warning_marker` asserts the success banner is absent from BOTH stdout AND stderr. Test passes; sibling `remove_partial_failure_does_not_save_disk_state` regression-clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 4: Add `remove_retry_succeeds_after_failure_resolved` + `lib_rs_remove_handler_prints_success_banner_before_regen_warnings` (TEST-02, TEST-04 regression)</name>
  <files>crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/tests/cli.rs lines 3375–3540 (existing partial-failure tests — fixture pattern to mirror)
    - crates/tome/tests/cli.rs lines 1–80 (test helpers — `tome()`, `create_skill`, `remove_test_env`)
    - crates/tome/src/lib.rs (the `Command::Remove` handler region, plus the Reassign and Fork handlers — all three contain `for w in &regen_warnings` loops; the source-order test must anchor to the Remove handler so it is not affected by Reassign/Fork ordering)
  </read_first>
  <behavior>
    - Test 1 (`remove_retry_succeeds_after_failure_resolved`): TEST-02 / P2. End-to-end retention contract:
      1. Set up source dir + target dir, `tome sync --no-triage` to prime.
      2. chmod 0o500 the target dir.
      3. `tome remove local --force` → expect exit failure, ⚠ marker, config entry preserved.
      4. chmod 0o755 the target dir (restore).
      5. `tome remove local --force` → expect exit success, success banner present.
      6. Assert config entry for `local` is gone (`tome.toml` no longer contains `[directories.local]`).
      7. Assert manifest no longer contains the `my-skill` entry.
      8. Assert library dir for `my-skill` is gone (`<tome_home>/library/my-skill` does not exist).
    - Test 2 (`lib_rs_remove_handler_prints_success_banner_before_regen_warnings`): TEST-04 / P4 regression. Source-order assertion: the `println!("Removed directory ...")` MUST appear earlier in `crates/tome/src/lib.rs` than the `for w in &regen_warnings` loop in the SAME `Command::Remove` handler region.

      **Anchor design:** `lib.rs` contains THREE `for w in &regen_warnings` loops — one each in Remove, Reassign, and Fork handlers. A naïve `lib_rs.find("for w in &regen_warnings")` returns the FIRST match (currently Remove, but only by coincidence of source ordering). To prevent a false-positive failure if a future refactor reorders the handlers (e.g., moves Reassign above Remove, or introduces a new handler with its own regen_warnings loop above Remove), the test FIRST locates the byte index of `"Command::Remove"` and uses that as the search start for both `"Removed directory"` and `"for w in &regen_warnings"`. This guarantees the test fails ONLY when the Remove handler itself regresses.

      We assert at the source level (file byte-position) rather than at the process-output level because stdout vs stderr ordering is determined by terminal interleaving, not by Rust flush order — assert_cmd captures them as separate streams and gives us no temporal ordering signal.

      ```rust
      #[test]
      fn lib_rs_remove_handler_prints_success_banner_before_regen_warnings() {
          // TEST-04 / P4 regression: pin the source-order in lib.rs Command::Remove
          // happy-path. The success banner `println!("Removed directory ...")` MUST
          // appear earlier in the file than the `for w in &regen_warnings ... eprintln!`
          // loop. If a future refactor reorders these, this test fails.
          //
          // ANCHORING: lib.rs contains three `for w in &regen_warnings` loops —
          // one each in Remove, Reassign, Fork handlers. Without anchoring to
          // `Command::Remove` first, a future reorder of Reassign or Fork (or
          // a new handler inserted above Remove with its own regen-warnings
          // loop) could create a false-positive failure unrelated to Remove.
          // We anchor all subsequent searches to `region_start` to keep the
          // test focused on the Remove handler contract.

          let lib_rs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
          let lib_rs = std::fs::read_to_string(&lib_rs_path)
              .unwrap_or_else(|e| panic!("lib.rs must exist at {}: {e}", lib_rs_path.display()));

          let region_start = lib_rs
              .find("Command::Remove")
              .expect("lib.rs must contain `Command::Remove` handler");

          let banner_offset = lib_rs[region_start..]
              .find("Removed directory")
              .expect("✓ Removed directory banner must appear inside Command::Remove region");
          let banner_idx = region_start + banner_offset;

          let warnings_offset = lib_rs[region_start..]
              .find("for w in &regen_warnings")
              .expect("regen_warnings loop must appear inside Command::Remove region");
          let warnings_idx = region_start + warnings_offset;

          assert!(
              banner_idx < warnings_idx,
              "TEST-04 option a: `Removed directory` banner (byte {}) MUST precede `for w in &regen_warnings` loop (byte {}) inside the Command::Remove handler region (starts at byte {})",
              banner_idx,
              warnings_idx,
              region_start,
          );
      }
      ```

      This lives in `tests/cli.rs` for locality with the other `remove_*` tests. It runs in <10ms with no fixture cost.

      The TEST-02 retry test stays as the integration test it should be.
  </behavior>
  <action>
**Step 1 — Add `remove_retry_succeeds_after_failure_resolved`** to `crates/tome/tests/cli.rs` immediately after `remove_partial_failure_does_not_save_disk_state` (~line 3540). Mirror the fixture pattern from the existing test:

```rust
#[cfg(unix)]
#[test]
fn remove_retry_succeeds_after_failure_resolved() {
    use std::os::unix::fs::PermissionsExt;

    // TEST-02 / P2: end-to-end I2/I3 retention contract.
    //   1. Partial failure → config entry + manifest preserved (existing v0.8 contract)
    //   2. User fixes the underlying condition (chmod 0o755)
    //   3. Second `tome remove` succeeds, leaves NO leftover state
    //
    // Without this test, the retry path is only exercised by manual UAT.
    // A future refactor that mutates config/manifest on the failure path
    // (regressing #461 H2) would silently break retry — the second
    // `tome remove` would fail with "directory not found in config".

    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // Prime: sync to wire library + target symlink.
    tome()
        .args(["--tome-home", tmp.path().to_str().unwrap(), "sync", "--no-triage"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    assert!(target_dir.join("my-skill").exists(), "fixture: target symlink must exist after sync");

    // Step 1 — partial failure: chmod 0o500 on target dir.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let first = tome()
        .args(["--tome-home", tmp.path().to_str().unwrap(), "remove", "local", "--force"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(!first.status.success(), "first remove must fail on chmod 0o500");
    let first_stderr = String::from_utf8_lossy(&first.stderr);
    assert!(first_stderr.contains("⚠"), "first remove stderr missing ⚠ marker: {first_stderr}");

    // Step 1.5 — assert config entry preserved (I2 retention).
    let config_after_fail = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        config_after_fail.contains("[directories.local]"),
        "config entry for 'local' must be preserved on partial failure; got: {config_after_fail}"
    );

    // Step 2 — user fixes the underlying cause.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Step 3 — retry: second `tome remove` should succeed cleanly.
    let second = tome()
        .args(["--tome-home", tmp.path().to_str().unwrap(), "remove", "local", "--force"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        second.status.success(),
        "retry remove must succeed after chmod restore; stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        second_stdout.contains("Removed directory"),
        "retry stdout must contain success banner; got: {second_stdout}"
    );

    // Step 4 — assert clean state: no config entry, no manifest entry, no library dir.
    let config_after_success = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        !config_after_success.contains("[directories.local]"),
        "config entry for 'local' must be removed after retry success; got: {config_after_success}"
    );

    let manifest_path = tmp.path().join(".tome-manifest.json");
    if manifest_path.exists() {
        let manifest = std::fs::read_to_string(&manifest_path).unwrap();
        assert!(
            !manifest.contains("\"my-skill\""),
            "manifest must not contain my-skill after retry success; got: {manifest}"
        );
    }

    let library_skill = tmp.path().join("library").join("my-skill");
    assert!(
        !library_skill.exists(),
        "library dir for my-skill must be gone after retry success; still exists at {}",
        library_skill.display()
    );
}
```

**Step 2 — Add `lib_rs_remove_handler_prints_success_banner_before_regen_warnings`** in the same file (place near the other `remove_*` tests, ~line 3540 after the test from Step 1). The test ANCHORS its `String::find()` searches to the `Command::Remove` handler region in `lib.rs` so a future reorder of the Reassign or Fork handlers (each of which contains its own `for w in &regen_warnings` loop) cannot create a false-positive failure unrelated to the Remove ordering contract:

```rust
#[test]
fn lib_rs_remove_handler_prints_success_banner_before_regen_warnings() {
    // TEST-04 / P4 regression: pin the source-order in lib.rs Command::Remove
    // happy-path. The success banner `println!("Removed directory ...")` MUST
    // appear earlier in the file than the `for w in &regen_warnings ... eprintln!`
    // loop. If a future refactor reorders these, this test fails.
    //
    // ANCHORING: lib.rs contains three `for w in &regen_warnings` loops —
    // one each in Remove, Reassign, Fork handlers. Without anchoring to
    // `Command::Remove` first, a future reorder of Reassign or Fork (or
    // a new handler inserted above Remove with its own regen-warnings
    // loop) could create a false-positive failure unrelated to Remove.
    // We anchor all subsequent searches to `region_start` to keep the
    // test focused on the Remove handler contract.
    //
    // We assert at the source level (file byte-position) rather than at the
    // process-output level because stdout vs stderr ordering is determined
    // by terminal interleaving, not by Rust flush order — assert_cmd captures
    // them as separate streams and gives us no temporal ordering signal.

    let lib_rs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let lib_rs = std::fs::read_to_string(&lib_rs_path)
        .unwrap_or_else(|e| panic!("lib.rs must exist at {}: {e}", lib_rs_path.display()));

    let region_start = lib_rs
        .find("Command::Remove")
        .expect("lib.rs must contain `Command::Remove` handler");

    let banner_offset = lib_rs[region_start..]
        .find("Removed directory")
        .expect("✓ Removed directory banner must appear inside Command::Remove region");
    let banner_idx = region_start + banner_offset;

    let warnings_offset = lib_rs[region_start..]
        .find("for w in &regen_warnings")
        .expect("regen_warnings loop must appear inside Command::Remove region");
    let warnings_idx = region_start + warnings_offset;

    assert!(
        banner_idx < warnings_idx,
        "TEST-04 option a: `Removed directory` banner (byte {}) MUST precede `for w in &regen_warnings` loop (byte {}) inside the Command::Remove handler region (starts at byte {})",
        banner_idx,
        warnings_idx,
        region_start,
    );
}
```

Run:
```bash
cargo test -p tome --test cli remove_retry_succeeds_after_failure_resolved
cargo test -p tome --test cli lib_rs_remove_handler_prints_success_banner_before_regen_warnings
```
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo test -p tome --test cli remove_retry_succeeds_after_failure_resolved 2>&1 | tail -10 && cargo test -p tome --test cli lib_rs_remove_handler_prints_success_banner_before_regen_warnings 2>&1 | tail -10</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "fn remove_retry_succeeds_after_failure_resolved" crates/tome/tests/cli.rs` returns exactly 1 match.
    - `rg -n "fn lib_rs_remove_handler_prints_success_banner_before_regen_warnings" crates/tome/tests/cli.rs` returns exactly 1 match.
    - `rg -n "Command::Remove" crates/tome/tests/cli.rs` returns at least 1 match (the anchor literal in the source-order test).
    - `rg -n "region_start" crates/tome/tests/cli.rs` returns at least 3 matches (variable declaration + 2 reuses for banner_idx and warnings_idx — proves the anchoring shape was implemented).
    - `cargo test -p tome --test cli remove_retry_succeeds_after_failure_resolved` passes.
    - `cargo test -p tome --test cli lib_rs_remove_handler_prints_success_banner_before_regen_warnings` passes.
    - `cargo test -p tome --test cli remove_partial_failure_exits_nonzero_with_warning_marker` still passes (regression).
    - `cargo test -p tome --test cli remove_partial_failure_does_not_save_disk_state` still passes (regression).
    - `make ci` passes.
  </acceptance_criteria>
  <done>
    `remove_retry_succeeds_after_failure_resolved` exercises the I2/I3 retention contract end-to-end (partial failure → fix → retry succeeds → no leftover state). `lib_rs_remove_handler_prints_success_banner_before_regen_warnings` pins the deferred-warnings ordering at source-byte level, anchored to the `Command::Remove` handler region so reorders of Reassign/Fork cannot create a false positive. Existing partial-failure tests still pass.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome remove::tests::failure_kind_all_length_matches_variant_count` — passes.
- `cargo test -p tome remove::tests::failure_kind_all_ordering_pinned` — passes.
- `cargo test -p tome remove::tests::remove_failure_new_relative_path_panics_in_debug` — passes.
- `cargo test -p tome remove::tests::remove_failure_new_absolute_path_succeeds` — passes.
- `cargo test -p tome --test cli remove_partial_failure_exits_nonzero_with_warning_marker` — passes (with new banner-absence asserts).
- `cargo test -p tome --test cli remove_retry_succeeds_after_failure_resolved` — passes (new e2e test).
- `cargo test -p tome --test cli lib_rs_remove_handler_prints_success_banner_before_regen_warnings` — passes (source-order regression, anchored to Command::Remove region).
- `cargo test -p tome --test cli remove_partial_failure_does_not_save_disk_state` — passes (regression).
- `make ci` — clean.
- `rg -nU "Removed directory.*\\n(.*\\n){0,8}for w in &regen_warnings" crates/tome/src/lib.rs` — at least 1 match.
- `rg -nU "for w in &regen_warnings.*\\n(.*\\n){0,8}Removed directory" crates/tome/src/lib.rs` — 0 matches (within the Command::Remove region — Reassign / Fork are unaffected and may legally have warnings before/after their own banners).
</verification>

<success_criteria>
- `FailureKind::ALL` cannot drift from the enum: `_ensure_failure_kind_all_exhaustive` const fn forces a compile error if a new variant is added without updating ALL (POLISH-04 option c).
- `RemoveFailure::new` carries `debug_assert!(path.is_absolute(), ...)` (POLISH-05 option a).
- `tome remove` happy-path success banner appears BEFORE the `regen_warnings` print loop in `lib.rs` (TEST-04 option a — deferred warnings).
- `remove_partial_failure_exits_nonzero_with_warning_marker` asserts the success banner is absent from BOTH stdout AND stderr on partial failure (TEST-01 / P1).
- `remove_retry_succeeds_after_failure_resolved` exercises the full I2/I3 retention contract: partial failure preserves config + manifest → user fixes condition → retry succeeds with no leftover state (TEST-02 / P2).
- `lib_rs_remove_handler_prints_success_banner_before_regen_warnings` pins the source-byte order so a future reorder regression is caught at test time. The test ANCHORS its searches to the `Command::Remove` handler region so reorders of unrelated handlers (Reassign / Fork — each with its own regen_warnings loop) cannot create a false-positive failure (TEST-04 regression guard).
</success_criteria>

<output>
After completion, create `.planning/phases/10-phase-8-review-tail/10-02-SUMMARY.md` recording:
- POLISH-04 option chosen: (c) exhaustive-match sentinel — `_ensure_failure_kind_all_exhaustive` + `const _: () = { assert!(...) }`.
- POLISH-05 option chosen: (a) keep `new()` + add `debug_assert!`.
- TEST-04 option chosen: (a) deferred warnings (success banner first).
- New test names (4 in remove::tests, 2 in tests/cli.rs).
- Updated test name (`remove_partial_failure_exits_nonzero_with_warning_marker` gains banner-absence asserts).
- Drift-guard manual verification: confirm a temporary `FailureKind::Bogus` triggers the compile error (revert before commit).
- One-line confirmation: POLISH-04 + POLISH-05 + TEST-01 + TEST-02 + TEST-04 closed.
</output>

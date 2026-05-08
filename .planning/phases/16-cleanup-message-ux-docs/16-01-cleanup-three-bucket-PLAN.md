---
phase: 16-cleanup-message-ux-docs
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/cleanup.rs
  - crates/tome/src/lib.rs
autonomous: true
requirements:
  - UX-01

must_haves:
  truths:
    - "`tome sync` cleanup output renders three distinct buckets — removed-from-config, missing-from-disk, now-in-exclude-list — each with its own header line, count, and per-skill lines that include an inline actionable resolution hint."
    - "The literal phrase `no longer configured` does not appear anywhere in `cleanup.rs` (it is the trigger phrase being rewritten away — see CONTEXT.md `<specifics>`)."
    - "All three bucket headers, per-skill lines, and inline hints write to stderr (no `println!` in the output paths) per D-UX01-4 / HARD-15 stderr discipline."
    - "Library content for Bucket A entries (removed-from-config) is preserved on disk and the manifest entry transitions to Unowned (`source_name = None`, `previous_source = Some(...)`); Bucket B entries (missing-from-disk) get deleted as today; Bucket C entries (now-in-exclude-list) lose their distribution symlinks but the library copy is preserved."
    - "`make test -- cleanup` passes; the existing 11 cleanup unit tests continue to pass (LIB-04 invariants preserved)."
  artifacts:
    - path: "crates/tome/src/cleanup.rs"
      provides: "Three-bucket partition: rewritten user-facing output across `cleanup_library` and `cleanup_target`; coordinator (struct or side-channel) collecting Bucket C entries from `cleanup_target` for the unified renderer; per-bucket header + per-skill line + inline hint format."
      contains: "removed-from-config"
    - path: "crates/tome/src/lib.rs"
      provides: "`sync()` invocation site updated to thread the Bucket C coordinator from `cleanup_target` calls back into the unified cleanup renderer (or otherwise wire the chosen coordination mechanism per D-UX01-2)."
  key_links:
    - from: "crates/tome/src/cleanup.rs::cleanup_library"
      to: "stderr renderer for Buckets A + B"
      via: "eprintln! (no println!)"
      pattern: "eprintln!.*removed.from.config|eprintln!.*missing.from.disk"
    - from: "crates/tome/src/cleanup.rs::cleanup_target (or sibling Bucket C path)"
      to: "stderr renderer for Bucket C"
      via: "eprintln! and a coordination shape (CleanupSummary struct or Vec<ExcludedSkill> side-channel)"
      pattern: "eprintln!.*exclude.list|now.in.exclude"
    - from: "crates/tome/src/lib.rs::sync"
      to: "Bucket C coordination — `cleanup_target` and `cleanup_disabled_from_target` either return or write into a shared structure that the unified renderer drains"
      via: "function-signature change OR side-channel parameter"
      pattern: "CleanupSummary|excluded_skills|now_in_exclude"
---

<objective>
Rewrite `tome sync`'s cleanup output into three actionable buckets per UX-01 and CONTEXT.md D-UX01-1..-4. Today's output is split between `cleanup_library` (two-bucket: Case 1 unowned-transition + Case 2 missing-from-disk; `cleanup.rs:43`) and `cleanup_target` (silent removal of stale exclude-listed symlinks; `cleanup.rs:236` + `lib.rs::cleanup_disabled_from_target` at `lib.rs:1773`). After this plan: one unified user-facing surface with three named buckets, each with a header, count, and per-skill lines carrying an inline actionable hint. All output goes to stderr.

The literal phrase "no longer configured" — the trigger of the entire v0.10 milestone discussion — must not appear in any bucket header, per-skill line, or hint string in `cleanup.rs`. CONTEXT.md `<specifics>` explicitly flags this.

Purpose: closes UX-01. Surface to the user the three distinct partial-cleanup mechanisms tome already runs, with directly-actionable resolution hints (re-add the directory; restore the file; remove the entry from the exclude list) so the message stops conflating distinct situations.

Output: `crates/tome/src/cleanup.rs` rewrites + a glue change in `lib.rs::sync` (around `lib.rs:1633`–`lib.rs:1689`) to coordinate Bucket C from the distribution-cleanup loop into the same renderer.
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
@.planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md

@crates/tome/src/cleanup.rs
@crates/tome/src/lib.rs

<interfaces>
<!-- Key types and call sites the executor needs. Extracted from the codebase. -->

From crates/tome/src/cleanup.rs (TODAY's shape):

```rust
pub struct CleanupResult {
    pub removed_from_library: usize,
    /// Skills transitioned from owned -> Unowned (Case 1 of LIB-04 / D-09).
    pub transitioned_to_unowned: usize,
}

pub fn cleanup_library(
    library_dir: &Path,
    discovered_names: &HashSet<String>,
    manifest: &mut Manifest,
    config: &crate::config::Config,
    dry_run: bool,
    quiet: bool,
    no_input: bool,
) -> Result<CleanupResult>;

pub fn cleanup_target(
    target_dir: &Path,
    library_dir: &Path,
    dry_run: bool,
) -> Result<usize>;  // returns count of removed symlinks
```

From crates/tome/src/lib.rs (TODAY's call site, around line 1633 and 1685):

```rust
let cleanup_result = cleanup::cleanup_library(
    paths.library_dir(),
    &discovered_names,
    &mut manifest,
    config,
    dry_run,
    quiet,
    no_input,
)?;
// ...
for (_name, dir_config) in config.distribution_dirs() {
    let skills_dir = &dir_config.path;
    removed_from_targets += cleanup::cleanup_target(skills_dir, paths.library_dir(), dry_run)?;
    // Also clean up symlinks for disabled skills
    removed_from_targets +=
        cleanup_disabled_from_target(skills_dir, paths.library_dir(), &machine_prefs, dry_run)?;
}
```

`cleanup_disabled_from_target` (lib.rs:1773) is the function that today silently removes symlinks for skills now in `machine_prefs.is_disabled(...)`. Bucket C surfaces those removals.

From crates/tome/src/manifest.rs (consumed for previous_source rendering):

```rust
pub struct SkillEntry {
    pub source_name: Option<DirectoryName>,
    pub previous_source: Option<DirectoryName>,
    // ...
}
```

From CONTEXT.md D-UX01-3 (illustrative output shape — exact wording is Claude's discretion):

```
3 skills no longer in any source (preserving as Unowned):
  foo (was: my-old-dir) — re-add my-old-dir, or run `tome reassign foo --to <dir>`
  bar (was: my-old-dir) — re-add my-old-dir, or run `tome reassign bar --to <dir>`
  baz (was: another-dir) — re-add another-dir, or run `tome reassign baz --to <dir>`

1 skill missing from configured source on disk (removing from library):
  qux (from: my-current-dir) — restore the file, or run `tome remove skill qux`

2 skills now in exclude list (distribution symlinks removed; library preserved):
  quux (excluded globally) — remove `quux` from `machine.toml::disabled` to re-distribute
  corge (excluded for: my-dir) — remove `corge` from `machine.toml::directories.my-dir.disabled` to re-distribute
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Rewrite cleanup_library to render Buckets A + B with per-skill inline hints (stderr)</name>
  <files>crates/tome/src/cleanup.rs</files>
  <read_first>
    - crates/tome/src/cleanup.rs (current shape — both `cleanup_library` and `cleanup_target` plus the 11 unit tests at the bottom; LIB-04 invariants are encoded in those tests and MUST keep passing)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-UX01-1, D-UX01-2, D-UX01-3, D-UX01-4 — full bucket definitions, output shape, stderr discipline)
    - crates/tome/src/manifest.rs (SkillEntry shape; previous_source field is the rendering target for Bucket A's "was: <dir>" attribution)
  </read_first>
  <behavior>
    - Test 1 (existing test must still pass): `cleanup_transitions_orphaned_to_unowned_when_source_removed_from_config` continues to pass (counts + manifest mutation unchanged for Bucket A).
    - Test 2 (existing test must still pass): `cleanup_case2_deletes_when_source_still_configured` continues to pass (Bucket B: library content removed).
    - Test 3 (existing test must still pass): `cleanup_already_unowned_entry_is_preserved_and_not_counted` (already-Unowned entries still skipped from stale set).
    - Test 4 (existing test must still pass): `cleanup_dry_run_does_not_mutate_manifest_for_unowned_transition` (Case 1 dry-run behavior intact).
    - Test 5 (NEW): When stale set partitions across both buckets (1 Case-1 + 1 Case-2 + same as `cleanup_case1_and_case2_in_same_run` test), the rendered output must be writable to stderr only — no `println!` calls in the new render path. Use a helper `fn render_buckets_to_writer(writer: &mut impl Write, ...)` so a `Vec<u8>` writer can capture output for assertion.
    - Test 6 (NEW): Render output for a Bucket A entry includes the actionable hint substring "tome reassign" (or equivalent locked verb from D-API-1 vocab).
    - Test 7 (NEW): Render output for a Bucket B entry includes the actionable hint substring "tome remove skill" (or equivalent locked verb from D-API-2 vocab).
    - Test 8 (NEW): Render output for a Bucket A entry NEVER contains the substring "no longer configured".
  </behavior>
  <action>
    Rewrite `cleanup_library` (cleanup.rs:43) to produce a three-bucket-aware render path. Concrete steps:

    **Step 1: Extend `CleanupResult` (cleanup.rs:16) to carry per-bucket detail for the unified renderer.** Add fields:
    ```rust
    pub struct CleanupResult {
        pub removed_from_library: usize,
        pub transitioned_to_unowned: usize,
        /// NEW: Bucket A entries (removed-from-config) for unified rendering.
        /// Each tuple: (skill name, last-known source name).
        pub bucket_a_removed_from_config: Vec<(SkillName, DirectoryName)>,
        /// NEW: Bucket B entries (missing-from-disk) for unified rendering.
        /// Each tuple: (skill name, currently-configured source name).
        pub bucket_b_missing_from_disk: Vec<(SkillName, DirectoryName)>,
    }
    ```
    Mark fields `pub` so the lib.rs renderer can inspect them.

    **Step 2: Populate the new fields.** In the existing Case 1 loop (cleanup.rs:103) push `(name.clone(), prev_source)` into `result.bucket_a_removed_from_config`. In the Case 2 deletion loop (cleanup.rs:188) push `(name.clone(), source_name_from_config)` into `result.bucket_b_missing_from_disk`. Today's `eprintln!` lines that emitted Case 1 info text (cleanup.rs:110) and Case 2 warning text (cleanup.rs:178) MUST be removed from `cleanup_library` and replaced by the unified renderer in step 4. Today's interactive Case 2 prompt (cleanup.rs:146-169) becomes unconditional `dialoguer::Confirm` (it already writes to stderr); the `println!` header above it (cleanup.rs:147-154) is dropped (the renderer takes over the user-facing summary).

    **Step 3: Add a `pub(crate) fn render_cleanup_buckets(...)` helper in cleanup.rs.** Signature:
    ```rust
    pub(crate) fn render_cleanup_buckets(
        writer: &mut impl std::io::Write,
        bucket_a: &[(SkillName, DirectoryName)],
        bucket_b: &[(SkillName, DirectoryName)],
        bucket_c: &[crate::cleanup::ExcludedSkill],
    ) -> std::io::Result<()>;
    ```
    Emit each non-empty bucket as: blank line separator, colored bold header line with count and bucket-distinct phrasing, then per-skill lines with the inline actionable hint. Use `console::style(...).yellow().bold()` for headers and `console::style(...).dim()` for source-attribution parens (matches today's `cleanup.rs:148-158`). Bucket-distinct header phrasing (NOT "no longer configured" — that phrase is forbidden):
    - Bucket A header literal substring: `removed from config` or `no longer in any source` (D-UX01-3 example uses the latter)
    - Bucket B header literal substring: `missing from configured source on disk` or `missing from disk`
    - Bucket C header literal substring: `now in exclude list`

    Per-skill line format follows D-UX01-3 illustrative shape:
    - Bucket A: `<name> (was: <prev_source>) — re-add <prev_source>, or run \`tome reassign <name> --to <dir>\``
    - Bucket B: `<name> (from: <source>) — restore the file, or run \`tome remove skill <name>\``
    - Bucket C: `<name> (excluded globally)` OR `<name> (excluded for: <dir>)` — followed by ` — remove \`<name>\` from \`machine.toml::disabled\` to re-distribute` OR ` — remove \`<name>\` from \`machine.toml::directories.<dir>.disabled\` to re-distribute`.

    (Exact wording within the Conflict/Why/Suggestion shape is Claude's discretion per CONTEXT.md `<decisions>` "Claude's Discretion" — the test acceptance criteria below pin the load-bearing substrings.)

    **Step 4: Add a public `pub struct ExcludedSkill` in cleanup.rs** (consumed by lib.rs Bucket C path, populated in Task 2):
    ```rust
    #[derive(Debug, Clone)]
    pub struct ExcludedSkill {
        pub name: SkillName,
        /// None = excluded globally via `machine.toml::disabled`.
        /// Some = excluded for a specific directory via per-dir `directories.<dir>.disabled`.
        pub directory: Option<DirectoryName>,
    }
    ```

    **Step 5: Remove the today's interactive `println!` block (cleanup.rs:146-165) and replace with `dialoguer::Confirm` directly** (default false, matches today's behavior; writes to stderr by default). The unified renderer in Task 3 (lib.rs) will print the buckets BEFORE this prompt, so the user sees the three buckets first, then the deletion confirmation for Bucket B if interactive.

    **Step 6: Verify ZERO uses of `println!` in cleanup.rs after the rewrite.** Every output line must be `eprintln!` or written through the renderer's writer. Run `rg -n 'println!' crates/tome/src/cleanup.rs` and confirm 0 hits.
  </action>
  <verify>
    <automated>cargo test -p tome --lib cleanup::tests</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'println!' crates/tome/src/cleanup.rs` outputs zero matches (stderr discipline per D-UX01-4)
    - `rg -n 'no longer configured' crates/tome/src/cleanup.rs` outputs zero matches (forbidden phrase per CONTEXT.md `<specifics>`)
    - `rg -n 'pub fn render_cleanup_buckets|pub\(crate\) fn render_cleanup_buckets' crates/tome/src/cleanup.rs` outputs at least one match (renderer exists)
    - `rg -n 'pub struct ExcludedSkill' crates/tome/src/cleanup.rs` outputs at least one match (Bucket C carrier type)
    - `rg -n 'bucket_a_removed_from_config|bucket_b_missing_from_disk' crates/tome/src/cleanup.rs` outputs at least two matches (CleanupResult fields)
    - `cargo test -p tome --lib cleanup::tests` exits 0
    - `cargo clippy -p tome --lib --tests -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `cleanup.rs` produces three-bucket-ready render output via a writer-based helper; `CleanupResult` carries Bucket A and Bucket B detail; `ExcludedSkill` type ready for lib.rs to populate; no `println!` calls; no "no longer configured" string anywhere in the file; all 11 existing tests still pass; new tests pinning the bucket-distinct substrings pass.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Wire Bucket C surfacing through lib.rs::sync via cleanup_disabled_from_target</name>
  <files>crates/tome/src/lib.rs, crates/tome/src/cleanup.rs</files>
  <read_first>
    - crates/tome/src/lib.rs (lines 1620-1710 — the cleanup invocation block in `sync()`; lines 1763-1825 — the `cleanup_disabled_from_target` helper)
    - crates/tome/src/cleanup.rs (the `ExcludedSkill` struct + `render_cleanup_buckets` helper from Task 1)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-UX01-1 Bucket C scope: "library skills whose distribution symlinks were just removed because the user added them to `machine.toml::disabled`, `disabled_directories`, or per-directory `directories.<name>.disabled`"; D-UX01-2 coordination shape)
    - crates/tome/src/machine.rs (MachinePrefs::is_disabled and per-directory disabled-list APIs — Bucket C must distinguish global vs per-directory exclusion)
  </read_first>
  <behavior>
    - Test 1 (NEW integration test in tests/cli_sync.rs OR cleanup unit test): A skill globally disabled in `machine.toml::disabled` with an existing distribution symlink: after `tome sync` completes, the symlink is gone AND a stderr line containing both the skill name and the substring "exclude" or "machine.toml::disabled" is emitted.
    - Test 2 (NEW): A skill disabled per-directory via `directories.<dir>.disabled`: after `tome sync`, the per-directory symlink is gone AND a stderr line names both the skill and the directory it was excluded for.
    - Test 3: dry-run preserves filesystem (existing `cleanup_target_dry_run_preserves_stale_links` continues to pass) AND still reports the would-be Bucket C entry in the rendered output.
  </behavior>
  <action>
    **Step 1: Modify `cleanup_disabled_from_target` (lib.rs:1773) to return both the count AND the populated Vec<ExcludedSkill>.** Change signature from `Result<usize>` to `Result<(usize, Vec<cleanup::ExcludedSkill>)>`. Inside the function, when a symlink is removed because `machine_prefs.is_disabled(...)` returned true, push an `ExcludedSkill { name: SkillName::new(&name)?, directory: None }` for global disable. For per-directory disable detection, also check `machine_prefs.directory_disabled_skills(...)` (or the equivalent API in machine.rs) and use `directory: Some(dir_name.clone())`. If a skill is both globally and per-directory disabled, prefer global (matches `MachinePrefs::is_skill_allowed` precedence).

    **Step 2: Update the call site in `sync()` (lib.rs:1683-1689) to collect the Vec<ExcludedSkill> across all distribution directories:**
    ```rust
    let mut excluded_skills: Vec<cleanup::ExcludedSkill> = Vec::new();
    let mut removed_from_targets = 0usize;
    for (_name, dir_config) in config.distribution_dirs() {
        let skills_dir = &dir_config.path;
        removed_from_targets += cleanup::cleanup_target(skills_dir, paths.library_dir(), dry_run)?;
        let (n, excluded) = cleanup_disabled_from_target(
            skills_dir, paths.library_dir(), &machine_prefs, dry_run
        )?;
        removed_from_targets += n;
        excluded_skills.extend(excluded);
    }
    ```

    **Step 3: After both cleanup phases complete and BEFORE the save chain (lib.rs:1691), invoke the unified renderer:**
    ```rust
    if !quiet {
        let mut stderr = std::io::stderr().lock();
        let _ = cleanup::render_cleanup_buckets(
            &mut stderr,
            &cleanup_result.bucket_a_removed_from_config,
            &cleanup_result.bucket_b_missing_from_disk,
            &excluded_skills,
        );
    }
    ```
    `_ =` is acceptable here — failure to write to stderr is non-fatal for sync (matches existing `eprintln!` semantics that ignore I/O errors).

    **Step 4: Audit and confirm no `println!` lines were introduced for cleanup output in lib.rs.** Bucket C output goes through the renderer; the per-distribution-dir spinner messages (lines 1654-1657, 1664) stay as today (they're dim verbose-mode chrome, unrelated to cleanup output).

    **Step 5: Update the function doc comment on `cleanup_disabled_from_target`** to describe the new return tuple and the Bucket C surfacing role. Mention the D-UX01-1 / D-UX01-2 contract.
  </action>
  <verify>
    <automated>cargo test -p tome cleanup &amp;&amp; cargo build -p tome</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'cleanup_disabled_from_target' crates/tome/src/lib.rs` shows the function returning `Result<(usize, Vec<cleanup::ExcludedSkill>)>` (or equivalent two-element tuple)
    - `rg -n 'render_cleanup_buckets' crates/tome/src/lib.rs` outputs at least one match (renderer is invoked from sync())
    - `rg -n 'excluded_skills' crates/tome/src/lib.rs` outputs at least one match (the Vec is being collected)
    - `cargo build -p tome` exits 0
    - `cargo test -p tome --lib cleanup` exits 0
    - `cargo clippy -p tome -- -D warnings` exits 0
    - Manual smoke test from `cargo run -p tome -- sync --dry-run` against a fixture with a globally-disabled skill emits a stderr line containing both the skill name and "exclude" (or equivalent Bucket C phrase locked in Task 1)
  </acceptance_criteria>
  <done>
    `lib.rs::sync` collects `ExcludedSkill` entries from all distribution directories during distribution cleanup, then renders all three buckets through the cleanup.rs renderer to stderr before the save chain runs. The exclusion-list cleanup is no longer silent: every globally- or per-directory-disabled skill that lost its distribution symlink shows up in Bucket C with the resolution hint pointing at the right `machine.toml` location.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Integration test pinning all three buckets render correctly with bucket-distinct phrasing</name>
  <files>crates/tome/tests/cli_sync.rs</files>
  <read_first>
    - crates/tome/tests/cli_sync.rs (existing integration tests; pattern for fixture setup via assert_cmd + tempfile)
    - crates/tome/tests/common/mod.rs (shared test helpers; check whether a fixture builder already exists for "library + manifest + machine.toml with a disabled skill")
    - crates/tome/src/cleanup.rs (the bucket-distinct header substrings landed in Task 1 — these are what the test asserts against)
  </read_first>
  <behavior>
    - Test 1: Build a fixture where (a) one skill's source dir was removed from `tome.toml` AND its manifest entry remains (Bucket A scenario), (b) one skill's source dir is still configured but the file vanished from disk (Bucket B scenario), (c) one skill is in `machine.toml::disabled` and has a distribution symlink (Bucket C scenario). Run `tome sync --no-input` against the fixture. Assert stderr contains all three bucket header substrings AND the three skill names. Assert stderr does NOT contain the substring "no longer configured".
  </behavior>
  <action>
    Add a single end-to-end integration test `cleanup_renders_all_three_buckets_with_distinct_phrasing` in `crates/tome/tests/cli_sync.rs`. Use `assert_cmd::Command::cargo_bin("tome")` and `tempfile::TempDir` per existing patterns. Wire the fixture:

    1. Create a tome_home dir + library subdir
    2. Pre-populate `.tome-manifest.json` with three skill entries:
       - `bucket-a-skill`: `source_name: "removed-source"`, `source_path: "/tmp/removed-source/bucket-a-skill"` (and create the library directory contents on disk so it appears as a real entry)
       - `bucket-b-skill`: `source_name: "active-source"`, `source_path: "<tmp>/active-source/bucket-b-skill"` (DO NOT create the source file on disk; this is the missing-from-disk scenario)
       - `bucket-c-skill`: `source_name: "active-source"`, real-dir copy in library (this skill IS still discovered)
    3. Write `tome.toml` with only `active-source` (i.e. NOT `removed-source`)
    4. Write `machine.toml` with `disabled = ["bucket-c-skill"]`
    5. Pre-create a distribution symlink for `bucket-c-skill` (so cleanup will tear it down and surface Bucket C)
    6. Run `tome sync --no-input` with the test tome_home env var
    7. Capture stderr; assert:
       - `stderr.contains("bucket-a-skill")` AND
       - `stderr.contains("bucket-b-skill")` AND
       - `stderr.contains("bucket-c-skill")` AND
       - `stderr.contains("removed from config") || stderr.contains("no longer in any source")` (Bucket A locked phrase from Task 1) AND
       - `stderr.contains("missing from")` (Bucket B locked phrase) AND
       - `stderr.contains("exclude list") || stderr.contains("now in exclude")` (Bucket C locked phrase) AND
       - `!stderr.contains("no longer configured")` (forbidden phrase per CONTEXT.md `<specifics>`)

    Use `predicates::str::contains` from the `predicates` crate (already a test dep) for assertion ergonomics. If no fixture builder exists in `tests/common/mod.rs`, inline the setup in the test — adding helper machinery is out of scope.
  </action>
  <verify>
    <automated>cargo test -p tome --test cli_sync cleanup_renders_all_three_buckets_with_distinct_phrasing</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'cleanup_renders_all_three_buckets_with_distinct_phrasing' crates/tome/tests/cli_sync.rs` outputs at least one match
    - `cargo test -p tome --test cli_sync cleanup_renders_all_three_buckets_with_distinct_phrasing` exits 0
    - The test asserts the absence of the forbidden phrase ("no longer configured") via `predicates::str::contains(...).not()` or `!stderr.contains(...)`
    - The test asserts the presence of all three bucket-distinct header phrases
  </acceptance_criteria>
  <done>
    A single failing-without-the-rewrite integration test in `cli_sync.rs` proves all three buckets render with distinct phrasing in a real `tome sync` invocation against a fixture. The forbidden "no longer configured" string never appears in cleanup output.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome --lib cleanup` passes (existing 11 tests + new render tests from Task 1)
- `cargo test -p tome --test cli_sync` passes (existing tests + new three-bucket integration test from Task 3)
- `make ci` (fmt-check + clippy -D warnings + tests) passes
- `rg -n 'no longer configured' crates/tome/src/cleanup.rs crates/tome/src/lib.rs` outputs zero matches
- `rg -n 'println!' crates/tome/src/cleanup.rs` outputs zero matches
</verification>

<success_criteria>
- UX-01 satisfied: `tome sync` cleanup output partitions stale-candidate skills into three named buckets with per-skill inline actionable hints
- LIB-04 invariants preserved: Bucket A library content stays on disk, manifest transitions to Unowned with `previous_source` recorded; Bucket C library content also preserved (only distribution symlinks change)
- D-UX01-4 stderr discipline honored: zero `println!` calls in cleanup output paths
- The literal "no longer configured" trigger phrase removed from `cleanup.rs`
- Existing 11 cleanup unit tests continue to pass (no LIB-04 regression)
- New integration test in `cli_sync.rs` fails on a build BEFORE this plan and passes AFTER
</success_criteria>

<output>
After completion, create `.planning/phases/16-cleanup-message-ux-docs/16-01-SUMMARY.md` documenting:
- Coordination shape chosen (CleanupResult fields vs. side-channel vs. shared CleanupSummary struct) and rationale
- Final bucket-header phrasing locked
- Per-skill hint phrasing locked for all three buckets
- Any changes to `CleanupResult` public fields (downstream consumers in `lib.rs::SyncReport` and JSON output may depend on these)
- Any deviations from the illustrative D-UX01-3 examples and why
</output>

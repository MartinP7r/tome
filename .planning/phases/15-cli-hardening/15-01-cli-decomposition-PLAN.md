---
phase: 15-cli-hardening
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/lib.rs
  - crates/tome/tests/cli.rs
  - crates/tome/tests/common/mod.rs
  - crates/tome/tests/cli_sync.rs
  - crates/tome/tests/cli_doctor.rs
  - crates/tome/tests/cli_remove.rs
  - crates/tome/tests/cli_reassign.rs
  - crates/tome/tests/cli_status.rs
  - crates/tome/tests/cli_browse.rs
  - crates/tome/tests/cli_init.rs
  - crates/tome/tests/cli_migrate_library.rs
autonomous: true
requirements:
  - HARD-02
  - HARD-13
must_haves:
  truths:
    - "lib.rs::run() dispatches each Command variant via a per-subcommand cmd_<name> helper, no match arm exceeds ~30 lines"
    - "Integration tests are split across per-domain cli_*.rs files plus tests/common/ helpers; the monolithic tests/cli.rs is removed (or reduced to a thin shim)"
    - "All existing integration tests still pass after the split (no behaviour change, only file boundaries change)"
  artifacts:
    - path: "crates/tome/src/lib.rs"
      provides: "Decomposed run() dispatch with cmd_<name> helpers"
      contains: "pub(crate) fn cmd_"
    - path: "crates/tome/tests/common/mod.rs"
      provides: "Shared fixtures, helpers, and assertions for split cli_*.rs files"
    - path: "crates/tome/tests/cli_remove.rs"
      provides: "Per-domain remove integration tests (HARD-11 lands here in 15-04)"
    - path: "crates/tome/tests/cli_sync.rs"
      provides: "Per-domain sync integration tests"
    - path: "crates/tome/tests/cli_doctor.rs"
      provides: "Per-domain doctor integration tests"
  key_links:
    - from: "crates/tome/src/lib.rs::run"
      to: "cmd_<name> helpers"
      via: "match arm dispatch on Command variant"
      pattern: "Command::\\w+\\([^)]*\\)\\s*=>\\s*cmd_\\w+\\("
    - from: "crates/tome/tests/cli_*.rs"
      to: "crates/tome/tests/common/mod.rs"
      via: "mod common;"
      pattern: "mod common"
---

<objective>
Decompose `lib.rs::run()` into per-subcommand `cmd_<name>` helpers (HARD-02, closes #486) and split the monolithic `tests/cli.rs` (6,703 LOC) into per-domain integration test files with shared `tests/common/` helpers (HARD-13, closes #499).

Purpose: Make `lib.rs::run()` legible (no single match arm exceeds ~30 lines) and unblock per-domain test maintenance. Reviewable diffs become per-command rather than 200-LOC match arms.
Output: Restructured `crates/tome/src/lib.rs` with `cmd_<name>` helpers; split `tests/cli_*.rs` files plus `tests/common/mod.rs`.
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

@crates/tome/src/lib.rs
@crates/tome/src/cli.rs
@crates/tome/tests/cli.rs
@crates/tome/tests/cli_sync_reconcile.rs

<interfaces>
<!-- Existing patterns to follow when extracting cmd_<name> helpers and splitting tests. -->

From crates/tome/src/cli.rs:
```rust
// Cli + Command enum (one variant per top-level subcommand). Each variant carries
// its own args struct (e.g. Command::Sync(SyncArgs), Command::Doctor(DoctorArgs)).
pub struct Cli { ... }
pub enum Command { Init(...), Sync(...), Status(...), Doctor(...), Lint(...),
                   Browse(...), Remove(...), Reassign(...), Fork(...), Add(...),
                   List(...), Eject(...), Relocate(...), MigrateLibrary(...),
                   Backup(...), Update(...), Version }
```

From crates/tome/src/lib.rs (current shape, line 164+):
```rust
pub fn run(cli: Cli) -> anyhow::Result<()> {
    // ... pre-dispatch setup (tome_home resolve, config load, etc.) ...
    match cli.command {
        Command::Sync(args) => { /* large inline body, often 30-100+ lines */ }
        Command::Doctor(args) => { /* large inline body */ }
        // ... etc for each variant
    }
}
```

Existing helper-extraction precedent (cli_sync_reconcile.rs already follows split pattern):
```rust
// crates/tome/tests/cli_sync_reconcile.rs
mod common;
use common::{Phase14Fixture, ...};
// per-test fns using assert_cmd
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Decompose lib.rs::run() into cmd_<name> helpers (HARD-02)</name>
  <files>crates/tome/src/lib.rs</files>
  <read_first>
    - crates/tome/src/lib.rs (current 2,251 LOC; entry at line 164, dispatch match starts ~line 325)
    - crates/tome/src/cli.rs (Command enum: enumerate every variant the dispatch must cover)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md §"HARD-02 cmd_<name> location" (Claude's Discretion: inline in lib.rs first)
    - .planning/REQUIREMENTS.md §"HARD-02"
  </read_first>
  <action>
    Refactor `pub fn run(cli: Cli) -> anyhow::Result<()>` so that each `Command::Foo(args) => { ... }` match arm becomes a one-line dispatch to a `pub(crate) fn cmd_foo(...)` helper defined later in `lib.rs`.

    Steps:

    1. Identify every `Command::*` variant in the dispatch match (read `cli.rs::Command` enum to get the exhaustive list — current variants include but are not limited to: Init, Sync, Status, Doctor, Lint, Browse, Remove, Reassign, Fork, Add, List, Eject, Relocate, MigrateLibrary, Backup, Update, Version). Confirm at least one `cmd_<name>` helper exists per variant.

    2. For each variant, extract the inline body into a `pub(crate) fn cmd_<name>(...) -> anyhow::Result<()>` defined later in `lib.rs`. Helper signature receives the variant's args struct plus whatever shared state the body needs (`paths: &TomePaths`, `config: &Config`, `machine_prefs: &MachinePrefs`, etc.) — pass by reference where possible. Helpers do NOT re-load config or paths; the caller (run) does that once.

    3. Reduce each `Command::Foo(args) => { ... }` arm to a single line: `Command::Foo(args) => cmd_foo(args, &paths, &config, ...)?` (or similar). NO match arm may exceed ~30 lines after the refactor.

    4. Preserve exact behaviour: `Command::Init` and `Command::Version` early-return shape (lib.rs lines 165-172) is preserved; pre-dispatch setup (tome_home resolve, config load, machine.toml load) stays in `run`.

    5. `lib.rs::run` after the refactor should read top-down as: pre-dispatch setup → match dispatch (each arm one line) → post-dispatch cleanup if any. Helper definitions follow `run` in the same file.

    6. Run `cargo check`, `cargo clippy --all-targets -- -D warnings`, and the full test suite to confirm behaviour-preservation (no test count change is expected for this task).

    Recommendation per CONTEXT.md "Claude's Discretion": **inline `cmd_<name>` in `lib.rs` first** (do not create a `commands/` module yet). If `lib.rs` is still >1,500 LOC after the refactor, lifting to a module is a follow-up — not a v0.10 blocker (deferred per CONTEXT.md "Deferred Ideas").
  </action>
  <verify>
    <automated>cargo build -p tome &amp;&amp; cargo clippy --all-targets -- -D warnings &amp;&amp; cargo test -p tome 2>&amp;1 | tee /tmp/15-01-task1.log</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "^pub(crate) fn cmd_" crates/tome/src/lib.rs` returns ≥ 14 (one helper per non-trivial Command variant; Init and Version may inline a `return Ok(())` early-return).
    - `grep -nE "^\s*Command::\w+\(" crates/tome/src/lib.rs` shows every dispatch arm; the body of each arm spans ≤ 30 lines (verify by visual inspection or `awk` counting between `Command::` arm starts).
    - `awk '/^\s*Command::\w+\(/,/^\s*}/' crates/tome/src/lib.rs | awk '/^\s*Command::\w+\(/{n++} END{print n}'` matches the variant count in cli.rs `Command` enum.
    - `cargo build -p tome` exits 0.
    - `cargo clippy --all-targets -- -D warnings` exits 0.
    - `cargo test -p tome` passes; baseline test count is preserved (this task is pure refactor — no new tests, no removed tests).
    - `lib.rs` LOC after refactor ≤ original (typical refactor reduces; allow ±5% drift).
  </acceptance_criteria>
  <done>
    `lib.rs::run` dispatch is a thin match where each arm is one line; per-subcommand bodies live in `cmd_<name>` helpers in the same file. Tests pass; clippy is clean.
  </done>
</task>

<task type="auto">
  <name>Task 2: Split tests/cli.rs into per-domain files with tests/common/ helpers (HARD-13)</name>
  <files>crates/tome/tests/cli.rs, crates/tome/tests/common/mod.rs, crates/tome/tests/cli_sync.rs, crates/tome/tests/cli_doctor.rs, crates/tome/tests/cli_remove.rs, crates/tome/tests/cli_reassign.rs, crates/tome/tests/cli_status.rs, crates/tome/tests/cli_browse.rs, crates/tome/tests/cli_init.rs, crates/tome/tests/cli_migrate_library.rs</files>
  <read_first>
    - crates/tome/tests/cli.rs (current 6,703 LOC monolith — read fully to map test functions to domains)
    - crates/tome/tests/cli_sync_reconcile.rs (already follows the split pattern — match its style)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md §"HARD-13 tests/cli.rs split granularity" (Claude's Discretion: per-domain files + tests/common/mod.rs)
    - .planning/REQUIREMENTS.md §"HARD-13"
    - .planning/phases/15-cli-hardening/15-CONTEXT.md §"Tests to write" (lists expected new test files)
  </read_first>
  <action>
    Split the monolithic `crates/tome/tests/cli.rs` (6,703 LOC) into per-domain integration test files plus a shared helpers module.

    Target file layout (slug names per CONTEXT.md `<canonical_refs>` "Codebase modules":

    ```
    crates/tome/tests/
      common/
        mod.rs              ← shared fixtures, helpers, assertions
      cli_sync.rs           ← tome sync, tome init (sync flavour)
      cli_doctor.rs         ← tome doctor + JSON output assertions
      cli_remove.rs         ← tome remove dir, tome remove skill (HARD-11 lands here in 15-04)
      cli_reassign.rs       ← tome reassign + tome fork
      cli_status.rs         ← tome status + JSON
      cli_browse.rs         ← tome browse smoke tests (interactive coverage in unit tests)
      cli_init.rs           ← tome init (wizard flavour, --no-input, --dry-run)
      cli_migrate_library.rs ← tome migrate-library (existing tests from cli.rs)
      cli_sync_reconcile.rs ← (existing — already split, leave in place)
    ```

    Steps:

    1. **Read `cli.rs` end-to-end** and tag each test function (`#[test] fn ...`) with its domain. Group helper functions (test fixtures, assert helpers like `Phase14Fixture`, env builders) by usage scope. Anything used by ≥2 future files → `common/mod.rs`. Anything single-use → goes with the test.

    2. **Create `tests/common/mod.rs`** containing the shared helpers. Add `#[allow(dead_code)]` on items not used by every consumer (cargo's `tests/common/mod.rs` convention requires this, since each `tests/cli_*.rs` file is a separate compilation unit and may not use every helper). Re-export commonly-used external types if convenient.

    3. **For each per-domain file**: copy the relevant `#[test]` fns from `cli.rs`. Add `mod common;` at the top and `use common::{Foo, Bar};` for needed helpers. Preserve test names verbatim — downstream tooling and CI logs reference them.

    4. **Delete or stub `tests/cli.rs`**: either remove the file entirely (preferred — `git rm`) or reduce it to a `// moved to per-domain cli_*.rs files (HARD-13)` comment block to flag the migration to future readers. Per CONTEXT.md, the deferred-items.md "Per-test tome binary pre-built once" perf optimisation is OUT OF SCOPE here — only revisit if measurement shows wall-time regression.

    5. **Verify test parity**: run the full test suite before and after; all tests that were named in `cli.rs` must still appear by name in the suite output. Use:
       ```
       cargo test -p tome --test cli_sync --test cli_doctor --test cli_remove --test cli_reassign --test cli_status --test cli_browse --test cli_init --test cli_migrate_library --test cli_sync_reconcile -- --list
       ```
       Compare to a pre-split snapshot of `cargo test -p tome --test cli -- --list`.

    6. Phase 14 forward-flagged HARD-13 hand-off — its integration tests landed in `tests/cli.rs` and need to be redistributed (per CONTEXT.md `<specifics>` "Phase 14 explicitly forward-flagged HARD-02, HARD-13, HARD-22"). Tag each Phase 14 test (look for `phase14_` prefix or related fixtures) into the matching per-domain file: `cli_remove.rs`, `cli_reassign.rs`, `cli_status.rs`, `cli_doctor.rs`.

    Note: HARD-11 (remove dir integration tests) and HARD-10 (overrides hostile-input tests) land in `cli_remove.rs` / new `cli_overrides.rs` in Plan 15-04; this task only creates the file scaffold + redistributes existing tests. New `tests/cli_overrides.rs` is created in Plan 15-04, not here.
  </action>
  <verify>
    <automated>cargo test -p tome --tests 2>&amp;1 | tee /tmp/15-01-task2.log &amp;&amp; cargo test -p tome --tests -- --list 2>&amp;1 | wc -l</automated>
  </verify>
  <acceptance_criteria>
    - `fd '^cli_.*\.rs$' crates/tome/tests` lists at least: `cli_sync.rs`, `cli_doctor.rs`, `cli_remove.rs`, `cli_reassign.rs`, `cli_status.rs`, `cli_browse.rs`, `cli_init.rs`, `cli_migrate_library.rs`, plus the existing `cli_sync_reconcile.rs`.
    - `crates/tome/tests/common/mod.rs` exists; `grep -l "mod common" crates/tome/tests/cli_*.rs` includes every new per-domain file.
    - `wc -l crates/tome/tests/cli.rs` returns 0 (file deleted) OR < 50 (file reduced to a stub comment).
    - `cargo test -p tome --tests` passes; total test count is ≥ baseline (no tests dropped). Track baseline via `cargo test -p tome --tests -- --list 2>&1 | grep -c '^test '` — record pre-split count, assert post-split ≥ pre-split.
    - `cargo clippy --all-targets -- -D warnings` exits 0.
    - Phase 14 integration tests (any `#[test] fn phase14_*`) appear in `cli_remove.rs` or `cli_reassign.rs` or `cli_status.rs` or `cli_doctor.rs` — NOT in any single dumping ground.
    - No test fn was renamed (test names preserved verbatim — verify with `cargo test -- --list | sort` diff before/after if a baseline was captured).
  </acceptance_criteria>
  <done>
    `tests/cli.rs` is split into per-domain `cli_*.rs` files using a shared `tests/common/mod.rs`. All tests still pass; clippy is clean; Phase 14's tests are correctly redistributed.
  </done>
</task>

</tasks>

<verification>
- `cargo build -p tome` exits 0
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo test -p tome` passes; total test count preserved (HARD-02 + HARD-13 are pure refactors — no test additions)
- `lib.rs::run` dispatch match: every arm ≤ 30 lines
- `tests/cli.rs` deleted or stubbed; `tests/cli_*.rs` per-domain files exist
- `tests/common/mod.rs` exists and is referenced by per-domain test files
</verification>

<success_criteria>
- HARD-02: `lib.rs::run()` decomposes into per-subcommand `cmd_<name>` helpers; no single match arm exceeds ~30 lines (closes #486)
- HARD-13: `tests/cli.rs` (6,703 LOC) splits into per-domain `cli_*.rs` files with shared `tests/common/` helpers; baseline test count preserved (closes #499)
- Both items are pure refactors — no behaviour change, no new tests required
- Builds on macOS + Linux CI green; clippy-D-warnings clean
</success_criteria>

<output>
After completion, create `.planning/phases/15-cli-hardening/15-01-SUMMARY.md` recording:
- Final lib.rs LOC (compare to pre-refactor 2,251)
- Number of `cmd_<name>` helpers extracted
- Per-domain test files created and their test counts
- Any helpers lifted to `tests/common/mod.rs`
- Issues closed: #486 (HARD-02), #499 (HARD-13)
</output>

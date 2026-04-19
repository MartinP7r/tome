# Phase 5: Wizard Test Coverage - Context

**Gathered:** 2026-04-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Close the testability gap left by Phase 4. The wizard is now correct (it refuses to save invalid configs) but its logic is hard to exercise without a TTY. Phase 5 delivers:

1. Unit tests for pure wizard helpers — `find_known_directories_in`, `KNOWN_DIRECTORIES` registry lookup, `DirectoryType::default_role`, and a newly-extracted config-assembly helper.
2. One integration test that runs `tome init --dry-run --no-input` and asserts the generated config passes `Config::validate()` and round-trips through TOML unchanged.
3. Coverage of every `(DirectoryType, DirectoryRole)` combination — valid combos save successfully, invalid combos fail at the Phase 4 validation path.
4. CI (ubuntu + macos) running these tests as non-optional gates.

Out of scope for this phase: ground-up wizard rewrite (WIZ-01–05; deferred), display polish via `tabled` (Phase 6), `tome init --no-input` saving real configs interactively (the new flag is wired so tests can drive it; broader `--no-input` UX is a future enhancement), env/stdin-driven wizard inputs.

</domain>

<decisions>
## Implementation Decisions

### `--no-input` Semantics

- **D-01:** `tome init --no-input` accepts the wizard's existing default at every prompt — no env var, no stdin, no scripting surface. Per-prompt defaults: include all auto-discovered `KNOWN_DIRECTORIES`, library = `~/.tome/skills`, no exclusions, no role edits, no custom dirs added, no `git init` for backup tracking.
- **D-02:** The hard-bail at `lib.rs:164-165` (`tome init requires interactive input — cannot use --no-input`) is removed. `--no-input` is already a global Cli flag (`cli.rs:43`); plumbing it into `wizard::run()` is a small, targeted change.
- **D-03:** `wizard::run()` signature gains a `no_input: bool` parameter (passed alongside the existing `dry_run: bool`). When `no_input` is true, every `dialoguer` call is replaced with the corresponding default — the assemble step still runs through the same code path that interactive mode uses, so behavior is uniform.
- **D-04:** `--no-input` and `--dry-run` are orthogonal. `--no-input` alone (without `--dry-run`) saves through `Config::save_checked` exactly as the interactive path does. The Phase 5 integration test always pairs them per WHARD-05's literal wording.

### Pure Helper Extraction

- **D-05:** Add `pub(crate) fn assemble_config(selected: &[(DirectoryName, DirectoryConfig)], library: PathBuf, exclude: BTreeSet<SkillName>) -> Config` (or equivalent) in `wizard.rs`. This pulls the inline assembly at `wizard.rs:421-436` and `wizard.rs:292-297` into one place that unit tests can call without dialoguer. Minimal extraction — `wizard::run()` still owns all interactive flow and just calls `assemble_config(...)` at the end.
- **D-06:** No `WizardInputs` struct in this phase. A struct-driven refactor lives closer to the deferred WIZ-01–05 rewrite and would expand the diff beyond what's needed for test coverage.

### Combo Coverage

- **D-07:** All 12 `(DirectoryType, DirectoryRole)` combinations are tested via a single table-driven test in `config.rs`. Mechanism is uniform: build a one-entry `Config` per combo, then assert that `Config::save_checked` (or `Config::validate` directly) either succeeds or fails per the expected outcome.
- **D-08:** The "wizard-producible" set is derived by iterating `DirectoryType::valid_roles()` rather than maintained as a separate hand-written list. This keeps the test honest: any change to `valid_roles()` automatically updates which combos are expected to pass vs fail.
- **D-09:** Invalid-combo assertions check error shape (Conflict + Why + Suggestion per Phase 4 D-10) at minimum by matching on a stable substring (e.g., the role description from `DirectoryRole::description()`). Snapshot tests are not required for this matrix — substring matching is enough to confirm the right error fires for the right combo.

### Integration Test Driver

- **D-10:** Two-pronged drive:
  - **(a)** One `assert_cmd` test in `tests/cli.rs` spawning `tome init --dry-run --no-input` with `HOME` overridden to a `tempfile::TempDir` containing pre-seeded known directory paths. Sets `NO_COLOR=1` (existing convention). Captures stdout, splits on the `Generated config:` marker, parses the trailing block as TOML, asserts `Config::validate().is_ok()` and TOML round-trip equality.
  - **(b)** Several direct unit tests in `wizard.rs` calling the new `assemble_config` helper for finer-grained cases: empty HOME → empty `directories` map; HOME with one of every `KNOWN_DIRECTORIES` entry; custom-dir variants exercised at the assemble-helper level.
- **D-11:** The integration test must cover at least two HOME states: empty HOME (no auto-discovery → empty directories config still validates and round-trips) and seeded HOME (e.g., `claude-skills` and `codex` directories pre-created → expected entries with correct types/roles).
- **D-12:** Stdout parsing relies on the existing `Generated config:` marker the wizard already prints (`wizard.rs:324`). No change to the wizard's dry-run output format is needed to make the test work.

### CI Gating

- **D-13:** No new CI infrastructure. The existing GitHub Actions pipeline (`make ci` → fmt + clippy + cargo test on ubuntu-latest + macos-latest) already runs all `#[cfg(test)]` tests and integration tests. New Phase 5 tests land into that pipeline and become non-optional gates by virtue of the existing `cargo test` step's pass/fail behavior — no thresholds, no separate jobs.

### Claude's Discretion

- Exact `assemble_config` signature shape (parameter ordering, whether customs are passed in the same `selected` slice or a separate one, naming).
- Whether the table-driven combo test lives in `config.rs::tests` or in a new `wizard.rs` test module — both are defensible; place it where the cross-cutting nature feels most natural.
- Use of `insta::assert_snapshot!` vs plain `assert_eq!` for the integration test's TOML output — plain assertions on `validate()` + round-trip equality satisfy the success criteria; snapshots are an upgrade Claude can take if useful.
- Whether `wizard::run()` keeps two `bool` params or accepts a small `WizardOptions { dry_run, no_input }` struct — both readable; the struct is mildly preferred if a third option appears, but two bools are fine for now.
- Help text and examples for `tome init` (`cli.rs:77-78`) — adding `--dry-run` and `--no-input` to the after_help block is reasonable polish.
- Whether to also add a fast unit test verifying the `lib.rs:164-165` bail removal is gone (i.e., that `cli.no_input` no longer rejects `init`) — nice-to-have regression guard.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap

- `.planning/REQUIREMENTS.md` — WHARD-04, WHARD-05, WHARD-06 definitions and Phase 5 traceability.
- `.planning/ROADMAP.md` §"Phase 5: Wizard Test Coverage" — four success criteria that must be TRUE after this phase.
- `.planning/PROJECT.md` — Constraints (Unix-only, single user, hard break OK), key decisions table.

### Prior Phase Context (decisions carried forward)

- `.planning/phases/01-unified-directory-foundation/01-CONTEXT.md` — Phase 1. D-05/D-06 (plain-english role parenthetical, role picker filtered by `valid_roles()`).
- `.planning/phases/02-git-sources-selection/02-CONTEXT.md` — Phase 2. UX patterns reference.
- `.planning/phases/03-import-reassignment-browse-polish/03-CONTEXT.md` — Phase 3. `tome add` is the canonical Git-entry creator (wizard does not produce Git entries).
- `.planning/phases/04-wizard-correctness/04-CONTEXT.md` — Phase 4. **Especially** D-01 (validation in `Config::validate`), D-03 (`Config::save_checked` does expand → validate → round-trip → write), D-10/D-11 (Conflict+Why+Suggestion error template with `DirectoryRole::description()` parentheticals). Phase 5 tests assert these are honored.

### Codebase Maps

- `.planning/codebase/TESTING.md` — Test framework, file organization, `TestEnvBuilder` pattern, `snapshot_settings()` filter, `assert_cmd` + `tempfile` + `insta` conventions. Pattern source for new tests.
- `.planning/codebase/STRUCTURE.md` and `CONVENTIONS.md` — Module layout and naming.

### Key Source Files

- `crates/tome/src/wizard.rs` — **Primary site for D-05 (assemble_config extraction) and D-03 (no_input handling).**
  - `wizard.rs:126` — `pub fn run(dry_run: bool) -> Result<Config>` — signature changes to add `no_input`.
  - `wizard.rs:144-175` — interactive flow that needs `if no_input { defaults } else { dialoguer call }` branches.
  - `wizard.rs:292-297` — final `Config` struct assembly; pulls into `assemble_config`.
  - `wizard.rs:306-322` — existing `--dry-run` branch already does expand → validate → round-trip → print. `--no-input` reaches this same branch unchanged.
  - `wizard.rs:514-541` — `find_known_directories_in()` already pure; existing tests at `wizard.rs:548-619` extend here.
- `crates/tome/src/cli.rs:43` — `pub no_input: bool` — global flag, already parsed.
- `crates/tome/src/cli.rs:77-78` — `Init` subcommand definition; reachable target for help-text polish.
- `crates/tome/src/lib.rs:164-165` — `anyhow::bail!("tome init requires interactive input — cannot use --no-input")` — **must be removed and replaced with passing `no_input` to `wizard::run`**.
- `crates/tome/src/config.rs:92-136` — `DirectoryType` enum + `default_role()` + `valid_roles()`. The `valid_roles()` matrix is the source of truth for D-08's combo table.
- `crates/tome/src/config.rs:142-186` — `DirectoryRole` enum + `description()` + `is_discovery()` + `is_distribution()`. Use `description()` for D-09's error-substring assertions.
- `crates/tome/src/config.rs:331+` — `Config::validate()` body. Combo test calls this directly.
- `crates/tome/src/config.rs:799+` — existing `validate_rejects_*` tests. Pattern for D-07's table-driven test; consider whether to consolidate or layer on top.
- `crates/tome/src/config.rs::save_checked` — Phase 4's combined validate + round-trip + write. Combo test's "save successfully" path uses this.

### Tests (extending in this phase)

- `crates/tome/tests/cli.rs` — `TestEnvBuilder`, `snapshot_settings()`, `tome()` helper, existing `NO_COLOR=1` convention. New `assert_cmd` test for D-10(a) lives here.
- `crates/tome/src/wizard.rs::tests` (lines 543-620) — existing 5 tests. New unit tests for `assemble_config` + helper coverage land here.
- `crates/tome/src/config.rs::tests` (lines 703+) — existing combo-rejection tests; new D-07 table-driven test lands here.

### Test Conventions

- `tempfile::TempDir` for isolated HOME — pattern at `wizard.rs:570-572`.
- `dirs::home_dir()` respects the `HOME` env var on Unix — `assert_cmd::Command::env("HOME", tmp.path())` is sufficient for HOME override.
- `NO_COLOR=1` strips ANSI codes from wizard output for stable substring matching.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `find_known_directories_in(home: &Path)` already exists at `wizard.rs:523` — pure, takes a HOME path, returns discovered entries. Currently has 3 unit tests covering empty HOME, file-where-dir-expected, and discovery of one known dir. Extend rather than rewrite.
- `KNOWN_DIRECTORIES` registry at `wizard.rs:41` is already fully testable via the `claude_plugins_always_managed` pattern (`wizard.rs:607`). Add coverage for every entry's `(directory_type, default_role)` matching `DirectoryType::default_role()` for that type.
- `DirectoryType::default_role()` at `config.rs:114` — already 3 lines, trivial. Existing test at `config.rs:711` covers all three types. WHARD-04's "DirectoryType::default_role" requirement is **already satisfied**; the phase just confirms this and treats it as a checked box.
- `DirectoryType::valid_roles()` at `config.rs:124` — already tested at `config.rs:724`. Same status as `default_role()`.
- `Config::save_checked` from Phase 4 — combined validate + round-trip + write. The combo test's "save successfully" path uses this directly without needing wizard at all.
- `tests/cli.rs::TestEnvBuilder` — won't help directly (it's geared toward sync tests with sources/targets), but its pattern of `tempfile::TempDir` + write_config + run binary is the template for the wizard integration test.
- `tests/cli.rs::snapshot_settings()` — if any snapshot is taken, this is the helper to use for `[TMPDIR]` redaction.
- `wizard.rs:306-322` (existing `--dry-run` branch) — already does the full validation dance. `--no-input` doesn't change what gets validated; it changes how we got there.

### Established Patterns

- Unit tests co-located via `#[cfg(test)] mod tests` in the same file as the code under test.
- Integration tests in `tests/cli.rs` invoke the binary via `assert_cmd::Command::cargo_bin("tome")` (helper `tome()`).
- Test names are descriptive present-tense verbs: `test_find_known_directories_in_empty_home_returns_empty`.
- `tempfile::TempDir` automatically dropped at end of scope — no manual cleanup.
- Existing `validate_rejects_*` tests (`config.rs:799+`) build a `Config` inline, call `validate()`, assert `is_err()` and check error message substring.
- `NO_COLOR=1` env var sets terminal mode for stable text comparison.

### Integration Points

- `wizard::run()` called from `lib.rs::run()` for the `Init` subcommand (around `lib.rs:160-180`). The `cli.no_input` value plumbs through here.
- Removing the `lib.rs:164-165` bail is a one-line change; replacing it is a `wizard::run(cli.dry_run, cli.no_input)` call.
- The test pipeline in `make ci` (and CI workflow) already runs all `cargo test` targets — no new Make targets, no new workflow files.

### Blast Radius

- Code changes: `wizard.rs` (signature + branches + new helper), `lib.rs:160-180` (bail removal + signature update at call site), possibly `cli.rs:77-78` (help text).
- Test additions: `wizard.rs::tests` (new), `config.rs::tests` (combo table), `tests/cli.rs` (one new wizard integration test).
- No changes to: `config.rs` non-test code (the wizard tests reach existing API), `discover.rs`, `library.rs`, `distribute.rs`, anything in `browse/`, `manifest.rs`, `lockfile.rs`.

</code_context>

<specifics>
## Specific Ideas

- The `Generated config:` marker at `wizard.rs:324` is the natural split point for the integration test's stdout parsing. Don't add a new flag like `--print-toml`; reuse what's already printed.
- The 12-combo test should iterate the cross-product of `[ClaudePlugins, Directory, Git]` × `[Managed, Synced, Source, Target]` and decide expected outcome (`save_ok` vs `validate_err`) by `DirectoryType::valid_roles().contains(&role)`. This guarantees no drift between wizard logic and test expectations.
- For each invalid combo, asserting that the error message contains the expected `DirectoryRole::description()` substring catches both wrong-error and silent-pass regressions.
- The integration test's seeded HOME should include both a managed and a synced known directory so the resulting `Config` has multiple entries — exercises BTreeMap ordering through the round-trip.
- The "empty HOME" sub-case of the integration test is important: it confirms the wizard doesn't choke when there's nothing to discover, and that an empty `directories` BTreeMap still validates cleanly.
- `lib.rs:164-165` removal is the single most user-visible change in Phase 5. Worth a short note in CHANGELOG when the milestone ships ("`tome init --no-input` is now supported and uses sensible defaults").

</specifics>

<deferred>
## Deferred Ideas

- **`WizardInputs` struct refactor** (D-06) — closer to the deferred WIZ-01–05 rewrite. Revisit if a third option appears or if the wizard's interactive flow itself needs restructuring.
- **Env/stdin-driven wizard inputs** — overkill for one integration test. If anyone ever needs scripted wizard runs (CI provisioning, demo automation), revisit then.
- **Snapshot testing for the combo matrix** (D-09) — substring matching is enough. Snapshots add value when error message wording itself needs to be reviewed; that's a Phase 6 polish concern, not a coverage one.
- **Coverage thresholds in CI** (D-13) — pragmatic-coverage policy in PROJECT.md; no enforced threshold. If this changes, add `cargo-tarpaulin` later as a separate phase.
- **`--no-input` for non-`init` subcommands beyond what already exists** — `--no-input` already works on `sync`, `doctor`, `cleanup`, `install`. Phase 5 only fixes the `init` gap. No new subcommand surfaces.
- **Generation of test fixtures from `KNOWN_DIRECTORIES`** — at 11 entries the registry could grow a generator. Today's manual approach is fine; revisit if entries triple.

</deferred>

---

*Phase: 05-wizard-test-coverage*
*Context gathered: 2026-04-19*

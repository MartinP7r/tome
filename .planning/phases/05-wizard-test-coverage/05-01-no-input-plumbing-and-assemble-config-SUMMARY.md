---
plan: 05-01-no-input-plumbing-and-assemble-config
status: complete
completed: 2026-04-20T09:23Z
commits:
  - 79bd7d3 feat(05-01): add read-only Config accessors for external crates
  - ff42faf feat(05-01): plumb --no-input through wizard, extract assemble_config
key-files:
  created: []
  modified:
    - crates/tome/src/wizard.rs
    - crates/tome/src/lib.rs
    - crates/tome/src/cli.rs
    - crates/tome/src/config.rs
    - crates/tome/tests/cli.rs
---

## What Was Built

Closed the two Phase 5 testing prerequisites in a single plumbing + extraction
refactor across four source files.

**Part A — `wizard.rs`:** `wizard::run` signature is now
`pub fn run(dry_run: bool, no_input: bool) -> Result<Config>`. Every dialoguer
prompt in the wizard path branches on `no_input` and takes its D-01 default
when true: include all auto-discovered known directories, library =
`~/.tome/skills`, empty exclusions, no role edits, no custom directories, save
accepted (for non-dry-run), no git init. The inline
`Config { library_dir, exclude, directories, ..Default::default() }` assembly
is replaced with a call to the newly extracted
`pub(crate) fn assemble_config(directories, library_dir, exclude) -> Config` —
a pure helper unit-testable without a TTY (unblocks Plan 05-02).

**Part B — `lib.rs`:** removed the `tome init requires interactive input` bail
at the old lines 163-165. The Init branch now calls
`wizard::run(cli.dry_run, cli.no_input)` directly. Added a regression-guard
unit test (`init_with_no_input_does_not_bail_from_lib_run`) that inspects
`lib.rs` source via `include_str!` and asserts the bail string is gone.

**Part C — `cli.rs`:** Init's `after_help` now documents all four flag combos
(`tome init`, `--dry-run`, `--no-input`, `--dry-run --no-input`).

**Part D — `config.rs`:** added three `pub fn` read-only accessors to
`Config` — `directories()`, `library_dir()`, `exclude()` — returning `&T`.
This is the minimal public-surface change required to unblock Plan 05-03's
integration test (which compiles as a separate crate and cannot reach
`pub(crate)` fields). Field visibility on `Config` is UNCHANGED — all three
fields remain `pub(crate)`.

## Deviations

- Updated the pre-existing `init_with_no_input_fails` test in
  `tests/cli.rs` (which asserted the OLD bail behavior) to
  `init_with_no_input_and_dry_run_succeeds` and exercises the new
  `tome init --no-input --dry-run` headless path, asserting exit 0 and the
  `Generated config:` stdout marker. This is in-scope: the bail removal
  directly invalidated the old test's assertion, and fixing it here avoids
  leaving CI broken for Plan 05-03.

## Tests Passing

- `cargo fmt -- --check` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo test -p tome --lib` — 406 passed, 0 failed
- `cargo build --tests -p tome` — integration test crate compiles against
  the new accessor surface (Plan 05-03 precondition satisfied)

## Interactive Behavior

Interactive TTY behavior is byte-for-byte unchanged: same prompts in the
same order. The only difference is that under `--no-input` those prompts
are not shown.

## Enables

- **Plan 05-02** — can now import `wizard::assemble_config` from the same
  crate and exercise it directly from `wizard.rs::tests`.
- **Plan 05-03** — can now run `tome init --dry-run --no-input` headlessly
  in `assert_cmd` and read the generated config via the new accessor
  methods on `Config`.

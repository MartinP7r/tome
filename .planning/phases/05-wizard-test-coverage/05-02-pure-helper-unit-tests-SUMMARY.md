---
plan: 05-02-pure-helper-unit-tests
status: complete
completed: 2026-04-20T12:00Z
commits:
  - 34f1a3e test(05-02): add pure wizard helper unit tests (WHARD-04)
key-files:
  created: []
  modified:
    - crates/tome/src/wizard.rs
requirements:
  - WHARD-04
---

## What Was Built

Extended the existing `#[cfg(test)] mod tests` block in `crates/tome/src/wizard.rs`
with 11 new unit tests (bringing the total from 6 → 17) that close WHARD-04's
"unit test coverage for pure wizard helpers" gate.

All new tests live in-process, call pure Rust functions with `TempDir`-isolated
HOMEs or inline `BTreeMap` construction, and never touch dialoguer, stdin, or
the real `$HOME`.

### Registry invariants (2 tests)

- `known_directories_default_role_matches_type` — iterates every
  `KNOWN_DIRECTORIES` entry and asserts `kd.default_role == kd.directory_type.default_role()`.
  Prevents silent drift when new entries are added or enum semantics change.
- `known_directories_default_role_is_in_valid_roles` — asserts every entry's
  `default_role` is accepted by its type's `valid_roles()`. If this fails,
  `tome init` would produce a Config that `Config::validate()` rejects.

### `find_known_directories_in` extensions (3 tests, on top of existing 3)

- `find_known_directories_in_discovers_every_registry_entry` — seeds HOME with
  one of every entry's `default_path` and asserts `KNOWN_DIRECTORIES.len()` results
  come back with matching names and absolute paths rooted at TempDir.
- `find_known_directories_in_discovers_multiple_entries` — seeds two known
  paths and asserts exactly those two come back.
- `find_known_directories_in_mixed_dir_and_file` — one valid directory plus
  one path-occupied-by-file; asserts only the real directory is returned.

### `assemble_config` coverage (6 tests, new helper introduced by Plan 05-01)

- `assemble_config_empty_inputs_produces_empty_config` — empty BTreeMap +
  empty exclude set → empty Config with passed-through library_dir.
- `assemble_config_single_entry_is_preserved` — one entry round-trips with
  correct path, directory_type, and role.
- `assemble_config_multi_entry_preserves_all` — three entries (one ClaudePlugins,
  two Directory) all present with correct types.
- `assemble_config_custom_entry_alongside_known` — non-registry-name entry
  coexists with known entry, each retaining its own type and role.
- `assemble_config_exclusions_preserved` — BTreeSet of SkillName exclusions
  passes through byte-identical.
- `assemble_config_library_dir_passed_through_verbatim` — neither tilde-paths
  nor absolute paths are mutated by `assemble_config` (it's a pure plumbing
  helper, not a path-expansion helper).

A small private `test_dir(path, kind, role)` factory function was added at the
top of the `assemble_config` test block to keep the 6 test bodies concise.

## Deviations

None. Plan executed exactly as written, no Rule 1-3 auto-fixes, no Rule 4
escalations. `cargo fmt` collapsed one let-statement line-break to fit on a
single line (cosmetic only).

## Tests Passing

- `cargo fmt -- --check` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo test -p tome --lib wizard::tests` — 17 passed (6 existing + 11 new),
  0 failed, 0 ignored

## Scope Boundaries Honored

- No production code touched in `wizard.rs` (tests-only plan).
- No `DirectoryType::default_role` in-isolation tests duplicated — already
  covered at `config.rs:711`.
- No `Config::validate` combo coverage — that's Plan 05-04's job.
- No integration tests invoking the binary — that's Plan 05-03's job.
- `KNOWN_DIRECTORIES` untouched; registry ordering preserved.

## Closes

- WHARD-04 — "Unit test coverage for pure wizard helpers". Registry invariants,
  `find_known_directories_in`, and `assemble_config` now all have in-crate
  unit tests that run on every `cargo test` invocation.

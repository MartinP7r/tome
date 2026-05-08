---
phase: 15-cli-hardening
plan: 02
subsystem: refactor
tags: [config, paths, tilde, serde, toml, port-02, hard-03, hard-22]

# Dependency graph
requires:
  - phase: 09-cross-machine-path-overrides
    provides: PORT-02 invariant — apply_machine_overrides mutates load-time-only copy; overrides never round-trip through tome.toml
  - phase: 15-cli-hardening (15-01)
    provides: lib.rs decomposed into pub(crate) cmd_<name> helpers; tests split into per-domain cli_*.rs files
provides:
  - config/{mod,types,overrides,validate}.rs four-file split (3122 LOC -> ~3450 distributed); Config::save_checked locked to mod.rs (S3 lock for Plan 15-04 Task 2)
  - paths::unexpand_tilde — inverse of expand_tilde; round-trip identity via dirs::home_dir()
  - Tilde-preserving Config::save_checked — under-$HOME paths auto-rewrite to ~/-shape; outside-$HOME stays absolute; already-tilde idempotent
  - D-TILDE-2 verbatim contract pinned in machine.rs by 3 regression tests (MachinePrefs::save MUST NOT rewrite override paths)
affects: [15-03, 15-04, 15-05, 15-06, 16-cleanup-message-ux, 17-migration-polish]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "config/ submodule split: types-only + validate + overrides + lifecycle (mod.rs hosts load/save_checked)"
    - "Re-exports preserve byte-identical public API across submodule split (callers use crate::config::Foo unchanged)"
    - "S3-locked landing site for Config::save_checked: mod.rs is the deterministic grep target"
    - "paths::unexpand_tilde / paths::expand_tilde — round-trip identity via shared dirs::home_dir() resolution"

key-files:
  created:
    - crates/tome/src/config/mod.rs
    - crates/tome/src/config/types.rs
    - crates/tome/src/config/overrides.rs
    - crates/tome/src/config/validate.rs
  modified:
    - crates/tome/src/paths.rs
    - crates/tome/src/machine.rs
    - crates/tome/tests/cli_init.rs
  deleted:
    - crates/tome/src/config.rs

key-decisions:
  - "tilde helpers (expand_tilde, unexpand_tilde) live in paths.rs not config — cross-cutting filesystem utility (CONTEXT.md Claude's Discretion); config/mod.rs re-exports expand_tilde for byte-identical public API"
  - "Config::save_checked operates on a serialisation clone (separate from the validation clone) so caller's Config is never mutated and PORT-02 is preserved by construction"
  - "MachinePrefs::save unchanged — D-TILDE-2 fences unexpand_tilde to Config::save_checked only; pinned by 3 regression tests"
  - "cli_init test updated to assert on-disk ~-shape + structural invariants (load() expands against test-process \$HOME, not subprocess \$HOME, so the previous absolute-path assertion was machine-environment-dependent)"

patterns-established:
  - "config/ split: data shapes -> types.rs; validation -> validate.rs; per-machine overrides -> overrides.rs; lifecycle (load/save_checked) -> mod.rs"
  - "Tilde round-trip: unexpand_tilde and expand_tilde both use dirs::home_dir(); applying one then the other returns the original path"
  - "Auto-portable serialisation: save_checked rewrites under-\$HOME paths to ~/ on emit; outside-\$HOME paths kept absolute"

requirements-completed:
  - HARD-03
  - HARD-22

# Metrics
duration: 22min
completed: 2026-05-08
---

# Phase 15 Plan 02: Config-module Split + Tilde-preserving save_checked Summary

**Split 3,122-LOC config.rs into four-file `config/` module with Config::save_checked locked to mod.rs (S3); added paths::unexpand_tilde so save_checked auto-rewrites under-$HOME paths to ~/-shape and a checked-in tome.toml stays portable across machines.**

## Performance

- **Duration:** ~22 min
- **Started:** 2026-05-08T05:27:16Z
- **Completed:** 2026-05-08T05:49:00Z
- **Tasks:** 2
- **Files modified:** 7 (4 created, 3 modified, 1 deleted)

## Accomplishments

- HARD-03 (#487): `crates/tome/src/config.rs` (3,122 LOC) split into `config/{mod,types,overrides,validate}.rs`. Public API surface preserved byte-identically — every existing `use crate::config::Foo` call site continues to compile unchanged via re-exports.
- HARD-22 (#457): `paths::unexpand_tilde` ships; `Config::save_checked` rewrites under-$HOME paths to `~/`-shape on serialise. Round-trip identity holds with `expand_tilde` via shared `dirs::home_dir()` resolution. The dotfiles workflow no longer rewrites `~/skills` to `/Users/martin/skills` on every save.
- D-TILDE-2 verbatim contract: `MachinePrefs::save` is verified by 3 regression tests to preserve user-supplied paths byte-for-byte. The unexpand pass is fenced to `Config::save_checked` only.
- PORT-02 invariant explicitly pinned by `save_checked_does_not_round_trip_override_paths_to_tome_toml` test.

## Task Commits

Each task was committed atomically:

1. **Task 1: Split config.rs into config/{mod,types,overrides,validate}.rs (HARD-03)** — `a586951` (refactor)
2. **Task 2: Add paths::unexpand_tilde + tilde-preserving Config::save_checked (HARD-22)** — `0947c80` (feat, TDD: combined RED+GREEN since unexpand_tilde tests don't compile until the function exists)

## Files Created/Modified

### Created

- `crates/tome/src/config/mod.rs` (1,401 LOC) — public re-exports + `Config::load`/`load_or_default`/`save`/`save_checked` (S3 lock) + `load_with_overrides` + tome-home/XDG-config helpers + tests for save_checked, load, expand_tilde re-export, write_xdg_tome_home, resolve_tome_home_with_source.
- `crates/tome/src/config/types.rs` (676 LOC) — data shapes only: `Config`, `DirectoryName`, `DirectoryConfig`, `DirectoryType`, `DirectoryRole`, `GitRef`, `BackupConfig`, `DirectoryConfigRaw` shim. Hosts the type-construction tests.
- `crates/tome/src/config/validate.rs` (710 LOC) — `Config::validate` (role/type combos + Cases A/B/C overlap detection) + `path_contains` helper + 12-combo matrix test.
- `crates/tome/src/config/overrides.rs` (663 LOC) — `Config::apply_machine_overrides`, `warn_unknown_overrides`, `format_override_validation_error` + PORT-01..05 tests.

### Modified

- `crates/tome/src/paths.rs` (+108 LOC, 485 total) — gained `expand_tilde` (lifted from config.rs per CONTEXT.md Claude's Discretion) + new `unexpand_tilde` + 7 unit tests.
- `crates/tome/src/machine.rs` (+95 LOC, 828 total) — 3 D-TILDE-2 verbatim regression tests proving `MachinePrefs::save` does NOT rewrite override paths.
- `crates/tome/tests/cli_init.rs` — `init_no_input_writes_config_and_reloads` updated to assert on-disk `~/.tome/skills` + structural invariants on the loaded Config (the test process's `$HOME` is not the subprocess's `$HOME`).

### Deleted

- `crates/tome/src/config.rs` (3,122 LOC).

## Decisions Made

- **Tilde helpers in `paths.rs` (not `config.rs`).** Per CONTEXT.md "Claude's Discretion": `expand_tilde` and `unexpand_tilde` are cross-cutting filesystem utilities, not config-specific. `config/mod.rs` re-exports `expand_tilde` (`pub use crate::paths::expand_tilde`) so call sites like `crate::config::expand_tilde` continue to compile byte-identically.
- **Two clones in `save_checked`: one for `validate()`, one for serialisation.** `validate()` needs absolute paths to detect overlaps (so it sees the `expand_tildes()`-applied clone). The TOML emitter sees a separate clone where every path field passes through `unexpand_tilde`. Starting the serialisation clone from `self` (not `expanded`) preserves user-supplied tildes verbatim instead of round-tripping them through expansion.
- **PORT-02 invariant preserved by construction.** `save_checked` operates on `&self`. `apply_machine_overrides` is never called in the save path. Override paths from `machine.toml` therefore never round-trip into `tome.toml` — pinned by `save_checked_does_not_round_trip_override_paths_to_tome_toml`.
- **D-TILDE-2 fence: `MachinePrefs::save` left untouched.** The plan's three new regression tests (`save_preserves_override_path_outside_home_verbatim`, `save_preserves_override_tilde_path_verbatim`, `save_preserves_override_absolute_under_home_verbatim`) lock in that machine.toml stays byte-identical for /Volumes/External, ~/skills, and under-$HOME absolute paths.
- **`save_checked` lives in `config/mod.rs` (S3 lock).** Plan 15-04 Task 2 (HARD-08 atomic-save regression) needs a deterministic grep target. `mod.rs` depends on submodules (validate, overrides), not vice versa, so co-locating the top-level lifecycle methods there keeps the dependency direction clean.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] cli_init integration test asserted obsolete `save_checked` contract**

- **Found during:** Task 2 verification
- **Issue:** `tests/cli_init.rs::init_no_input_writes_config_and_reloads` asserted that `loaded.library_dir() == tmp.path().join(".tome/skills")` based on the OLD contract that `save_checked` expands `~` before writing. With the new D-TILDE-1 contract (`save_checked` preserves `~/`-shape), the on-disk file contains `library_dir = "~/.tome/skills"`, and `Config::load` re-expands using the *test-process* `$HOME` (because `dirs::home_dir()` does not honor a child process's $HOME). The previous assertion held only by accident on machines where the subprocess's `$HOME=tmp.path()` resolved to a path that matched what the test process saw.
- **Fix:** Updated the test to (a) assert the on-disk file contains `library_dir = "~/.tome/skills"` (the new portability guarantee), (b) assert the loaded `library_dir` is absolute and ends with `.tome/skills` (structural invariants only).
- **Files modified:** `crates/tome/tests/cli_init.rs`
- **Verification:** `cargo test -p tome --test cli_init init_no_input_writes_config_and_reloads` passes.
- **Committed in:** `0947c80` (Task 2 commit).

**2. [Rule 1 - Bug] Useless `format!` macro flagged by clippy in new D-TILDE-2 test**

- **Found during:** Task 2 verification (clippy run)
- **Issue:** `format!("path = \"~/")` triggers `clippy::useless_format` (-D warnings); the format string has no interpolations.
- **Fix:** Replaced with the bare string literal `"path = \"~/"`.
- **Files modified:** `crates/tome/src/machine.rs`
- **Verification:** `cargo clippy --all-targets -- -D warnings` exits 0.
- **Committed in:** `0947c80` (Task 2 commit).

---

**Total deviations:** 2 auto-fixed (both Rule 1, narrow-scope bugs in test code I added).
**Impact on plan:** Both auto-fixes are necessary for correctness. No scope creep — both changes are inside files the plan already lists in `files_modified`.

## Issues Encountered

- **Pre-existing intermittent test flake:** `git::tests::read_head_sha_returns_40_char_hex` failed once during the baseline run, then passed in isolation. This matches the carry-over flake category in STATE.md (`backup::tests::push_and_pull_roundtrip` is the documented flaky test). Out of scope per phase 15 plan; tracked separately as HARD-14 (#500).
- **`make ci` cannot run without `typos` CLI installed** (returns `make: typos: No such file or directory`). The plan's verification block lists `cargo build && cargo clippy && cargo test` rather than the full `make ci` so this does not block plan completion. The 3 quality gates the plan mandates (fmt-check, clippy `-D warnings`, tests) all pass.

## Test Count

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Total tests | 845 | 860 | +15 |
| paths::tests | 12 | 19 | +7 (unexpand_tilde) |
| config::tests (mod/types/overrides/validate combined) | 60 | 65 | +5 (save_checked tilde-preservation) |
| machine::tests | 33 | 36 | +3 (D-TILDE-2 verbatim) |

Plan target: ≥6 new tests. Delivered: +15. All 860 tests pass under `cargo test -p tome`.

## User Setup Required

None — pure refactor + serialisation behaviour change. The behaviour change is intentional and observable only on `tome.toml` save: paths under `$HOME` now write as `~/...` instead of expanded absolute paths. Existing `tome.toml` files continue to load unchanged.

## Next Phase Readiness

- **Plan 15-03 (Type-system tightening)** can land next — depends only on the v0.10 type lifts (already in main) and Phase 11 LIB-* requirements.
- **Plan 15-04 (Safety guards + integration tests, including HARD-08 atomic-save regression)** has a deterministic grep target for `Config::save_checked` (mod.rs S3 lock).
- **No blockers** for downstream plans.

## Self-Check: PASSED

Verified post-write:

- `crates/tome/src/config/{mod,types,overrides,validate}.rs` all exist.
- `crates/tome/src/config.rs` is deleted.
- Both task commits (`a586951`, `0947c80`) found in `git log`.
- `cargo build -p tome` clean, `cargo clippy --all-targets -- -D warnings` clean, `cargo test -p tome` 860 / 860 pass.

---
*Phase: 15-cli-hardening*
*Completed: 2026-05-08*

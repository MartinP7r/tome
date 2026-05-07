# Phase 11 Deferred Items

Items discovered during phase execution that fall outside the current plan's scope.

## symlink_chain_managed_skill (cli.rs:1775) — RESOLVED in Plan 11-05

**Found during:** Plan 11-03 execution, post Wave 2 commits
**Cause:** Plan 11-02 (consolidate_managed-as-copy) changed managed skill library
shape from symlink to real directory. The test `symlink_chain_managed_skill` in
`crates/tome/tests/cli.rs` still asserts `library_skill.is_symlink()` which is
the v0.9 shape.
**Owner:** Plan 11-02 territory (library.rs / managed-skill consolidation behavior).
Out of scope for Plan 11-03 (cleanup.rs + remove.rs only).
**Suggested action:** Plan 11-05 (integration tests) or a follow-up tweak in
Plan 11-02 should rewrite `symlink_chain_managed_skill` to assert the v0.10
shape: `library_skill.is_dir() && !library_skill.is_symlink()`, then verify
content_hash equality against the source via manifest::hash_directory.

**Resolution (2026-05-03):** Plan 11-05 Task 1 commit (`5e70031`) updated
`symlink_chain_managed_skill` to assert the v0.10 shape:
- `library_skill.is_dir() && !library_skill.is_symlink()` (LIB-01 invariant)
- Content fidelity verified via `tome::hash_directory(library) == tome::hash_directory(source)`
  using the new crate-root re-export from Plan 11-05 Task 0 (`e5bf045`).
- Target → library symlink relationship preserved (still asserts target.is_symlink()
  and resolves into the library copy).
Test passes in the full integration suite alongside the five new Plan 11-05 tests.

## Pre-existing dead-code warning on `SkillEntry::new_unowned`

**Found during:** Plan 11-04 (noted in 11-04-SUMMARY.md "Issues Encountered"); also surfaces
in Plan 11-05 when running `cargo clippy --package tome --all-targets -- -D warnings`.
**Cause:** Plan 11-01 lifted `SkillEntry::new_unowned` constructor for use by Phase 14's
`tome adopt`/`tome forget` commands. It's not consumed in Phase 11 (no production
call-site yet).
**Owner:** Phase 14 (UNOWN-01..03) when adopt/forget commands are implemented and
consume the constructor.
**Status:** `cargo test --package tome` (without clippy `-D warnings`) passes 558+141.
`make ci` fails on the dead-code warning. Out of scope for Plan 11-05 (integration tests
only) per parallel-wave scope safety. Not introduced by this plan.
**Suggested action:** Phase 14 will provide the production call-site naturally; if Phase
12 or 13 needs the warning silenced earlier, add `#[allow(dead_code)]` with a comment
pointing at Phase 14.

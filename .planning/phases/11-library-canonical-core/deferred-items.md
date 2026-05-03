# Phase 11 Deferred Items

Items discovered during phase execution that fall outside the current plan's scope.

## symlink_chain_managed_skill (cli.rs:1775)

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

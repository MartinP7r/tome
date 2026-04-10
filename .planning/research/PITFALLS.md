# Domain Pitfalls

**Domain:** Rust CLI config refactor with git source integration (tome v0.6)
**Researched:** 2026-04-10

## Critical Pitfalls

Mistakes that cause rewrites or major issues.

### Pitfall 1: Serde Deserialization Silently Succeeds on Partial Old Config

**What goes wrong:** After replacing `sources: Vec<Source>` + `targets: BTreeMap<TargetName, TargetConfig>` with `directories: BTreeMap<DirectoryName, DirectoryConfig>`, serde's `#[serde(default)]` on the new `directories` field means an old `tome.toml` (with `[[sources]]` and `[targets.*]`) will parse *successfully* with an empty `directories` map. The old fields are silently ignored. `tome sync` runs, finds zero directories, does nothing, and the user thinks sync is broken.

**Why it happens:** Serde's `#[serde(default)]` is designed for forward-compatible evolution but creates a trap during breaking migrations. The old fields become "unknown keys" which TOML deserialization silently discards (unless `#[serde(deny_unknown_fields)]` is used).

**Consequences:** User runs `tome sync` after upgrade, gets zero skills synced, no error message. Library may even get cleaned up (cleanup removes skills not in any directory). Data loss if library contents were the only copies.

**Prevention:**
1. Add `#[serde(deny_unknown_fields)]` to `Config` struct during the transition. This makes old configs fail loudly with "unknown field `sources`" instead of silently succeeding.
2. Alternatively, keep the old field names as `#[serde(skip)]` with a custom deserializer that detects them and returns an actionable error: "Config uses old format. See migration guide at [URL]."
3. Add a config version field: `config_version = 2` at the top of the new format. Check for its absence.

**Detection:** Integration test that loads a v0.5 config into the v0.6 `Config` struct and asserts it fails with a meaningful error, not silently succeeds.

**Phase mapping:** Must be addressed in the foundation PR (#396) before any other v0.6 work.

### Pitfall 2: Cleanup Deletes Library Skills During Config Migration Gap

**What goes wrong:** Between upgrading the binary and updating `tome.toml`, there's a window where the new binary parses zero directories (see Pitfall 1) and the cleanup phase removes all library skills as "stale" (not referenced by any directory).

**Why it happens:** The cleanup module removes library entries that don't correspond to any configured source/directory. With zero directories configured, *all* skills are stale.

**Consequences:** Irreversible library data loss. Even if the library is git-backed (`tome backup`), the user may not realize what happened until skills are missing from their tools.

**Prevention:**
1. Add a safety check in cleanup: if `directories.is_empty()`, skip cleanup entirely and warn "No directories configured -- skipping cleanup to protect library contents."
2. This guard is cheap and prevents the catastrophic case regardless of how the zero-directory state is reached.

**Detection:** Integration test: sync with zero sources/directories configured, verify library contents are preserved (not deleted).

**Phase mapping:** Foundation PR (#396). This is a day-one safety rail.

### Pitfall 3: Git Clone in Sync Pipeline Makes Sync Unreliable

**What goes wrong:** Adding `git clone --depth 1` (or `git pull`) to the sync pipeline means `tome sync` now has a network dependency. If the remote is unreachable (offline, auth expired, DNS failure), sync fails entirely even though local directories are fine.

**Why it happens:** Mixing network I/O with local filesystem operations in a single pipeline without fault isolation.

**Consequences:** Users who are offline (airplane, VPN down) cannot sync local skills. The entire sync breaks because one git remote is unreachable.

**Prevention:**
1. Git fetch/clone must be a **separate, failable phase** that runs before the main sync pipeline. Failed git sources should be logged as warnings, not errors that abort sync.
2. Use cached state: if `~/.tome/repos/<hash>/` already has a previous clone, use it as-is when fetch fails. Log "Using cached version of [repo] (fetch failed: [reason])".
3. Consider a separate command (`tome fetch` or `tome pull`) for explicit git updates, with `tome sync` only using whatever is already cached locally.

**Detection:** Integration test that mocks a git remote failure (point at nonexistent URL) and verifies sync still completes for non-git directories.

**Phase mapping:** Git sources implementation phase. Design the failure mode *before* writing the clone logic.

### Pitfall 4: Shelling Out to `git` Without Checking `git` Exists

**What goes wrong:** The existing `backup.rs` pattern uses `Command::new("git")` which fails with an opaque `io::Error` ("No such file or directory") if git is not installed. The same pattern for git sources would fail similarly.

**Why it happens:** `Command::new("git").output()` returns `Err(io::Error)` when the binary is not found, but the error message doesn't mention that git is missing -- it looks like a file operation failure.

**Consequences:** Confusing error message. Users without git installed (rare but possible in containers) get an unhelpful error.

**Prevention:**
1. At startup of any git operation, check `Command::new("git").arg("--version").output()` and convert the error to a human-readable "git is not installed. Git sources require git to be available in PATH."
2. Extract this into a shared `git::require_git()` function that both backup and git-sources can call.
3. Already partially exists in `backup.rs` -- extend the pattern, don't duplicate it.

**Detection:** Unit test that verifies the error message when git binary is not found (set PATH to empty in test).

**Phase mapping:** Git sources implementation. Can reuse/extend the `backup.rs` git helpers.

## Moderate Pitfalls

### Pitfall 5: BTreeMap Alphabetical Priority Is Surprising for Users

**What goes wrong:** With `directories: BTreeMap<DirectoryName, DirectoryConfig>`, duplicate skill names are resolved by alphabetical order of the directory name. A directory named "aaa-plugins" always wins over "zzz-local" regardless of which the user considers more important.

**Why it happens:** `BTreeMap` iterates in `Ord` order of keys. This is a valid choice for determinism but creates an implicit priority that users don't control.

**Consequences:** User adds a new directory with a name that alphabetically precedes their preferred source, and skills silently switch provenance. No warning, no error. The "wrong" version of a skill gets distributed.

**Prevention:**
1. During discovery deduplication, when a conflict is detected, log a warning: "Skill '[name]' found in both '[dir-a]' and '[dir-b]'. Using '[dir-a]' (alphabetically first). Add a `priority` field to override."
2. Document the behavior clearly in config reference.
3. Design the `priority` field now (as an optional integer, defaulting to 0) even if you don't implement sorting by it yet. This avoids a second config-breaking change later.

**Detection:** Unit test that configures two directories with the same skill and verifies which one wins, plus verifies the warning is emitted.

**Phase mapping:** Foundation PR (#396). The dedup warning should ship with the new config format.

### Pitfall 6: Shallow Clone `--depth 1` Breaks on Subsequent `git pull`

**What goes wrong:** `git clone --depth 1` creates a shallow clone. A naive `git pull` on a shallow clone may:
- Download more history than expected (unshallowing)
- Fail with merge conflicts if the remote force-pushed
- Behave unpredictably with `git fetch` depending on git version

**Why it happens:** Shallow clones have truncated history. Operations that need ancestor commits (merge-base, rebase) may fail or silently download full history, defeating the purpose of shallow cloning.

**Consequences:** `tome sync` with git sources becomes slow (full history downloaded on update) or fails with confusing git errors that tome doesn't handle.

**Prevention:**
1. Use `git fetch --depth 1 origin main && git reset --hard origin/main` instead of `git pull`. This keeps the clone shallow and avoids merge logic entirely (tome only needs the latest snapshot, not history).
2. Never use `git pull` on shallow clones. The fetch+reset pattern is simpler and more predictable.
3. Consider `--filter=blob:none` (blobless partial clone) as an alternative if you need sparse checkout later, but for now `--depth 1` with fetch+reset is sufficient.

**Detection:** Integration test that creates a local bare repo, clones it shallow, pushes a new commit to the bare repo, then verifies fetch+reset works correctly.

**Phase mapping:** Git sources implementation.

### Pitfall 7: Nested Git Repos When Library Dir Is Inside a Git Repo

**What goes wrong:** If `~/.tome/` is itself a git repo (via `tome backup`), and git source clones go inside `~/.tome/` (e.g., `~/.tome/repos/`), git may get confused by nested `.git` directories. The outer repo's `git add` may try to add the inner repos as submodules.

**Why it happens:** Git detects nested `.git` directories and treats them as submodules during `git add -A` or similar operations.

**Consequences:** `tome backup` snapshots may include (or fail on) git source clones. Backup repo becomes bloated or corrupt.

**Prevention:**
1. The PROJECT.md already specifies `~/.tome/repos/` for clones. Add `repos/` to the backup repo's `.gitignore` during `backup init`.
2. Verify this in an integration test: init backup, clone a git source, run `tome backup snapshot`, verify `repos/` is not committed.

**Detection:** `tome doctor` should check for nested `.git` directories inside `~/.tome/` and warn if they're not in `.gitignore`.

**Phase mapping:** Git sources implementation. Must be coordinated with backup module.

### Pitfall 8: `TestEnvBuilder` Rewrite Loses Edge Case Coverage

**What goes wrong:** The `TestEnvBuilder` in `cli.rs` generates config TOML strings with the old `[[sources]]` + `[targets.*]` format. Rewriting it for `[directories.*]` is straightforward, but edge cases in the old test suite (managed vs. local transitions, circular symlink detection, cleanup behavior) may be silently dropped during the rewrite.

**Why it happens:** When rewriting test infrastructure, it's natural to port the happy-path tests first and lose coverage for edge cases that were added incrementally over months.

**Consequences:** Regressions in consolidation strategy transitions, circular symlink prevention, or cleanup behavior that aren't caught until a user hits them.

**Prevention:**
1. Before rewriting `TestEnvBuilder`, catalog all existing integration tests and their scenarios in a checklist. Port each one explicitly.
2. Use `cargo test` with the old code as a baseline: count tests, note their names. After rewrite, verify the count is >= the original.
3. The CONCERNS.md already flags "symlink strategy transitions" and "circular symlink prevention" as coverage gaps. Fix these gaps *during* the rewrite, not after.

**Detection:** Compare test counts before and after the refactor. CI should not show fewer tests passing.

**Phase mapping:** Foundation PR (#396). Test infrastructure changes are part of the config refactor.

### Pitfall 9: GIT_DIR/GIT_WORK_TREE Environment Variable Leakage

**What goes wrong:** When shelling out to `git` from within a git repository (which `~/.tome/` may be, via backup), environment variables like `GIT_DIR`, `GIT_WORK_TREE`, or `GIT_INDEX_FILE` from the parent process can leak into child `git` commands. This causes git source operations to target the wrong repository.

**Why it happens:** `std::process::Command` inherits the parent's environment by default. If tome is run from a directory that has its own git context, or if the backup module sets git env vars, those leak to git-source operations.

**Consequences:** `git clone` or `git fetch` for a git source operates on the wrong repo, corrupting the backup repo or producing confusing errors.

**Prevention:**
1. For all git-source operations, explicitly clear git environment variables: `.env_remove("GIT_DIR").env_remove("GIT_WORK_TREE").env_remove("GIT_INDEX_FILE").env_remove("GIT_CEILING_DIRECTORIES")`.
2. Better: use `.env_clear()` and selectively pass only PATH, HOME, SSH_AUTH_SOCK, and GIT_SSH_COMMAND. This is the nuclear option but guarantees isolation.
3. The existing `backup.rs` `git()` helper uses `.current_dir(repo_dir)` which helps, but does not override explicit GIT_DIR if set.

**Detection:** Test that sets `GIT_DIR` to a bogus path before calling git-source operations, verifying they still work correctly.

**Phase mapping:** Git sources implementation. Apply to the shared git helper module.

## Minor Pitfalls

### Pitfall 10: TOML Table vs. Inline Table Serialization Surprise

**What goes wrong:** `BTreeMap<DirectoryName, DirectoryConfig>` serialized with `toml::to_string_pretty` may produce `[directories.my-dir]` tables or inline tables depending on the structure depth. If `DirectoryConfig` has nested fields (e.g., `git: { url, branch }`), the TOML output may be ugly or use unexpected nesting.

**Prevention:** Test the round-trip serialization of the new config format and assert the TOML output matches the expected human-readable format. Use `#[serde(rename)]` and flattening as needed to control the output shape.

**Phase mapping:** Foundation PR. Include a config round-trip test.

### Pitfall 11: Path Canonicalization Differences Between macOS and Linux

**What goes wrong:** On macOS, `/var` is a symlink to `/private/var` and `/tmp` is a symlink to `/private/tmp`. Path comparisons using `canonicalize()` produce different results than `Path::starts_with()` on raw paths. The CONCERNS.md already flags this for cleanup, but the new unified directories model adds more path comparison points.

**Prevention:** Use a consistent path normalization strategy across the codebase. Canonicalize once at config load time and store the canonical path. Never compare raw user-provided paths with canonicalized paths.

**Phase mapping:** Foundation PR. Ensure `DirectoryConfig.path` is canonicalized during deserialization.

### Pitfall 12: Role Transitions (Source -> Target or Vice Versa) Leave Stale State

**What goes wrong:** If a user changes a directory's role from `source` to `target` (or adds the `target` role), the manifest still records the old provenance. Cleanup may not remove the now-stale library entries because the manifest says they came from that directory.

**Prevention:** When a directory's role changes, detect the mismatch between manifest provenance and current role during sync. Log a warning and re-consolidate affected skills.

**Phase mapping:** Post-foundation. This is an edge case for the consolidation pipeline.

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Config struct refactor (#396) | Silent parse success on old config (Pitfall 1) | `deny_unknown_fields` or version check |
| Config struct refactor (#396) | Cleanup deletes everything on empty config (Pitfall 2) | Empty-directories guard in cleanup |
| Config struct refactor (#396) | Test coverage regression (Pitfall 8) | Catalog tests before rewrite |
| Config struct refactor (#396) | BTreeMap priority surprise (Pitfall 5) | Dedup warning + document behavior |
| Wizard rewrite (#362) | Auto-assigned roles don't match user intent | Show role summary, allow override |
| Git sources (#58) | Network failure breaks entire sync (Pitfall 3) | Separate fetch phase, use cached state |
| Git sources (#58) | Shallow clone update issues (Pitfall 6) | fetch+reset pattern, never git pull |
| Git sources (#58) | Nested git repos in backup (Pitfall 7) | `.gitignore` repos/ in backup |
| Git sources (#58) | GIT_DIR leakage (Pitfall 9) | Clear git env vars in Command |
| Git sources (#58) | Missing git binary (Pitfall 4) | `require_git()` check at start |

## Sources

- [Serde issue #1652: Migrating serialized config to new schema](https://github.com/serde-rs/serde/issues/1652) — config migration patterns
- [Rust issue #73126: Command output() error handling hazards](https://github.com/rust-lang/rust/issues/73126) — shelling out pitfalls
- [GitHub Blog: Partial clone and shallow clone](https://github.blog/open-source/git/get-up-to-speed-with-partial-clone-and-shallow-clone/) — shallow clone limitations
- [BTreeMap documentation](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html) — iteration order guarantees
- [Gitoxide 2025 retrospective](https://github.com/GitoxideLabs/gitoxide/discussions/2323) — gitoxide vs git2 maturity
- [Rust forum: Command error handling](https://users.rust-lang.org/t/best-error-handing-practices-when-using-std-command/42259) — Command patterns

---

*Pitfalls research: 2026-04-10*

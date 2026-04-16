---
phase: 02-git-sources-selection
verified: 2026-04-15T00:00:00Z
status: passed
score: 13/13 must-haves verified
gaps: []
---

# Phase 02: Git Sources & Selection Verification Report

**Phase Goal:** Users can add remote git repos as skill sources and control which skills reach which directories on a per-machine basis
**Verified:** 2026-04-15
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | git.rs module exists with clone, update, and SHA-reading functions | VERIFIED | `crates/tome/src/git.rs` contains `clone_repo`, `update_repo`, `read_head_sha`, `repo_cache_dir`, `effective_path`, `is_git_available` |
| 2  | All git subprocess calls clear GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE env vars | VERIFIED | `env_remove("GIT_DIR")`, `env_remove("GIT_WORK_TREE")`, `env_remove("GIT_INDEX_FILE")` present in git_command helper (line 17-19), clone_repo (lines 99-101), and is_git_available (lines 158-160) |
| 3  | DirectoryConfig has an optional subdir field that passes config validation | VERIFIED | `pub subdir: Option<String>` at line 220 of config.rs; git-only validation guard at line 372 |
| 4  | TomePaths exposes a repos_dir() method returning tome_home/repos/ | VERIFIED | `pub fn repos_dir` at line 90 of paths.rs, returns `self.tome_home.join("repos")` |
| 5  | machine.toml supports per-directory [directory.<name>] sections with disabled and enabled keys | VERIFIED | `DirectoryPrefs` struct at line 24 of machine.rs; `pub(crate) directory: BTreeMap<DirectoryName, DirectoryPrefs>` at line 48 |
| 6  | Setting both disabled and enabled on the same directory produces a validation error | VERIFIED | `validate()` at line 76 bails with "both 'disabled' and 'enabled'" message; `prefs.validate()?` called at line 132 in load() |
| 7  | Per-directory enabled (allowlist) overrides global disabled per D-08 locality principle | VERIFIED | `is_skill_allowed` at line 95 resolves: per-dir enabled > per-dir disabled > global disabled; 7 unit tests covering all combinations pass |
| 8  | tome sync clones git-type directories to ~/.tome/repos/<sha256>/ before discovery | VERIFIED | `resolve_git_directories` at line 462 of lib.rs; `git::clone_repo` called at line 534; wired into sync at line 652 |
| 9  | Subsequent tome sync fetches updates without re-cloning | VERIFIED | `already_cloned` branch at lines 523ff calls `git::update_repo` instead of `git::clone_repo` |
| 10 | Failed git operations warn to stderr and continue syncing local directories | VERIFIED | "could not update" at line 554 and "could not clone" at line 563 in lib.rs; failed clone skips directory, failed update falls back to cached state |
| 11 | Distribution uses is_skill_allowed for per-directory filtering instead of global is_disabled | VERIFIED | `machine_prefs.is_skill_allowed(&skill_name_str, dir_name.as_str())` at line 83 of distribute.rs; old `is_disabled` call removed |
| 12 | tome remove deletes directory entry from config and cleans up all artifacts | VERIFIED | `remove.rs` has `plan`, `render_plan`, `execute`; handles symlinks, library entries, manifest, git cache, config entry in correct order |
| 13 | tome remove --dry-run and --force flags work; git cache cleaned for git-type dirs | VERIFIED | `git_cache_path` computed via `repo_cache_dir` in plan(); `dry_run` branches throughout execute(); `force` flag in CLI; 4 integration tests all pass |

**Score:** 13/13 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/tome/src/git.rs` | Git clone, update, SHA reading, URL hashing | VERIFIED | All 6 pub(crate) functions present; SHA-256 via sha2 crate; env var clearing confirmed |
| `crates/tome/src/config.rs` | subdir field on DirectoryConfig | VERIFIED | `pub subdir: Option<String>` with git-only validation |
| `crates/tome/src/paths.rs` | repos_dir() on TomePaths | VERIFIED | `pub fn repos_dir` returns `tome_home.join("repos")` |
| `crates/tome/src/machine.rs` | DirectoryPrefs struct, per-directory filtering, validation | VERIFIED | `DirectoryPrefs`, `is_skill_allowed`, `validate`, BTreeMap directory field all present |
| `crates/tome/src/lib.rs` | resolve_git_directories pre-discovery step, updated sync pipeline | VERIFIED | Function at line 462; wired into sync at line 652; both clone and update paths present |
| `crates/tome/src/distribute.rs` | Per-directory skill filtering via is_skill_allowed | VERIFIED | `machine_prefs.is_skill_allowed` at line 83; old `is_disabled` removed |
| `crates/tome/src/discover.rs` | Discovery uses resolved paths map for git directories | VERIFIED | `resolved_paths` parameter accepted; `resolved_paths.get(dir_name)` used to override scan path; git_commit_sha attached to skills |
| `crates/tome/src/remove.rs` | Remove command logic: plan, preview, execute | VERIFIED | All three functions present; git cache cleanup via `git_cache_path`; dry_run branches |
| `crates/tome/src/cli.rs` | Remove subcommand definition | VERIFIED | `Remove { name: String, force: bool }` with `value_name = "NAME"` and help text |
| `crates/tome/tests/cli.rs` | Integration tests for tome remove | VERIFIED | `test_remove_nonexistent_directory`, `test_remove_local_directory`, `test_remove_dry_run`, `test_remove_no_input_without_force_fails` all present and passing |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `git.rs` | sha2 crate | URL hashing for cache directory names | VERIFIED | `use sha2::{Digest, Sha256};` at line 10; `Sha256::new()` in `repo_cache_dir` |
| `git.rs` | std::process::Command | git subprocess calls with env clearing | VERIFIED | `env_remove("GIT_DIR/..")` in git_command, clone_repo, is_git_available |
| `machine.rs` | `distribute.rs` | `is_skill_allowed` called during distribution | VERIFIED | `machine_prefs.is_skill_allowed` at distribute.rs line 83 |
| `lib.rs` | `git.rs` | resolve_git_directories calls clone_repo/update_repo | VERIFIED | `git::clone_repo` at line 534, `git::update_repo` at line 523 |
| `lib.rs` | `discover.rs` | resolved_paths passed to discover_all | VERIFIED | `discover::discover_all(&config, &resolved_git_paths, &mut warnings)` at line 652 |
| `lib.rs` | `remove.rs` | Command::Remove dispatch | VERIFIED | `Command::Remove { name, force }` match arm at line 252 |
| `remove.rs` | `config.rs` | Config modification and save | VERIFIED | `config.directories.remove`; `config::save` called after execute |
| `remove.rs` | `manifest.rs` | Manifest entry cleanup | VERIFIED | `manifest.remove` called per skill in execute(); `manifest::save` after |

### Data-Flow Trace (Level 4)

Git directories are resolved to local paths before discovery — no component renders dynamic remote data directly, so traditional data-flow tracing does not apply. The critical flow is:

- `resolve_git_directories` → `(PathBuf, Option<String>)` map → `discover_all` → `DiscoveredSkill.provenance.git_commit_sha` → `lockfile::generate` → lockfile entry. This chain is wired end-to-end per the discover.rs evidence (lines 189-232).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All 459 tests pass | `cargo test -p tome -q` | 366 unit + 93 integration: 0 failed | PASS |
| No clippy warnings | `cargo clippy -p tome --all-targets -- -D warnings` | Finished with no warnings | PASS |
| `tome remove --help` available | binary builds without error | `cargo build` exits 0 (confirmed via clippy run) | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| GIT-01 | 02-01 | `type = "git"` directory config with URL in `path` field | SATISFIED | `DirectoryType::Git` in config.rs; `subdir` field git-only validated |
| GIT-02 | 02-01 | Shallow clone (`--depth 1`) to `~/.tome/repos/<sha256(url)>/` | SATISFIED | `clone_repo` uses `--depth 1`; `repo_cache_dir` uses SHA-256 of URL |
| GIT-03 | 02-01 | Update via `git fetch --depth 1 origin <ref> && git reset --hard FETCH_HEAD` | SATISFIED | `update_repo` uses this exact sequence |
| GIT-04 | 02-01 | Branch/tag/SHA pinning via `branch`, `tag`, `rev` fields | SATISFIED | `ref_spec_for_config` handles all three; `clone_repo`/`update_repo` accept all three |
| GIT-05 | 02-01 | Resolved commit SHA recorded in lockfile for reproducibility | SATISFIED | `read_head_sha` in git.rs; SHA flows via resolved_paths tuple into `DiscoveredSkill.git_commit_sha` into lockfile |
| GIT-06 | 02-01 | All git commands clear GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE | SATISFIED | All three vars cleared in git_command, clone_repo, is_git_available |
| GIT-07 | 02-03 | Git resolution runs as pre-discovery step, resolves URLs to local cache paths | SATISFIED | `resolve_git_directories` before `discover_all` in sync() |
| GIT-08 | 02-03 | Failed git operations fall back to cached state, don't abort sync | SATISFIED | Failed update falls back to cached path; failed clone skips directory with warning |
| MACH-02 | 02-02 | Per-directory `disabled` set (blocklist) in machine.toml | SATISFIED | `DirectoryPrefs.disabled: BTreeSet<SkillName>` |
| MACH-03 | 02-02 | Per-directory `enabled` set (exclusive allowlist) in machine.toml | SATISFIED | `DirectoryPrefs.enabled: Option<BTreeSet<SkillName>>` |
| MACH-04 | 02-02 | `disabled` + `enabled` on same directory = validation error | SATISFIED | `validate()` bails with clear message; called from `load()` |
| MACH-05 | 02-02 | Resolution: per-dir enabled > per-dir disabled > global disabled | SATISFIED | `is_skill_allowed` locality chain; 7 unit tests verify all paths |
| CLI-01 | 02-04 | `tome remove <directory-name>` removes entry from config, cleans up library + symlinks | SATISFIED | `remove.rs` plan/render/execute; 4 integration tests pass |

No orphaned requirements detected — all 13 IDs appear in plan frontmatter and REQUIREMENTS.md.

### Anti-Patterns Found

No blockers or warnings found. Spot-check of key modified files:

- `crates/tome/src/git.rs` — no TODOs, no empty implementations, no placeholder returns
- `crates/tome/src/machine.rs` — no stubs; all 13+ test cases populated with real logic
- `crates/tome/src/lib.rs` — `resolve_git_directories` is substantive (clone/update/warn/fallback paths)
- `crates/tome/src/remove.rs` — execute() does real filesystem work; dry_run branches skip writes but count
- `crates/tome/src/distribute.rs` — `is_skill_allowed` wired; old `is_disabled` removed

### Human Verification Required

#### 1. Live git clone via `tome sync`

**Test:** Configure a real GitHub URL as a `type = "git"` directory entry in `~/.tome/tome.toml` and run `tome sync`.
**Expected:** Repository cloned to `~/.tome/repos/<sha256>/`, skills discovered, distributed to target directories.
**Why human:** Requires outbound network access and a real git repository; cannot verify in static analysis.

#### 2. `tome sync` after network failure on cached repo

**Test:** With a previously cloned git source, disconnect from network and run `tome sync`.
**Expected:** Warning on stderr ("could not update ... using cached state"), sync continues with cached skills.
**Why human:** Cannot simulate network failure programmatically in a static check.

#### 3. Per-directory filtering in `machine.toml`

**Test:** Add `[directory.my-source]\ndisabled = ["unwanted-skill"]` to `~/.config/tome/machine.toml`, run `tome sync`.
**Expected:** `unwanted-skill` is not symlinked into `my-source` distribution directory but remains in other directories.
**Why human:** Requires a live sync with a real populated library.

### Gaps Summary

No gaps. All 13 requirements are satisfied, all 13 observable truths are verified, all key links are wired, and all 459 tests pass with no clippy warnings.

---

_Verified: 2026-04-15_
_Verifier: Claude (gsd-verifier)_

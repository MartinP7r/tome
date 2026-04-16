# Phase 2: Git Sources & Selection - Research

**Researched:** 2026-04-15
**Domain:** Git subprocess management, TOML nested config deserialization, CLI subcommand design
**Confidence:** HIGH

## Summary

Phase 2 adds three capabilities to tome: (1) git repository cloning/updating as a skill source, (2) per-directory skill filtering in machine.toml, and (3) a `tome remove` command for directory cleanup. The codebase is well-prepared — `DirectoryType::Git` already exists with validated branch/tag/rev fields, the discover module already routes Git type to flat directory scanning, and the lockfile already has `git_commit_sha` provenance fields.

The core technical work is: a new `git.rs` module that shells out to `git` for clone/fetch operations, extending `MachinePrefs` with nested TOML tables for per-directory filtering, wiring git resolution into the sync pipeline as a pre-discovery step, and adding the `Remove` CLI subcommand with full cleanup logic. All patterns needed (subprocess management, atomic writes, TTY-aware confirmation, dry_run threading) already exist in the codebase.

**Primary recommendation:** Implement in this order: (1) git.rs module with clone/fetch, (2) TomePaths repos_dir + git resolution wiring in sync, (3) subdir field on DirectoryConfig, (4) per-directory machine.toml filtering, (5) tome remove command.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Repo cache directory named by SHA-256 of URL: `~/.tome/repos/<sha256(url)>/`. Deterministic and path-safe.
- **D-02:** Optional `subdir` field on `DirectoryConfig` for git directories. After clone, discovery scans `<clone_path>/<subdir>/` instead of root. Handles monorepos and dotfiles repos where skills aren't at the root level.
- **D-03:** When branch/tag/rev are all omitted, track remote HEAD (whatever the repo's default branch is). No hardcoded default.
- **D-04:** Shallow clone (`--depth 1`) per GIT-02. Update via `git fetch --depth 1 origin <ref> && git reset --hard FETCH_HEAD` per GIT-03. Not `git pull`.
- **D-05:** branch/tag/rev fields are mutually exclusive (already validated in config.rs from Phase 1).
- **D-06:** Per-directory skill filtering uses nested TOML tables in machine.toml: `[directory.<name>]` sections with `disabled` and `enabled` keys.
- **D-07:** `disabled` + `enabled` on the same directory = validation error (MACH-04).
- **D-08:** Resolution order follows locality principle (most specific wins): per-directory `enabled` (allowlist, strongest) > per-directory `disabled` (blocklist) > global `disabled` (broad default). This overrides MACH-05 as originally written.
- **D-09:** Git fetch failure during sync: warn to stderr, continue using last successfully cloned state. Local directories sync normally. If no cached clone exists (first-time failure), skip that directory entirely with a warning.
- **D-10:** Distinct messages for "never cloned" vs "clone exists but fetch failed".
- **D-11:** Git operations clear `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` env vars to prevent interference from calling environment (GIT-06).
- **D-12:** `tome remove <name>` does full cleanup: config entry + library skills from that directory + symlinks in distribution directories + cached git repo (if git type) + manifest/lockfile entries.
- **D-13:** Interactive confirmation in TTY (show what will be removed, y/N default No), auto-remove in non-TTY with warning output. Matches existing cleanup behavior.
- **D-14:** Supports `--dry-run` (preview without acting) and `--force` (skip confirmation). Consistent with `tome sync` flag patterns.

### Claude's Discretion
- Git error message wording (as long as it distinguishes clone-vs-update failure)
- Internal module organization for git operations (separate `git.rs` module or inline in discovery)
- Whether `subdir` field appears in the wizard flow or is config-only for now
- Exact format of the `tome remove` preview table

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GIT-01 | `type = "git"` directory config with URL in `path` field | Already exists in config.rs. `DirectoryType::Git` variant + validation in place. |
| GIT-02 | Shallow clone (`--depth 1`) to `~/.tome/repos/<sha256(url)>/` with `.git` intact | New git.rs module. Use `std::process::Command` + SHA-256 URL hashing (sha2 crate already in deps). |
| GIT-03 | Update via `git fetch --depth 1 origin <ref> && git reset --hard FETCH_HEAD` | git.rs `update_repo()` function. Pattern from D-04. |
| GIT-04 | Branch/tag/SHA pinning via `branch`, `tag`, `rev` fields (mutually exclusive) | Fields already exist on DirectoryConfig. Git module reads them to determine ref for fetch. |
| GIT-05 | Resolved commit SHA recorded in lockfile for reproducibility | Read HEAD SHA via `git rev-parse HEAD` after clone/fetch. Wire into DiscoveredSkill's provenance. |
| GIT-06 | All git commands clear `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` env vars | Use `.env_remove()` on Command builder in git.rs helper functions. |
| GIT-07 | Git resolution runs as pre-discovery step, resolves URLs to local cache paths | New function in lib.rs sync pipeline, called before `discover::discover_all()`. Mutates a map of dir_name -> resolved_local_path. |
| GIT-08 | Failed git operations fall back to cached state, don't abort sync of local directories | Result-based error handling with warn-and-continue pattern. Already established in sync pipeline. |
| MACH-02 | Per-directory `disabled` set (blocklist) in machine.toml | Extend MachinePrefs with `directory: BTreeMap<DirectoryName, DirectoryPrefs>` where DirectoryPrefs has `disabled: BTreeSet<SkillName>`. |
| MACH-03 | Per-directory `enabled` set (exclusive allowlist) in machine.toml | Same DirectoryPrefs struct with `enabled: Option<BTreeSet<SkillName>>`. |
| MACH-04 | `disabled` + `enabled` on same directory = validation error | Validation in MachinePrefs load or a dedicated validate method. |
| MACH-05 | Resolution: locality principle (per-directory enabled > per-directory disabled > global disabled) — as amended by D-08 | New method on MachinePrefs: `is_skill_disabled_for_directory(skill, dir_name) -> bool`. Called in distribute.rs. |
| CLI-01 | `tome remove <directory-name>` removes entry from config, cleans up library + symlinks | New Remove variant in Command enum. Handler in lib.rs dispatches to a remove module. |
</phase_requirements>

## Architecture Patterns

### New Module: `git.rs`

Create a dedicated `crates/tome/src/git.rs` module for all git subprocess operations. This follows the existing pattern where `backup.rs` wraps git commands for backup operations.

```
crates/tome/src/
├── git.rs           # NEW: clone, fetch, update, SHA reading
├── config.rs        # MODIFY: add subdir field to DirectoryConfig
├── machine.rs       # MODIFY: add per-directory filtering
├── distribute.rs    # MODIFY: use new filtering logic
├── cli.rs           # MODIFY: add Remove subcommand
├── lib.rs           # MODIFY: git resolution pre-step, Remove dispatch
├── paths.rs         # MODIFY: add repos_dir() to TomePaths
└── ...
```

### Pattern: Git Subprocess Helper with Env Clearing

The git.rs module should provide a helper similar to backup.rs but with mandatory env clearing per GIT-06:

```rust
// Source: project pattern from backup.rs + GIT-06 decision
use std::process::Command;
use std::path::Path;
use anyhow::{Context, Result};

fn git_command(repo_dir: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(repo_dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))
}
```

### Pattern: Pre-Discovery Git Resolution

Git resolution fits as a pre-step in the sync pipeline, transforming git directory configs into usable local paths before discovery runs. The key insight is that discover.rs already handles `DirectoryType::Git` identically to `DirectoryType::Directory` (flat scan). The only missing piece is materializing the repo to a local path first.

```rust
// In lib.rs sync(), before discover_all():
// 1. Iterate config.directories where type == Git
// 2. For each: clone or update to repos_dir/<sha256(url)>/
// 3. Store resolved local path (accounting for subdir) in a map
// 4. Pass resolved paths to discovery (or mutate config temporarily)
```

The cleanest approach: create a `ResolvedDirectories` struct or use a `BTreeMap<DirectoryName, PathBuf>` that maps directory names to their effective local paths. During discovery, check this map first; if a directory name has a resolved path, use that instead of `dir_config.path`.

### Pattern: Per-Directory Machine Prefs with Nested TOML

The TOML format for per-directory filtering:

```toml
# Global disabled (existing)
disabled = ["unwanted-skill"]

# Per-directory sections (new)
[directory.my-git-source]
disabled = ["skill-i-dont-want-from-here"]

[directory.another-source]
enabled = ["only-this-skill", "and-this-one"]
```

serde handles nested TOML tables naturally with `BTreeMap<DirectoryName, DirectoryPrefs>`.

### Pattern: Remove Command Cleanup Order

The `tome remove` cleanup must happen in dependency order to avoid orphaned state:

1. Remove symlinks from distribution directories (uses manifest to find which skills came from this directory)
2. Remove library entries for skills from this directory
3. Remove manifest entries
4. Remove cached git repo (if git type)
5. Remove directory entry from config
6. Regenerate lockfile

### Anti-Patterns to Avoid
- **Don't use `git pull`:** The `fetch + reset --hard` pattern (D-04) avoids merge conflicts and handles shallow repos correctly.
- **Don't hardcode "main" as default branch:** Use remote HEAD detection when no branch/tag/rev specified (D-03).
- **Don't share subprocess helpers with backup.rs:** The git.rs module has different env-clearing requirements. Keep separate, even if similar.
- **Don't mutate config in memory without saving:** `tome remove` must persist config changes atomically.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| URL-to-path hashing | Custom hash function | `sha2::Sha256` (already in deps) | Deterministic, collision-resistant, hex-encode for path safety |
| TOML nested table parsing | Manual key extraction | `serde` derive with `BTreeMap<DirectoryName, DirectoryPrefs>` | serde handles `[directory.*]` tables natively |
| TTY detection for prompts | Manual stdin check | `std::io::IsTerminal` + `dialoguer::Confirm` (already in deps) | Established pattern in cleanup.rs and eject.rs |
| Git ref resolution | Parse `.git/HEAD` manually | `git rev-parse HEAD` subprocess | Works for all ref types (branch, tag, detached HEAD) |

## Common Pitfalls

### Pitfall 1: Git Environment Variable Interference
**What goes wrong:** tome runs inside a git worktree (or is invoked from a git hook). The calling environment's `GIT_DIR`, `GIT_WORK_TREE`, or `GIT_INDEX_FILE` vars cause git operations to target the wrong repo.
**Why it happens:** `std::process::Command` inherits the parent environment by default.
**How to avoid:** Every `Command::new("git")` in git.rs MUST chain `.env_remove("GIT_DIR").env_remove("GIT_WORK_TREE").env_remove("GIT_INDEX_FILE")`. Centralize in a single helper function.
**Warning signs:** Tests pass locally but fail in CI, or git operations affect the project repo instead of the cache repo.

### Pitfall 2: Shallow Clone Update Race
**What goes wrong:** `git fetch --depth 1 origin <ref>` fails because the ref doesn't exist on the remote (deleted tag, force-pushed branch).
**Why it happens:** Pinned refs can become stale.
**How to avoid:** Catch fetch failures and fall back to existing cached state (D-09). Log the specific ref that failed.
**Warning signs:** `tome sync` fails for a single git source and aborts the entire pipeline.

### Pitfall 3: TOML Deserialization with deny_unknown_fields
**What goes wrong:** Adding `directory` field to `MachinePrefs` works, but `[directory.*]` nested tables may conflict with the existing flat `disabled`/`disabled_directories` fields if serde ordering is wrong.
**Why it happens:** `#[serde(deny_unknown_fields)]` on MachinePrefs (if present) would reject the new `[directory.*]` sections.
**How to avoid:** MachinePrefs currently does NOT have `deny_unknown_fields` (verified in code review). Keep it that way for forward compatibility. Use `#[serde(default)]` on the new `directory` field.
**Warning signs:** Existing machine.toml files fail to parse after the change.

### Pitfall 4: Config File Concurrent Modification
**What goes wrong:** `tome remove` reads config, modifies it, and writes back. If another process modified config between read and write, changes are lost.
**Why it happens:** No file locking.
**How to avoid:** Use the atomic temp+rename pattern already used for manifest/lockfile. For a single-user tool this is adequate — document that concurrent tome invocations are unsupported.

### Pitfall 5: Subdir Discovery Path Confusion
**What goes wrong:** Discovery scans the repo root instead of `<clone_path>/<subdir>/`, finding no skills or the wrong skills.
**Why it happens:** The `subdir` field needs to be applied when constructing the discovery path, not just stored in config.
**How to avoid:** In the git resolution step, compute the effective path as `clone_path.join(subdir)` and pass THAT to discovery. The `subdir` resolution should be invisible to the rest of the pipeline.

### Pitfall 6: Remove Command Partial Cleanup
**What goes wrong:** `tome remove` deletes config entry but fails partway through cleanup, leaving orphaned library entries and symlinks.
**Why it happens:** Filesystem operations can fail (permission denied, file in use).
**How to avoid:** Perform cleanup in the correct order (symlinks first, then library entries, then config). Collect warnings for individual failures rather than aborting. The `doctor` command can repair remaining issues.

## Code Examples

### Git Clone (Shallow)

```rust
// Source: project patterns from backup.rs + decisions D-01, D-02, D-04
pub fn clone_repo(url: &str, dest: &Path, ref_spec: Option<&str>) -> Result<()> {
    let mut args = vec!["clone", "--depth", "1"];
    if let Some(r) = ref_spec {
        args.extend(["--branch", r]);
    }
    args.push(url);
    args.push(dest.to_str().context("non-UTF8 path")?);

    let output = Command::new("git")
        .args(&args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .context("failed to run git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git clone failed: {}", stderr.trim());
    }
    Ok(())
}
```

### Git Update (Fetch + Reset)

```rust
// Source: decision D-04
pub fn update_repo(repo_dir: &Path, ref_spec: Option<&str>) -> Result<()> {
    let fetch_ref = ref_spec.unwrap_or("HEAD");
    git_success(repo_dir, &["fetch", "--depth", "1", "origin", fetch_ref])?;
    git_success(repo_dir, &["reset", "--hard", "FETCH_HEAD"])?;
    Ok(())
}
```

### URL to Cache Path

```rust
// Source: decision D-01
use sha2::{Digest, Sha256};

pub fn repo_cache_dir(repos_dir: &Path, url: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    repos_dir.join(hash)
}
```

### Per-Directory Skill Filter Resolution

```rust
// Source: decision D-08
pub fn is_skill_allowed(
    &self,
    skill_name: &str,
    dir_name: &str,
) -> bool {
    // Check per-directory enabled (allowlist — strongest)
    if let Some(dir_prefs) = self.directory.get(dir_name) {
        if let Some(enabled) = &dir_prefs.enabled {
            return enabled.contains(skill_name);
        }
        if dir_prefs.disabled.contains(skill_name) {
            return false;
        }
    }
    // Fall back to global disabled
    !self.disabled.contains(skill_name)
}
```

### MachinePrefs Extended Structure

```rust
// Source: decisions D-06, D-07, D-08
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MachinePrefs {
    #[serde(default)]
    pub(crate) disabled: BTreeSet<SkillName>,
    #[serde(default)]
    pub(crate) disabled_directories: BTreeSet<DirectoryName>,
    #[serde(default)]
    pub(crate) directory: BTreeMap<DirectoryName, DirectoryPrefs>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectoryPrefs {
    #[serde(default)]
    pub(crate) disabled: BTreeSet<SkillName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) enabled: Option<BTreeSet<SkillName>>,
}
```

### Remove CLI Subcommand

```rust
// Source: decisions D-12, D-13, D-14
#[derive(Subcommand)]
pub enum Command {
    // ... existing variants ...

    /// Remove a directory entry and clean up its artifacts
    #[command(after_help = "Examples:\n  tome remove my-git-source\n  tome remove my-git-source --dry-run\n  tome remove my-git-source --force")]
    Remove {
        /// Name of the directory to remove (as shown in `tome status`)
        #[arg(value_name = "NAME")]
        name: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `git pull` for updates | `git fetch --depth 1 + reset --hard` | Project decision D-04 | Avoids merge conflicts, handles shallow repos, deterministic state |
| Deep clones for source repos | Shallow clone `--depth 1` | Standard practice, project decision | Massive bandwidth/time savings for large repos |
| Flat machine.toml disabled list | Nested per-directory filtering | Phase 2 design | Granular control matching Claude Code's specificity model |

## Open Questions

1. **Git clone for `rev` (SHA) pinning**
   - What we know: `git clone --branch` works for branches and tags, but NOT for arbitrary SHAs. You can't `git clone --branch <sha>`.
   - What's unclear: The exact clone flow for SHA-pinned repos.
   - Recommendation: For `rev` field: clone without `--branch`, then `git fetch --depth 1 origin <sha> && git reset --hard FETCH_HEAD`. Note: shallow fetch of specific SHAs requires `uploadpack.allowReachableSHA1InWant=true` on the server. GitHub supports this. Other hosts may not. Document this limitation.

2. **Repos dir lifecycle management**
   - What we know: `~/.tome/repos/` stores cloned repos. `tome remove` cleans up the specific repo.
   - What's unclear: Should `tome doctor` also detect orphaned repos in `~/.tome/repos/` that no longer correspond to any config entry?
   - Recommendation: Yes, add orphan repo detection to `doctor.rs` as a follow-up task. Not blocking for Phase 2 but natural extension.

3. **Git binary availability**
   - What we know: git is a runtime dependency for this feature. Currently git is available (v2.53.0).
   - What's unclear: Error messaging when git is not installed.
   - Recommendation: Check `command -v git` equivalent (probe in Rust) before git operations. Bail with "git is required for git-type directories but was not found in PATH" if missing.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git | Git source clone/fetch | Yes | 2.53.0 | None (feature requires git) |
| sha2 crate | URL hashing for cache dirs | Yes | 0.11 (in Cargo.toml) | -- |
| dialoguer | Remove confirmation prompts | Yes | 0.12 (in Cargo.toml) | -- |

**Missing dependencies with no fallback:** None.

## Project Constraints (from CLAUDE.md)

- **Rust edition 2024.** Strict clippy with `-D warnings`.
- **Unix-only.** Symlinks via `std::os::unix::fs::symlink`.
- **Non-interactive shell commands:** Always use `-f` flags for cp/mv/rm.
- **Error handling:** `anyhow::Result<T>` throughout, `.with_context()` for operation context.
- **Testing:** Unit tests co-located in modules. Integration tests via `assert_cmd` in `tests/cli.rs`. Use `tempfile::TempDir` for isolation.
- **Atomic writes:** temp+rename pattern for config/manifest/lockfile changes.
- **dry_run threading:** All operations accept `dry_run: bool`, skip filesystem writes.
- **No backward compat:** Single user, hard-breaking changes OK with migration docs.
- **Quality gates:** `make ci` (fmt-check + lint + test) before merge.
- **bd (beads)** for issue tracking, not markdown TODOs.

## Sources

### Primary (HIGH confidence)
- Direct code review: `config.rs`, `machine.rs`, `backup.rs`, `discover.rs`, `distribute.rs`, `lib.rs`, `cli.rs`, `lockfile.rs`, `manifest.rs` — all current codebase state verified
- `docs/v06-implementation-plan.md` — original design for git sources
- `.planning/phases/02-git-sources-selection/02-CONTEXT.md` — locked decisions from discussion

### Secondary (MEDIUM confidence)
- Git shallow clone behavior with SHA refs — verified via general git documentation knowledge; GitHub support for `allowReachableSHA1InWant` is well-established

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates already in Cargo.toml, no new dependencies needed
- Architecture: HIGH — patterns established in Phase 1, integration points clearly identified in code review
- Pitfalls: HIGH — git subprocess patterns well-understood, TOML nesting verified against serde behavior

**Research date:** 2026-04-15
**Valid until:** 2026-05-15 (stable domain, no external API dependencies)

# Phase 12: Marketplace adapter - Research

**Researched:** 2026-05-05
**Domain:** Trait-based marketplace adapter abstraction (Claude CLI subprocess + git wrap)
**Confidence:** HIGH (all citations verified against current codebase + claude 2.1.128 probes)

## User Constraints (from CONTEXT.md)

### Locked Decisions
D-01..D-11 verbatim from `12-CONTEXT.md::<decisions>`. The planner MUST honor:
- D-01: stdin closed (`</dev/null`); no env vars; capture stderr verbatim into `InstallFailure::source`; non-zero exit = failure; no probing.
- D-02: `available()` reads cached `claude plugin list --json` snapshot's `errors[]` field — zero extra subprocess calls.
- D-03: no subprocess timeout in v0.10.
- D-04: internal cache (`RefCell<Option<Vec<InstalledPlugin>>>` or equiv) auto-invalidates on `install()`/`update()` Ok; public `refresh()`.
- D-05: one `GitAdapter` per `[directories.<git-name>]` entry; bound to URL + ref pin.
- D-05a: existing git-source integration tests in `crates/tome/tests/cli.rs` continue to pass byte-for-byte.
- D-06: `InstallFailure` struct + `InstallOp` enum + `InstallFailureKind` enum + `ALL: &'static [..]` (POLISH-04 pattern). NO `path` field.
- D-07: `Vec<InstallFailure>` aggregator + grouped renderer (mirrors SAFE-01); planner picks `marketplace.rs` vs `lib.rs` for the helper.
- D-08: trait surface locked exactly as in CONTEXT.md (`id`, `current_version`, `install`, `update`, `list_installed`, `available`).
- D-09: `claude plugin install` uses default scope (user) — no `--scope` flag.
- D-10: `MockMarketplaceAdapter` lives `#[cfg(test)]` in `marketplace.rs`.
- D-11: dispatch by `DirectoryType`; Phase 13 owns the dispatcher; Phase 12 ships only the trait + adapters.

### Claude's Discretion
- Exact rendering text of `⚠ N install operations failed`.
- Heuristic mapping `claude` stderr → `InstallFailureKind` (default `Unknown`).
- `RefCell` vs `OnceCell` vs `Mutex` for the snapshot cache (planner picks based on Send/Sync).
- `GitAdapter::available()` network probe vs trust local-clone (recommendation: trust local-clone).
- `claude` binary detection mechanism (recommendation: `Command::new("claude").arg("--version").output()` — `which` not in deps; see §Binary Detection).
- `MockMarketplaceAdapter` shape (failure injection vs static fixtures).
- Whether `InstallFailure` renderer lives in `marketplace.rs` or `lib.rs`.

### Deferred Ideas (OUT OF SCOPE)
- Subprocess timeout knob (D-03).
- `--scope project|local` (D-09).
- `claude plugin list --available --json` catalog query.
- Async `MarketplaceAdapter` variant.
- Non-Claude/non-git marketplace adapters.
- Lifting `MockMarketplaceAdapter` to `pub(crate)` for integration-test reuse (Phase 13 may need this).
- Drift detection / install-consent / sync flow integration → Phase 13.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ADP-01 | `MarketplaceAdapter` trait in new `marketplace.rs` with locked surface; `Result` everywhere | §Pattern Verification (mirror remove.rs `pub(crate)` shape); §File Layout (single file) |
| ADP-02 | `ClaudeMarketplaceAdapter` shells out to `claude plugin install/update/list --json`; clear error if `claude` not on PATH | §claude CLI JSON Shape (verified 2026-05-05); §Subprocess Pattern (mirror git.rs); §Binary Detection (`Command::new("claude").arg("--version")`) |
| ADP-03 | `GitAdapter` wraps `git.rs::clone_repo` / `update_repo`; behavior unchanged | §git.rs API Surface (5 helpers all `pub(crate)` — no widening needed; visible from sibling module) |
| ADP-04 | `Vec<InstallFailure>` aggregates; grouped failure summary mirrors v0.8 SAFE-01; sync exits non-zero on partial install failure | §Pattern Verification (FailureKind/RemoveFailure template); §Existing Failure Renderer (lib.rs:444-468 — direct template) |

---

## Pattern Verification

### `FailureKind` enum (template for `InstallFailureKind`)

`crates/tome/src/remove.rs:62-70`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureKind {
    /// Distribution-dir symlink removal — emitted when `remove_file` fails
    /// while iterating `plan.symlinks_to_remove`.
    DistributionSymlink,
    /// Git repo cache removal — emitted when `remove_dir_all` fails on the
    /// plan's `git_cache_path`.
    GitCache,
}
```

Derives: `Debug, Clone, Copy, PartialEq, Eq`. Visibility: `pub(crate)`. Note CONTEXT.md D-06 says `InstallFailureKind` should be `pub` because the trait is the public crate surface — planner decides whether `pub` or `pub(crate)`. The `enum InstallOp { Install, Update }` should mirror these derives.

### `ALL` array + label (template for `InstallFailureKind::ALL`)

`crates/tome/src/remove.rs:72-90`:

```rust
impl FailureKind {
    /// All variants, in the order preferred for user-facing grouped output.
    pub(crate) const ALL: [FailureKind; 2] = [
        FailureKind::DistributionSymlink,
        FailureKind::GitCache,
    ];

    /// Human-readable label used in the grouped failure summary.
    pub(crate) fn label(self) -> &'static str {
        match self {
            FailureKind::DistributionSymlink => "Distribution symlinks",
            FailureKind::GitCache => "Git cache",
        }
    }
}
```

Note the type signature: `[FailureKind; 2]` (fixed-size array, NOT slice). CONTEXT.md D-06 wrote it as `&'static [InstallFailureKind]` — the planner should match the existing convention (`[InstallFailureKind; N]`) for consistency unless a slice is actively needed.

### Compile-time exhaustiveness sentinel (POLISH-04)

`crates/tome/src/remove.rs:92-117`:

```rust
/// Compile-time drift guard for `FailureKind::ALL` (POLISH-04 option c).
///
/// If a new variant is added to `FailureKind`, this `const fn` fails to
/// compile because the match below is exhaustive.
#[allow(dead_code)]
const fn _ensure_failure_kind_all_exhaustive(k: FailureKind) -> usize {
    match k {
        FailureKind::DistributionSymlink => 0,
        FailureKind::GitCache => 1,
    }
}

const _: () = {
    // If this fails: FailureKind::ALL is missing or has extra variants.
    // The match arms in _ensure_failure_kind_all_exhaustive are the source
    // of truth — ALL must contain exactly one entry per arm.
    assert!(FailureKind::ALL.len() == 2);
};
```

The pattern is **two-part**: (1) a `#[allow(dead_code)] const fn _ensure_*_exhaustive` that exhaustively matches every variant, paired with (2) a `const _: () = { assert!(... ALL.len() == N); };` block. Both must be present. Mirror this exactly for `InstallFailureKind` with its variant count.

### `RemoveFailure` struct (template for `InstallFailure`)

`crates/tome/src/remove.rs:119-148`:

```rust
/// A single partial-cleanup failure aggregated from `execute`.
#[derive(Debug)]
pub(crate) struct RemoveFailure {
    pub path: PathBuf,
    pub kind: FailureKind,
    pub error: std::io::Error,
}

impl RemoveFailure {
    pub(crate) fn new(kind: FailureKind, path: PathBuf, error: std::io::Error) -> Self {
        debug_assert!(
            path.is_absolute(),
            "RemoveFailure::path must be absolute, got: {}",
            path.display()
        );
        RemoveFailure { kind, path, error }
    }
}
```

Single derive: `Debug` only (no `Clone` — `std::io::Error` isn't `Clone`). `InstallFailure` per D-06 carries `anyhow::Error` (also not `Clone`); same single-derive shape applies.

**Differences `InstallFailure` vs `RemoveFailure`:**
- No `path` field (D-06: install-time failures have no stable filesystem path)
- Adds `adapter_id: String`, `plugin_id: String`, `operation: InstallOp`
- `error: std::io::Error` → `source: anyhow::Error` (per D-06 verbatim)

**Test scaffolding to mirror** (`remove.rs::tests::*`):
- `failure_kind_label_coverage` (line 640): exhaustive label assertion
- `failure_kind_all_pinned_size_two` (line 653): pin `ALL.len()` + variant membership
- `failure_kind_all_length_matches_variant_count` (line 664): pairwise-uniqueness check (`FailureKind` only derives `PartialEq/Eq`, no `Ord/Hash` — uses nested loop, NOT `BTreeSet`/`HashSet`)
- `failure_kind_all_ordering_pinned` (line 687): pin declaration order (user-visible grouping contract)

---

## git.rs API Surface

All five helpers `GitAdapter` will wrap. All are `pub(crate)` already — `marketplace.rs` is a sibling module so **no visibility widening required**.

### `clone_repo` — `crates/tome/src/git.rs:77-117`

```rust
pub(crate) fn clone_repo(
    url: &str,
    dest: &Path,
    branch: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
) -> Result<()>
```

**Behavior:** Shallow clone (`--depth 1`). Branch/tag → `--branch <ref>`. Rev (SHA) → post-clone `fetch --depth 1 origin <sha>` + `reset --hard FETCH_HEAD`. Clears `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE` env. Bails with stderr on non-zero exit.

### `update_repo` — `crates/tome/src/git.rs:123-133`

```rust
pub(crate) fn update_repo(
    repo_dir: &Path,
    branch: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
) -> Result<()>
```

**Behavior:** Computes `fetch_ref = branch.or(tag).or(rev).unwrap_or("HEAD")`, runs `git fetch --depth 1 origin <ref> && git reset --hard FETCH_HEAD`.

### `read_head_sha` — `crates/tome/src/git.rs:138-140`

```rust
pub(crate) fn read_head_sha(repo_dir: &Path) -> Result<String>
```

**Behavior:** `git rev-parse HEAD`, returns the 40-char hex string. Used for `current_version()` per D-05 mapping.

### `repo_cache_dir` — `crates/tome/src/git.rs:48-57`

```rust
pub(crate) fn repo_cache_dir(repos_dir: &Path, url: &str) -> PathBuf
```

**Behavior:** Returns `repos_dir/<sha256(url)>` (64-char lowercase hex). Deterministic. Used for `install_path` per D-08 `InstalledPlugin`.

### `ref_spec_for_config` — `crates/tome/src/git.rs:64-71`

```rust
pub(crate) fn ref_spec_for_config<'a>(
    branch: Option<&'a str>,
    tag: Option<&'a str>,
    rev: Option<&'a str>,
) -> Option<&'a str>
```

**Behavior:** Returns `branch.or(tag)` (rev returns `None` because rev uses different clone flow). Likely not directly needed by `GitAdapter` since `clone_repo`/`update_repo` already accept the three Options — but useful if `available()` ever wants to check ref spec.

### Bonus helpers

- `effective_path(clone_path: &Path, subdir: Option<&str>) -> PathBuf` — `git.rs:145-150`. Returns `clone_path/<subdir>` or `clone_path`. May be useful when constructing `InstalledPlugin::install_path` if subdir is set.
- `is_git_available() -> bool` — `git.rs:155-164`. Probes `git --version` with env clearing. **This is the prior-art template for `is_claude_available()`** (see §Binary Detection).

### Mapping to `MarketplaceAdapter` (per D-05)

| Trait method | GitAdapter implementation |
|--------------|---------------------------|
| `id()` | the git URL string (held in adapter state) |
| `current_version(_)` | `git::read_head_sha(repo_cache_dir(repos, url))` if cache exists; `Ok(None)` otherwise |
| `install(_)` | `git::clone_repo(url, repo_cache_dir(...), branch, tag, rev)` |
| `update(_)` | `git::update_repo(repo_cache_dir(...), branch, tag, rev)` |
| `list_installed()` | `vec![InstalledPlugin { id: url, version: HEAD sha, install_path: cache_dir, errors: vec![] }]` if cloned; `vec![]` otherwise |
| `available(_)` | `Ok(true)` if cache_dir exists (recommendation: trust local existence; URLs don't "vanish") |

GitAdapter ignores its `plugin_id: &str` argument because there's only one "plugin" per git directory (the repo itself).

---

## config.rs Types

### `DirectoryType` enum — `crates/tome/src/config.rs:90-100`

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryType {
    /// Reads installed_plugins.json for plugin-based discovery
    ClaudePlugins,
    /// Scans for */SKILL.md directly
    #[default]
    Directory,
    /// Clones/pulls a remote git repository
    Git,
}
```

Three variants, no payloads. D-11 dispatch is straightforward `match`:
- `Git` → `GitAdapter::for_directory(&dir_config)`
- `ClaudePlugins` → `ClaudeMarketplaceAdapter::new()` (singleton)
- `Directory` → no adapter (skill discovery only)

### `DirectoryConfig` struct — `crates/tome/src/config.rs:244-271`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "DirectoryConfigRaw", into = "DirectoryConfigRaw")]
pub struct DirectoryConfig {
    pub path: PathBuf,                          // For git: this IS the URL (string in PathBuf)
    pub directory_type: DirectoryType,
    pub(crate) role: Option<DirectoryRole>,
    pub git_ref: Option<GitRef>,                // Branch/Tag/Rev — exactly one
    pub subdir: Option<String>,                 // git type only
    pub(crate) override_applied: bool,          // machine-local; never written to tome.toml
}
```

For a git directory:
- **URL:** `dir_config.path.to_string_lossy()` (see `lib.rs:831` and `remove.rs:241-243` for the existing pattern — note that `remove.rs` uses `to_str().ok_or_else(...)` which bails on invalid UTF-8; `lib.rs:831` uses `to_string_lossy()`. Planner picks; `to_str()` + bail is the safer pattern).
- **Ref pin:** `dir_config.git_ref.as_ref().and_then(|r| r.branch())`, `.tag()`, `.rev()` — see `GitRef` impls at `config.rs:210-232`.

### `GitRef` enum — `crates/tome/src/config.rs:201-208`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRef {
    Branch(String),
    Tag(String),
    Rev(String),
}
```

Accessor methods (config.rs:212-231): `branch() -> Option<&str>`, `tag() -> Option<&str>`, `rev() -> Option<&str>`. Mutually-exclusive by construction — exactly one `Some()` at any time.

### `DirectoryName` newtype — `crates/tome/src/config.rs:18`

```rust
pub struct DirectoryName(String);
```

Validating newtype with transparent serde. Not directly needed for Phase 12 (the adapter takes `plugin_id: &str` per D-08), but appears in upstream `InstalledPlugin` consumers (Phase 13).

---

## Existing Failure Renderer

### Location: `crates/tome/src/lib.rs:444-468`

This is the SAFE-01 grouped failure renderer for the `Command::Remove` arm. Direct template for the install-failure renderer (D-07).

```rust
if !result.failures.is_empty() {
    let k = result.failures.len();
    eprintln!(
        "{} {} operations failed during remove of '{}' — config entry and \
         manifest retained so you can retry after addressing these. \
         Run {} after resolving:",
        style("⚠").yellow(),
        k,
        name,
        style("`tome doctor`").bold(),
    );

    for kind in crate::remove::FailureKind::ALL {
        let group: Vec<&crate::remove::RemoveFailure> =
            result.failures.iter().filter(|f| f.kind == kind).collect();
        if group.is_empty() {
            continue;
        }
        eprintln!("  {} ({}):", kind.label(), group.len());
        for f in group {
            eprintln!("    {}: {}", paths::collapse_home(&f.path), f.error);
        }
    }

    return Err(anyhow::anyhow!("remove completed with {k} failures"));
}
```

### Rendering pattern (key elements to mirror)

1. **Glyph + count + summary line** — `style("⚠").yellow()`, count, plain-English line, retry hint.
2. **Per-kind grouping loop** — iterates `FailureKind::ALL` (NOT `result.failures` directly). Skips empty groups.
3. **Per-kind header** — `eprintln!("  {} ({}):", kind.label(), group.len())`.
4. **Per-failure indented line** — uses `paths::collapse_home(&f.path)` for `~/`-prefixed display.
5. **Returns `Err`** — non-zero exit code via `anyhow::anyhow!("... completed with {k} failures")`.

### Reusability assessment

**NOT generic over kind enum.** It's hand-coded for `RemoveFailure` and `FailureKind` types directly (`crate::remove::RemoveFailure`, filter on `.kind == kind` — not via a trait). Per D-07, **marketplace.rs ships its own renderer**. This is consistent with the existing pattern: each domain (`remove`, future `install`) gets its own typed renderer. Trying to make this generic would require lifting `FailureKind` to a trait — out of scope for Phase 12.

**Adaptation for `InstallFailure`:** the per-failure line drops `paths::collapse_home(&f.path)` (no path field) and instead uses something like `format!("{}/{} ({:?}): {:#}", f.adapter_id, f.plugin_id, f.operation, f.source)`. The `source: anyhow::Error` should render with `{:#}` (debug-with-context) to surface the chain. Exact format is Claude's discretion per CONTEXT.md.

---

## claude CLI JSON Shape

### Verified probe (claude 2.1.128, 2026-05-05)

`claude plugin list --json </dev/null` returns a flat JSON array. Verified union of keys across 37 entries on Martin's machine:

```
['enabled', 'id', 'installPath', 'installedAt', 'lastUpdated', 'mcpServers', 'scope', 'version']
```

**Important:** `errors` was NOT present on any entry in the live snapshot (zero plugins currently in error state). CONTEXT.md `<empirical_findings>` documented `errors[]` from a 2026-05-04 probe — that empirical observation stands; the field is **conditional** (only present when an entry has marketplace errors) and `Option<Vec<String>>` with `#[serde(default)]` is the right shape.

**Note:** the live snapshot also revealed `mcpServers` as a key — not in CONTEXT.md's enumeration. Adapter should ignore it (or capture as opaque `serde_json::Value` for forward compat). `InstalledPlugin` per D-08 doesn't carry it.

### Sample entry (canonical happy path)

```json
{
  "id": "axiom@axiom-marketplace",
  "version": "3.3.0",
  "scope": "user",
  "enabled": true,
  "installPath": "/Users/martin/.claude/plugins/cache/axiom-marketplace/axiom/3.3.0",
  "installedAt": "2026-03-17T12:18:08.229Z",
  "lastUpdated": "2026-05-04T11:49:50.948Z"
}
```

### Sample entry (with errors — from CONTEXT.md empirical_findings)

```json
{
  "id": "claude-md-management@claude-plugins-official",
  "version": "1.0.0",
  ...
  "errors": ["Plugin claude-md-management not found in marketplace claude-plugins-official"]
}
```

### Recommended serde shape for `marketplace.rs`

```rust
#[derive(Debug, Deserialize)]
struct ClaudePluginListEntry {
    id: String,                          // "axiom@axiom-marketplace"
    version: String,                     // "3.3.0" or sometimes "unknown"
    #[serde(rename = "installPath")]
    install_path: PathBuf,
    #[serde(default)]                    // absent when no errors
    errors: Vec<String>,
    // scope, enabled, installedAt, lastUpdated, mcpServers — currently unused;
    // add fields here only when a consumer needs them (YAGNI).
}
```

Note: at least one entry observed has `version: "unknown"` (literal string). Adapter should not parse `version` as semver — keep it as `String` per D-08.

### Subprocess invocations (verified 2026-05-05)

| Command | Probe result | Notes |
|---------|-------------|-------|
| `claude plugin list --json </dev/null` | exits 0, ~37 entries | Cache target |
| `claude plugin install nonexistent@nonexistent </dev/null` | exits 1; stderr: `✘ Failed to install plugin "nonexistent@nonexistent": Plugin "nonexistent" not found in marketplace "nonexistent". Your local copy may be out of date — try \`claude plugin marketplace update nonexistent\`.` | Heuristic: stderr contains `not found in marketplace` → `InstallFailureKind::NotFound` |
| `claude plugin update nonexistent </dev/null` | exits 1; stderr: `✘ Failed to update plugin "nonexistent": Plugin "nonexistent" not found` | Bare id (no `@marketplace`) is ACCEPTED at the CLI level — exit 1 only because plugin doesn't exist. Either `plugin` or `plugin@marketplace` is a valid arg shape. |
| `claude plugin update nonexistent@nonexistent </dev/null` | exits 1; stderr: `✘ Failed to update plugin "nonexistent@nonexistent": Plugin "nonexistent" not found` | Both formats accepted. |
| `claude --version </dev/null` | exits 0; stdout: `2.1.128 (Claude Code)` | Binary detection probe |

**Update id format resolution:** CONTEXT.md `<specifics>` flagged "exact id format requires verification". **Confirmed:** `claude plugin update <plugin>` and `claude plugin update <plugin>@<marketplace>` are BOTH valid argument shapes. The `@<marketplace>` qualifier is optional. Adapter should pass whichever form `plugin_id` arrives in (per D-08, callers pass `&str` verbatim — adapter doesn't reformat).

### Help text (verified)

```
Usage: claude plugin install|i [options] <plugin>
Options:
  -s, --scope <scope>  Installation scope: user, project, or local (default: "user")

Usage: claude plugin update [options] <plugin>
Options:
  -s, --scope <scope>  Installation scope: user, project, local, managed (default: user)
```

Per D-09 install uses default scope (no `--scope` flag). Update accepts `managed` as an additional scope per help text — irrelevant for Phase 12.

---

## Binary Detection

### `which` crate — NOT in `Cargo.toml`

`workspace.dependencies` does not include `which`. Adding it would be a new dep. Avoid.

### Recommended approach: `Command::new(...).arg("--version").output()`

Mirror `git::is_git_available()` at `crates/tome/src/git.rs:155-164`:

```rust
pub(crate) fn is_git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
```

**Verified:** `claude --version </dev/null` exits 0 with stdout `2.1.128 (Claude Code)`.

**Recommended `is_claude_available()`** (no env clearing needed — claude has no analogous `GIT_DIR`-like env vars):

```rust
fn is_claude_available() -> bool {
    std::process::Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
```

### When to probe

Per CONTEXT.md "Claude's Discretion": probe at adapter construction (`ClaudeMarketplaceAdapter::new()`). Surface as anyhow error: `error: claude CLI not found on PATH — install Claude Code or remove [directories.<...>] entries with type = "claude-plugins" from tome.toml.` (Wording is Claude's discretion per CONTEXT.md.)

Alternative: probe lazily on first call to a subprocess-invoking method; map `std::io::ErrorKind::NotFound` from `Command::output()` to a clean error. **Prior-art reference** at `crates/tome/src/install.rs:52-63`:

```rust
let output = match std::process::Command::new("claude")
    .args(["plugin", "install", registry_id])
    .output()
{
    Ok(output) => output,
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
    Err(e) => {
        return Err(anyhow::anyhow!(e).context(format!(
            "failed to run `claude plugin install {registry_id}`"
        )));
    }
};
```

**Recommendation:** probe at construction (eager). Single error message, clearer failure mode. The lazy approach made sense in `install.rs` (which silently degrades to "skipped"); the adapter contract per ADP-02 should fail loudly.

---

## Subprocess Pattern

### Reference: `git::clone_repo` — `crates/tome/src/git.rs:97-108`

```rust
let output = std::process::Command::new("git")
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
```

### Reference: `git::git_command` (helper) — `crates/tome/src/git.rs:13-22`

```rust
fn git_command(repo_dir: &Path, args: &[&str]) -> Result<std::process::Output> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(repo_dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))
}
```

### Pattern to mirror for `claude` subprocess

1. **`std::process::Command::new("claude")`** — synchronous, no tokio.
2. **`.args(["plugin", "install", plugin_id])`** — slice form (matches install.rs:53).
3. **stdin:** per D-01, set `stdin = /dev/null`. Use `.stdin(std::process::Stdio::null())`.
4. **No env_remove needed** — claude doesn't have `GIT_DIR`-style env vars.
5. **`.output()`** — captures stdout AND stderr (vs `.status()` which inherits).
6. **Map `ErrorKind::NotFound`** — see install.rs:57 pattern (already a binary-not-found path the codebase handles).
7. **Check `output.status.success()`** — bail with `String::from_utf8_lossy(&output.stderr).trim()` content.
8. **Per D-01, capture stderr verbatim into `InstallFailure::source`** — wrap as `anyhow::anyhow!("{stderr}")` or similar so the full upstream message survives unaltered.

### Suggested helper inside `marketplace.rs`

```rust
fn run_claude_subcommand(args: &[&str]) -> Result<std::process::Output> {
    std::process::Command::new("claude")
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .with_context(|| format!("failed to run `claude {}`", args.join(" ")))
}
```

Then `install`/`update`/`list_installed` each call this and inspect `Output`.

---

## Test Strategy for Shelled Code

### How `git.rs` handles the binary dependency

`crates/tome/src/git.rs` has tests that **assume `git` is installed** with no skip mechanism:

```rust
#[test]
fn git_is_available_on_dev_machine() {
    // This test verifies git is present; CI also has git
    assert!(is_git_available());
}

#[test]
fn read_head_sha_returns_40_char_hex() {
    let tmp = TempDir::new().unwrap();
    // ... runs `git init`, `git config`, `git add`, `git commit` directly via Command::new("git")
    let sha = read_head_sha(dir).unwrap();
    assert_eq!(sha.len(), 40);
}
```

CI guarantees `git` is on PATH (GitHub Actions ubuntu-latest + macos-latest both include git). No `#[ignore]`, no conditional `#[cfg]`, no skip helpers in the codebase (`rg -n "ignore.*macos|ignore.*linux|cfg\(test\)|#\[ignore\]" crates/tome/src/install.rs crates/tome/src/git.rs` returns only `#[cfg(test)]` mod declarations).

### Recommendation for `ClaudeMarketplaceAdapter` tests

**CI does NOT have `claude` installed.** Three viable strategies:

1. **All real-claude tests behind `#[ignore]`** — opt-in via `cargo test -- --ignored`. Run locally during dev; CI doesn't exercise. **Downside:** silent regressions in CI.

2. **Mock-only unit tests** — every adapter test goes through `MockMarketplaceAdapter` (D-10). Real `ClaudeMarketplaceAdapter` only tested via parser unit tests (give it raw JSON, assert parsed shape). **Downside:** subprocess invocation flow itself untested.

3. **Hybrid** (recommended): 
   - Trait shape + drift detection logic tested via `MockMarketplaceAdapter` (always run in CI).
   - Pure parser tests (`parse_claude_plugin_list_json(input: &str) -> Result<Vec<InstalledPlugin>>`) test the JSON deserialization with hand-rolled fixtures (always run in CI). Use the verified shape from §claude CLI JSON Shape.
   - One smoke test that runs `claude --version` and skips gracefully if missing — pattern:
     ```rust
     #[test]
     fn smoke_claude_available() {
         if !is_claude_available() {
             eprintln!("SKIP: claude CLI not on PATH");
             return;
         }
         // ... real probe
     }
     ```
   This isn't a true skip (test passes), but stderr captures the skip note. Matches the pragmatic convention — `git.rs` doesn't even bother with this because git IS on CI.

**Decision:** the planner picks; recommendation #3 balances coverage against CI portability. The `MockMarketplaceAdapter` (D-10) is the primary test surface; the real adapter gets parser-level tests + an opt-in smoke test.

### `MockMarketplaceAdapter` shape recommendation

Per D-10, lives `#[cfg(test)] mod tests` in `marketplace.rs`. Per CONTEXT.md "Claude's Discretion", the planner picks between failure injection vs static fixtures. Recommendation: **static fixtures with optional failure injection via constructor knobs**. Example shape:

```rust
#[cfg(test)]
struct MockMarketplaceAdapter {
    id: String,
    installed: Vec<InstalledPlugin>,
    fail_install: HashSet<String>,    // plugin_ids that should fail install
    fail_update: HashSet<String>,
}
```

Both modes are needed: (a) "happy path: list shows X" tests use static fixtures; (b) "partial-failure aggregation" tests need failure injection to exercise `Vec<InstallFailure>` accumulation per ADP-04.

---

## File Layout Recommendation

### Single file: `crates/tome/src/marketplace.rs`

**Evidence supporting single file:**

| File | LOC | Structure | Notes |
|------|-----|-----------|-------|
| `crates/tome/src/git.rs` | 290 | Single file | Pure subprocess + helpers + tests |
| `crates/tome/src/remove.rs` | 728 | Single file | Plan/render/execute + FailureKind + RemoveFailure + 9 tests |
| `crates/tome/src/install.rs` | 312 | Single file | Subprocess + struct + reconcile + tests |
| `crates/tome/src/migration_v010.rs` | 724 | Single file | Detection + plan + execute + multiple test scaffolds |

**Subdirectory modules in the codebase:** only `browse/` (TUI with 5 sub-modules: app, fuzzy, markdown, mod, theme, ui). Subdirectory is reserved for genuinely multi-concern modules.

**Phase 12 estimated content:**
- Trait definition (~20 LOC)
- `InstalledPlugin` + `InstallFailure` + `InstallFailureKind` + `InstallOp` types (~80 LOC including ALL array, exhaustiveness sentinel, label)
- `ClaudeMarketplaceAdapter` (~150–200 LOC: cache + parser + subprocess invocations + heuristic kind mapper)
- `GitAdapter` (~80–120 LOC: thin wrapper over git.rs)
- Failure renderer (`render_install_failures()` ~30 LOC)
- `MockMarketplaceAdapter` (~50 LOC `#[cfg(test)]`)
- Tests (~200–400 LOC)

**Estimate: 600–1000 LOC.** Within range of `remove.rs` and `migration_v010.rs`. **Single `marketplace.rs` file is the right shape.** A subdirectory is unwarranted unless a future phase grows the module substantially.

### Module declaration in `lib.rs`

Add (matching the alphabetical sibling ordering visible at `crates/tome/src/lib.rs:29-51`):

```rust
pub(crate) mod marketplace;
```

Insertion point: between `library` (line 37) and `lint` (line 38). Per CONTEXT.md `<code_context>`: "**No `sync()` call-site changes in Phase 12.** Phase 13 wires the dispatch."

The trait + types should likely be `pub` (not `pub(crate)`) inside `marketplace.rs` since Phase 13/14 will consume from `lib.rs::sync` and Phase 12 lays the foundation. But the **module itself** stays `pub(crate)` until Phase 13 needs to expose anything outward.

---

## Open Questions

CONTEXT.md is unusually thorough. The remaining true-unknowns the planner must resolve in plans:

1. **Cache type — `RefCell` vs `OnceCell` vs `Mutex`** (CONTEXT.md Discretion).  
   Send/Sync impact: if Phase 13's sync flow ever needs to share the adapter across threads, `RefCell` and `Cell` won't work. Today's `lib.rs::sync` is single-threaded synchronous (verified by reading `lib.rs:912 fn sync(...)`). Recommendation: **`RefCell<Option<Vec<InstalledPlugin>>>`** is sufficient for v0.10. If a future phase parallelizes, swap to `Mutex` then. Phase 12 carries no concurrency requirement.

2. **`InstallFailureKind::ALL` array type — `[T; N]` vs `&'static [T]`** (CONTEXT.md D-06 wrote `&'static [InstallFailureKind]`; remove.rs uses `[FailureKind; N]`).  
   Recommendation: **mirror remove.rs (`[InstallFailureKind; N]`)** for codebase consistency. The `assert!(ALL.len() == N)` exhaustiveness check works for both shapes; fixed-size array is one fewer indirection.

3. **`InstallFailureKind` variant count and exact spelling.**  
   CONTEXT.md D-06 lists: `NotFound`, `NetworkError`, `PermissionDenied`, `Unknown` (4 variants). Recommendation: **ship exactly those 4**. Refine via experience post-v0.10.

4. **Heuristic stderr → kind mapping.**  
   Empirical stderr shapes verified:
   - `Plugin "X" not found in marketplace "Y"` → `NotFound` (verified literal substring `not found in marketplace`)
   - `Plugin "X" not found` (update path) → `NotFound` (substring `not found`)
   - Network/auth errors not yet observed empirically. Use defensive substring checks; fall back to `Unknown`. Add cases as they surface.

5. **`adapter_id` value for `GitAdapter::id()`.**  
   CONTEXT.md D-05 says "the git URL string". Confirm: this is the `dir_config.path.to_string_lossy()` URL exactly as it appears in `tome.toml`, NOT the `repo_cache_dir(...)` path. Planner should pin this in test assertions.

6. **`tests/cli.rs` regression contract for D-05a.**  
   The phrasing "byte-for-byte" is important — CI must show NO diffs in any existing git-source test output. Planner should run `cargo test -p tome --test cli` before/after and diff stdout/stderr; surface ANY change, even cosmetic. Likely safe because `GitAdapter` is a pass-through, but worth an explicit verification step in the plan.

7. **`InstallFailure` derive set — should it be `Debug` only?**  
   `RemoveFailure` derives only `Debug` because `std::io::Error` isn't `Clone`. `InstallFailure` carries `anyhow::Error` (also not `Clone`, not `PartialEq`). Recommendation: **`#[derive(Debug)]` only.** No `Clone`, no `PartialEq`. Tests assert on individual fields rather than struct equality.

8. **Renderer location — `marketplace.rs` vs `lib.rs`** (CONTEXT.md D-07 Discretion).  
   Recommendation: **`marketplace.rs`** (named e.g. `render_install_failures(&[InstallFailure])`). Keeps the rendering close to the type definitions; `lib.rs::sync` will simply call it. The `Command::Remove` arm renders inline in `lib.rs:444-468` because it's hand-coded against `RemoveFailure` — for marketplace, the helper-in-marketplace pattern matches Phase 13's needs better (Phase 13 will call it, doesn't need to know rendering details).

## Sources

### Primary (HIGH confidence — codebase + verified probes)
- `/Users/martin/dev/opensource/tome/crates/tome/src/remove.rs:62-148` — `FailureKind`, `ALL`, exhaustiveness sentinel, `RemoveFailure`
- `/Users/martin/dev/opensource/tome/crates/tome/src/remove.rs:640-693` — failure-kind test scaffolding
- `/Users/martin/dev/opensource/tome/crates/tome/src/git.rs:48-164` — all 5 helpers GitAdapter wraps + `is_git_available`
- `/Users/martin/dev/opensource/tome/crates/tome/src/lib.rs:444-468` — SAFE-01 grouped failure renderer (template)
- `/Users/martin/dev/opensource/tome/crates/tome/src/lib.rs:29-51` — module declaration order/visibility
- `/Users/martin/dev/opensource/tome/crates/tome/src/config.rs:90-271` — `DirectoryType`, `DirectoryConfig`, `GitRef`
- `/Users/martin/dev/opensource/tome/crates/tome/src/install.rs:51-74` — prior-art `claude` subprocess invocation + `ErrorKind::NotFound` handling
- `/Users/martin/dev/opensource/tome/crates/tome/src/manifest.rs:100-163` — confirms `InstalledPlugin` is a NEW type, distinct from `SkillEntry`
- `/Users/martin/dev/opensource/tome/Cargo.toml:13-49` — workspace deps; confirms `which` not present
- claude 2.1.128 live probes (2026-05-05): `claude plugin list --json`, `claude plugin install nonexistent@nonexistent`, `claude plugin update nonexistent[@nonexistent]`, `claude --version`, `claude plugin install --help`, `claude plugin update --help`
- `.planning/phases/12-marketplace-adapter/12-CONTEXT.md` — locked decisions D-01..D-11 + empirical findings

### Secondary (MEDIUM confidence — design baseline)
- `.planning/research/v0.10-library-canonical-design.md` — adapter trait rationale
- `.planning/REQUIREMENTS.md::<ADP>` — ADP-01..04 verbatim
- `.planning/PROJECT.md` D-LIB-03 — adapter rationale at milestone level
- `.planning/phases/11-library-canonical-core/11-CONTEXT.md` D-08 — drift basis = content_hash; version display-only

### Tertiary (negative confirmations)
- `rg -n "InstalledPlugin" crates/tome/src/ crates/tome/tests/` → empty
- `rg -n "InstallFailure|MarketplaceAdapter" crates/tome/src/ crates/tome/tests/` → empty
- `rg -n "OnceCell|RefCell|Cell<|Mutex<" crates/tome/src/` → only `Mutex` in test code (`config.rs:2343`); no production interior-mutability pattern to mirror

## Metadata

**Confidence breakdown:**
- Pattern verification (FailureKind/RemoveFailure): HIGH — citations are exact
- git.rs API surface: HIGH — all 5 helpers verified `pub(crate)`, no widening needed
- config.rs types: HIGH — DirectoryType is the simple 3-variant enum CONTEXT.md described
- Failure renderer: HIGH — single canonical site; not generic; confirms D-07 separate marketplace renderer
- claude CLI JSON shape: HIGH — verified live with 37-entry snapshot + failure path probes
- Binary detection: HIGH — `which` not in deps; `Command::new("claude").arg("--version")` verified to exit 0
- Subprocess pattern: HIGH — `git.rs` is the canonical reference; `install.rs` shows the `ErrorKind::NotFound` map
- Test strategy: MEDIUM — no existing pattern for "binary may be absent in CI"; recommendation is novel but principled
- File layout: HIGH — codebase strongly favors single-file modules; `browse/` is the only exception

**Research date:** 2026-05-05
**Valid until:** ~2026-06-05 (claude CLI minor versions land frequently; re-verify JSON shape if 3+ months elapse before implementation)

## RESEARCH COMPLETE

All research targets from the additional_context (1–10) have been answered with file:line citations and verified probes. Empirical claims about `claude` CLI verified against version 2.1.128 (2026-05-05). All locked decisions (D-01..D-11) cross-referenced against the actual code patterns being mirrored.

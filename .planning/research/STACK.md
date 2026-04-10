# Technology Stack

**Project:** tome v0.6 -- Unified Directory Model (Git Sources, Config Refactoring)
**Researched:** 2026-04-10

## Recommended Stack

### Git Operations (clone, pull, checkout)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `std::process::Command` (git CLI) | N/A (system git) | Clone repos, pull updates, checkout refs | Simplest, zero new deps, tome already requires git for backup. See rationale below. |

**Rationale -- shell out to `git` instead of git2 or gix:**

1. **tome's needs are narrow.** The git operations are: `git clone <url> <path>`, `git -C <path> pull --ff-only`, `git -C <path> checkout <ref>`. No merge, no commit, no push. This is ~30 lines of wrapper code around `Command::new("git")`.

2. **git2 (v0.20.4) adds libgit2 as a C dependency.** This complicates cross-compilation, increases binary size, and adds build complexity (cmake or pkg-config). For three shell commands, it is not worth it.

3. **gix (v0.81.0) is pure Rust but heavyweight.** Clone/fetch are functional (`gix::prepare_clone()`), but pull is not implemented. The crate pulls in ~60 sub-crates and significantly increases compile time. The API is still evolving -- Cargo itself is still migrating from git2 to gix incrementally ([tracking issue](https://github.com/rust-lang/cargo/issues/11813)). The gix maintainer has previously recommended git2 over gix for clone/push workflows.

4. **git is already a runtime dependency.** tome v0.5 uses git for backup (`git init`, `git add`, `git commit`, `git push`). Users already have git installed. Adding a library to avoid calling a binary they already need is over-engineering.

5. **Error handling is straightforward.** `Command` output gives exit code + stderr. Wrap in a `GitError` variant with the stderr message. Done.

**Confidence: HIGH** -- This is a well-established pattern. Cargo itself shells out to git for some operations. Multiple Rust CLI tools (rustup, cargo-edit) use this approach for simple git interactions.

### Config Schema Evolution (TOML with serde)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `serde` + `toml` (existing) | serde 1, toml 1 | Parse unified `[directories.*]` config | Already in the project; no new deps needed. |

**Rationale -- no schema migration library needed:**

The PROJECT.md explicitly states: "Backward compat: None. Old `tome.toml` files will fail to parse. Migration is documented, not automated." This eliminates the need for versioned schema migration.

The approach:

1. **Hard break.** Define the new `Config` struct with `directories: BTreeMap<String, DirectoryConfig>`. Remove `sources` and `targets` fields entirely. Old configs fail at deserialization with a clear serde error.

2. **Use serde's `#[serde(deny_unknown_fields)]`** on the top-level Config to catch old field names and produce actionable error messages. Alternatively, implement a custom deserializer that detects `[[sources]]` or `[targets.*]` and returns a helpful "config format changed, see migration docs" error.

3. **Use `#[serde(default)]` for optional fields** like `ref` (git branch/tag), `role`, etc. This allows progressive config complexity -- simple directories need only `path`, while git sources add `url` and optionally `ref`.

4. **Enum-based role/type discrimination via `#[serde(tag = "type")]`** or a simpler approach: separate fields (`role = "synced"`, `type = "git"`) with serde enums. Given the small number of variants, flat fields with `#[serde(rename_all = "kebab-case")]` are cleaner than tagged enums.

**Example target config shape:**

```toml
[directories.my-skills]
path = "~/code/my-skills"
role = "synced"

[directories.community-skills]
url = "https://github.com/example/skills.git"
path = "~/.tome/repos/community-skills"
role = "source"
type = "git"
ref = "main"
```

**Libraries considered and rejected:**

| Library | Why Not |
|---------|---------|
| `serde-evolve` (crates.io) | Designed for wire-format versioning with migration chains. Overkill for a hard-break single-user config. |
| `version-migrate-macro` (crates.io) | Same -- migration framework for backward compat we explicitly don't need. |
| `config` crate | Layered config merging. Adds complexity for no benefit; tome's config is a single TOML file. |

**Confidence: HIGH** -- serde + toml is the standard Rust pattern. The hard-break decision eliminates the hardest part of schema evolution.

### URL Hashing for Cache Paths

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `sha2` (existing) | 0.11 | Hash git URLs to deterministic cache directory names | Already in the project for content hashing. Reuse for URL hashing. |

**Rationale -- use what you have:**

tome needs to map `https://github.com/example/skills.git` to a stable directory name under `~/.tome/repos/`. The pattern is:

```
~/.tome/repos/<name>-<hash>/
     where hash = sha256(url)[..16]  (first 16 hex chars)
     and name = last path segment without .git
```

This follows Cargo's convention: `<name>-<short-hash>`. The name provides human readability; the hash prevents collisions between repos with the same name from different hosts.

`sha2` is already a dependency. No new crate needed. The implementation is ~5 lines:

```rust
use sha2::{Sha256, Digest};

fn repo_dir_name(url: &str) -> String {
    let name = url.rsplit('/').next().unwrap_or("repo").trim_end_matches(".git");
    let hash = Sha256::digest(url.as_bytes());
    format!("{}-{}", name, hex::encode(&hash[..8]))  // 16 hex chars from 8 bytes
}
```

**Note:** The `hex` encoding can use `format!("{:x}", ...)` on the hash bytes directly, avoiding a `hex` crate dependency. Or use the existing pattern from `manifest.rs` which already does hex encoding of SHA-256 hashes.

**Confidence: HIGH** -- This is a trivial application of an existing dependency.

## Full Stack Summary (Existing + New)

### No New Dependencies Required

The entire v0.6 scope (git sources, unified config, URL-based cache paths) requires **zero new crate dependencies**. Everything is covered by existing deps + `std::process::Command`.

### Existing Dependencies (Unchanged)

| Category | Crates | Role in v0.6 |
|----------|--------|--------------|
| Config | `serde` 1, `toml` 1 | New unified `DirectoryConfig` struct |
| Hashing | `sha2` 0.11 | URL hashing for repo cache paths |
| CLI | `clap` 4 | No changes needed |
| Interactive | `dialoguer` 0.12 | Wizard rewrite for unified directories |
| Filesystem | `walkdir` 2, `dirs` 6 | Unchanged |
| Error handling | `anyhow` 1 | Unchanged |
| Testing | `assert_cmd` 2, `tempfile` 3, `insta` 1 | Integration tests for git source sync |

### New Internal Modules (No External Deps)

| Module | Purpose | Uses |
|--------|---------|------|
| `git.rs` (new) | Thin wrapper around `git` CLI commands | `std::process::Command`, `anyhow` |
| `directory.rs` (new or refactored) | Unified directory config and role logic | `serde`, `toml` |
| `repo_cache.rs` (new) | URL-to-path mapping for `~/.tome/repos/` | `sha2` |

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Git operations | `std::process::Command` | `git2` 0.20 | C dependency (libgit2), build complexity, overkill for 3 commands |
| Git operations | `std::process::Command` | `gix` 0.81 | 60+ sub-crates, no pull support, API still evolving, massive compile time increase |
| Schema migration | Hard break (serde only) | `serde-evolve` | No backward compat needed; single user |
| URL hashing | `sha2` (existing) | `url-hash` crate | Adds a dependency for 5 lines of code |
| URL hashing | `sha2` (existing) | `blake3` | Faster but adds a dep; SHA-256 is already in the project and speed is irrelevant for hashing a URL string |

## Installation

```bash
# No new dependencies to install.
# Existing Cargo.toml covers everything needed for v0.6.
```

## Testing Considerations

- **Git operations:** Integration tests should use `git init --bare` to create local test repos, avoiding network calls in CI. The `tempfile` crate (already a dev dependency) handles temporary repo directories.
- **Config parsing:** Unit tests with TOML string literals. Snapshot tests (insta) for error messages when old config format is detected.
- **URL hashing:** Pure function, trivial unit tests. Test stability (same URL always produces same hash) and collision resistance (different URLs produce different hashes).

## Sources

- [gix crate docs (v0.81.0)](https://docs.rs/gix/latest/gix/)
- [gix `prepare_clone` function](https://docs.rs/gix/latest/gix/fn.prepare_clone.html)
- [gitoxide crate-status.md](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md) -- clone/fetch complete, pull not implemented
- [Cargo `-Zgitoxide` tracking issue](https://github.com/rust-lang/cargo/issues/11813) -- Cargo still migrating from git2 to gix
- [gitoxide discussion #1381](https://github.com/GitoxideLabs/gitoxide/discussions/1381) -- maintainer recommends git2 over gix for write workflows
- [git2 crate (v0.20.4)](https://crates.io/crates/git2)
- [git2-rs GitHub](https://github.com/rust-lang/git2-rs)
- [Cargo git source cache naming](https://users.rust-lang.org/t/origin-of-hash-in-folder-name-for-cargo-git-dependencies/110930)
- [serde-evolve crate](https://crates.io/crates/serde-evolve)
- [version-migrate-macro crate](https://crates.io/crates/version-migrate-macro)
- [Rust Cookbook: External Commands](https://rust-lang-nursery.github.io/rust-cookbook/os/external.html)

---

*Stack analysis: 2026-04-10*

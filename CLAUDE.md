# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Current State

**v0.3** — Connector Architecture shipped (`BTreeMap<String, TargetConfig>` targets, `KnownTarget` registry, npm skill source research). Next up: **v0.4 Format Transforms** (pluggable transform pipeline, Copilot/Cursor/Windsurf format support).

## Quick Reference

| Document | Purpose |
|----------|---------|
| `ROADMAP.md` | Version-by-version feature roadmap (v0.1.x → v0.7) |
| `CHANGELOG.md` | Release history and what changed per version |
| `docs/src/architecture.md` | Detailed sync pipeline and module breakdown |
| `docs/src/test-setup.md` | Test architecture, module coverage, CI pipeline |
| `docs/src/configuration.md` | TOML config format and examples |
| `docs/src/commands.md` | CLI command reference |
| `docs/src/tool-landscape.md` | Research: AI tool config layers across 7+ tools |
| `docs/src/frontmatter-compatibility.md` | SKILL.md frontmatter spec across platforms |
| `docs/src/agent-skills-invocation-syntax-research.md` | Research: skill invocation syntax across tools |

## Project & Task Workflow

- Tasks and roadmap tracked via **GitHub Issues** with milestones (v0.2, v0.3, v0.4, etc.)
- Project board: **"tome Execution Board"** on GitHub Projects
- Labels: `bug`, `enhancement`, `architecture`, `testing`, `documentation`, `dependencies`
- Workflow: check open issues → create feature branch linked to issue → draft PR → CI must pass → merge

## Tech Stack

Rust edition 2024. Key crates: `clap` (CLI), `rmcp` (MCP server), `dialoguer` (interactive prompts), `indicatif` (progress bars), `tabled` (table output), `walkdir` (dir traversal), `sha2` (hashing), `serde`/`toml` (config). Test crates: `assert_cmd`, `tempfile`, `assert_fs`.

## Build & Development Commands

```bash
make build          # cargo build
make test           # cargo test (unit + integration)
make lint           # cargo clippy -- -D warnings
make fmt            # cargo fmt
make fmt-check      # cargo fmt -- --check
make ci             # fmt-check + lint + test (matches CI pipeline)
make install        # install both binaries via cargo install
make build-release  # cargo build --release (LTO + strip)
make release VERSION=0.1.4  # bump version, PR, merge, tag, push (triggers CI release)
```

Run a single test:
```bash
cargo test test_name                          # by test function name
cargo test -p tome -- discover::tests        # module-scoped in a crate
cargo test -p tome --test cli                # integration tests only
```

## Architecture

> For the full deep-dive, see `docs/src/architecture.md`.

Rust workspace (edition 2024) with two crates producing two binaries:

### `crates/tome` — CLI (`tome`)
The main binary. All domain logic lives here as a library (`lib.rs` re-exports all modules) with a thin `main.rs` that parses CLI args and calls `tome::run()`.

**Sync pipeline** (`lib.rs::sync`) — the core flow that `tome sync` and `tome init` both invoke:
1. **Discover** (`discover.rs`) — Scan configured sources for `*/SKILL.md` dirs. Two source types: `ClaudePlugins` (reads `installed_plugins.json`) and `Directory` (flat walkdir scan). First source wins on name conflicts; exclusion list applied.
2. **Consolidate** (`library.rs`) — Copy each discovered skill directory into `~/.local/share/tome/skills/{name}`. A manifest (`.tome-manifest.json`) tracks SHA-256 content hashes for idempotent updates: unchanged skills are skipped, changed skills are re-copied.
3. **Distribute** (`distribute.rs`) — Push library skills to target tools. Two methods: `Symlink` (creates links in target's skills dir pointing to library copies) and `Mcp` (writes a `tome` entry into the target's `.mcp.json`).
4. **Cleanup** (`cleanup.rs`) — Remove stale entries from library (skills no longer in any source) and broken symlinks from targets. Interactive in TTY mode; auto-removes with warning otherwise.

**Other modules:**
- `wizard.rs` — Interactive `tome init` setup using `dialoguer` (MultiSelect, Input, Confirm, Select). Auto-discovers known source locations (`~/.claude/plugins/cache`, `~/.claude/skills`, `~/.codex/skills`, `~/.gemini/antigravity/skills`).
- `config.rs` — TOML config at `~/.config/tome/config.toml`. `Config::load_or_default` handles missing files gracefully. All path fields support `~` expansion.
- `manifest.rs` — Library manifest (`.tome-manifest.json`): tracks provenance, content hashes, and sync timestamps for each skill. Provides `hash_directory()` for deterministic SHA-256 of directory contents.
- `doctor.rs` — Diagnoses library issues (orphan directories, missing manifest entries, broken legacy symlinks) and missing source paths; optionally repairs.
- `status.rs` — Read-only summary of library, sources, targets, and health.
- `mcp.rs` — MCP server implementation using `rmcp`. Exposes `list_skills` and `read_skill` tools over stdio.

### `crates/tome-mcp` — Standalone MCP binary (`tome-mcp`)
Thin wrapper: loads config, calls `tome::mcp::serve()`. Exists so MCP-only consumers don't need the full CLI. The same server is also reachable via `tome serve`.

## Key Patterns

- **Two-tier model**: Sources →(copy)→ Library →(symlink)→ Targets. The library is the source of truth, containing real copies of each skill directory. Distribution to targets uses Unix symlinks (`std::os::unix::fs::symlink`) pointing into the library. This means the project is Unix-only.
- **Targets are data-driven**: `config::targets` is a `BTreeMap<String, TargetConfig>` — any tool can be added as a target without code changes. The wizard uses a `KnownTarget` registry for auto-discovery of common tools. Future: connector trait (#192) for unified source/target abstraction.
- **`dry_run` threading**: Most operations accept a `dry_run: bool` that skips filesystem writes but still counts what *would* change. Results report the same counts either way.
- **Error handling**: `anyhow` for the application. Missing sources/paths produce warnings (stderr) rather than hard errors.

## Testing

> For test architecture details, see `docs/src/test-setup.md`.

Unit tests are co-located with each module (`#[cfg(test)] mod tests`). Integration tests in `crates/tome/tests/cli.rs` exercise the binary via `assert_cmd`. Tests use `tempfile::TempDir` for filesystem isolation — no cleanup needed.

## CI

GitHub Actions runs on both `ubuntu-latest` and `macos-latest`: fmt check, clippy with `-D warnings`, tests, and release build.

## Releases

Releases are managed by [cargo-dist](https://opensource.axo.dev/cargo-dist/). The release workflow (`release.yml`) is **generated and owned by cargo-dist** — don't edit it manually.

**Important:** When bumping `cargo-dist-version` in `Cargo.toml`, always run `cargo dist init` afterwards to regenerate `release.yml`. We use `allow-dirty = ["ci"]` to tolerate Dependabot action bumps, but cargo-dist upgrades may require real workflow changes that won't be applied automatically.

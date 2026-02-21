# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
make build          # cargo build
make test           # cargo test (unit + integration)
make lint           # cargo clippy -- -D warnings
make fmt            # cargo fmt
make fmt-check      # cargo fmt -- --check
make ci             # fmt-check + lint + test (matches CI pipeline)
make install        # install both binaries via cargo install
make release        # cargo build --release (LTO + strip)
```

Run a single test:
```bash
cargo test test_name                          # by test function name
cargo test -p skillet -- discover::tests        # module-scoped in a crate
cargo test -p skillet --test cli                # integration tests only
```

## Architecture

Rust workspace (edition 2024) with two crates producing two binaries:

### `crates/skillet` — CLI (`skillet`)
The main binary. All domain logic lives here as a library (`lib.rs` re-exports all modules) with a thin `main.rs` that parses CLI args and calls `skillet::run()`.

**Sync pipeline** (`lib.rs::sync`) — the core flow that `skillet sync` and `skillet init` both invoke:
1. **Discover** (`discover.rs`) — Scan configured sources for `*/SKILL.md` dirs. Two source types: `ClaudePlugins` (reads `installed_plugins.json`) and `Directory` (flat walkdir scan). First source wins on name conflicts; exclusion list applied.
2. **Consolidate** (`library.rs`) — Symlink each discovered skill into `~/.local/share/skillet/skills/{name}` → original path. Idempotent: unchanged links are skipped, stale links updated.
3. **Distribute** (`distribute.rs`) — Push library skills to target tools. Two methods: `Symlink` (creates links in target's skills dir) and `Mcp` (writes a `skillet` entry into the target's `.mcp.json`).
4. **Cleanup** (`cleanup.rs`) — Remove broken symlinks from library and targets.

**Other modules:**
- `wizard.rs` — Interactive `skillet init` setup using `dialoguer` (MultiSelect, Input, Confirm, Select). Auto-discovers known source locations (`~/.claude/plugins/cache`, `~/.claude/skills`, `~/.codex/skills`, `~/.gemini/antigravity/skills`).
- `config.rs` — TOML config at `~/.config/skillet/config.toml`. `Config::load_or_default` handles missing files gracefully. All path fields support `~` expansion.
- `doctor.rs` — Diagnoses broken symlinks and missing source paths; optionally repairs via cleanup.
- `status.rs` — Read-only summary of library, sources, targets, and health.
- `mcp.rs` — MCP server implementation using `rmcp`. Exposes `list_skills` and `read_skill` tools over stdio.

### `crates/skillet-mcp` — Standalone MCP binary (`skillet-mcp`)
Thin wrapper: loads config, calls `skillet::mcp::serve()`. Exists so MCP-only consumers don't need the full CLI. The same server is also reachable via `skillet serve`.

## Key Patterns

- **Symlinks everywhere**: Library and target distribution both use Unix symlinks (`std::os::unix::fs::symlink`). Originals are never moved or copied. This means the project is Unix-only.
- **Targets struct is hardcoded**: `config::Targets` has named fields (antigravity, codex, openclaw) — not a generic vec. The v0.2 roadmap plans to replace this with a connector trait and `Vec<Target>`.
- **`dry_run` threading**: Most operations accept a `dry_run: bool` that skips filesystem writes but still counts what *would* change. Results report the same counts either way.
- **Error handling**: `anyhow` for the application. Missing sources/paths produce warnings (stderr) rather than hard errors.

## Testing

Unit tests are co-located with each module (`#[cfg(test)] mod tests`). Integration tests in `crates/skillet/tests/cli.rs` exercise the binary via `assert_cmd`. Tests use `tempfile::TempDir` for filesystem isolation — no cleanup needed.

## CI

GitHub Actions runs on both `ubuntu-latest` and `macos-latest`: fmt check, clippy with `-D warnings`, tests, and release build.

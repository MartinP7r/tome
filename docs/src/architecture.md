# Architecture

> **[System Diagram (Excalidraw)](https://excalidraw.com/#json=5-pjpDsna4Way3lfGW5km,p0bQwpcJEl6do68RrnKAgw)** — interactive diagram showing the two-tier source → library → target flow.

Rust workspace (edition 2024) with two crates producing two binaries.

## `crates/tome` — CLI (`tome`)

The main binary. All domain logic lives here as a library (`lib.rs` re-exports all modules) with a thin `main.rs` that parses CLI args and calls `tome::run()`.

### Sync Pipeline

The core flow that `tome sync` and `tome init` both invoke (`lib.rs::sync`):

1. **Discover** (`discover.rs`) — Scan configured sources for `*/SKILL.md` dirs. Two source types: `ClaudePlugins` (reads `installed_plugins.json`) and `Directory` (flat walkdir scan). First source wins on name conflicts; exclusion list applied.
2. **Consolidate** (`library.rs`) — Two strategies based on source type: **managed** skills (ClaudePlugins) are symlinked from library → source dir (package manager owns the files); **local** skills (Directory) are copied into the library (library is the canonical home). A manifest (`.tome-manifest.json`) tracks SHA-256 content hashes for idempotent updates: unchanged skills are skipped, changed skills are re-copied or re-linked. Stale directory state (e.g., a plain directory where a symlink should be) is automatically repaired.
3. **Distribute** (`distribute.rs`) — Push library skills to target tools. Two methods: `Symlink` (creates links in target's skills dir) and `Mcp` (writes a `tome` entry into the target's `.mcp.json`). Skills disabled in machine preferences are skipped.
4. **Cleanup** (`cleanup.rs`) — Remove stale entries from library (skills no longer in any source), broken symlinks from targets, and disabled skill symlinks from target directories. Verifies symlinks point into the library before removing.
5. **Lockfile** (`lockfile.rs`) — Generate `tome.lock` capturing a reproducible snapshot of the library state for diffing in `tome update`.

### Other Modules

- `wizard.rs` — Interactive `tome init` setup using `dialoguer` (MultiSelect, Input, Confirm, Select). Auto-discovers known source locations (`~/.claude/plugins/cache`, `~/.claude/skills`, `~/.codex/skills`, `~/.gemini/antigravity/skills`).
- `config.rs` — TOML config at `~/.config/tome/config.toml`. `Config::load_or_default` handles missing files gracefully. All path fields support `~` expansion.
- `doctor.rs` — Diagnoses library issues (orphan directories, missing manifest entries, broken legacy symlinks) and missing source paths; optionally repairs.
- `status.rs` — Read-only summary of library, sources, targets, and health. Single-pass directory scan for efficiency.
- `mcp.rs` — MCP server implementation using `rmcp`. Exposes `list_skills` and `read_skill` tools over stdio. Filters out disabled skills based on machine preferences.
- `manifest.rs` — Library manifest (`.tome-manifest.json`): tracks provenance, content hashes, and sync timestamps for each skill. Provides `hash_directory()` for deterministic SHA-256 of directory contents.
- `lockfile.rs` — Generates and loads `tome.lock` files. Each entry records skill name, content hash, source, and provenance metadata. Uses atomic temp+rename writes to prevent corruption.
- `machine.rs` — Per-machine preferences (`~/.config/tome/machine.toml`). Tracks a `disabled` set of skill names. Uses atomic temp+rename writes. Loaded during sync and MCP serving to filter skills.
- `update.rs` — Implements `tome update`: loads the previous lockfile, diffs against current state, presents added/changed/removed skills interactively, and offers to disable unwanted new skills.
- `paths.rs` — Symlink path utilities: resolves relative symlink targets to absolute paths and checks whether a symlink points to a given destination.

## `crates/tome-mcp` — Standalone MCP binary (`tome-mcp`)

Thin wrapper: loads config, calls `tome::mcp::serve()`. Exists so MCP-only consumers don't need the full CLI. The same server is also reachable via `tome serve`.

## Key Patterns

- **Two-tier model**: Sources →(consolidate)→ Library →(distribute)→ Targets. The library is the source of truth. Managed skills (from package managers like Claude plugins) are symlinked from library → source dir (the package manager owns the files); local skills (from directory sources) are copied into the library (the library is canonical home). Distribution to targets always uses symlinks pointing into the library. This means the project is Unix-only (`std::os::unix::fs::symlink`).
- **Targets are data-driven**: `config::targets` is a `BTreeMap<String, TargetConfig>` — any tool can be added as a target without code changes. The wizard uses a `KnownTarget` registry for auto-discovery of common tools.
- **`dry_run` threading**: Most operations accept a `dry_run: bool` that skips filesystem writes but still counts what *would* change. Results report the same counts either way.
- **Error handling**: `anyhow` for the application. Missing sources/paths produce warnings (stderr) rather than hard errors.

## Testing

Unit tests are co-located with each module (`#[cfg(test)] mod tests`). Integration tests in `crates/tome/tests/cli.rs` exercise the binary via `assert_cmd`. Tests use `tempfile::TempDir` for filesystem isolation — no cleanup needed.

## CI

GitHub Actions runs on both `ubuntu-latest` and `macos-latest`: fmt check, clippy with `-D warnings`, tests, and release build.

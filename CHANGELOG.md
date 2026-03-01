# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4] - 2026-02-25

### Added
- Progress spinners during sync pipeline stages (discover, consolidate, distribute, cleanup)
- Table-formatted output for `tome list` using `tabled`
- Dry-run banner (`[dry-run] No changes will be made`) when running with `--dry-run`
- Verbose output mode showing per-stage details during sync

### Fixed
- Error handling and silent failure bugs across discover, distribute, and MCP modules
- Symlink escape vulnerability in MCP `read_skill` tool
- Non-object `mcpServers` now returns a clear error instead of panicking

## [0.1.3] - 2026-02-24

### Added
- Graceful handling of pre-init state in `tome status` and `tome doctor`
- `status` shows a helpful "run `tome init`" message when unconfigured
- `doctor` shows init prompt instead of erroring when no config exists

### Changed
- Updated GitHub Actions checkout from v4 to v6
- Dependabot config now ignores cargo-dist-managed workflows

## [0.1.2] - 2026-02-22

### Fixed
- Exclude `tome-mcp` binary from Homebrew installer (only `tome` is needed)
- Updated dependencies

## [0.1.1] - 2026-02-20

### Added
- README badges (crates.io, CI, license)
- Mascot image in README

## [0.1.0] - 2026-02-19

### Added
- Initial release
- **Sync pipeline**: discover → consolidate → distribute → cleanup
- **Discovery**: `ClaudePlugins` (reads `installed_plugins.json` v1 and v2) and `Directory` source types
- **Library**: symlink-based consolidation with idempotent create/update/skip
- **Distribution**: symlink targets (Antigravity) and MCP targets (Codex, OpenClaw)
- **Cleanup**: removes broken symlinks from library and stale links from targets
- **Interactive wizard** (`tome init`): auto-discovers known source locations, configures targets
- **Doctor** (`tome doctor`): diagnoses broken symlinks and missing sources, optional repair
- **Status** (`tome status`): read-only summary of library, sources, targets, and health
- **MCP server** (`tome serve` / `tome-mcp`): exposes `list_skills` and `read_skill` tools over stdio
- **Config**: TOML at `~/.config/tome/config.toml` with tilde expansion
- `--dry-run`, `--quiet`, `--verbose` global flags
- `tome list` / `tome ls` for listing discovered skills
- `tome config --path` for printing config location
- CI on Ubuntu and macOS (fmt, clippy, test, release build)
- cargo-dist release workflow for cross-platform binaries

[Unreleased]: https://github.com/MartinP7r/tome/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/MartinP7r/tome/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/MartinP7r/tome/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/MartinP7r/tome/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/MartinP7r/tome/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/MartinP7r/tome/releases/tag/v0.1.0

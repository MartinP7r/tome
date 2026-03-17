# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `tome version` subcommand and `-V` short flag (#298)
- `typos-cli`, `cargo-machete`, and `cargo-semver-checks` CI checks (#297)
- `TomePaths` struct bundles `tome_home` and `library_dir` to prevent parameter swaps (#287)
- `TargetName` newtype for type-safe target identifiers (#285)
- Log disabled target skips during sync (#284)
- Warn on unknown `disabled_targets` entries in `machine.toml` (#281)
- Validate parent directory in `resolve_tome_home()` (#280)
- Test to verify `tome_home` / `library_dir` separation (#279)

### Changed
- Restructured tome home directory to `~/.tome/` (#271)

### Fixed
- Corrected `tome config` help text from "Show or edit" to "Show" (#296)
- Added missing `.unwrap()` on `TomePaths::new()` in `repair_library` test
- Fix stale path references after `~/.tome/` restructure (#283, #282)
- Suppress noisy `canonicalize` warnings in dry-run mode (#266)

## [0.3.3] - 2026-03-15

### Changed
- Removed MCP server and MCP distribution method (#263)
- Updated milestone naming from v0.4/v0.4.x to v0.4.1/v0.4.2 (#264)

### Fixed
- Pass `--head` to `gh pr create` in release target (#256)

## [0.3.2] - 2026-03-15

### Added
- Vercel Skills comparison research doc (#254)

### Fixed
- Allow Zlib license in cargo-deny config (#251)

## [0.3.1] - 2026-03-14

### Added
- `tome browse` interactive skill browser (#249)
- Audit known targets/sources against current platform docs (#248)

### Fixed
- Suppress noisy `installed_plugins.json` parent dir warning (#247)

## [0.3.0] - 2026-03-13

### Added
- **Per-machine preferences**: `~/.config/tome/machine.toml` with `disabled` list — skills stay in library but are skipped during distribution
- **`tome update` command**: loads lockfile, diffs against current state, presents added/changed/removed skills interactively, offers to disable unwanted new skills
- **`tome.lock` lockfile**: reproducible library snapshots with provenance metadata
- **Connector architecture**: `BTreeMap<String, TargetConfig>` replaces hardcoded Targets struct — any tool can be added as a target without code changes
- **KnownTarget registry**: wizard auto-discovers common tool locations for target configuration
- `--json` flag for `tome list`, structured warning collection, data struct extraction
- **Two-tier consolidation**: managed skills (ClaudePlugins) are symlinked, local skills (Directory) are copied into the library
- **Content hashing**: SHA-256 manifest (`.tome-manifest.json`) for idempotent sync — unchanged skills are skipped
- **`.gitignore` generation** for library directory to support git-friendly skill tracking
- `--machine` global CLI flag to override machine preferences path

### Changed
- `Config::exclude` changed from `Vec<String>` to `BTreeSet<SkillName>` for type safety
- `count_entries` now skips hidden directories

### Fixed
- Atomic lockfile and machine prefs saves (temp+rename) to prevent corruption on crash
- `sync` now cleans up disabled skill symlinks from targets (previously only `update` did this)
- MCP server now filters out disabled skills from machine preferences
- `offer_git_commit` scopes `git add` to tome-managed paths instead of `git add .`
- `cleanup_disabled_from_target` verifies symlinks point into the library before removing
- `count_health_issues` no longer double-counts broken managed symlinks
- Managed skill consolidation repairs stale directory state instead of silently skipping
- Various security and correctness fixes (MCP path validation, doctor repair, config validation)
- Sync lifecycle and `--force` integration test coverage

## [0.2.0] - 2026-02-25

### Added
- **Library copies**: library is the source of truth for local skills — `tome sync` copies from sources into the library instead of symlinking
- **Git init** offered during wizard for library directory
- **Git commit** offered after sync when library is a git repo with changes

### Changed
- Consolidation model: sources → (copy) → library → (symlink) → targets (previously sources → (symlink) → library → (symlink) → targets)

### Fixed
- Skip distribution to targets where skills already originate (prevents circular symlinks)
- MCP `read_skill` path validation (symlink escape vulnerability)
- Doctor repair checks `target.enabled` before operating
- Config validation errors on nonexistent parent directory
- Wizard surfaces discovery errors instead of silently swallowing
- Various hardening (canonicalization, error propagation, `expect()` removal)

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

[Unreleased]: https://github.com/MartinP7r/tome/compare/v0.3.3...HEAD
[0.3.3]: https://github.com/MartinP7r/tome/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/MartinP7r/tome/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/MartinP7r/tome/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/MartinP7r/tome/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MartinP7r/tome/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/MartinP7r/tome/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/MartinP7r/tome/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/MartinP7r/tome/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/MartinP7r/tome/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/MartinP7r/tome/releases/tag/v0.1.0

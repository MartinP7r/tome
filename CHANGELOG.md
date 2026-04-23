# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2026-04-23

The **v0.7 Wizard Hardening** milestone. The Phase 4–6 code shipped to users interim as v0.6.2 (on 2026-04-17); this release is the formal milestone cut and bundles the post-milestone safety patches on top. Users upgrading from v0.6.2 get the safety patches; users on v0.6.1 or earlier get the full wizard hardening surface.

### Changed

- Migrated `tome init` directory summary table to `tabled` with `Style::rounded()` borders and terminal-width-aware truncation via `Width::truncate(..).priority(PriorityMax::right())`. Long paths (including git-repo clones under `~/.tome/repos/<sha>/`) now render cleanly on narrow terminals without breaking column alignment. (WHARD-07)
- Marked WIZ-01 through WIZ-05 as validated / hardened in `PROJECT.md`; removed the stale "Known Gaps (deferred from v0.6)" entry. Phases 4 + 5 closed the correctness gaps (validation, overlap detection, test coverage) and this release reflects that in the project docs. (WHARD-08)
- `Config::validate()` now rejects invalid type/role combinations, library-vs-distribution path overlaps (Cases A/B/C), and circular library paths before save. All four validation bail sites use the Conflict/Why/Suggestion error template. (WHARD-01/02/03)
- `Config::save_checked` enforces expand → validate → TOML round-trip → atomic write. The wizard save path and `--dry-run` branch share this pipeline, so invalid configs can no longer reach disk.
- Removed deprecated internal APIs: `DistributeResult.target_name` alias and `SyncReport.warnings` field (no external impact — `DistributeResult` is not serialized to JSON).

### Added

- `--no-input` flag now supported on `tome init` (previously bailed). Runs the wizard non-interactively using sensible defaults — required for CI smoke tests and headless provisioning.
- Integration tests for `tome init --dry-run --no-input` on empty and seeded `HOME` directories, asserting the generated config validates and round-trips through TOML byte-equal.
- Table-driven `(DirectoryType × DirectoryRole)` matrix test — exhaustive 12-combo coverage verifying `valid_roles()` ↔ `Config::validate()` agreement. (WHARD-04/05/06)
- Regression test for `tome backup restore` bail-on-failure: `restore_bails_when_pre_snapshot_fails` guards against future simplification of the safety-snapshot propagation.

### Fixed

- `tome backup restore` now aborts if the pre-restore safety snapshot fails, instead of silently proceeding with the destructive `git checkout`. The safety snapshot is the user's only recovery path if a restore was accidental. (#415)
- Warn on git cache HEAD-sha read failure instead of silently recording `git_commit_sha: null` in the lockfile (false "no provenance" claim). (#417)
- Warn on SKILL.md read failure post-scan instead of silently dropping frontmatter metadata. `tome browse` no longer hides affected skills' descriptions. (#418)

### Docs

- Enable `mdbook-mermaid` preprocessor on GitHub Pages — all 10 existing mermaid diagrams across `introduction.md`, `test-setup.md`, and `tool-landscape.md` now render correctly (previously shown as raw code blocks). (#450)
- Refresh introduction diagram to reflect the v0.6+ unified directory model with current tool names (Codex, Antigravity, Cursor) and type + role annotations.
- Archive v0.7 milestone planning artifacts to `.planning/milestones/v0.7-ROADMAP.md` and `.planning/milestones/v0.7-REQUIREMENTS.md`.

### Internal

- `/pr-review-toolkit` whole-codebase review produced 36 prioritized findings; 5 shipped in this release (P0 safety fixes + 2 dead-code cleanups). 30 remaining findings filed as issues for v0.8 scoping.

## [0.6.0] - 2026-04-16

The **v0.6 Unified Directory Model** milestone. Phases 1–3 shipped (config type system + git sources + browse TUI polish). Interim patch releases v0.6.1 and v0.6.2 followed without formal CHANGELOG entries; v0.6.2 also carried the v0.7 Wizard Hardening code surface ahead of its formal v0.7.0 release.

### Breaking Changes

The `[[sources]]` and `[targets.*]` config sections have been replaced by a single `[directories.*]` section. tome will refuse to parse old-format config files and show a migration hint.

**Before (v0.5):**
```toml
library_dir = "~/.tome/skills"

[[sources]]
name = "my-skills"
path = "~/skills"
type = "directory"

[[sources]]
name = "plugins"
path = "~/.claude/plugins/cache"
type = "claude-plugins"

[targets.claude]
enabled = true
method = "symlink"
skills_dir = "~/.claude/skills"
```

**After (v0.6):**
```toml
library_dir = "~/.tome/skills"

[directories.my-skills]
path = "~/skills"
type = "directory"
role = "source"

[directories.plugins]
path = "~/.claude/plugins/cache"
type = "claude-plugins"

[directories.claude]
path = "~/.claude/skills"
type = "directory"
role = "target"
```

**Key changes:**
- Each directory has a `type` (directory, claude-plugins, git) and a `role` (source, target, synced, managed)
- `role` defaults based on type: `directory` → source, `claude-plugins` → managed
- `synced` role = both source and target (discovered here AND distributed here)
- `enabled`/`method`/`skills_dir` fields removed — `path` is the only location field
- `disabled_targets` in `machine.toml` renamed to `disabled_directories`

### Added

- Git source type: `type = "git"` directories clone a remote repo into `~/.tome/repos/<sha256>/` with shallow clone, ref pinning (branch / tag / rev), and offline fallback to cached state.
- Per-directory skill selection via `machine.toml`: `[directories.<name>]` blocks accept `enabled = [...]` allowlist OR `disabled = [...]` blocklist (mutually exclusive).
- `tome add <url>` — register a git skill repo from URL.
- `tome remove <name>` — remove a directory from config with full cleanup (symlinks, library entries, manifest, lockfile).
- `tome reassign` / `tome fork` — change a skill's provenance between directories.
- Browse TUI polish: adaptive dark/light theming, fuzzy-match highlighting, scrollbar, markdown preview rendering, and help overlay.

## [0.5.4] - 2026-04-10

### Added
- **Config-based tool root detection** (#390): `shares_tool_root` now derives tool roots from configured source paths instead of hardcoded directory names. No code changes needed when new tools are added.
- **`--json` for status and doctor** (#374): `tome status --json` and `tome doctor --json` produce structured JSON. Uses `CountOrError` type for clean API shape (`{"count": 254}` not `{"Ok": 254}`)
- **Graceful Ctrl-C** (#373): Signal handler prints "interrupted — run `tome sync` to resume" and exits with code 130. Sync pipeline is crash-safe by design.
- **Frontmatter parsed during discovery** (#393): `DiscoveredSkill` now carries parsed `SkillFrontmatter`. Parse errors reported as warnings instead of silently swallowed.
- **XDG config for tome_home** (#369): `~/.config/tome/config.toml` with `tome_home` field as alternative to `TOME_HOME` env var. Resolution: CLI flag → env var → config file → default.

### Fixed
- **Lockfile write failure is now an error** (#394): Previously demoted to warning; now blocks sync with actionable message
- **Safer managed skill skip default**: When path canonicalization fails for managed skills, default to skip (preventing circular symlinks) instead of distribute
- **Tighter tool root matching**: Parent directory check guards against matching paths that only share the home directory

### Closed
- **#370** Merge lockfile/manifest — not planned; separation is by design (internal tracking vs portable snapshot)
- **#391** Extract TOOL_DIRS — superseded by #390

### Deferred
- **#362** Init consolidation — moved to v0.6 (unified directory model)

## [0.5.3] - 2026-04-09

### Added
- **`--no-input` global flag** (#376): Suppresses all interactive prompts (cleanup, triage, install, doctor). Implies `--no-triage` for sync. Errors on `tome init`.
- **Managed skill counts in sync output** (#389): Per-target output shows `skipped (managed)` count for skills not distributed to their own tool
- **Group triage output by source** (#380): Changes grouped under source headers with `+`/`~`/`-` indicators instead of flat list
- **Batch stale cleanup prompt** (#382): Shows all stale skills grouped by previous source, confirms once instead of per-skill
- **Keybinding hints on triage** (#381): "(space to toggle, enter to confirm)" on MultiSelect prompt
- **Subcommand help examples** (#378): Every subcommand `--help` includes usage examples
- **Updated docs** (#368): README command table and `docs/src/commands.md` updated with all commands and new flags (`--no-input`, `--tome-home`)

### Fixed
- **NO_COLOR support verified** (#371): `console` crate already respects `NO_COLOR` env var; added integration test
- **Semantic exit codes verified** (#375): clap returns exit code 2 for invalid arguments; runtime errors return 1; added integration tests
- **`--no-input` threaded through doctor** — `tome doctor --no-input` skips repair prompt
- **Legacy symlink removal warns on failure** instead of silently discarding errors
- **Plugin reconciliation runs with `--no-input`** — users get info message about missing plugins instead of silent skip

## [0.5.2] - 2026-04-05

### Fixed
- **Legacy managed symlink cleanup** during sync: removes stale symlinks from targets on first run after v0.5.1 upgrade

## [0.5.1] - 2026-04-05

### Fixed
- **Default `library_dir` from TOME_HOME** (#383): defaults to `TOME_HOME/skills` instead of hardcoded `~/.tome/skills`
- **Skip managed skills to own tool** (#385): managed plugin skills no longer distributed to their source tool's skills directory, preventing duplicates

## [0.5.0] - 2026-03-28

### Added
- **Auto-install missing managed plugins** during `tome sync` (#347, #355): detects plugins in the lockfile that aren't installed locally, prompts to install via `claude plugin install`
- **Remote sync** integrated into `tome sync` (#349, #353): pull from remote before sync, push after commit. Fast-forward-only merges with actionable error on divergence
- **Remote setup wizard** in `tome backup init`: offers to add a remote URL, verifies connectivity, pushes initial commit
- `--no-triage` flag for `tome sync` to skip interactive prompts (for CI/scripts)
- `tome version` subcommand and `-V` short flag (#298)
- `typos-cli`, `cargo-machete`, and `cargo-semver-checks` CI checks (#297)
- `TomePaths` struct bundles `tome_home` and `library_dir` to prevent parameter swaps (#287)
- `TargetName` newtype for type-safe target identifiers (#285)
- Log disabled target skips during sync (#284)
- Warn on unknown `disabled_targets` entries in `machine.toml` (#281)
- Validate parent directory in `resolve_tome_home()` (#280)
- Test to verify `tome_home` / `library_dir` separation (#279)

### Changed
- **BREAKING: `tome update` removed** (#352): functionality merged into `tome sync`, which now includes lockfile diffing and interactive triage
- **Git repo root moved** from `~/.tome/skills/` to `~/.tome/` (#348, #350): backup repo now tracks skills, `tome.toml`, `tome.lock`, and future config
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

[Unreleased]: https://github.com/MartinP7r/tome/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/MartinP7r/tome/compare/v0.6.2...v0.7.0
[0.6.0]: https://github.com/MartinP7r/tome/compare/v0.5.4...v0.6.0
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

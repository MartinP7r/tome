# Codebase Concerns

**Analysis Date:** 2026-04-05

## Tech Debt

**Frontmatter parsing in discovery deferred:**
- Issue: `tome lint` validates SKILL.md frontmatter, but `tome sync` doesn't parse frontmatter during discovery to enrich discovered skills
- Files: `crates/tome/src/skill.rs` (parse/extract_frontmatter), `crates/tome/src/discover.rs` (discover_source/discover_all)
- Impact: Discovered skills miss metadata (name, description, compatibility) that could be validated earlier or cached. Lint must re-parse files on every run.
- Fix approach: Extend `DiscoveredSkill` struct with optional `frontmatter: SkillFrontmatter` field. Call `skill::parse()` during discovery, handle parse errors gracefully (log warning, continue with empty metadata).
- Roadmap reference: ROADMAP.md v0.4.2 section notes "deferred to follow-up"

**Skill frontmatter not stored in status display:**
- Issue: `tome status` doesn't show parsed metadata (name mismatch, description length warnings) even though `tome lint` can detect them
- Files: `crates/tome/src/status.rs`, `crates/tome/src/skill.rs`
- Impact: Users run `tome lint` to diagnose frontmatter issues, but `tome status` doesn't surface them. Inconsistent visibility of metadata.
- Fix approach: Enhance `SkillRow` struct in status output to include parsed name and truncated description. Call `skill::parse()` once per skill during status gathering.
- Roadmap reference: ROADMAP.md v0.4.2 "Enhance Existing Commands" section

**Per-target skill management design incomplete:**
- Issue: `machine.toml` tracks per-machine disabled skills, but the design for per-target skill selection (e.g., "enable skill A for Claude, disable for Cursor") is tentative
- Files: `crates/tome/src/machine.rs`, `crates/tome/src/config.rs`
- Impact: No way to activate/deactivate a skill for a specific target without editing TOML. Manual file edits are error-prone.
- Fix approach: Decide on central vs. local config: central `tome.toml` with `[targets.<name>.skills]` namespacing (breaks portability), or per-target `~/.claude/tome.toml` that overrides central (requires merge logic). ROADMAP tentatively leans toward "local replaces central" but decision is unfinalized.
- Roadmap reference: ROADMAP.md "Tentative — Per-Target Skill Management" section

**Format transforms pipeline unimplemented:**
- Issue: No support for tool-specific format transforms (e.g., Copilot `.instructions.md`, Cursor `.mdc`, Windsurf rules format)
- Files: N/A (planned feature not yet started)
- Impact: Skills in tome's canonical SKILL.md format must be manually translated for each tool. Can't sync `tome.toml` or `.claude/rules/` to other tools' equivalents.
- Fix approach: Design a pluggable transform pipeline: connectors declare input/output formats, pipeline resolves translation chain. Requires v0.3 connector trait. See `crates/tome/src/distribute.rs` for current symlink-only distribution.
- Roadmap reference: ROADMAP.md "Tentative — Format Transforms" section

**Large file complexity in library.rs:**
- Issue: `crates/tome/src/library.rs` (1346 lines) handles multiple consolidation scenarios: managed→symlink, local→copy, strategy transitions, stale repair
- Files: `crates/tome/src/library.rs`
- Impact: High risk of regressions when modifying symlink/copy logic. Tests cover happy paths but edge cases (e.g., transition from managed to local) are sparse.
- Fix approach: Extract `consolidate_managed()` and `consolidate_local()` into separate modules with dedicated test suites. Add integration tests for all state transitions.

**Large file complexity in distribute.rs:**
- Issue: `crates/tome/src/distribute.rs` (809 lines) implements symlink distribution plus `shares_tool_root()` circular symlink detection
- Files: `crates/tome/src/distribute.rs`, specifically `shares_tool_root()` (line 66) and `find_tool_dir()` (line 74)
- Impact: `find_tool_dir()` hardcodes tool directory names (`.claude`, `.codex`, `.cursor`, etc.) — adding a new tool requires code change. Symlink logic is complex.
- Fix approach: Move known tool directories to a shared registry (e.g., `config::KNOWN_TOOLS`). Use `KnownTarget` registry already built in v0.3 architecture.

## Known Bugs

**Managed skills distributed to same tool as source (#384, fixed in v0.5.1):**
- Symptoms: Claude plugins appear twice in `~/.claude/skills/` — once from plugin system, once from tome symlink
- Files: `crates/tome/src/distribute.rs` (lines 28–58, distribute_to_target function with shares_tool_root check)
- Trigger: Run `tome sync` with a ClaudePlugins source and a target pointing to `~/.claude/skills/`
- Workaround: Disabled by `shares_tool_root()` check in v0.5.1. Safe.
- Fix status: Fixed in commit f5557a3 by comparing source and target tool roots before distributing

**Git identity fallback uses hardcoded "tome@localhost":**
- Symptoms: When git user identity is not configured, `tome backup init` silently sets email to `tome@localhost` instead of using a meaningful identifier
- Files: `crates/tome/src/backup.rs` (lines 50–55, ensure_git_identity function)
- Trigger: Run `tome backup init` on a fresh machine with no global git config
- Workaround: User can manually edit `.git/config` or set global git config
- Fix approach: Use system username + hostname, or prompt user for email. Current approach works but is not user-friendly.

**Lockfile write failures demoted to warning but may cause issues:**
- Symptoms: If lockfile write fails (disk full, permission denied), sync completes successfully but lockfile is stale. User's next `tome sync` may show phantom changes.
- Files: `crates/tome/src/lockfile.rs`, integration in `crates/tome/src/lib.rs`
- Trigger: Fill disk during `tome sync` after library consolidation but before lockfile write
- Impact: User's lockfile state is out of sync with library. `tome update` may fail to detect changes correctly.
- Fix approach: Lockfile write failure should block sync completion (return error) unless `--force` is used. Currently demoted to warning per ROADMAP.md v0.5 changelog #224.

## Security Considerations

**Unicode Tag codepoint detection relies on character scanning:**
- Risk: `tome lint` scans for U+E0001–U+E007F (hidden Unicode Tag codepoints used in prompt injection). Character iteration is correct but no logging of which skills are unsafe.
- Files: `crates/tome/src/lint.rs` (lines 184–192, Unicode check)
- Current mitigation: `tome lint` exits with error code 1 on errors. Users see warning but lint doesn't prevent distribution.
- Recommendations: Add `--strict` flag to exit with error on warnings (not just errors). Log offending skill names during `tome lint`. Consider blocking distribution to targets if lint errors exist (unless `--force`).

**Symlink target validation checks path containment:**
- Risk: `cleanup::cleanup_target()` verifies symlinks point into library using `target.starts_with(library_dir)`. This is correct but doesn't prevent malicious config from pointing cleanup at wrong directories.
- Files: `crates/tome/src/cleanup.rs` (lines 149–150, points_into_library check), `crates/tome/src/distribute.rs` (symlink creation with unix_fs::symlink)
- Current mitigation: All symlinks are created by tome itself; config is local (not fetched from untrusted source). User edits TOML directly.
- Recommendations: Document that config files should not be edited by untrusted scripts. Add config path validation in `config::load()`.

**Relative symlinks created without validation of source path:**
- Risk: `distribute.rs` creates symlinks pointing to source directories. If source path is symlink itself, relative symlink may have confusing target.
- Files: `crates/tome/src/distribute.rs` (lines 100–150, create_symlink function)
- Current mitigation: Symlinks use absolute paths in most cases. Relative symlinks only created when source is in same directory tree as target (rare).
- Recommendations: Always use absolute paths for symlinks to library. Add test coverage for relative symlink edge cases.

**Git commands invoked via Command::new without validation:**
- Risk: `backup.rs` runs git commands with user-controlled arguments (target ref in restore). Input is validated but format is simple string.
- Files: `crates/tome/src/backup.rs` (lines 12–39, git/git_success/git_stdout functions)
- Current mitigation: `restore()` validates target ref locally before passing to git. Error handling catches git failures.
- Recommendations: Use `git_success()` consistently (already done). Avoid string concatenation in args—use array form consistently.

## Performance Bottlenecks

**Directory hashing during consolidation:**
- Problem: `consolidate()` calls `manifest::hash_directory()` for each skill during each sync. Large skills (>1GB) may be slow.
- Files: `crates/tome/src/library.rs` (line 137, hash_directory call), `crates/tome/src/manifest.rs` (hash_directory implementation)
- Cause: SHA-256 is computed naively by walking the directory tree. No incremental hashing.
- Improvement path: Implement incremental hashing that skips files if mtimes haven't changed. Cache mtimes in manifest alongside hashes.

**Manifest and lockfile loaded on every sync:**
- Problem: `consolidate()` loads manifest, `distribute()` loads manifest, `cleanup()` may load again. Multiple disk reads per sync.
- Files: `crates/tome/src/library.rs` (line 99), `crates/tome/src/lib.rs` (sync pipeline)
- Cause: Each operation loads independently. No shared context passed between stages.
- Improvement path: Load manifest once at top of `sync` flow, pass through to consolidate→distribute→cleanup. Library.rs already returns manifest from consolidate, allowing this.

**Fuzzy search in browse re-indexes on every keystroke:**
- Problem: `tome browse` recomputes fuzzy search matches on every character input. For large libraries (1000+ skills), latency may be noticeable.
- Files: `crates/tome/src/browse/fuzzy.rs`, `crates/tome/src/browse/app.rs` (refilter function)
- Cause: No caching of matcher state between keystrokes.
- Improvement path: Cache nucleo matcher state, only recompute on input change. Benchmark impact with real skill libraries (>500 skills).

**WalkDir traversal during discovery is not cached:**
- Problem: `discover_source()` walks directory trees on every sync. Plugin discovery reads `installed_plugins.json` which may list hundreds of plugins.
- Files: `crates/tome/src/discover.rs` (discover_source function, lines ~200+)
- Cause: No caching between discovery phases.
- Improvement path: For ClaudePlugins source, cache parsed JSON and only re-read if mtime changes. For Directory sources, implement incremental discovery (requires sqlite-backed cache).

## Fragile Areas

**Managed skill symlink repair in library.rs:**
- Files: `crates/tome/src/library.rs` (consolidate_managed, lines 140–160)
- Why fragile: When consolidating a managed skill that's already a real directory (from previous local→managed transition or manual intervention), code removes the directory and replaces with symlink. If manifest entry exists and matches the skill, this is correct. But if manifest is out of sync, this could delete user data.
- Safe modification: Always check manifest before removing. Add integration tests for all state transitions: empty→managed, local→managed, managed→local.
- Test coverage: Unit tests cover happy path (DestinationState::Directory → symlink). No tests for manifest mismatch scenarios.

**Symlink cleanup across canonical and relative paths:**
- Files: `crates/tome/src/cleanup.rs` (cleanup_target, lines 121–151)
- Why fragile: On macOS, `/var` is a symlink to `/private/var`. Cleanup compares both canonical and original paths. If library_dir contains symlinks, canonicalization may fail on older systems.
- Safe modification: Test on both macOS and Linux. Verify behavior when library is `/var/foo` (symlink) vs. `/private/var/foo` (canonical).
- Test coverage: Two unit tests verify relative and absolute symlinks, but no macOS-specific /var test.

**Git command output parsing in backup.rs:**
- Files: `crates/tome/src/backup.rs` (list function, lines 131–143, parsing git log output)
- Why fragile: `list()` splits git log output by tabs: `%h\t%ci\t%s`. If commit message contains tabs, parser breaks.
- Safe modification: Use null terminator (`%x00`) or structured format like `--format=tformat:%H%n%ai%n%s` with newline separators.
- Test coverage: No tests for commit messages containing special characters.

**Tilde expansion in config paths:**
- Files: `crates/tome/src/config.rs` (load_or_default function), `crates/tome/src/paths.rs` (collapse_home)
- Why fragile: `collapse_home()` uses `std::env::var("HOME")` which may not be set in all environments (CI, containers). Expansion logic is correct but failures are silent.
- Safe modification: Validate that HOME is set during config load. Warn if tilde expansion fails on write.
- Test coverage: Unit tests for path collapsing exist, but no tests for missing HOME env var.

## Scaling Limits

**Dictionary-based deduplication during discovery:**
- Current capacity: `discover_all()` uses HashMap to track seen skill names. For 10,000 unique skills, memory is negligible (<1MB).
- Limit: No hard limit, but HashMap allocation will dominate for very large libraries (100,000+ skills unlikely but possible with aggregated sources).
- Scaling path: No change needed for reasonable library sizes. If needed, use streaming deduplication with a bloom filter.

**Linear manifest manifest storage (all-in-memory):**
- Current capacity: Manifest is loaded into `BTreeMap<SkillName, SkillEntry>`. Each entry is ~200 bytes. 1000 skills = ~200KB.
- Limit: 100,000 skills would be ~20MB in memory. Not a problem for modern systems but serialization time may be slow.
- Scaling path: For very large libraries, consider incremental lockfile format (append-only log) instead of full rewrite on each sync.

**Browse UI terminal size assumptions:**
- Current capacity: `browse::app` assumes terminal height of ~20 lines. Modern terminals are typically 25–60 lines.
- Limit: If terminal is very small (<10 lines), list paging breaks.
- Scaling path: Query actual terminal size from crossterm, adjust visible_height dynamically. Already partially done via `App::new()` visible_height field.

## Scaling Limits

**Distributed CI matrix for release builds:**
- Current capacity: Cargo-dist generates builds for `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`. 3 targets.
- Limit: No current limit, but release build time is ~5 min per target. Adding Windows or ARM Linux targets would increase matrix time.
- Scaling path: Use `cargo-dist` scheduled batching or parallel jobs if CI time becomes an issue.

## Dependencies at Risk

**serde_yaml older than latest:**
- Risk: `serde_yaml` 0.9 has known issues with large YAML files and Unicode normalization. Version 0.10+ recommended.
- Impact: Frontmatter parsing may fail on edge case YAML in some skills.
- Migration plan: Update to `serde_yaml >= 0.10` in Cargo.toml. Test with real-world SKILL.md files.

**Rust MSRV 1.85.0 may be conservative:**
- Risk: Edition 2024 requires Rust 1.85. Newer features (e.g., precise async drop) are available in 1.86+. No features are currently blocked.
- Impact: None currently. MSRV is pragmatically chosen.
- Migration plan: Monitor for blocking issues. Can bump MSRV in next major version if needed.

**cargo-dist version pinning in Cargo.toml:**
- Risk: `cargo-dist-version = "0.30.3"` is pinned. Newer versions may have breaking changes in workflow generation.
- Impact: If Dependabot bumps version, `cargo dist init` must be re-run to regenerate `release.yml`.
- Migration plan: Document the requirement in CLAUDE.md (already done). Consider CI check that ensures `cargo dist init` is run after version bumps.

## Missing Critical Features

**No validation of incompatible skill combinations:**
- Problem: No way to detect if two skills conflict (e.g., both define the same tool keybinding). Multiple sources can publish conflicting instructions.
- Blocks: Advanced skill composition (v0.7 Wolpertinger feature) cannot proceed without conflict detection.
- Roadmap reference: ROADMAP.md v0.7 "Skill Composition"

**No skill evaluation/validation against agent skills standard:**
- Problem: `tome lint` validates platform compatibility but doesn't check against an emerging agent skills standard.
- Blocks: Community skill publishing cannot have quality gates.
- Roadmap reference: ROADMAP.md v0.7 "Skill evaluation/creation skill"

**No plugin registry or marketplace integration:**
- Problem: `tome sync` can only auto-install plugins from Claude marketplace. No discovery of community plugins or third-party registries.
- Blocks: Skill discovery (v0.7) requires plugin registry.
- Roadmap reference: ROADMAP.md "Plugin registry" under Future Ideas

## Test Coverage Gaps

**Symlink strategy transitions:**
- What's not tested: Changing a skill's source from local to managed (library.rs transitions from copy to symlink) and vice versa
- Files: `crates/tome/src/library.rs`, `crates/tome/tests/cli.rs`
- Risk: Regressions in consolidate_managed/consolidate_local could silently corrupt library state
- Priority: High — affects core sync pipeline

**Circular symlink prevention:**
- What's not tested: `shares_tool_root()` function in distribute.rs with real paths and edge cases (symlinks in tool directory itself)
- Files: `crates/tome/src/distribute.rs`, no dedicated test
- Risk: Regression could allow circular symlinks, breaking target tools
- Priority: High — security-adjacent

**Git backup operations with diverged history:**
- What's not tested: `backup::pull()` behavior when local and remote have diverged commits
- Files: `crates/tome/src/backup.rs` (pull function), `crates/tome/tests/cli.rs`
- Risk: User data loss if merge logic is incorrect
- Priority: High — affects backup integrity

**Manifest corruption recovery:**
- What's not tested: What happens when `.tome-manifest.json` is corrupted (invalid JSON, missing fields)
- Files: `crates/tome/src/manifest.rs` (load function)
- Risk: Sync fails with confusing error. No graceful fallback.
- Priority: Medium — edge case but can block user workflow

**Frontmatter parsing with pathological YAML:**
- What's not tested: Very large frontmatter blocks (>1MB), deeply nested maps, circular references
- Files: `crates/tome/src/skill.rs` (parse function)
- Risk: serde_yaml may panic or consume excessive memory on malformed input
- Priority: Medium — lint must be robust to untrusted SKILL.md files

**Browse UI with large libraries:**
- What's not tested: Browse app with 1000+ skills. Fuzzy search performance, memory usage, rendering lag
- Files: `crates/tome/src/browse/app.rs`, `crates/tome/src/browse/fuzzy.rs`
- Risk: UI becomes unusable for power users with large skill collections
- Priority: Medium — performance issue for advanced users

**Distribution to non-existent target directories:**
- What's not tested: What happens when target skills_dir doesn't exist (e.g., user configures `~/.cursor/skills/` but Cursor is not installed)
- Files: `crates/tome/src/distribute.rs` (distribute_to_target)
- Risk: Silent success (creates directory) or error depending on filesystem. Behavior is not documented.
- Priority: Low — current behavior (create directory) is reasonable

---

*Concerns audit: 2026-04-05*

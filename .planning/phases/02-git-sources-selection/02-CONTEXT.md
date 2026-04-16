# Phase 2: Git Sources & Selection - Context

**Gathered:** 2026-04-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can add remote git repos as skill sources (`type = "git"` in config), control which skills reach which directories on a per-machine basis via `machine.toml`, and remove directory entries from config with full cleanup.

This phase does NOT include: `tome add <url>` convenience command (Phase 3), `tome reassign` (Phase 3), browse polish (Phase 3).

</domain>

<decisions>
## Implementation Decisions

### Git Clone/Update Strategy
- **D-01:** Repo cache directory named by SHA-256 of URL: `~/.tome/repos/<sha256(url)>/`. Deterministic and path-safe.
- **D-02:** Optional `subdir` field on `DirectoryConfig` for git directories. After clone, discovery scans `<clone_path>/<subdir>/` instead of root. Handles monorepos and dotfiles repos where skills aren't at the root level.
- **D-03:** When branch/tag/rev are all omitted, track remote HEAD (whatever the repo's default branch is). No hardcoded default.
- **D-04:** Shallow clone (`--depth 1`) per GIT-02. Update via `git fetch --depth 1 origin <ref> && git reset --hard FETCH_HEAD` per GIT-03. Not `git pull`.
- **D-05:** branch/tag/rev fields are mutually exclusive (already validated in config.rs from Phase 1).

### Per-Directory Skill Selection
- **D-06:** Per-directory skill filtering uses nested TOML tables in machine.toml: `[directory.<name>]` sections with `disabled` and `enabled` keys.
- **D-07:** `disabled` + `enabled` on the same directory = validation error (MACH-04).
- **D-08:** Resolution order follows locality principle (most specific wins): per-directory `enabled` (allowlist, strongest) > per-directory `disabled` (blocklist) > global `disabled` (broad default). **This overrides MACH-05 as originally written** — per-directory `enabled` overrides global `disabled`, matching Claude Code's convention where more specific settings win.

### Failure & Offline Behavior
- **D-09:** Git fetch failure during sync: warn to stderr, continue using last successfully cloned state. Local directories sync normally. If no cached clone exists (first-time failure), skip that directory entirely with a warning.
- **D-10:** Distinct messages for "never cloned" vs "clone exists but fetch failed": first-time = "warning: could not clone 'name' — skipping (no cached state)"; subsequent = "warning: could not update 'name' — using cached state from <date>".
- **D-11:** Git operations clear `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` env vars to prevent interference from calling environment (GIT-06).

### tome remove UX
- **D-12:** `tome remove <name>` does full cleanup: config entry + library skills from that directory + symlinks in distribution directories + cached git repo (if git type) + manifest/lockfile entries.
- **D-13:** Interactive confirmation in TTY (show what will be removed, y/N default No), auto-remove in non-TTY with warning output. Matches existing cleanup behavior.
- **D-14:** Supports `--dry-run` (preview without acting) and `--force` (skip confirmation). Consistent with `tome sync` flag patterns.

### Claude's Discretion
- Git error message wording (as long as it distinguishes clone-vs-update failure)
- Internal module organization for git operations (separate `git.rs` module or inline in discovery)
- Whether `subdir` field appears in the wizard flow or is config-only for now
- Exact format of the `tome remove` preview table

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Design & Architecture
- `docs/v06-implementation-plan.md` — Original v0.6 design. Git source design decisions and type definitions.
- `docs/src/architecture.md` — Sync pipeline (discover -> consolidate -> distribute -> cleanup). Git resolution is a **pre-discovery step** that resolves URLs to local cache paths.

### Requirements
- `.planning/REQUIREMENTS.md` — GIT-01 through GIT-08, MACH-02 through MACH-05, CLI-01. All map to Phase 2. **Note: MACH-05 needs updating** to reflect D-08 (locality wins over global).
- `.planning/ROADMAP.md` — Phase 2 success criteria (5 criteria).

### Prior Phase Context
- `.planning/phases/01-unified-directory-foundation/01-CONTEXT.md` — Phase 1 decisions. D-01 through D-11 from Phase 1 are locked.
- `.planning/phases/01-unified-directory-foundation/01-VERIFICATION.md` — Phase 1 verification (20/20 passed).

### Key Source Files
- `crates/tome/src/config.rs` — `DirectoryType::Git` already exists with `branch`/`tag`/`rev` fields, validation rejects `role = "target"` on git type. **Needs `subdir` field added.**
- `crates/tome/src/discover.rs` — `discover_directory_entry()` already routes `Git` type same as `Directory` (flat scan). Git resolution is a pre-step before this.
- `crates/tome/src/machine.rs` — Has `disabled` (skill-level) and `disabled_directories` (directory-level). **Needs per-directory `[directory.*]` sections with disabled/enabled.**
- `crates/tome/src/lockfile.rs` — Already has `git_commit_sha` field for provenance.
- `crates/tome/src/backup.rs` — Has `git_success()` helper pattern reusable for clone/fetch operations.
- `crates/tome/src/cli.rs` — Needs `Remove` subcommand added.
- `crates/tome/src/lib.rs` — Sync pipeline orchestrator. Git resolution step goes before discover call.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `backup.rs::git_success()` — runs git commands with error handling. Reusable pattern for clone/fetch.
- `config.rs::DirectoryType::Git` — type enum variant already exists with validation.
- `config.rs` branch/tag/rev fields — already on `DirectoryConfig`, validated as mutually exclusive and git-only.
- `lockfile.rs::git_commit_sha` — provenance field already plumbed through discovery → lockfile.
- `discover.rs` — `Git` type already routes to flat directory scan. Only pre-step (clone/fetch) is missing.

### Established Patterns
- Atomic temp+rename writes for config/manifest/lockfile changes
- `dry_run: bool` threading through all operations
- Interactive TTY detection for confirmation prompts (cleanup.rs pattern)
- `anyhow::Context` for error chaining with operation descriptions
- Warning collection via `&mut Vec<String>` in discovery pipeline

### Integration Points
- `lib.rs::sync()` — git resolution step inserts before `discover::discover_all()` call
- `lib.rs::run()` — needs `Command::Remove` dispatch
- `machine.rs::MachinePrefs` — new per-directory filtering checked during distribution in `distribute.rs`
- `config.rs::DirectoryConfig` — needs `subdir: Option<String>` field

</code_context>

<specifics>
## Specific Ideas

- Resolution order for skill filtering follows locality principle (most specific wins), matching Claude Code's settings precedent
- MACH-05 requirement text needs updating to match the decided resolution order
- Git error messages should be actionable: distinguish "never cloned" from "update failed, using cache"
- `tome remove` preview should show a clear breakdown of what will be removed (config, library skills, symlinks, cache)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 02-git-sources-selection*
*Context gathered: 2026-04-15*

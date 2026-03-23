# Agent Instructions

This file provides guidance to Claude Code (claude.ai/code) and other AI agents when working with code in this repository.

## Current State

**v0.3.3 released** — Portable Library milestone shipped: `tome.lock` lockfile, per-machine preferences via `machine.toml`, `tome update` command with lockfile diffing and interactive triage, connector architecture with data-driven targets. Unified home directory restructured to `~/.tome/`. Next up: **v0.4.1 Browse + Validation** (`tome browse`, `tome lint`, frontmatter parsing) and **v0.4.2 Format Transforms** (rules/instructions syncing, pluggable transform pipeline, Copilot/Cursor/Windsurf format support).

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

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i` (interactive) mode on some systems, causing the agent to hang indefinitely waiting for y/n input.

**Use these forms instead:**
```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**Other commands that may prompt:**
- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag
- `brew` - use `HOMEBREW_NO_AUTO_UPDATE=1` env var

## Project & Task Workflow

- Tasks and roadmap tracked via **GitHub Issues** with milestones (v0.4.1, v0.4.2, v0.5, etc.)
- Project board: **"tome Execution Board"** on GitHub Projects
- Labels: `bug`, `enhancement`, `architecture`, `testing`, `documentation`, `dependencies`
- Default workflow for substantial changes: GitHub issue/idea → OpenSpec change → Beads execution tasks → implementation → archive/close
- Reference doc: `docs/src/development-workflow.md`
- Small fixes (typos, tiny bugs, narrowly scoped cleanups) do **not** need full OpenSpec + Beads overhead

## Tech Stack

Rust edition 2024. Key crates: `clap` (CLI), `dialoguer` (interactive prompts), `indicatif` (progress bars), `tabled` (table output), `walkdir` (dir traversal), `sha2` (hashing), `serde`/`toml` (config). Test crates: `assert_cmd`, `tempfile`, `assert_fs`.

## OpenSpec + Traceability

For substantial changes (new features, significant refactors, architecture-impacting work, or process changes), use the repo workflow described in `docs/src/development-workflow.md`.

### OpenSpec

Use OpenSpec to capture the planning layer:
- proposal
- design
- task checklist
- any spec deltas needed for changed behavior

Typical commands:
```bash
openspec new change <change-id>
openspec show <change-id>
openspec status --change <change-id>
openspec validate <change-id>
openspec archive <change-id>
```

### Traceability Convention

For meaningful changes, link the layers when they exist:
- GitHub issue: `#123`
- OpenSpec change: `<change-id>`
- Beads task: `tome-xyz` / `tome-xyz.1`
- Commit / PR: implementation evidence

Suggested commit body or PR footer:
```text
Refs #123
OpenSpec: <change-id>
Beads: <task-id>[, <task-id>...]
```

This repo uses:
- **GitHub Issues** for backlog / roadmap intent
- **OpenSpec** for requirements + design + checklist
- **Beads** for execution state
- **git / PRs** for shipped evidence

## Build & Development Commands

```bash
make build          # cargo build
make test           # cargo test (unit + integration)
make lint           # cargo clippy --all-targets -- -D warnings
make fmt            # cargo fmt
make fmt-check      # cargo fmt -- --check
make ci             # fmt-check + lint + test (matches CI pipeline)
make install        # install binary via cargo install
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

Rust workspace (edition 2024) with a single crate:

### `crates/tome` — CLI (`tome`)
The main binary. All domain logic lives here as a library (`lib.rs` re-exports all modules) with a thin `main.rs` that parses CLI args and calls `tome::run()`.

**Sync pipeline** (`lib.rs::sync`) — the core flow that `tome sync` and `tome init` both invoke:
1. **Discover** (`discover.rs`) — Scan configured sources for `*/SKILL.md` dirs. Two source types: `ClaudePlugins` (reads `installed_plugins.json`) and `Directory` (flat walkdir scan). First source wins on name conflicts; exclusion list applied.
2. **Consolidate** (`library.rs`) — Two strategies based on source type: managed skills (ClaudePlugins) are symlinked from library → source dir (package manager owns the files); local skills (Directory) are copied into the library (library is the canonical home). A manifest (`.tome-manifest.json`) tracks SHA-256 content hashes for idempotent updates: unchanged skills are skipped, changed skills are re-copied or re-linked.
3. **Distribute** (`distribute.rs`) — Push library skills to target tools via symlinks in each target's skills directory. Skills disabled in machine preferences are skipped.
4. **Cleanup** (`cleanup.rs`) — Remove stale entries from library (skills no longer in any source), broken symlinks from targets, and disabled skill symlinks from target directories. Interactive in TTY mode; auto-removes with warning otherwise.

**Other modules:**
- `wizard.rs` — Interactive `tome init` setup using `dialoguer` (MultiSelect, Input, Confirm, Select). Auto-discovers known source locations (see `KNOWN_SOURCES` in `wizard.rs` for the full list).
- `config.rs` — TOML config at `~/.tome/tome.toml`. `Config::load_or_default` handles missing files gracefully. All path fields support `~` expansion.
- `manifest.rs` — Library manifest (`.tome-manifest.json`): tracks provenance, content hashes, and sync timestamps for each skill. Provides `hash_directory()` for deterministic SHA-256 of directory contents.
- `doctor.rs` — Diagnoses library issues (orphan directories, missing manifest entries, broken legacy symlinks) and missing source paths; optionally repairs.
- `status.rs` — Read-only summary of library, sources, targets, and health. Single-pass directory scan for efficiency.
- `lockfile.rs` — Generates and loads `tome.lock` files. Each entry records skill name, content hash, source, and provenance metadata. Uses atomic temp+rename writes.
- `machine.rs` — Per-machine preferences (`~/.config/tome/machine.toml`). Tracks a `disabled` set of skill names and a `disabled_targets` set of target names. Uses atomic temp+rename writes.
- `update.rs` — Implements `tome update`: loads the previous lockfile, diffs against current state, presents changes interactively, and offers to disable unwanted new skills.
- `paths.rs` — Defines `TomePaths` (bundles `tome_home` and `library_dir` to prevent parameter swaps). Also provides symlink path utilities: resolves relative symlink targets and checks symlink destinations.

## Key Patterns

- **Two-tier model**: Sources →(consolidate)→ Library →(distribute)→ Targets. The library is the source of truth. Managed skills (from package managers) are symlinked from library → source dir; local skills are copied into the library. Distribution to targets uses Unix symlinks (`std::os::unix::fs::symlink`) pointing into the library. Unix-only.
- **Targets are data-driven**: `config::targets` is a `BTreeMap<TargetName, TargetConfig>` — any tool can be added as a target without code changes. The wizard uses a `KnownTarget` registry for auto-discovery of common tools.
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

<!-- BEGIN BEADS INTEGRATION v:1 profile:full hash:f65d5d33 -->
## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Dolt-powered version control with native sync
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json
bd create "Issue title" --description="What this issue is about" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update <id> --claim --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task atomically**: `bd update <id> --claim`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details about what was found" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Quality
- Use `--acceptance` and `--design` fields when creating issues
- Use `--validate` to check description completeness

### Lifecycle
- `bd defer <id>` / `bd supersede <id>` for issue management
- `bd stale` / `bd orphans` / `bd lint` for hygiene
- `bd human <id>` to flag for human decisions
- `bd formula list` / `bd mol pour <name>` for structured workflows

### Auto-Sync

bd automatically syncs via Dolt:

- Each write auto-commits to Dolt history
- Use `bd dolt push`/`bd dolt pull` for remote sync
- No manual export/import needed!

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and docs/QUICKSTART.md.

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

<!-- END BEADS INTEGRATION -->

# Roadmap

## v0.1.x — Polish & UX

- **Wizard interaction hints**: Show keybinding hints in MultiSelect prompts (space to toggle, enter to confirm) — `dialoguer` doesn't surface these by default
- **Clarify plugin cache source**: Make it clear that `~/.claude/plugins/cache` refers to *active* plugins installed from the Claude Code marketplace, not arbitrary cached files
- **Wizard visual polish**: Add more color, section dividers, and summary output using `console::style()` — helpful cues without clutter
- **Modern TUI with welcome ASCII art**: Replace plain text output with a polished TUI; open `tome init` with ASCII art of a tome/spellbook as the welcome screen
- ~~**Explain symlink model in wizard**: Clarify that the library uses symlinks (originals are never moved or copied), so users understand there's no data loss risk~~
- **Optional git init for library**: Ask during `tome init` whether to initialize a git repo in the library directory for change tracking across syncs
- **Expand wizard auto-discovery**: ~~Added `~/.gemini/antigravity/skills`.~~ `~/.copilot/skills/` and `~/.cursor/` don't exist as official home-dir paths — Copilot uses per-project `.github/skills/` and Cursor uses per-project `.cursor/rules/`. Per-project sources deferred to v0.2 connector architecture.
- ~~**Fix `installed_plugins.json` v2 parsing**: Current parser expects a flat JSON array (v1); v2 wraps plugins in `{ "version": 2, "plugins": { "name@registry": [...] } }` — discovery silently finds nothing. Support both formats going forward.~~
- ~~**Finalize tool name**: Decided on **tome** — *"Cook once, serve everywhere."*~~
- **Improve doc comments for `cargo doc`**: Add module-level `//!` docs, expand struct/function docs, add `# Examples` to key public APIs.
- **GitHub Pages deployment**: Add CI workflow to build and deploy mdBook + `cargo doc` to GitHub Pages.

## v0.2 — Connector Architecture

The current model hardcodes targets as struct fields and keeps source/target logic separate. Both sides are really the same concept: an **endpoint with a connector type** that knows how to discover, read, write, and translate skills.

- **Generic `[[targets]]` array**: Replace the hardcoded `Targets` struct with a `Vec<Target>` — same shape as sources. Each target has a `name`, `path`, `type`, and connector-specific options
- **Connector trait**: Unified interface for both source and target behavior — discovery format, distribution method (symlink, MCP config, copy), and format translation needs
- **Built-in connectors**: Claude (plugins + standalone), Codex, Antigravity, Cursor, Windsurf, OpenCode, Nanobot, PicoClaw, OpenClaw, VS Code Copilot, Amp, Goose
- **Gemini CLI connector**: Investigate sandbox mode — Gemini CLI may use a different skills/context folder depending on whether it runs in sandbox or not; connector may need to detect or allow configuring which path to target
- **Bidirectional by design**: Any connector can act as both source and target — discover skills *from* Cursor rules and distribute *to* Cursor rules
- **Format awareness per connector**: Each connector declares its native format — the pipeline handles translation between them (e.g., SKILL.md ↔ Cursor rules ↔ Windsurf conventions)
- Support syncing `.claude/rules/` and agent definitions alongside skills
- **Instruction file syncing**: Bidirectional sync of tool instruction files (CLAUDE.md ↔ AGENTS.md ↔ GEMINI.md ↔ copilot-instructions.md) — extract shared sections and distribute to each tool's native format

## v0.3 — Format Transforms

- Pluggable transform pipeline driven by connector format declarations
- Preserve original format — transforms are output-only
- Connectors declare input/output formats; the pipeline resolves the translation chain
- **Copilot `.instructions.md` format**: Support Copilot's `.instructions.md` as a transform target alongside Cursor `.mdc` and Windsurf rules

## v0.3.x — Skill Validation & Linting

Add YAML frontmatter parsing and a `tome lint` command that catches cross-tool compatibility issues. See [Frontmatter Compatibility](docs/src/frontmatter-compatibility.md) for the full spec comparison.

### Frontmatter Parsing

- Add `serde_yaml` dependency
- Create a `Skill` struct with typed fields for the base standard (name, description, license, compatibility, metadata, allowed-tools)
- Parse frontmatter during discovery (enrich `DiscoveredSkill`)
- Store parsed metadata for validation, MCP responses, and status display

### `tome lint` Command

Validation checks ordered by severity:

**Errors** (skill will break on one or more targets):
- Missing required `name` field
- Missing required `description` field
- `name` doesn't match containing directory name
- `name` exceeds 64 chars or uses invalid characters (must be lowercase letters, numbers, hyphens)
- `description` exceeds 1024 chars

**Warnings** (cross-platform compatibility issues):
- Non-standard fields (`version`, `category`, `tags`, `last-updated`) — suggest moving to `metadata`
- Platform-specific fields used (`disable-model-invocation`, `excludeAgent`, etc.) — note which target they're for
- Multiline YAML description without block scalar indicator (`|`) — will break on Claude Code
- `description` exceeds 500 chars (Copilot limit)
- Body exceeds 6000 chars (Windsurf limit)
- **Hidden Unicode Tag codepoint scanning**: Detect U+E0001–U+E007F tag characters that can smuggle invisible instructions (security)

**Info** (best practices):
- `allowed-tools` used (experimental, may not be supported everywhere)
- Body exceeds ~5000 tokens (general recommendation)

### Enhance Existing Commands

- **`tome doctor`**: Add frontmatter health checks alongside existing symlink diagnostics — parse all library skills and report validation results
- **`tome status`**: Show parsed frontmatter summary per skill — name, description (truncated), field count, and any validation issues inline

### Target-Aware Warnings (Future)

Requires the v0.2 connector architecture. When distributing to specific targets, warn about:
- Fields unsupported by that target
- Description length exceeding target's limit
- Body syntax incompatible with target (e.g., XML tags, `!command`, `$ARGUMENTS`)

## v0.4 — Portable Library

Make the skill library reproducible across machines via a lockfile and per-machine preferences.

- **Library as canonical home**: Local skills live directly in the library (real directories, not symlinks). Managed skills (Claude marketplace, future registries) are symlinked in from their package manager locations.
- **`tome.lock`**: Tracked lockfile in the library recording every skill's type (local/managed), source, and install metadata. For managed plugins: `plugin-name@registry` identifier + version (from `installed_plugins.json` v2 key format). Enough info to reproduce the library on a fresh machine.
- **Per-machine preferences** (`~/.config/tome/machine.toml`): Per-machine opt-in/opt-out for managed plugins — machine A installs plugins 1,2,3 while machine B only wants 1 and 3.
- **`tome update` command**: Reads lockfile, diffs against local state, prompts user about new/missing managed plugins, actively runs `claude plugin install <name@registry>` for approved plugins, then syncs.
- **Claude marketplace first**: First managed source targeting the Claude plugin marketplace. Version pinning via version string or git commit SHA.
- **Git-friendly library**: Library directory works as a git repo — local skills tracked in git, managed symlinks recreated by `tome update` (gitignored), lockfile tracked.

## v0.5 — Git Sources

- Add `type = "git"` source for remote skill repositories
- Clone/pull on sync with caching
- Pin to branch, tag, or commit SHA
- Support private repos via SSH keys or token auth

## v0.6 — Watch Mode

- `tome watch` for auto-sync on filesystem changes
- Debounced fsnotify-based watcher
- Optional desktop notification on sync

## Future Ideas

- **Plugin registry**: Browse and install community skill packs
- **Conflict resolution UI**: Interactive merge when skills collide
- **Shell completions**: Generate completions for bash, zsh, fish
- **Homebrew formula**: `brew install tome`
- **Backup snapshots**: Optional tarball backup of library state before destructive operations
- **Token budget estimation**: Show estimated token cost per skill per target tool in `tome status` output
- **Security audit command**: `tome audit` to scan skills for prompt injection vectors, hidden unicode, and suspicious patterns
- **Portable memory extraction**: Suggest MEMORY.md entries that could be promoted to reusable skills (`tome suggest-skills`)
- **Plugin output generation**: Package the skill library as a distributable Claude plugin, Cursor plugin, etc.

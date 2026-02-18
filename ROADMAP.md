# Roadmap

## v0.1.x — Polish & UX

- **Wizard interaction hints**: Show keybinding hints in MultiSelect prompts (space to toggle, enter to confirm) — `dialoguer` doesn't surface these by default
- **Clarify plugin cache source**: Make it clear that `~/.claude/plugins/cache` refers to *active* plugins installed from the Claude Code marketplace, not arbitrary cached files
- **Wizard visual polish**: Add more color, section dividers, and summary output using `console::style()` — helpful cues without clutter
- **Explain symlink model in wizard**: Clarify that the library uses symlinks (originals are never moved or copied), so users understand there's no data loss risk
- **Optional git init for library**: Ask during `skync init` whether to initialize a git repo in the library directory for change tracking across syncs
- **Expand wizard auto-discovery**: Add `~/.copilot/skills/`, `.github/skills/`, `$HOME/.agents/skills/`, `.cursor/`, `.gemini/antigravity/` to the wizard's known source locations
- **Fix `installed_plugins.json` v2 parsing**: Current parser expects a flat JSON array (v1); v2 wraps plugins in `{ "version": 2, "plugins": { "name@registry": [...] } }` — discovery silently finds nothing. Support both formats going forward.
- **Finalize tool name**: Decide on final name before v0.2, when the name gets written into targets' MCP configs and becomes harder to change.

## v0.2 — Connector Architecture

The current model hardcodes targets as struct fields and keeps source/target logic separate. Both sides are really the same concept: an **endpoint with a connector type** that knows how to discover, read, write, and translate skills.

- **Generic `[[targets]]` array**: Replace the hardcoded `Targets` struct with a `Vec<Target>` — same shape as sources. Each target has a `name`, `path`, `type`, and connector-specific options
- **Connector trait**: Unified interface for both source and target behavior — discovery format, distribution method (symlink, MCP config, copy), and format translation needs
- **Built-in connectors**: Claude (plugins + standalone), Codex, Antigravity, Cursor, Windsurf, OpenCode, Nanobot, PicoClaw, OpenClaw, VS Code Copilot, Amp, Goose
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

Expand the basic validation idea into a real lint pass that catches cross-tool compatibility issues:

- **Multiline YAML description bug detection**: Warn when a SKILL.md description uses multiline YAML that Claude Code's parser silently truncates
- **Description length validation**: Enforce spec limits (1024 chars Claude Code, 500 chars Copilot) and warn when exceeded
- **Skill name vs directory name mismatch**: Flag when the `name:` frontmatter field doesn't match the containing directory name
- **Body size warnings**: Warn when skill body exceeds target limits (6000 chars for Windsurf, ~5000 tokens general recommendation)
- **Hidden Unicode Tag codepoint scanning**: Detect U+E0001–U+E007F tag characters that can smuggle invisible instructions (security)

## v0.4 — Portable Library

Make the skill library reproducible across machines via a lockfile and per-machine preferences.

- **Library as canonical home**: Local skills live directly in the library (real directories, not symlinks). Managed skills (Claude marketplace, future registries) are symlinked in from their package manager locations.
- **`skync.lock`**: Tracked lockfile in the library recording every skill's type (local/managed), source, and install metadata. For managed plugins: `plugin-name@registry` identifier + version (from `installed_plugins.json` v2 key format). Enough info to reproduce the library on a fresh machine.
- **Per-machine preferences** (`~/.config/skync/machine.toml`): Per-machine opt-in/opt-out for managed plugins — machine A installs plugins 1,2,3 while machine B only wants 1 and 3.
- **`skync update` command**: Reads lockfile, diffs against local state, prompts user about new/missing managed plugins, actively runs `claude plugin install <name@registry>` for approved plugins, then syncs.
- **Claude marketplace first**: First managed source targeting the Claude plugin marketplace. Version pinning via version string or git commit SHA.
- **Git-friendly library**: Library directory works as a git repo — local skills tracked in git, managed symlinks recreated by `skync update` (gitignored), lockfile tracked.

## v0.5 — Git Sources

- Add `type = "git"` source for remote skill repositories
- Clone/pull on sync with caching
- Pin to branch, tag, or commit SHA
- Support private repos via SSH keys or token auth

## v0.6 — Watch Mode

- `skync watch` for auto-sync on filesystem changes
- Debounced fsnotify-based watcher
- Optional desktop notification on sync

## Future Ideas

- **Plugin registry**: Browse and install community skill packs
- **Conflict resolution UI**: Interactive merge when skills collide
- **Skill validation**: Lint SKILL.md for common issues (missing frontmatter, broken links)
- **Shell completions**: Generate completions for bash, zsh, fish
- **Homebrew formula**: `brew install skync`
- **Backup snapshots**: Optional tarball backup of library state before destructive operations
- **Token budget estimation**: Show estimated token cost per skill per target tool in `skync status` output
- **Security audit command**: `skync audit` to scan skills for prompt injection vectors, hidden unicode, and suspicious patterns
- **Portable memory extraction**: Suggest MEMORY.md entries that could be promoted to reusable skills (`skync suggest-skills`)
- **Plugin output generation**: Package the skill library as a distributable Claude plugin, Cursor plugin, etc.

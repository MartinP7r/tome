# Roadmap

## v0.2 — Rules & Agents

- Support syncing `.claude/rules/` and agent definitions alongside skills
- Discover rules from plugin caches and standalone directories
- Distribute rules to target tool locations

## v0.3 — Format Transforms

- Transform SKILL.md into alternative formats for tools that don't support it natively
- Pluggable transform pipeline (SKILL.md → Cursor rules, Windsurf conventions, etc.)
- Preserve original format — transforms are output-only

## v0.4 — Git Sources

- Add `type = "git"` source for remote skill repositories
- Clone/pull on sync with caching
- Pin to branch, tag, or commit SHA
- Support private repos via SSH keys or token auth

## v0.5 — Watch Mode

- `skync watch` for auto-sync on filesystem changes
- Debounced fsnotify-based watcher
- Optional desktop notification on sync

## Future Ideas

- **Plugin registry**: Browse and install community skill packs
- **Conflict resolution UI**: Interactive merge when skills collide
- **Skill validation**: Lint SKILL.md for common issues (missing frontmatter, broken links)
- **Shell completions**: Generate completions for bash, zsh, fish
- **Homebrew formula**: `brew install skync`

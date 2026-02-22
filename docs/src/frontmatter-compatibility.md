# Frontmatter Compatibility

SKILL.md files use YAML frontmatter to declare metadata. The base standard comes from the [Agent Skills spec](https://agentskills.io), but each platform extends it with its own fields. This page documents the current state of compatibility across tools.

## Base Standard (agentskills.io)

| Field | Required | Constraints |
|-------|----------|-------------|
| `name` | Yes | Max 64 chars. Lowercase letters, numbers, hyphens only. Must match directory name. |
| `description` | Yes | Max 1024 chars. Non-empty. |
| `license` | No | License name or reference. |
| `compatibility` | No | Max 500 chars. Environment requirements. |
| `metadata` | No | Arbitrary key-value map. |
| `allowed-tools` | No | Space-delimited tool list. (Experimental) |

## Platform Extensions

These fields are valid on their respective platforms but will be silently ignored (or warned about) elsewhere.

| Field | Platform | Purpose |
|-------|----------|---------|
| `disable-model-invocation` | Claude Code | User-only invocation (no auto-trigger) |
| `user-invocable` | Claude Code | `false` = model-only background knowledge |
| `argument-hint` | Claude Code | Hint for argument parsing |
| `context` | Claude Code | `fork` = run in isolated subagent |
| `agent` | Claude Code | Specify subagent type (e.g., `Explore`) |
| `hooks` | Claude Code | Lifecycle hooks scoped to the skill |
| `excludeAgent` | VS Code Copilot | Target `coding-agent` vs `code-review` |

Codex uses a separate `agents/openai.yaml` file instead of extending SKILL.md frontmatter.

## Non-Standard Fields Found in the Wild

These appear in community skills but are **not part of any spec**. They will be silently ignored by standard-compliant tools.

| Field | Issue | Recommendation |
|-------|-------|----------------|
| `version` | Not in any spec | Move to `metadata.version` |
| `category` | Not in any spec | Move to `metadata.category` |
| `tags` | Not in any spec | Move to `metadata.tags` |
| `last-updated` | Not in any spec | Move to `metadata.last-updated` |
| `model` | Agent frontmatter field, not SKILL.md | Remove or move to agent config |

## Known Bugs & Gotchas

### VSCode validator flags valid fields

The VS Code Copilot extension's skill validator has an outdated schema that flags `allowed-tools` as unsupported, even though it's part of the base spec. This is a [known issue](https://github.com/microsoft/vscode-copilot-release/issues).

### Multiline YAML descriptions break on Claude Code

Claude Code's SKILL.md parser does not handle implicit YAML folding (Prettier-style wrapped lines). Descriptions that span multiple lines without an explicit block scalar will be silently truncated.

**Breaks:**
```yaml
---
description: This is a long description that has been
  wrapped by Prettier across multiple lines
---
```

**Works:**
```yaml
---
description: This is a long description on a single line
---
```

**Also works (explicit block scalar):**
```yaml
---
description: |
  This is a long description that uses
  an explicit block scalar indicator
---
```

### Unknown fields are silently ignored

All standard-compliant tools silently ignore unknown frontmatter fields. The VS Code extension is an exception — it shows warnings for unrecognized fields. This means non-standard fields won't cause errors but also won't do anything.

### Case sensitivity

- All field names must be **lowercase**
- The filename must be exactly `SKILL.md` (uppercase)

## Platform Limits

| Constraint | Limit | Platform |
|------------|-------|----------|
| `name` length | 64 chars | All (base spec) |
| `description` length | 1024 chars | All (base spec) |
| `description` length | 500 chars | VS Code Copilot (stricter) |
| `compatibility` length | 500 chars | All (base spec) |
| Skill body size | ~6000 chars | Windsurf |
| Skill body size | ~5000 tokens | General recommendation |

## How tome Uses This

tome currently symlinks skill directories as-is without parsing frontmatter. The v0.3.x release will add:

- **Frontmatter parsing** during discovery
- **`tome lint`** command with tiered validation (errors, warnings, info)
- **`tome doctor`** frontmatter health checks
- **`tome status`** metadata summary per skill

See the [Roadmap](roadmap.md) for details.

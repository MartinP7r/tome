# Frontmatter Compatibility

SKILL.md files use YAML frontmatter to declare metadata. The base standard comes from the [Agent Skills specification](https://agentskills.io/specification).[1] Platforms may add their own fields, but those extensions do not change the base format.

## Base Standard (agentskills.io)

| Field | Required | Constraints |
|-------|----------|-------------|
| `name` | Yes | 1–64 lowercase letters, digits, and hyphens; no leading or trailing hyphen; must match the parent directory name.[1] |
| `description` | Yes | 1–1024 characters. Non-empty.[1] |
| `license` | No | License name or reference to a bundled license file.[1] |
| `compatibility` | No | Max 500 characters. Environment requirements.[1] |
| `metadata` | No | String-to-string mapping (`string` keys and `string` values).[1] |
| `allowed-tools` | No | Space-separated string of pre-approved tools. Experimental.[1] |

The published specification defines the name constraints above.[1] The
`skills-ref` reference validator additionally trims surrounding whitespace from
the frontmatter value, applies Unicode NFKC normalization to both the value and
the directory name before comparing them, and rejects consecutive hyphens.
Tome follows those reference-validator behaviors; they should not be confused
with additional constraints stated by the published specification.[2]

## Platform Extensions

These are platform-specific extensions, not fields in the base standard. Their behavior is host-specific; Tome's base-format validation does not guarantee support by a target agent.

| Field | Platform | Purpose |
|-------|----------|---------|
| `disable-model-invocation` | Claude Code | User-only invocation (no auto-trigger).[8] |
| `user-invocable` | Claude Code | `false` = model-only background knowledge.[8] |
| `argument-hint` | Claude Code | Hint for argument parsing.[8] |
| `context` | Claude Code | `fork` = run in isolated subagent.[8] |
| `agent` | Claude Code | Specify subagent type (e.g., `Explore`).[8] |
| `hooks` | Claude Code | Lifecycle hooks scoped to the skill.[8] |
| `excludeAgent` | VS Code Copilot | Target `coding-agent` vs `code-review`.[5] |

Codex uses a separate `agents/openai.yaml` file for UI metadata and invocation policy instead of extending the standard SKILL.md frontmatter.[6]

## Non-Standard Fields Found in the Wild

These appear in community skills but are **not defined by the base specification**.[1] Prefer `metadata` for non-standard values; host behavior for unknown fields varies.

| Field | Issue | Recommendation |
|-------|-------|----------------|
| `version` | Not in any spec | Move to `metadata.version` |
| `category` | Not in any spec | Move to `metadata.category` |
| `tags` | Not in any spec | Move to `metadata.tags` |
| `last-updated` | Not in any spec | Move to `metadata.last-updated` |
| `model` | Agent frontmatter field, not SKILL.md | Remove or move to agent config |

## Compatibility Notes

The specification includes `allowed-tools` as an experimental, space-separated string.[1] Host parsers and validators can differ; for example, OpenCode documents its own handling of unknown fields.[13] Validate the target agent's current documentation when relying on a platform extension. Tome reports base-format issues only and does not certify runtime compatibility.

### Case sensitivity

- Use the exact lowercase field names defined by the format.[1]
- The filename must be exactly `SKILL.md` (uppercase).

## Platform Limits

| Constraint | Limit | Platform |
|------------|-------|----------|
| `name` length | 1–64 characters | Agent Skills base specification.[1] |
| `description` length | 1–1024 characters | Agent Skills base specification.[1] |
| `compatibility` length | Up to 500 characters | Agent Skills base specification.[1] |

This page does not claim stricter limits for a particular host; check current vendor documentation before relying on host-specific limits.

## How Tome Uses This

`tome lint` parses `SKILL.md` frontmatter and reports format violations. It
errors on missing or invalid required fields, invalid names or directory-name
mismatches, a blank or over-1024-character description, compatibility text over
500 characters, non-string metadata values, and a non-string `allowed-tools`
value. Name comparison and the name-length check use NFKC-normalized text. The
linter also reports selected compatibility and non-standard-field warnings;
those warnings do not replace validation by a target agent.

This is format validation only. It does not guarantee that every platform
extension or tool named in a skill is supported by a particular agent.

## Sources

[1] https://agentskills.io/specification — Agent Skills specification
[2] https://github.com/agentskills/agentskills/blob/main/skills-ref/src/skills_ref/validator.py — Agent Skills reference validator
[5] https://code.visualstudio.com/docs/agent-customization/agent-skills — VS Code Agent Skills docs
[6] https://developers.openai.com/codex/skills — Codex skills docs
[8] https://code.claude.com/docs/en/skills — Claude Code skills documentation
[13] https://opencode.ai/docs/skills — OpenCode Agent Skills documentation

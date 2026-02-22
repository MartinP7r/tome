# Agent Skills Invocation Syntax: Cross-Platform Investigation

> Research compiled: 2026-02-03

## Executive Summary

The Agent Skills open standard (agentskills.io) defines only the **file format** (SKILL.md structure, frontmatter, directory layout). **Invocation syntax is not standardized** and varies by implementation.

| Tool | User Invocation | Automatic | Notes |
|------|-----------------|-----------|-------|
| **Claude Code** | `/<skill-name>` | Yes | Slash prefix |
| **OpenAI Codex CLI** | `$<skill-name>` | Yes | Dollar sign prefix |
| **Cursor** | `/<skill-name>` | Yes | Slash prefix (Agent chat) |
| **Windsurf** | `@<skill-name>` | Yes | At-mention prefix |
| **Gemini CLI** | None documented | Yes | Automatic only via `activate_skill` tool |
| **GitHub Copilot** | None documented | Yes | Automatic only (description matching) |
| **Amp** | Command palette | Yes | `skill: invoke` command |
| **Roo Code** | None documented | Yes | Automatic only (description matching) |

## Detailed Findings

### Claude Code (Anthropic)

**Documentation:** [code.claude.com/docs/en/skills](https://code.claude.com/docs/en/skills)

**User Invocation:**
- Syntax: `/<skill-name>` (e.g., `/explain-code`)
- Can pass arguments: `/fix-issue 123`

**Automatic Invocation:**
- Claude loads skills when relevant based on description
- Can be disabled with `disable-model-invocation: true` in frontmatter

**Permission Rules:**
- `Skill(name)` for exact match
- `Skill(name *)` for prefix match with arguments

**Skill Locations:**
- Personal: `~/.claude/skills/<skill-name>/SKILL.md`
- Project: `.claude/skills/<skill-name>/SKILL.md`

---

### OpenAI Codex CLI

**Documentation:** [developers.openai.com/codex/skills](https://developers.openai.com/codex/skills/)

**User Invocation:**
- Syntax: `$<skill-name>` (e.g., `$skill-creator`, `$create-plan`)
- Interactive: `/skills` slash command opens skill selector
- Start typing `$` to mention a skill

**Automatic Invocation:**
- Codex can automatically invoke skills when task matches description

**Examples:**
```
$skill-installer install the linear skill from the .experimental folder
$create-plan
$skill-creator Create a skill that drafts commit messages
```

**Skill Locations:**
- Repository: `.codex/skills/`
- User: `~/.codex/skills/`
- Also reads from `.agents/skills/`

---

### Cursor

**Documentation:** [cursor.com/docs/context/skills](https://cursor.com/docs/context/skills)

**User Invocation:**
- Syntax: `/<skill-name>` in Agent chat
- Type `/` to search and select skills

**Automatic Invocation:**
- Skills auto-activate based on context relevance
- Disable with `disable-model-invocation: true`

**Note:** Agent Skills currently only available in nightly release channel.

**Skill Locations:**
- Per repo: `[.provider]/<skill-name>/SKILL.md`

---

### Windsurf (Codeium)

**Documentation:** [docs.windsurf.com/windsurf/cascade/skills](https://docs.windsurf.com/windsurf/cascade/skills)

**User Invocation:**
- Syntax: `@<skill-name>` in Cascade input (e.g., `@deploy-to-staging`)

**Automatic Invocation:**
- Progressive disclosure - skills activate when relevant to task

**Workflows (separate feature):**
- Syntax: `/<workflow-name>` (e.g., `/0-task`, `/1-discovery`)
- Stored in `.windsurf/workflows/`

**Skill Locations:**
- Project: `.windsurf/skills/`

---

### Google Gemini CLI

**Documentation:** [geminicli.com/docs/cli/skills](https://geminicli.com/docs/cli/skills/)

**User Invocation:**
- No explicit invocation syntax documented
- Management: `/skills list`, `/skills enable <name>`, `/skills disable <name>`

**Automatic Invocation:**
- Gemini autonomously decides based on task and skill description
- Uses internal `activate_skill` tool
- User receives confirmation prompt before activation

**Skill Locations:**
- Project: `.gemini/skills/`
- User: `~/.gemini/skills/`

**Note:** Agent Skills are experimental; must be enabled via `/settings`.

---

### GitHub Copilot

**Documentation:** [code.visualstudio.com/docs/copilot/customization/agent-skills](https://code.visualstudio.com/docs/copilot/customization/agent-skills)

**User Invocation:**
- No manual invocation syntax documented
- Purely automatic based on description matching

**Automatic Invocation:**
- Three-level loading: Discovery → Conditional Loading → Resource Access
- Skills auto-activate when request matches description

**Skill Locations:**
- Project: `.github/skills/` (recommended) or `.claude/skills/` (legacy)
- Personal: `~/.copilot/skills/` (recommended) or `~/.claude/skills/` (legacy)

**Cross-compatibility:** Reads skills from Claude Code locations automatically.

---

### Amp

**Documentation:** [ampcode.com/news/user-invokable-skills](https://ampcode.com/news/user-invokable-skills)

**User Invocation:**
- Command palette: `skill: invoke`
  - Amp Editor: `Cmd/Alt-Shift-A`
  - Amp CLI: `Ctrl-O`
- CLI: `amp skill` for management

**Automatic Invocation:**
- Model determines when to invoke based on name/description

**Skill Locations:**
- Project: `.agents/skills/`
- User: `~/.config/agents/skills/`
- Also reads: `.claude/skills/`, `~/.claude/skills/`

---

### Roo Code

**Documentation:** [docs.roocode.com/features/skills](https://docs.roocode.com/features/skills)

**User Invocation:**
- No manual invocation syntax
- Purely automatic based on description matching

**Automatic Invocation:**
- Progressive disclosure system
- Uses `read_file` to load matching skills

**Skill Locations:**
- Global: `~/.roo/skills/`
- Project: `.roo/skills/`
- Mode-specific: `.roo/skills-code/`, `.roo/skills-architect/`, etc.

---

## Pattern Analysis

### Invocation Prefix Patterns

| Pattern | Tools |
|---------|-------|
| `/` (slash) | Claude Code, Cursor |
| `$` (dollar) | OpenAI Codex CLI |
| `@` (at-mention) | Windsurf |
| Command palette | Amp |
| Automatic only | Gemini CLI, GitHub Copilot, Roo Code |

### Common Themes

1. **Automatic invocation is universal** - All tools support automatic skill activation based on description matching

2. **Manual invocation varies widely** - No consensus on syntax (`/`, `$`, `@`, or none)

3. **Progressive disclosure** - Most tools use a 3-level loading system:
   - Level 1: Description only (always loaded)
   - Level 2: Full SKILL.md (on match)
   - Level 3: Supporting files (on demand)

4. **Cross-compatibility efforts** - Several tools read from multiple skill directories for compatibility (e.g., Copilot reads `.claude/skills/`, Amp reads `.claude/skills/`)

5. **Disable flags** - `disable-model-invocation: true` is common for preventing automatic activation

---

## Recommendations for Documentation

When writing prompts or documentation that reference skills:

1. **For automatic invocation**: Just describe the task naturally; all tools support this

2. **For manual invocation**: Use tool-specific syntax:
   - Claude Code/Cursor: `/<skill-name>`
   - Codex CLI: `$<skill-name>`
   - Windsurf: `@<skill-name>`

3. **For cross-tool compatibility**: Focus on automatic invocation via good descriptions

4. **For programmatic references in prompts** (e.g., sub-agent delegation):
   - No official standard exists
   - Community convention: `Skill(skill: '<name>')` (from meta-skills plugin)
   - Alternative: Just mention the skill naturally and let the tool invoke it

---

## Sources

### Official Documentation
- [Agent Skills Open Standard](https://agentskills.io)
- [Agent Skills Specification](https://agentskills.io/specification)
- [Claude Code Skills](https://code.claude.com/docs/en/skills)
- [OpenAI Codex Skills](https://developers.openai.com/codex/skills/)
- [Cursor Agent Skills](https://cursor.com/docs/context/skills)
- [Windsurf Cascade Skills](https://docs.windsurf.com/windsurf/cascade/skills)
- [Gemini CLI Skills](https://geminicli.com/docs/cli/skills/)
- [GitHub Copilot Agent Skills](https://code.visualstudio.com/docs/copilot/customization/agent-skills)
- [Amp User-Invokable Skills](https://ampcode.com/news/user-invokable-skills)
- [Roo Code Skills](https://docs.roocode.com/features/skills)

### GitHub Repositories
- [anthropics/skills](https://github.com/anthropics/skills) - Anthropic's official skills repository
- [agentskills/agentskills](https://github.com/agentskills/agentskills) - Open standard repository
- [openai/skills](https://github.com/openai/skills) - OpenAI's skills catalog

### Additional Resources
- [GitHub Copilot Skills Changelog](https://github.blog/changelog/2025-12-18-github-copilot-now-supports-agent-skills/)
- [About Agent Skills - GitHub Docs](https://docs.github.com/en/copilot/concepts/agents/about-agent-skills)

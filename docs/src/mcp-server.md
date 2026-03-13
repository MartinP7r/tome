# MCP Server

tome includes a built-in MCP server for tools that support the Model Context Protocol:

```bash
# Standalone binary
tome-mcp

# Or via the CLI
tome serve
```

The server exposes two tools:
- `list_skills` — List all discovered skills with name and description (excludes disabled skills per machine preferences)
- `read_skill` — Read a skill's SKILL.md content by name (returns "not found" for disabled skills)

## Machine Preferences

The MCP server respects per-machine preferences from `~/.config/tome/machine.toml`. Skills listed in the `disabled` set are excluded from `list_skills` results and return "not found" from `read_skill`. This ensures that disabled skills are invisible to MCP-consuming tools, matching the behavior of `tome sync`.

Use the `--machine <path>` flag to override the default machine preferences path when starting the server.

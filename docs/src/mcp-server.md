# MCP Server

skillet includes a built-in MCP server for tools that support the Model Context Protocol:

```bash
# Standalone binary
skillet-mcp

# Or via the CLI
skillet serve
```

The server exposes two tools:
- `list_skills` — List all discovered skills
- `read_skill` — Read a skill's SKILL.md content by name

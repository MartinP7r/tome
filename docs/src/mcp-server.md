# MCP Server

tome includes a built-in MCP server for tools that support the Model Context Protocol:

```bash
# Standalone binary
tome-mcp

# Or via the CLI
tome serve
```

The server exposes two tools:
- `list_skills` — List all discovered skills
- `read_skill` — Read a skill's SKILL.md content by name

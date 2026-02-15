# Remaining Work Plan

## Status: Tasks #1–#7 Complete

- Workspace scaffolded at `~/Development/skync`
- All CLI modules implemented with 23 passing unit tests
- Commands working: init, sync, status, doctor, list, config
- rust-cli skill created at `~/.claude/skills/rust-cli/` (11 files)
- Git repo initialized with initial commit

## Step 1: Code Review with rust-cli Skill

Review all existing modules against the rust-cli skill patterns before building more:
- Error handling: thiserror for lib boundaries, anyhow with context in application code
- CLI: clap derive patterns, arg groups, validation
- Config: serde + TOML, tilde expansion, defaults
- Filesystem: walkdir usage, symlink creation, PathBuf patterns
- Testing: unit test coverage, assert_fs fixtures, test helpers
- Project structure: main.rs/lib.rs split, module organization
- Fix any deviations from idiomatic Rust patterns

**Files to review:**
- `crates/skync/src/cli.rs`
- `crates/skync/src/config.rs`
- `crates/skync/src/discover.rs`
- `crates/skync/src/library.rs`
- `crates/skync/src/distribute.rs`
- `crates/skync/src/cleanup.rs`
- `crates/skync/src/status.rs`
- `crates/skync/src/doctor.rs`
- `crates/skync/src/wizard.rs`
- `crates/skync/src/lib.rs`
- `crates/skync/src/main.rs`

## Step 2: MCP Server (`skync-mcp` crate)

### 2a. Verify rmcp API compatibility
- Check `rmcp` 0.15 actual API (docs.rs) against skill examples
- The skill's MCP-Server.md examples were written for 0.1 — API may differ at 0.15
- Key types to verify: `ServerHandler`, `ServiceExt`, `tool` macro, `CallToolResult`

### 2b. Add skync-mcp dependency on skync
- skync-mcp needs access to config, discover, library modules
- Add `skync = { path = "../skync" }` to skync-mcp/Cargo.toml
- Or extract shared types — but sharing the lib crate is simpler for v1

### 2c. Implement server.rs
- `SkyncServer` struct holding `Arc<Config>`
- Tool: `list_skills` — discover and return skills as JSON
- Tool: `read_skill` — read a specific SKILL.md by name from library
- Tool: `sync_skills` — trigger sync pipeline
- `ServerHandler` impl with `get_info()`

### 2d. Wire main.rs (skync-mcp)
- `#[tokio::main]` entry point
- Load config, create server, serve on stdio
- Log to stderr (stdout = MCP transport)

### 2e. Wire `skync serve` subcommand
- In `lib.rs`, replace placeholder `serve_mcp()` with actual rmcp server
- Use `tokio::runtime::Runtime::new()?.block_on()` (keep main sync)

## Step 3: Polish & Release

### 3a. Integration tests
- `tests/integration/` with assert_cmd tests
- Test: `skync --help`, `skync --version`
- Test: `skync sync --dry-run` with fixture config
- Test: `skync list` with fixture skills
- Test: idempotency (sync twice = same result)

### 3b. README.md
- Project description, install instructions, usage examples
- Example of `skync status` output

### 3c. ROADMAP.md
- Rules/agents/commands support (beyond skills)
- Format transformations (SKILL.md → other formats)
- Git sources (remote skill repos)
- Watch mode (`skync watch` for auto-sync)

### 3d. CI (GitHub Actions)
- `.github/workflows/ci.yml`: cargo test, cargo clippy, cargo fmt --check
- Matrix: ubuntu-latest + macos-latest

### 3e. Git commit and initial push
- Commit remaining work
- Create GitHub repo `martinP7r/skync`
- Push

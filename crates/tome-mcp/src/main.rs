//! Standalone MCP binary for tome.
//!
//! Thin wrapper that loads config and starts the MCP server over stdio.
//! The same server is also reachable via `tome serve`.

use tome::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load_or_default(None)?;
    config.validate()?;
    tome::mcp::serve(config, None).await
}

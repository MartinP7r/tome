use skillet::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load_or_default(None)?;
    skillet::mcp::serve(config).await
}

use tome::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load_or_default(None)?;
    config.validate()?;
    tome::mcp::serve(config).await
}

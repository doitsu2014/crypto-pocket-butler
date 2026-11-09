use portfolio::config::ApiConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ApiConfig::from_env()?;
    portfolio::run(config).await
}

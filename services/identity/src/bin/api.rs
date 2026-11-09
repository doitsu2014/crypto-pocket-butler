use identity::config::ApiConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ApiConfig::from_env()?;
    identity::run(config).await
}

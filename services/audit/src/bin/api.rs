use audit::config::ApiConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ApiConfig::from_env()?;
    audit::run(config).await
}

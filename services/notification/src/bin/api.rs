use notification::config::ApiConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ApiConfig::from_env()?;
    notification::run_api(config).await
}

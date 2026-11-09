use ledger::config::ApiConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ApiConfig::from_env()?;
    ledger::run_api(config).await
}

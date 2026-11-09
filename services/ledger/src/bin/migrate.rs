use ledger::config::MigrateConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = MigrateConfig::from_env()?;
    ledger::run_migrate(config).await
}

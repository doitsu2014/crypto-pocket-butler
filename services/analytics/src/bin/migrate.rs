use analytics::config::MigrateConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = MigrateConfig::from_env()?;
    analytics::run_migrate(config).await
}

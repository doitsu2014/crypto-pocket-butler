use notification::config::MigrateConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = MigrateConfig::from_env()?;
    notification::run_migrate(config).await
}

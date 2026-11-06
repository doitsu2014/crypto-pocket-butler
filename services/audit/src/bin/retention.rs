use audit::config::RetentionConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = RetentionConfig::from_env()?;
    audit::run_retention(config).await
}

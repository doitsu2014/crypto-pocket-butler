use analytics::config::WorkerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = WorkerConfig::from_env()?;
    analytics::run_worker(config).await
}

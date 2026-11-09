use identity::config::WorkerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = WorkerConfig::from_env()?;
    identity::run_worker(config).await
}

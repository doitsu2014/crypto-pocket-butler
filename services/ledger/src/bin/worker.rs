use ledger::config::WorkerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = WorkerConfig::from_env()?;
    ledger::run_worker(config).await
}

use job_orchestration::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    job_orchestration::run(config).await
}

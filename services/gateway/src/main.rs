use gateway::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    gateway::run(config).await
}

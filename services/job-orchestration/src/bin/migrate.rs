use std::path::Path;

/// Applies migrations and runs the retention prune with `--prune-runs`.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = read_database_url()?;
    let pool = job_orchestration_infrastructure::connect(&database_url, 2).await?;
    job_orchestration_infrastructure::migrate(&pool).await?;
    if std::env::args().any(|argument| argument == "--prune-runs")
        || std::env::var("PRUNE_RUNS")
            .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
    {
        let removed = job_orchestration_infrastructure::prune_runs(&pool, 400).await?;
        println!("pruned {removed} run rows older than 400 days");
    }
    Ok(())
}

fn read_database_url() -> anyhow::Result<String> {
    if let Ok(path) = std::env::var("DATABASE_URL_FILE") {
        let value = std::fs::read_to_string(Path::new(&path))?;
        let value = value.trim().to_string();
        if value.is_empty() {
            anyhow::bail!("DATABASE_URL_FILE points at an empty secret file");
        }
        return Ok(value);
    }
    std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL_FILE must be set"))
}

use std::path::Path;

/// Applies the reporting migrations and exits. The release job runs this once
/// against the migrator DSN before the API and worker are updated.
///
/// It also runs the two-year brief retention prune when invoked as
/// `reporting-migrate --prune-briefs` (or with `PRUNE_BRIEFS=true`). That path is
/// deliberately separate from the service: the runtime role holds no DELETE
/// grant on `briefs`, so only the migrator/operator DSN may delete.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = read_database_url()?;
    let pool = reporting_infrastructure::connect(&database_url, 2).await?;
    reporting_infrastructure::migrations::run(&pool).await?;
    if prune_requested() {
        let removed = reporting_infrastructure::migrations::prune_briefs(&pool, 730).await?;
        println!("pruned {removed} briefs older than two years");
    }
    Ok(())
}

fn prune_requested() -> bool {
    std::env::args().any(|argument| argument == "--prune-briefs")
        || std::env::var("PRUNE_BRIEFS")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
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
    std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL_FILE must point at the migrator DSN"))
}

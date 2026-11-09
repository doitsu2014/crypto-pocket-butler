use std::path::Path;

/// Applies the portfolio migrations and exits. The release job runs this once
/// against the migrator DSN before the API and worker are updated.
///
/// It also runs the two-year snapshot retention prune when invoked as
/// `portfolio-migrate --prune-snapshots` (or with `PRUNE_SNAPSHOTS=true`). That
/// path is deliberately separate from the service: the runtime role is
/// insert-only on `valuation_snapshots`, so only the migrator/operator DSN may
/// delete. A release procedure records the prune as an operator step.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = read_database_url()?;
    let pool = portfolio_infrastructure::connect(&database_url, 2).await?;
    portfolio_infrastructure::migrations::run(&pool).await?;
    if prune_requested() {
        let removed = portfolio_infrastructure::migrations::prune_snapshots(&pool, 730).await?;
        println!("pruned {removed} valuation snapshots older than two years");
    }
    Ok(())
}

fn prune_requested() -> bool {
    std::env::args().any(|argument| argument == "--prune-snapshots")
        || std::env::var("PRUNE_SNAPSHOTS")
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

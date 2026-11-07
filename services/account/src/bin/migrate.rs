use std::path::Path;

/// Applies the account migrations and exits. The release job runs this once
/// against the migrator DSN before the API and worker are updated.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = read_database_url()?;
    let pool = account_infrastructure::connect(&database_url, 2).await?;
    account_infrastructure::migrations::run(&pool).await?;
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
    std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL_FILE must point at the migrator DSN"))
}

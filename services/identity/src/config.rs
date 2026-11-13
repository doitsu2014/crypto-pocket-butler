//! Non-secret configuration. Every credential is read from a secret file
//! (`*_FILE`) — a Docker secret mounted from Supabase Vault — and never from an
//! env file committed to the repository.
//!
//! The parsers take an injected [`Lookup`] so the composition root can be
//! exercised without mutating the process environment.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Looks up one environment variable. `from_env` reads the real process
/// environment; the tests supply a map.
type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The API service's configuration.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// HTTP port.
    pub port: u16,
    /// Postgres connection string for the `identity` schema (secret).
    pub database_url: String,
    /// The service-token registry file (secret).
    pub service_tokens_path: PathBuf,
    /// Supabase JWKS URL (public).
    pub jwks_url: String,
    /// Expected JWT issuer.
    pub jwt_issuer: String,
    /// Expected JWT audience.
    pub jwt_audience: String,
    /// Supabase Auth admin base URL.
    pub supabase_auth_url: String,
    /// Supabase service-role key (secret); used only for Auth admin and Storage.
    pub service_role_key: String,
    /// Supabase Storage base URL.
    pub storage_url: String,
    /// The private export bucket.
    pub export_bucket: String,
    /// Audit service base URL.
    pub audit_url: String,
    /// Account service base URL (privacy contributions).
    pub account_url: String,
    /// Ledger service base URL (privacy contributions).
    pub ledger_url: String,
    /// Portfolio service base URL (privacy contributions).
    pub portfolio_url: String,
    /// Identity worker wake URL.
    pub worker_url: String,
    /// The service name whose token the API presents to other services.
    pub token_service: String,
    /// The HMAC key that signs the short-lived pre-sign-up gate token (secret).
    pub gate_token_key: String,
    /// The geolocation provider base URL; empty means not configured, and every
    /// jurisdiction check then fails closed (secret, mounted from Vault).
    pub jurisdiction_provider_url: String,
    /// The geolocation provider API key (secret).
    pub jurisdiction_provider_key: Option<String>,
    /// The services allowed on the C4 `/internal/authorize` endpoint. Each must
    /// present its own token, which the service-token registry resolves.
    pub authorize_services: Vec<String>,
    /// The services allowed on the C6 consent-state route. The source-connection
    /// path is the only consumer.
    pub consent_services: Vec<String>,
    /// How often the audit-outbox drainer runs.
    pub drainer_interval_seconds: u64,
    /// Maximum pooled connections for the API.
    pub pool_connections: u32,
}

impl ApiConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(&|name| std::env::var(name).ok())
    }

    /// Reads configuration from an injected lookup.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    fn from_lookup(get: Lookup) -> Result<Self> {
        let drainer_interval_seconds: u64 = env_or(get, "IDENTITY_DRAINER_INTERVAL_SECONDS", "5")
            .parse()
            .context("IDENTITY_DRAINER_INTERVAL_SECONDS must be a number")?;
        if drainer_interval_seconds == 0 {
            anyhow::bail!(
                "IDENTITY_DRAINER_INTERVAL_SECONDS must be at least 1; a zero interval panics"
            );
        }
        Ok(Self {
            port: env_or(get, "IDENTITY_PORT", "8083")
                .parse()
                .context("IDENTITY_PORT must be a valid port")?,
            database_url: secret(get, "DATABASE_URL")?,
            service_tokens_path: PathBuf::from(required(get, "SERVICE_TOKENS_FILE")?),
            jwks_url: required(get, "JWT_JWKS_URL")?,
            jwt_issuer: required(get, "JWT_ISSUER")?,
            jwt_audience: required(get, "JWT_AUDIENCE")?,
            supabase_auth_url: required(get, "SUPABASE_AUTH_URL")?,
            service_role_key: secret(get, "SUPABASE_SERVICE_ROLE_KEY")?,
            storage_url: required(get, "SUPABASE_STORAGE_URL")?,
            export_bucket: env_or(get, "IDENTITY_EXPORT_BUCKET", "identity-exports"),
            audit_url: env_or(get, "AUDIT_URL", "http://audit-api:8084"),
            account_url: env_or(get, "ACCOUNT_URL", "http://account:8091"),
            ledger_url: env_or(get, "LEDGER_URL", "http://ledger:8092"),
            portfolio_url: env_or(get, "PORTFOLIO_URL", "http://portfolio:8093"),
            worker_url: env_or(get, "IDENTITY_WORKER_URL", "http://identity-worker:8085"),
            token_service: env_or(get, "IDENTITY_TOKEN_SERVICE", "identity"),
            gate_token_key: secret(get, "GATE_TOKEN_KEY")?,
            jurisdiction_provider_url: file_or(get, "JURISDICTION_PROVIDER_URL", ""),
            jurisdiction_provider_key: optional_secret(get, "JURISDICTION_PROVIDER_KEY"),
            authorize_services: csv_or(
                get,
                "IDENTITY_AUTHORIZE_SERVICES",
                "account-api,portfolio,identity-smoke",
            ),
            consent_services: csv_or(get, "IDENTITY_CONSENT_SERVICES", "account-api"),
            drainer_interval_seconds,
            pool_connections: env_or(get, "IDENTITY_API_POOL", "12")
                .parse()
                .context("IDENTITY_API_POOL must be a number")?,
        })
    }
}

/// The privacy worker's configuration.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// HTTP port for the internal wake endpoint and probes.
    pub port: u16,
    /// Postgres connection string for the `identity` schema as `identity_app`
    /// (secret).
    pub database_url: String,
    /// The retention role's own connection string as `identity_retention`
    /// (secret). The main pool cannot delete consent, so the erasure delete uses
    /// this.
    pub retention_database_url: String,
    /// The service-token registry file (secret).
    pub service_tokens_path: PathBuf,
    /// Supabase Auth admin base URL.
    pub supabase_auth_url: String,
    /// Supabase service-role key (secret).
    pub service_role_key: String,
    /// Supabase Storage base URL.
    pub storage_url: String,
    /// The private export bucket.
    pub export_bucket: String,
    /// Audit service base URL.
    pub audit_url: String,
    /// Account service base URL (privacy contributions).
    pub account_url: String,
    /// Ledger service base URL (privacy contributions).
    pub ledger_url: String,
    /// Portfolio service base URL (privacy contributions).
    pub portfolio_url: String,
    /// The service name whose token the worker presents to other services.
    pub token_service: String,
    /// How long the supervisor sleeps between sweeps.
    pub sweep_interval_seconds: u64,
    /// Maximum pooled connections for the worker.
    pub pool_connections: u32,
}

impl WorkerConfig {
    /// Reads configuration from the environment.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(&|name| std::env::var(name).ok())
    }

    /// Reads configuration from an injected lookup.
    ///
    /// # Errors
    /// Returns an error naming the missing variable when a required value is absent.
    fn from_lookup(get: Lookup) -> Result<Self> {
        Ok(Self {
            port: env_or(get, "IDENTITY_WORKER_PORT", "8085")
                .parse()
                .context("IDENTITY_WORKER_PORT must be a valid port")?,
            database_url: secret(get, "DATABASE_URL")?,
            retention_database_url: secret(get, "IDENTITY_RETENTION_DATABASE_URL")?,
            service_tokens_path: PathBuf::from(required(get, "SERVICE_TOKENS_FILE")?),
            supabase_auth_url: required(get, "SUPABASE_AUTH_URL")?,
            service_role_key: secret(get, "SUPABASE_SERVICE_ROLE_KEY")?,
            storage_url: required(get, "SUPABASE_STORAGE_URL")?,
            export_bucket: env_or(get, "IDENTITY_EXPORT_BUCKET", "identity-exports"),
            audit_url: env_or(get, "AUDIT_URL", "http://audit-api:8084"),
            account_url: env_or(get, "ACCOUNT_URL", "http://account:8091"),
            ledger_url: env_or(get, "LEDGER_URL", "http://ledger:8092"),
            portfolio_url: env_or(get, "PORTFOLIO_URL", "http://portfolio:8093"),
            token_service: env_or(get, "IDENTITY_TOKEN_SERVICE", "identity"),
            sweep_interval_seconds: env_or(get, "IDENTITY_SWEEP_INTERVAL_SECONDS", "60")
                .parse()
                .context("IDENTITY_SWEEP_INTERVAL_SECONDS must be a number")?,
            pool_connections: env_or(get, "IDENTITY_WORKER_POOL", "8")
                .parse()
                .context("IDENTITY_WORKER_POOL must be a number")?,
        })
    }
}

fn required(get: Lookup, name: &str) -> Result<String> {
    get(name).with_context(|| format!("{name} must be set"))
}

fn env_or(get: Lookup, name: &str, fallback: &str) -> String {
    get(name).unwrap_or_else(|| fallback.to_string())
}

/// Reads a comma-separated list, trimming blanks.
fn csv_or(get: Lookup, name: &str, fallback: &str) -> Vec<String> {
    env_or(get, name, fallback)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn secret(get: Lookup, name: &str) -> Result<String> {
    let file_variable = format!("{name}_FILE");
    if let Some(path) = get(&file_variable) {
        let value = std::fs::read_to_string(Path::new(&path))
            .with_context(|| format!("{file_variable} points at an unreadable file"))?;
        let value = value.trim().to_string();
        if value.is_empty() {
            anyhow::bail!("{file_variable} points at an empty secret file");
        }
        return Ok(value);
    }
    if let Some(value) = get(name) {
        tracing::warn!("reading {name} from the environment; use {file_variable} in deployment");
        return Ok(value);
    }
    anyhow::bail!(
        "{file_variable} must be set (the secret lives in Supabase Vault or a Docker secret)"
    )
}

/// Reads `{name}_FILE` when set, otherwise the plain variable, otherwise
/// `fallback`. Used for values that may be supplied as a Docker secret but are
/// not required for the process to start.
fn file_or(get: Lookup, name: &str, fallback: &str) -> String {
    let file_variable = format!("{name}_FILE");
    if let Some(path) = get(&file_variable) {
        if let Ok(value) = std::fs::read_to_string(Path::new(&path)) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return value;
            }
        }
    }
    get(name).unwrap_or_else(|| fallback.to_string())
}

/// Reads an optional secret from `{name}_FILE` or `name`; `None` when neither is
/// set or the file is empty.
fn optional_secret(get: Lookup, name: &str) -> Option<String> {
    let file_variable = format!("{name}_FILE");
    if let Some(path) = get(&file_variable) {
        if let Ok(value) = std::fs::read_to_string(Path::new(&path)) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    get(name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    fn api_pairs() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DATABASE_URL", "postgres://db/identity"),
            (
                "SERVICE_TOKENS_FILE",
                "/run/secrets/identity_service_tokens",
            ),
            ("JWT_JWKS_URL", "https://auth.example/jwks.json"),
            ("JWT_ISSUER", "https://auth.example"),
            ("JWT_AUDIENCE", "authenticated"),
            ("SUPABASE_AUTH_URL", "https://auth.example"),
            ("SUPABASE_SERVICE_ROLE_KEY", "service-role"),
            ("SUPABASE_STORAGE_URL", "https://storage.example"),
            ("GATE_TOKEN_KEY", "gate-key"),
        ]
    }

    fn worker_pairs() -> Vec<(&'static str, &'static str)> {
        vec![
            ("DATABASE_URL", "postgres://db/identity"),
            ("IDENTITY_RETENTION_DATABASE_URL", "postgres://db/retention"),
            (
                "SERVICE_TOKENS_FILE",
                "/run/secrets/identity_service_tokens",
            ),
            ("SUPABASE_AUTH_URL", "https://auth.example"),
            ("SUPABASE_SERVICE_ROLE_KEY", "service-role"),
            ("SUPABASE_STORAGE_URL", "https://storage.example"),
        ]
    }

    /// A scratch directory inside the workspace, removed by the test.
    fn scratch_dir(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "identity-config-test-{name}-{}",
                std::process::id()
            ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("create the scratch directory");
        dir
    }

    #[test]
    fn api_config_parses_values_and_defaults() {
        let get = lookup(&api_pairs());
        let config = ApiConfig::from_lookup(&get).expect("a complete environment is valid");

        assert_eq!(config.port, 8083);
        assert_eq!(config.database_url, "postgres://db/identity");
        assert_eq!(
            config.service_tokens_path,
            PathBuf::from("/run/secrets/identity_service_tokens")
        );
        assert_eq!(config.jwt_issuer, "https://auth.example");
        assert_eq!(config.export_bucket, "identity-exports");
        assert_eq!(config.worker_url, "http://identity-worker:8085");
        assert_eq!(config.token_service, "identity");
        assert_eq!(config.jurisdiction_provider_url, "");
        assert_eq!(config.jurisdiction_provider_key, None);
        assert_eq!(
            config.authorize_services,
            vec![
                "account-api".to_string(),
                "portfolio".to_string(),
                "identity-smoke".to_string()
            ]
        );
        assert_eq!(config.consent_services, vec!["account-api".to_string()]);
        assert_eq!(config.drainer_interval_seconds, 5);
        assert_eq!(config.pool_connections, 12);
    }

    #[test]
    fn api_config_refuses_a_zero_drainer_interval() {
        let mut pairs = api_pairs();
        pairs.push(("IDENTITY_DRAINER_INTERVAL_SECONDS", "0"));
        let get = lookup(&pairs);

        let error = ApiConfig::from_lookup(&get).unwrap_err().to_string();
        assert!(error.contains("at least 1"), "{error}");
    }

    #[test]
    fn api_config_refuses_a_bad_port() {
        let mut pairs = api_pairs();
        pairs.push(("IDENTITY_PORT", "not-a-port"));
        let get = lookup(&pairs);

        let error = ApiConfig::from_lookup(&get).unwrap_err().to_string();
        assert!(
            error.contains("IDENTITY_PORT must be a valid port"),
            "{error}"
        );
    }

    #[test]
    fn api_config_names_a_missing_required_variable() {
        let pairs: Vec<(&str, &str)> = api_pairs()
            .into_iter()
            .filter(|(name, _)| *name != "JWT_ISSUER")
            .collect();
        let get = lookup(&pairs);

        let error = ApiConfig::from_lookup(&get).unwrap_err().to_string();
        assert!(error.contains("JWT_ISSUER must be set"), "{error}");
    }

    #[test]
    fn worker_config_parses_values_and_defaults() {
        let get = lookup(&worker_pairs());
        let config = WorkerConfig::from_lookup(&get).expect("a complete environment is valid");

        assert_eq!(config.port, 8085);
        assert_eq!(config.retention_database_url, "postgres://db/retention");
        assert_eq!(config.sweep_interval_seconds, 60);
        assert_eq!(config.pool_connections, 8);
        assert_eq!(config.export_bucket, "identity-exports");
    }

    #[test]
    fn worker_config_names_a_missing_retention_database() {
        let pairs: Vec<(&str, &str)> = worker_pairs()
            .into_iter()
            .filter(|(name, _)| *name != "IDENTITY_RETENTION_DATABASE_URL")
            .collect();
        let get = lookup(&pairs);

        let error = WorkerConfig::from_lookup(&get).unwrap_err().to_string();
        assert!(
            error.contains("IDENTITY_RETENTION_DATABASE_URL_FILE must be set"),
            "{error}"
        );
    }

    #[test]
    fn csv_or_trims_and_drops_blanks() {
        let pairs = [("LIST", " a , ,b, c ")];
        let get = lookup(&pairs);

        assert_eq!(csv_or(&get, "LIST", "fallback"), vec!["a", "b", "c"]);
        assert_eq!(csv_or(&get, "ABSENT", "x, y"), vec!["x", "y"]);
        assert!(csv_or(&get, "ABSENT", "").is_empty());
    }

    #[test]
    fn secret_reads_a_trimmed_file() {
        let dir = scratch_dir("secret-file");
        let path = dir.join("secret");
        std::fs::write(&path, "  file-secret\n").expect("write the secret file");
        let pairs = [("DATABASE_URL_FILE", path.to_str().expect("utf-8 path"))];
        let get = lookup(&pairs);

        assert_eq!(secret(&get, "DATABASE_URL").unwrap(), "file-secret");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn secret_refuses_an_empty_file() {
        let dir = scratch_dir("secret-empty");
        let path = dir.join("secret");
        std::fs::write(&path, "   \n").expect("write the empty secret file");
        let pairs = [("GATE_TOKEN_KEY_FILE", path.to_str().expect("utf-8 path"))];
        let get = lookup(&pairs);

        let error = secret(&get, "GATE_TOKEN_KEY").unwrap_err().to_string();
        assert!(error.contains("empty secret file"), "{error}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn secret_falls_back_to_the_plain_variable_and_names_the_file_variable() {
        let plain = [("DATABASE_URL", "plain-secret")];
        let get = lookup(&plain);
        assert_eq!(secret(&get, "DATABASE_URL").unwrap(), "plain-secret");

        let absent: Vec<(&str, &str)> = Vec::new();
        let get = lookup(&absent);
        let error = secret(&get, "DATABASE_URL").unwrap_err().to_string();
        assert!(error.contains("DATABASE_URL_FILE must be set"), "{error}");
    }

    #[test]
    fn file_or_prefers_the_file_then_the_plain_value() {
        let dir = scratch_dir("file-or");
        let path = dir.join("value");
        std::fs::write(&path, "from-file").expect("write the value file");
        let pairs = [("THING_FILE", path.to_str().expect("utf-8 path"))];
        let get = lookup(&pairs);
        assert_eq!(file_or(&get, "THING", "fallback"), "from-file");

        let plain = [("THING", "from-env")];
        let get = lookup(&plain);
        assert_eq!(file_or(&get, "THING", "fallback"), "from-env");

        let absent: Vec<(&str, &str)> = Vec::new();
        let get = lookup(&absent);
        assert_eq!(file_or(&get, "THING", "fallback"), "fallback");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn optional_secret_reads_the_file_or_the_plain_value_and_is_none_when_absent() {
        let dir = scratch_dir("optional");
        let path = dir.join("value");
        std::fs::write(&path, "from-file").expect("write the value file");
        let pairs = [("TOKEN_FILE", path.to_str().expect("utf-8 path"))];
        let get = lookup(&pairs);
        assert_eq!(optional_secret(&get, "TOKEN").as_deref(), Some("from-file"));

        let plain = [("TOKEN", "from-env")];
        let get = lookup(&plain);
        assert_eq!(optional_secret(&get, "TOKEN").as_deref(), Some("from-env"));

        let absent: Vec<(&str, &str)> = Vec::new();
        let get = lookup(&absent);
        assert_eq!(optional_secret(&get, "TOKEN"), None);
        std::fs::remove_dir_all(&dir).ok();
    }
}

//! Infrastructure adapter tests that need no database: token hashing, retry
//! classification, the secret token file, the clock, and identifier generation.

use std::path::PathBuf;

use chrono::Utc;
use portfolio_application::ports::{Clock, IdGenerator, ServiceTokenRegistry};
use portfolio_domain::{AuditOutcome, PriceSource};
use portfolio_infrastructure::audit_client::outcome_name;
use portfolio_infrastructure::clients::ledger::price_source_label;
use portfolio_infrastructure::http_retry::is_transient;
use portfolio_infrastructure::service_token::{hash_token, PgServiceTokenRegistry};
use portfolio_infrastructure::{SystemClock, UuidGenerator};
use sqlx::postgres::PgPoolOptions;

fn lazy_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .connect_lazy("postgres://portfolio_svc:unused@127.0.0.1:5432/portfolio")
        .unwrap()
}

fn token_file(content: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "portfolio-token-{}-{}.json",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, content).unwrap();
    path
}

#[test]
fn hash_token_is_a_stable_sha256_hex() {
    let first = hash_token("secret-token");
    let second = hash_token(" secret-token ");
    assert_eq!(first, second);
    assert_eq!(first.len(), 64);
    assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
}

#[test]
fn different_tokens_hash_differently() {
    assert_ne!(hash_token("a"), hash_token("b"));
}

#[test]
fn transient_statuses_are_retried() {
    assert!(is_transient(408));
    assert!(is_transient(429));
    assert!(is_transient(500));
    assert!(is_transient(503));
    assert!(!is_transient(400));
    assert!(!is_transient(404));
    assert!(!is_transient(422));
}

#[tokio::test]
async fn token_file_accepts_the_flat_shape() {
    let file = token_file("{\"portfolio\":\"outbound\"}");
    let registry = PgServiceTokenRegistry::from_file(lazy_pool(), &file).unwrap();
    assert_eq!(
        registry.token_for("portfolio"),
        Some("outbound".to_string())
    );
}

#[tokio::test]
async fn token_file_accepts_the_services_shape() {
    let file = token_file("{\"services\":{\"portfolio\":\"outbound\"}}");
    let registry = PgServiceTokenRegistry::from_file(lazy_pool(), &file).unwrap();
    assert_eq!(
        registry.token_for("portfolio"),
        Some("outbound".to_string())
    );
}

#[tokio::test]
async fn token_file_refuses_an_empty_token() {
    let file = token_file("{\"portfolio\":\"  \"}");
    assert!(PgServiceTokenRegistry::from_file(lazy_pool(), &file).is_err());
}

#[tokio::test]
async fn token_file_refuses_malformed_json() {
    let file = token_file("not json");
    assert!(PgServiceTokenRegistry::from_file(lazy_pool(), &file).is_err());
}

#[tokio::test]
async fn token_file_refuses_a_missing_path() {
    let path = std::env::temp_dir().join("portfolio-token-does-not-exist.json");
    assert!(PgServiceTokenRegistry::from_file(lazy_pool(), &path).is_err());
}

#[test]
fn outcome_name_matches_the_c10_contract() {
    assert_eq!(outcome_name(AuditOutcome::Success), "success");
    assert_eq!(outcome_name(AuditOutcome::Denied), "denied");
    assert_eq!(outcome_name(AuditOutcome::Failed), "failed");
}

#[test]
fn price_source_label_matches_the_ledger_contract() {
    assert_eq!(price_source_label(PriceSource::Venue), "venue");
    assert_eq!(price_source_label(PriceSource::User), "user");
}

#[test]
fn the_system_clock_reads_utc() {
    let before = Utc::now();
    let now = SystemClock.now();
    let after = Utc::now();
    assert!(now >= before && now <= after);
}

#[test]
fn the_identifier_generator_is_unique() {
    let generator = UuidGenerator;
    assert_ne!(generator.next(), generator.next());
}

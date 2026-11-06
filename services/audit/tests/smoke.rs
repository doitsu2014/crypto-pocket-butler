//! Build-wiring smoke test. The real suites (domain, application, repositories,
//! HTTP) live in sibling files.

use audit_application::export::EXPORT_ROW_CAP;

#[test]
fn api_identity_is_stable() {
    assert_eq!(audit::service_name(), "audit-api");
}

#[test]
fn retention_worker_identity_is_stable() {
    assert_eq!(audit::RETENTION_SERVICE_NAME, "audit-retention");
    assert_ne!(audit::service_name(), audit::RETENTION_SERVICE_NAME);
}

#[test]
fn the_migration_set_is_embedded_and_every_version_has_a_down_path() {
    let migrations: Vec<_> = audit_infrastructure::migrations::MIGRATOR.iter().collect();
    assert_eq!(
        migrations.len(),
        2,
        "one reversible migration appears as an up and a down"
    );
    assert!(migrations.iter().all(|migration| migration.version == 1));
}

#[test]
fn the_export_cap_is_one_hundred_thousand_rows() {
    assert_eq!(EXPORT_ROW_CAP, 100_000);
}

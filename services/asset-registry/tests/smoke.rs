//! Build-wiring smoke test. The real suites (domain, application, repositories,
//! HTTP) live in sibling files and are added layer by layer.

#[test]
fn service_identity_is_stable() {
    assert_eq!(asset_registry::service_name(), "asset-registry");
}

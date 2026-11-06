//! Build-wiring smoke test. The real suites (domain, application, JWKS, HTTP)
//! live in sibling files and are added layer by layer.

#[test]
fn service_identity_is_stable() {
    assert_eq!(gateway::service_name(), "gateway");
}

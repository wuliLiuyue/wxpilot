use wxp_proxy::MockRegistry;

#[test]
fn mock_registry_matches_https_default_port_urls() {
    let registry = MockRegistry::default();
    registry.upsert(
        "POST",
        "https://api.example.com/v1/products",
        r#"{"status":200,"body":{"mocked":true}}"#,
    );

    assert!(registry
        .match_url(
            "POST",
            "https://api.example.com:443/v1/products",
        )
        .is_some());
}

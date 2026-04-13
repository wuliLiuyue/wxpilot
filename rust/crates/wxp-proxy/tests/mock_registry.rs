use wxp_proxy::MockRegistry;

#[test]
fn mock_registry_matches_https_default_port_urls() {
    let registry = MockRegistry::default();
    registry.upsert(
        "POST",
        "https://testapi.casamiel.cn/api/v2/WaiMai/Product/GetProductList",
        r#"{"status":200,"body":{"mocked":true}}"#,
    );

    assert!(registry
        .match_url(
            "POST",
            "https://testapi.casamiel.cn:443/api/v2/WaiMai/Product/GetProductList",
        )
        .is_some());
}

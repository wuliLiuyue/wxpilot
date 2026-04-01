use wxp_proxy::MockRegistry;

#[test]
fn exact_url_mock_can_be_added_and_removed() {
    let mut registry = MockRegistry::default();
    registry.upsert("GET", "https://wxpilot.test/api", "{\"ok\":true}");
    assert!(registry
        .match_url("GET", "https://wxpilot.test/api")
        .is_some());
    registry.remove("https://wxpilot.test/api");
    assert!(registry
        .match_url("GET", "https://wxpilot.test/api")
        .is_none());
}

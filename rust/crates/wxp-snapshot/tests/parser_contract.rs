use wxp_snapshot::{find_nodes, parse_wxml};

#[test]
fn find_matches_tag_placeholder_and_class_case_insensitively() {
    let tree = parse_wxml(
        r#"<view><input class="PhoneInput" placeholder="手机号" /></view>"#,
        None,
    );
    let result = find_nodes(&tree, "phone", true, 10);
    assert_eq!(result.total_count, 1);
    assert_eq!(result.matches[0].placeholder.as_deref(), Some("手机号"));
}

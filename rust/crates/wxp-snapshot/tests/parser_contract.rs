use wxp_snapshot::{find_nodes, parse_wxml, render_find_matches, render_tree};

#[test]
fn parse_render_and_find_match_ts_behavior() {
    let tree = parse_wxml(
        r#"<page><view class="card"><input class="PhoneInput" placeholder="手机号" /></view></page>"#,
        20,
        None,
    );
    let rendered = render_tree(&tree, true, 5, Some(50), 30);
    assert!(rendered.output.contains("placeholder=\"手机号\""));

    let result = find_nodes(&tree, "phone", true, Some(10));
    assert_eq!(result.total_count, 1);
    assert_eq!(result.matches[0].placeholder.as_deref(), Some("手机号"));
    let summary = render_find_matches(&result.matches, 30);
    assert!(summary.contains("%1 [input].PhoneInput"));
}

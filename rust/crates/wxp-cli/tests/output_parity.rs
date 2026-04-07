use serde_json::json;
use wxp_cli::error::CliError;
use wxp_cli::output::{render_error, render_ok, render_ref_expired, OutputMode};

#[test]
fn renders_summary_with_hint_in_human_mode() {
    let rendered = render_ok(
        &json!({
            "summary": {
                "title": "string(4)"
            },
            "hint": "use --full or specify a path to get full data"
        }),
        OutputMode::Human,
    );

    assert_eq!(
        rendered,
        "{\n  \"title\": \"string(4)\"\n}\n[use --full or specify a path to get full data]"
    );
}

#[test]
fn renders_json_error_envelope() {
    let rendered = render_error("ref_expired", Some(json!({ "ref": 7 })), OutputMode::Json);
    assert_eq!(rendered, r#"{"ok":false,"error":"ref_expired","ref":7}"#);
}

#[test]
fn truncates_human_output_at_4000_chars() {
    let payload = "x".repeat(4005);
    let rendered = render_ok(&json!({ "output": payload }), OutputMode::Human);

    assert!(rendered.contains("[truncated, originalLength=4005]"));
    assert_eq!(rendered.matches('x').count(), 4000);
}

#[test]
fn renders_tap_branch_in_human_mode() {
    let rendered = render_ok(
        &json!({
            "action": "tap",
            "ref": 7
        }),
        OutputMode::Human,
    );

    assert_eq!(rendered, "OK: tap %7");
}

#[test]
fn renders_generic_human_output() {
    let rendered = render_ok(
        &json!({
            "count": 2,
            "foo": "bar"
        }),
        OutputMode::Human,
    );

    assert_eq!(rendered, r#"OK: count=2 foo="bar""#);
}

#[test]
fn cli_error_exit_codes_match_minimal_contract() {
    assert_eq!(CliError::UnsupportedDaemonKind.exit_code(), 2);
    assert_eq!(CliError::Message("boom".into()).exit_code(), 1);
    assert_eq!(
        CliError::DaemonUnavailable("daemon unavailable".into()).exit_code(),
        1
    );
}

#[test]
fn renders_ref_expired_human_message() {
    let rendered = render_ref_expired(7, OutputMode::Human);
    assert_eq!(rendered, "ERR: %7 expired, run 'wxpilot view' to refresh");
}

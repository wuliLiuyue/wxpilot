use chrono::{TimeZone, Utc};
use serde_json::json;
use wxp_cli::command::CliCommand;
use wxp_cli::error::CliError;
use wxp_cli::output::{render_command_ok, render_error, render_ok, render_ref_expired, OutputMode};

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

#[test]
fn renders_net_start_human_output_like_ts_cli() {
    let rendered = render_command_ok(
        &CliCommand::NetStart {
            port: 8899,
            https: true,
            timeout_ms: 30_000,
        },
        &json!({
            "ok": true,
            "port": 8899,
            "https": true,
            "caPath": "/tmp/cert.pem"
        }),
        OutputMode::Human,
    );

    assert_eq!(
        rendered,
        "OK: proxy listening on 127.0.0.1:8899 (HTTPS enabled)\n请在微信开发者工具设置代理 → 127.0.0.1:8899\nCA 证书已生成: /tmp/cert.pem"
    );
}

#[test]
fn renders_net_install_ca_human_output_like_ts_cli() {
    let rendered = render_command_ok(
        &CliCommand::NetInstallCa,
        &json!({
            "ok": true,
            "certFile": "/tmp/cert.pem"
        }),
        OutputMode::Human,
    );

    assert_eq!(
        rendered,
        "OK: CA 证书已安装到系统钥匙串\ncert: /tmp/cert.pem\n请完全重启微信开发者工具后生效"
    );
}

#[test]
fn renders_activate_human_output() {
    let rendered = render_command_ok(
        &CliCommand::Activate {
            key: "lic_123".into(),
        },
        &json!({
            "ok": true,
            "licenseId": "lic_123",
            "machineFingerprint": "abc123",
            "path": "/tmp/license.json"
        }),
        OutputMode::Human,
    );

    assert_eq!(
        rendered,
        "OK: 激活成功\nlicense: lic_123\nmachine: abc123\npath: /tmp/license.json"
    );
}

#[test]
fn renders_net_log_human_output_like_ts_cli() {
    let ts = Utc
        .with_ymd_and_hms(2026, 4, 8, 3, 0, 0)
        .single()
        .expect("datetime")
        .timestamp_millis();
    let rendered = render_command_ok(
        &CliCommand::NetLog {
            filter: None,
            limit: 20,
            with_body: true,
            json: false,
        },
        &json!({
            "ok": true,
            "logs": [
                {
                    "ts": ts,
                    "method": "GET",
                    "url": "https://example.com/api/orders",
                    "status": 200,
                    "mocked": 1,
                    "duration": 12,
                    "req_body": "{\"page\":1}",
                    "res_body": "{\"ok\":true}"
                }
            ],
            "truncated": true,
            "shownCount": 1,
            "totalCount": 3
        }),
        OutputMode::Human,
    );

    assert_eq!(
        rendered,
        "[显示 1/3 条，使用 --limit 调整]\n2026-04-08T03:00:00.000Z GET https://example.com/api/orders 200 [MOCK] 12ms\n  req: {\"page\":1}\n  res: {\"ok\":true}"
    );
}

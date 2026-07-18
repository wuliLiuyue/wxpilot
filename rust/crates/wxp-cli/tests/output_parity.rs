// SPDX-License-Identifier: AGPL-3.0-or-later
// wxpilot - 面向 AI Agent 的微信小程序自动化 CLI
// Copyright (C) 2026  wuliLiuyue
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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

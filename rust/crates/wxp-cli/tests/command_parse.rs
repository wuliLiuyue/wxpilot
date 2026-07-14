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

use clap::Parser;
use wxp_cli::args::Cli;
use wxp_cli::command::CliCommand;

#[test]
fn parses_start_command_with_existing_flags() {
    let cli = Cli::try_parse_from([
        "wxpilot",
        "--timeout",
        "12345",
        "start",
        "/tmp/demo/dist",
        "--cli-path",
        "/Applications/Wechat Devtools.app/Contents/MacOS/cli",
        "--appid",
        "wx123",
        "--auto-port",
        "9423",
    ])
    .expect("parse");

    let command = CliCommand::try_from(cli).expect("normalize");
    assert_eq!(
        command,
        CliCommand::Start {
            project_path: Some("/tmp/demo/dist".into()),
            cli_path: Some("/Applications/Wechat Devtools.app/Contents/MacOS/cli".into()),
            appid: Some("wx123".into()),
            auto_port: 9423,
            proxy: false,
            proxy_https: false,
            timeout_ms: 35000,
        }
    );
}

#[test]
fn parses_start_command_with_proxy_flags() {
    let cli = Cli::try_parse_from(["wxpilot", "start", "/tmp/demo/dist", "--proxy", "--https"])
        .expect("parse");

    let command = CliCommand::try_from(cli).expect("normalize");
    assert_eq!(
        command,
        CliCommand::Start {
            project_path: Some("/tmp/demo/dist".into()),
            cli_path: None,
            appid: None,
            auto_port: 9420,
            proxy: true,
            proxy_https: true,
            timeout_ms: 35000,
        }
    );
}

#[test]
fn parses_connect_command_with_command_level_timeout() {
    let cli = Cli::try_parse_from([
        "wxpilot",
        "--timeout",
        "999",
        "connect",
        "--ws",
        "ws://127.0.0.1:9420",
    ])
    .expect("parse");

    let command = CliCommand::try_from(cli).expect("normalize");
    assert_eq!(
        command,
        CliCommand::Connect {
            ws_endpoint: Some("ws://127.0.0.1:9420".into()),
            timeout_ms: 30000,
        }
    );
}

#[test]
fn parses_net_start_and_stop_with_command_level_timeouts() {
    let start = Cli::try_parse_from(["wxpilot", "net", "start", "--port", "8899", "--https"])
        .expect("parse");
    let start_command = CliCommand::try_from(start).expect("normalize");
    assert_eq!(
        start_command,
        CliCommand::NetStart {
            port: 8899,
            https: true,
            timeout_ms: 30000,
        }
    );

    let stop = Cli::try_parse_from(["wxpilot", "net", "stop"]).expect("parse");
    let stop_command = CliCommand::try_from(stop).expect("normalize");
    assert_eq!(stop_command, CliCommand::NetStop { timeout_ms: 15000 });
}

#[test]
fn parses_nested_net_log_flags() {
    let cli = Cli::try_parse_from([
        "wxpilot",
        "--json",
        "net",
        "log",
        "--filter",
        "/order/list",
        "--limit",
        "5",
        "--with-body",
    ])
    .expect("parse");

    let command = CliCommand::try_from(cli).expect("normalize");
    assert_eq!(
        command,
        CliCommand::NetLog {
            filter: Some("/order/list".into()),
            limit: 5,
            with_body: true,
            json: true,
        }
    );
}

#[test]
fn preserves_view_compact_tristate() {
    let cli = Cli::try_parse_from(["wxpilot", "view"]).expect("parse");
    let command = CliCommand::try_from(cli).expect("normalize");
    assert_eq!(
        command,
        CliCommand::View {
            full: false,
            compact: None,
            depth: None,
            limit: None,
        }
    );

    let compact_cli = Cli::try_parse_from(["wxpilot", "view", "--compact"]).expect("parse");
    let compact_command = CliCommand::try_from(compact_cli).expect("normalize");
    assert_eq!(
        compact_command,
        CliCommand::View {
            full: false,
            compact: Some(true),
            depth: None,
            limit: None,
        }
    );
}

#[test]
fn parses_state_shot_daemon_restart_and_wx_smoke_cases() {
    let state = Cli::try_parse_from(["wxpilot", "state", "cart.items"]).expect("parse");
    assert_eq!(
        CliCommand::try_from(state).expect("normalize"),
        CliCommand::State {
            path: Some("cart.items".into()),
            full: false,
        }
    );

    let shot = Cli::try_parse_from(["wxpilot", "shot", "/tmp/result.png"]).expect("parse");
    assert_eq!(
        CliCommand::try_from(shot).expect("normalize"),
        CliCommand::Shot {
            path: Some("/tmp/result.png".into()),
            base64: false,
        }
    );

    let daemon = Cli::try_parse_from(["wxpilot", "daemon", "restart"]).expect("parse");
    assert_eq!(
        CliCommand::try_from(daemon).expect("normalize"),
        CliCommand::DaemonRestart
    );

    let wx = Cli::try_parse_from(["wxpilot", "wx", "setStorageSync", "\"k\"", "123", "true"])
        .expect("parse");
    assert_eq!(
        CliCommand::try_from(wx).expect("normalize"),
        CliCommand::Wx {
            method: "setStorageSync".into(),
            args: vec!["\"k\"".into(), "123".into(), "true".into()],
        }
    );
}

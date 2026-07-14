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

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use wxp_daemon::server::{dispatch_request, RuntimePaths, ServerState};

fn unique_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("wxp-net-mock-method-{name}-{nanos}"))
}

fn test_paths(name: &str) -> RuntimePaths {
    let root = unique_test_dir(name);
    fs::create_dir_all(&root).expect("create temp root");
    RuntimePaths {
        root_dir: root.clone(),
        rust_socket: root.join("rust-daemon.sock"),
        rust_pid: root.join("rust-daemon.pid"),
        state_file: root.join("rust-daemon-state.json"),
        db_file: root.join("rust-daemon.db"),
        ca_dir: root.join("ca"),
        log_dir: root.join("logs"),
    }
}

#[tokio::test]
async fn net_mock_registers_explicit_post_method() {
    let mut state = ServerState::new(test_paths("net-mock-method")).expect("server state");

    let mock = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "net.mock",
            "params": {
                "method": "POST",
                "url": "https://wxpilot.test/api",
                "body": {"ok": true}
            },
        }),
    )
    .await
    .expect("net mock");

    assert_eq!(mock["result"]["ok"], true);
    assert!(state
        .runtime
        .mock_registry
        .match_url("POST", "https://wxpilot.test/api")
        .is_some());
    assert!(state
        .runtime
        .mock_registry
        .match_url("GET", "https://wxpilot.test/api")
        .is_none());
}

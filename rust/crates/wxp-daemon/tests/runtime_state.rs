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
    io::{self, BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use wxp_daemon::runtime::{DaemonState, StateStore};
use wxp_daemon::server::{
    dispatch_local_request, prepare_rust_socket, serve_forever, RuntimePaths, ServerState,
};

fn unique_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("wxp-daemon-{name}-{nanos}"))
}

fn short_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("wxp-{name}-{nanos}"))
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

fn short_test_paths(name: &str) -> RuntimePaths {
    let root = short_test_dir(name);
    fs::create_dir_all(&root).expect("create short temp root");
    RuntimePaths {
        root_dir: root.clone(),
        rust_socket: root.join("s.sock"),
        rust_pid: root.join("s.pid"),
        state_file: root.join("s.json"),
        db_file: root.join("s.db"),
        ca_dir: root.join("ca"),
        log_dir: root.join("logs"),
    }
}

fn wait_for_socket(socket_path: &PathBuf) {
    for _ in 0..50 {
        if socket_path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("socket was not created in time: {}", socket_path.display());
}

fn call_socket(socket_path: &PathBuf, request: serde_json::Value) -> io::Result<serde_json::Value> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.set_read_timeout(Some(Duration::from_millis(300)))?;
    stream.write_all(format!("{request}\n").as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str(line.trim()).map_err(io::Error::other)
}

#[test]
fn state_store_round_trips_project_and_endpoint() {
    let paths = test_paths("state-store");
    let store = StateStore::new(paths.state_file.clone());
    let state = DaemonState {
        project_path: Some("/tmp/project".into()),
        ws_endpoint: Some("ws://127.0.0.1:9420".into()),
        connected: true,
    };

    store.save(&state).expect("save state");
    let loaded = store.load().expect("load state");

    assert_eq!(loaded.project_path, state.project_path);
    assert_eq!(loaded.ws_endpoint, state.ws_endpoint);
    assert!(loaded.connected);
}

#[test]
fn local_status_reports_rust_daemon_without_saved_connection() {
    let state = ServerState::new(test_paths("status")).expect("server state");

    let response = dispatch_local_request(
        &state,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "status",
            "params": {},
        }),
    )
    .expect("dispatch status");

    assert_eq!(response["result"]["ok"], true);
    assert_eq!(response["result"]["connected"], false);
    assert_eq!(response["result"]["projectPath"], serde_json::Value::Null);
    assert_eq!(response["result"]["proxy"], serde_json::Value::Null);
    assert_eq!(response["result"]["daemonKind"], "rust");
}

#[test]
fn state_store_loads_partial_state_files_with_defaults() {
    let paths = test_paths("partial-state");
    fs::write(&paths.state_file, r#"{"connected":true}"#).expect("write partial state");
    let store = StateStore::new(paths.state_file.clone());

    let loaded = store.load().expect("load partial state");

    assert_eq!(loaded.project_path, None);
    assert_eq!(loaded.ws_endpoint, None);
    assert!(loaded.connected);
}

#[test]
fn active_socket_is_not_unlinked_during_startup_preparation() {
    let root = short_test_dir("sock");
    fs::create_dir_all(&root).expect("create short temp root");
    let paths = RuntimePaths {
        root_dir: root.clone(),
        rust_socket: root.join("s.sock"),
        rust_pid: root.join("s.pid"),
        state_file: root.join("s.json"),
        db_file: root.join("s.db"),
        ca_dir: root.join("ca"),
        log_dir: root.join("logs"),
    };
    let _listener = UnixListener::bind(&paths.rust_socket).expect("bind active socket");

    let error = prepare_rust_socket(&paths.rust_socket).expect_err("should reject active socket");

    assert!(matches!(
        error.kind(),
        io::ErrorKind::AddrInUse | io::ErrorKind::AlreadyExists
    ));
    assert!(
        paths.rust_socket.exists(),
        "active socket path should remain in place"
    );
}

#[test]
fn default_runtime_paths_match_cli_probe_locations() {
    let home = std::env::var("HOME").expect("home env");
    let paths = RuntimePaths::default();

    assert_eq!(
        paths.rust_socket,
        PathBuf::from(&home).join(".wxpilot/rust-daemon.sock")
    );
    assert_eq!(
        paths.rust_pid,
        PathBuf::from(&home).join(".wxpilot/rust-daemon.pid")
    );
    assert_eq!(
        paths.state_file,
        PathBuf::from(&home).join(".wxpilot/rust-daemon-state.json")
    );
    assert_eq!(
        paths.db_file,
        PathBuf::from(&home).join(".wxpilot/rust-daemon.db")
    );
}

#[test]
fn uds_server_handles_probe_status_and_stop_requests() {
    let paths = short_test_paths("uds");
    let server_paths = paths.clone();
    let handle = thread::spawn(move || {
        let state = ServerState::new(server_paths).expect("server state");
        serve_forever(state)
    });

    wait_for_socket(&paths.rust_socket);

    let health = call_socket(
        &paths.rust_socket,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "health",
            "params": {},
        }),
    )
    .expect("health response");
    assert_eq!(health["result"]["status"], "ok");
    assert_eq!(health["result"]["daemonKind"], "rust");

    let status = call_socket(
        &paths.rust_socket,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "status",
            "params": {},
        }),
    )
    .expect("status response");
    assert_eq!(status["result"]["ok"], true);
    assert_eq!(status["result"]["connected"], false);
    assert_eq!(status["result"]["daemonKind"], "rust");

    let stop = call_socket(
        &paths.rust_socket,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "daemon.stop",
            "params": {},
        }),
    )
    .expect("daemon stop response");
    assert_eq!(stop["result"]["ok"], true);

    handle.join().expect("server thread").expect("server exit");
    assert!(
        !paths.rust_socket.exists(),
        "socket should be removed on exit"
    );
    assert!(!paths.rust_pid.exists(), "pid should be removed on exit");
}

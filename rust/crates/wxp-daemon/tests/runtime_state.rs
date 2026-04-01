use std::{
    fs, io,
    os::unix::net::UnixListener,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use wxp_daemon::runtime::{DaemonState, StateStore};
use wxp_daemon::server::{dispatch_local_request, prepare_rust_socket, RuntimePaths, ServerState};

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
    let state = ServerState::new(test_paths("status"));

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

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

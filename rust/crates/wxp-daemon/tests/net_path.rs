use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use wxp_daemon::server::{dispatch_request, RuntimePaths, ServerState};
use wxp_store::RequestLog;

fn unique_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("wxp-net-path-{name}-{nanos}"))
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
async fn net_mock_log_and_clear_requests_work() {
    let mut state = ServerState::new(test_paths("net"));
    let oversized = "x".repeat(2100);
    state
        .runtime
        .request_store
        .insert_request(RequestLog::new(
            "GET",
            "https://wxpilot.test/api",
            Some(oversized),
            None,
        ))
        .expect("insert request log");

    let mock = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "net.mock",
            "params": {
                "method": "GET",
                "url": "https://wxpilot.test/api",
                "body": { "ok": true }
            },
        }),
    )
    .await
    .expect("net mock");
    assert_eq!(mock["result"]["ok"], true);
    assert!(state
        .runtime
        .mock_registry
        .match_url("GET", "https://wxpilot.test/api")
        .is_some());

    let logs = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "net.log",
            "params": {
                "limit": 20,
                "withBody": true
            },
        }),
    )
    .await
    .expect("net log");
    assert_eq!(logs["result"]["ok"], true);
    assert_eq!(
        logs["result"]["logs"][0]["reqBody"]
            .as_str()
            .expect("req body")
            .len(),
        2001
    );

    let unmock = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "net.unmock",
            "params": {
                "url": "https://wxpilot.test/api"
            },
        }),
    )
    .await
    .expect("net unmock");
    assert_eq!(unmock["result"]["ok"], true);
    assert!(state
        .runtime
        .mock_registry
        .match_url("GET", "https://wxpilot.test/api")
        .is_none());

    let cleared = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "net.clear",
            "params": {},
        }),
    )
    .await
    .expect("net clear");
    assert_eq!(cleared["result"]["ok"], true);

    let logs_after_clear = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "net.log",
            "params": {},
        }),
    )
    .await
    .expect("net log after clear");
    assert_eq!(
        logs_after_clear["result"]["logs"].as_array().unwrap().len(),
        0
    );
}

#[tokio::test]
async fn net_start_and_stop_manage_proxy_runtime() {
    let mut state = ServerState::new(test_paths("net-start-stop"));

    let started = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "net.start",
            "params": {
                "port": 0
            },
        }),
    )
    .await
    .expect("net start");
    assert_eq!(started["result"]["ok"], true);
    assert!(
        started["result"]["port"].as_u64().expect("proxy port") > 0,
        "net.start should report bound proxy port"
    );
    assert!(state.runtime.proxy.is_running());

    let stopped = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "net.stop",
            "params": {},
        }),
    )
    .await
    .expect("net stop");
    assert_eq!(stopped["result"]["ok"], true);
    assert!(!state.runtime.proxy.is_running());
}

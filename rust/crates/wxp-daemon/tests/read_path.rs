use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use wxp_daemon::server::{dispatch_local_request, dispatch_request, RuntimePaths, ServerState};
use wxp_daemon::DevtoolsLauncher;

fn unique_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("wxp-read-path-{name}-{nanos}"))
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
async fn start_connect_status_and_state_work_on_rust_daemon() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        while let Some(message) = ws.next().await {
            let payload = message.expect("message").into_text().expect("text");
            let request: Value = serde_json::from_str(&payload).expect("request json");
            let id = request.get("id").cloned().unwrap_or(Value::Null);
            let method = request
                .get("method")
                .and_then(Value::as_str)
                .expect("request method");

            let response = match method {
                "App.getCurrentPage" => {
                    json!({ "id": id, "result": { "pageId": 1, "path": "pages/index/index", "query": {} } })
                }
                "Page.getData" => {
                    json!({ "id": id, "result": { "data": { "title": "首页", "count": 3 } } })
                }
                _ => json!({ "id": id, "result": {} }),
            };

            ws.send(Message::Text(response.to_string().into()))
                .await
                .expect("send");
        }
    });

    let paths = test_paths("mainline");
    let mut state = ServerState::new(paths.clone());
    state.launcher = DevtoolsLauncher::new(vec![]).with_run_auto_hook(move |request| {
        assert_eq!(request.project_path, "/tmp/project");
        assert_eq!(request.port, 9420);
        assert_eq!(request.appid.as_deref(), Some("wx1234567890"));
        Ok(format!("ws://{addr}"))
    });

    let start = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "start",
            "params": {
                "projectPath": "/tmp/project",
                "port": 9420,
                "appid": "wx1234567890"
            },
        }),
    )
    .await
    .expect("start");
    assert_eq!(start["result"]["ok"], true);
    assert_eq!(start["result"]["wsEndpoint"], format!("ws://{addr}"));
    assert_eq!(start["result"]["port"], 9420);

    let saved_after_start = fs::read_to_string(&paths.state_file).expect("state file after start");
    let saved_after_start: Value =
        serde_json::from_str(&saved_after_start).expect("parse state after start");
    assert_eq!(saved_after_start["project_path"], "/tmp/project");
    assert_eq!(saved_after_start["ws_endpoint"], format!("ws://{addr}"));
    assert_eq!(saved_after_start["connected"], false);

    let connect = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "connect",
            "params": {},
        }),
    )
    .await
    .expect("connect");
    assert_eq!(connect["result"]["ok"], true);
    assert_eq!(connect["result"]["wsEndpoint"], format!("ws://{addr}"));

    let status = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "status",
            "params": {},
        }),
    )
    .await
    .expect("status");
    assert_eq!(status["result"]["ok"], true);
    assert_eq!(status["result"]["connected"], true);
    assert_eq!(status["result"]["projectPath"], "/tmp/project");
    assert_eq!(status["result"]["daemonKind"], "rust");

    let saved_after_connect =
        fs::read_to_string(&paths.state_file).expect("state file after connect");
    let saved_after_connect: Value =
        serde_json::from_str(&saved_after_connect).expect("parse state after connect");
    assert_eq!(saved_after_connect["connected"], true);

    let state_result = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "state",
            "params": { "full": true },
        }),
    )
    .await
    .expect("state");
    assert_eq!(state_result["result"]["ok"], true);
    assert_eq!(state_result["result"]["data"]["title"], "首页");
    assert_eq!(state_result["result"]["data"]["count"], 3);
}

#[test]
fn status_does_not_report_connected_after_restart_without_live_connection() {
    let paths = test_paths("restart-status");
    fs::write(
        &paths.state_file,
        r#"{"project_path":"/tmp/project","ws_endpoint":"ws://127.0.0.1:9420","connected":true}"#,
    )
    .expect("write state file");

    let state = ServerState::new(paths);
    let status = dispatch_local_request(
        &state,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "status",
            "params": {},
        }),
    )
    .expect("status");

    assert_eq!(status["result"]["connected"], false);
    assert_eq!(status["result"]["projectPath"], "/tmp/project");
}

#[tokio::test]
async fn state_failure_clears_connected_status_after_socket_closes() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let _ws = accept_async(stream).await.expect("ws accept");
    });

    let paths = test_paths("disconnect-status");
    let mut state = ServerState::new(paths.clone());
    state
        .runtime
        .set_ws_endpoint_for_test(format!("ws://{addr}"));

    let connect = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "connect",
            "params": {},
        }),
    )
    .await
    .expect("connect");
    assert_eq!(connect["result"]["ok"], true);

    let error = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "state",
            "params": {},
        }),
    )
    .await
    .expect_err("state should fail after connection closes");
    assert_eq!(error.kind(), std::io::ErrorKind::Other);

    let status = dispatch_local_request(
        &state,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "status",
            "params": {},
        }),
    )
    .expect("status");
    assert_eq!(status["result"]["connected"], false);

    let saved = fs::read_to_string(&paths.state_file).expect("read state file");
    let saved: Value = serde_json::from_str(&saved).expect("parse state file");
    assert_eq!(saved["connected"], false);
}

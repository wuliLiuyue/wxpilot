use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
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
async fn start_connect_status_and_state_supports_summary_path_and_full_contract() {
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
                    json!({
                        "id": id,
                        "result": {
                            "data": {
                                "title": "首页标题",
                                "count": 3,
                                "cardMap": {
                                    "vipCardPrice": "1,234,367.73"
                                }
                            }
                        }
                    })
                }
                _ => json!({ "id": id, "result": {} }),
            };

            ws.send(Message::Text(response.to_string().into()))
                .await
                .expect("send");
        }
    });

    let paths = test_paths("mainline");
    let mut state = ServerState::new(paths.clone()).expect("server state");
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

    let state_summary = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "state",
            "params": {},
        }),
    )
    .await
    .expect("state summary");
    assert_eq!(state_summary["result"]["ok"], true);
    assert_eq!(state_summary["result"]["summary"]["title"], "string(6)");
    assert_eq!(state_summary["result"]["summary"]["count"], "number(1)");
    assert_eq!(state_summary["result"]["summary"]["cardMap"], "object(31)");
    assert_eq!(
        state_summary["result"]["hint"],
        "use --full or specify a path to get full data"
    );
    assert!(state_summary["result"].get("data").is_none());

    let state_path = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "state",
            "params": { "path": "cardMap.vipCardPrice" },
        }),
    )
    .await
    .expect("state path");
    assert_eq!(state_path["result"]["ok"], true);
    assert_eq!(state_path["result"]["data"], "1,234,367.73");

    let state_full = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "state",
            "params": { "full": true },
        }),
    )
    .await
    .expect("state full");
    assert_eq!(state_full["result"]["ok"], true);
    assert_eq!(state_full["result"]["data"]["title"], "首页标题");
    assert_eq!(state_full["result"]["data"]["count"], 3);
}

#[tokio::test]
async fn state_defaults_to_data_when_top_level_is_not_object() {
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
                    json!({
                        "id": id,
                        "result": {
                            "data": "ready"
                        }
                    })
                }
                _ => json!({ "id": id, "result": {} }),
            };

            ws.send(Message::Text(response.to_string().into()))
                .await
                .expect("send");
        }
    });

    let paths = test_paths("state-non-object");
    let mut state = ServerState::new(paths).expect("server state");
    state
        .runtime
        .set_ws_endpoint_for_test(format!("ws://{addr}"));

    dispatch_request(
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

    let state_result = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "state",
            "params": {},
        }),
    )
    .await
    .expect("state");

    assert_eq!(state_result["result"]["ok"], true);
    assert_eq!(state_result["result"]["data"], "ready");
    assert!(state_result["result"].get("summary").is_none());
    assert!(state_result["result"].get("hint").is_none());
}

#[tokio::test]
async fn start_retries_with_proxy_after_failed_to_fetch_error() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_hook = attempts.clone();

    let paths = test_paths("start-proxy-retry");
    let mut state = ServerState::new(paths).expect("server state");
    state.default_proxy_port = 0;
    state.launcher = DevtoolsLauncher::new(vec![]).with_run_auto_hook(move |_request| {
        let attempt = attempts_for_hook.fetch_add(1, Ordering::SeqCst);
        if attempt == 0 {
            return Err(std::io::Error::other(
                "cli auto failed: Error: Failed to fetch via proxy 8899",
            ));
        }
        Ok("ws://127.0.0.1:9420".to_string())
    });

    let start = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 40,
            "method": "start",
            "params": {
                "projectPath": "/tmp/project",
                "port": 9420
            },
        }),
    )
    .await
    .expect("start should retry after starting proxy");

    assert_eq!(start["result"]["ok"], true);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert!(state.runtime.proxy.is_running());
    assert!(
        state.runtime.proxy.port().expect("proxy port") > 0,
        "proxy retry should bind a runtime port"
    );
}


#[test]
fn status_does_not_report_connected_after_restart_without_live_connection() {
    let paths = test_paths("restart-status");
    fs::write(
        &paths.state_file,
        r#"{"project_path":"/tmp/project","ws_endpoint":"ws://127.0.0.1:9420","connected":true}"#,
    )
    .expect("write state file");

    let state = ServerState::new(paths).expect("server state");
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
        let mut ws = accept_async(stream).await.expect("ws accept");
        if let Some(message) = ws.next().await {
            let payload = message.expect("message").into_text().expect("text");
            let request: Value = serde_json::from_str(&payload).expect("request json");
            let id = request.get("id").cloned().unwrap_or(Value::Null);
            assert_eq!(request["method"], "App.getCurrentPage");
            ws.send(Message::Text(
                json!({
                    "id": id,
                    "result": { "pageId": 1, "path": "pages/index/index", "query": {} }
                })
                .to_string()
                .into(),
            ))
            .await
            .expect("send");
        }
    });

    let paths = test_paths("disconnect-status");
    let mut state = ServerState::new(paths.clone()).expect("server state");
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

#[tokio::test]
async fn disconnect_clears_runtime_connection_and_status() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        if let Some(message) = ws.next().await {
            let payload = message.expect("message").into_text().expect("text");
            let request: Value = serde_json::from_str(&payload).expect("request json");
            let id = request.get("id").cloned().unwrap_or(Value::Null);
            assert_eq!(request["method"], "App.getCurrentPage");
            ws.send(Message::Text(
                json!({
                    "id": id,
                    "result": { "pageId": 1, "path": "pages/index/index", "query": {} }
                })
                .to_string()
                .into(),
            ))
            .await
            .expect("send");
        }
    });

    let paths = test_paths("disconnect-rpc");
    let mut state = ServerState::new(paths.clone()).expect("server state");
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

    let disconnected = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "disconnect",
            "params": {},
        }),
    )
    .await
    .expect("disconnect");
    assert_eq!(disconnected["result"]["ok"], true);

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

#[tokio::test]
async fn connect_retries_until_websocket_becomes_available() {
    let reserved = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = reserved.local_addr().expect("addr");
    drop(reserved);

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let listener = TcpListener::bind(addr).await.expect("rebind");
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        if let Some(message) = ws.next().await {
            let payload = message.expect("message").into_text().expect("text");
            let request: Value = serde_json::from_str(&payload).expect("request json");
            let id = request.get("id").cloned().unwrap_or(Value::Null);
            assert_eq!(request["method"], "App.getCurrentPage");
            ws.send(Message::Text(
                json!({
                    "id": id,
                    "result": { "pageId": 1, "path": "pages/index/index", "query": {} }
                })
                .to_string()
                .into(),
            ))
            .await
            .expect("send");
        }
    });

    let paths = test_paths("connect-retry");
    let mut state = ServerState::new(paths.clone()).expect("server state");
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
    .expect("connect should retry until websocket is ready");

    assert_eq!(connect["result"]["ok"], true);
    assert_eq!(connect["result"]["wsEndpoint"], format!("ws://{addr}"));
}

#[tokio::test]
async fn connect_waits_until_current_page_probe_succeeds() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let probes = Arc::new(AtomicUsize::new(0));
    let probes_for_server = probes.clone();

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
                    let attempt = probes_for_server.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        json!({
                            "id": id,
                            "error": {
                                "message": "not_ready_yet"
                            }
                        })
                    } else {
                        json!({
                            "id": id,
                            "result": { "pageId": 1, "path": "pages/index/index", "query": {} }
                        })
                    }
                }
                _ => json!({ "id": id, "result": {} }),
            };

            ws.send(Message::Text(response.to_string().into()))
                .await
                .expect("send");
        }
    });

    let paths = test_paths("connect-probe");
    let mut state = ServerState::new(paths).expect("server state");
    state
        .runtime
        .set_ws_endpoint_for_test(format!("ws://{addr}"));

    let connect = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "connect",
            "params": {},
        }),
    )
    .await
    .expect("connect should wait for current page probe");

    assert_eq!(connect["result"]["ok"], true);
    assert!(
        probes.load(Ordering::SeqCst) >= 3,
        "connect should keep probing current page until success"
    );
}

#[tokio::test]
async fn view_find_wait_read_and_assert_work_on_snapshot_refs() {
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
                "Page.getElement" => {
                    json!({
                        "id": id,
                        "result": {
                            "elementId": "root-1",
                            "nodeId": "node-root-1",
                            "tagName": "body"
                        }
                    })
                }
                "Element.getWXML" => {
                    assert_eq!(request["params"]["nodeId"], "node-root-1");
                    json!({
                        "id": id,
                        "result": {
                            "wxml": r#"<page><view class="card"><button class="ConfirmBtn">确定</button><input class="PhoneInput" placeholder="手机号" /></view></page>"#
                        }
                    })
                }
                _ => json!({ "id": id, "result": {} }),
            };

            ws.send(Message::Text(response.to_string().into()))
                .await
                .expect("send");
        }
    });

    let paths = test_paths("snapshot-refs");
    let mut state = ServerState::new(paths).expect("server state");
    state
        .runtime
        .set_ws_endpoint_for_test(format!("ws://{addr}"));

    dispatch_request(
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

    let view = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "view",
            "params": {},
        }),
    )
    .await
    .expect("view");
    assert_eq!(view["result"]["ok"], true);
    assert_eq!(view["result"]["pagePath"], "pages/index/index");
    assert!(view["result"]["output"]
        .as_str()
        .expect("view output")
        .contains("%1"));

    let find = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "find",
            "params": {
                "query": "confirm"
            },
        }),
    )
    .await
    .expect("find");
    assert_eq!(find["result"]["ok"], true);
    assert_eq!(find["result"]["pagePath"], "pages/index/index");
    assert_eq!(find["result"]["matches"][0]["ref"], 1);
    assert!(find["result"]["output"]
        .as_str()
        .expect("find output")
        .contains("查找: \"confirm\""));

    let wait = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "wait",
            "params": {
                "ref": 1
            },
        }),
    )
    .await
    .expect("wait");
    assert_eq!(wait["result"]["ok"], true);
    assert_eq!(wait["result"]["ref"], 1);

    let read = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "read",
            "params": {
                "ref": 1
            },
        }),
    )
    .await
    .expect("read");
    assert_eq!(read["result"]["ok"], true);
    assert_eq!(read["result"]["text"], "确定");

    let assert_result = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "assert",
            "params": {
                "ref": 1,
                "expected": "确定"
            },
        }),
    )
    .await
    .expect("assert");
    assert_eq!(assert_result["result"]["ok"], true);
    assert_eq!(assert_result["result"]["actual"], "确定");
}

use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use wxp_daemon::server::{dispatch_request, RuntimePaths, ServerState};

fn unique_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("wxp-action-path-{name}-{nanos}"))
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
async fn route_and_element_actions_forward_to_automator() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let seen_payloads = Arc::new(Mutex::new(Vec::<String>::new()));
    let server_payloads = seen_payloads.clone();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        while let Some(message) = ws.next().await {
            let payload = message.expect("message").into_text().expect("text");
            server_payloads
                .lock()
                .expect("lock payloads")
                .push(payload.to_string());
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
                            "elementId": "element-1",
                            "nodeId": "node-element-1",
                            "tagName": "button"
                        }
                    })
                }
                "Element.getWXML" => {
                    assert_eq!(request["params"]["nodeId"], "node-element-1");
                    json!({
                        "id": id,
                        "result": {
                            "outerWxml": r#"<page><view class="card"><button class="ConfirmBtn">确定</button><input class="PhoneInput" placeholder="手机号" /></view></page>"#
                        }
                    })
                }
                "Element.tap" | "Element.callFunction" => {
                    assert_eq!(request["params"]["nodeId"], "node-element-1");
                    json!({ "id": id, "result": { "ok": true } })
                }
                _ => json!({ "id": id, "result": { "ok": true } }),
            };

            ws.send(Message::Text(response.to_string().into()))
                .await
                .expect("send");
        }
    });

    let mut state = ServerState::new(test_paths("actions")).expect("server state");
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

    dispatch_request(
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

    for (id, method, params) in [
        (3, "tap", json!({ "ref": 1 })),
        (4, "type", json!({ "ref": 2, "text": "13800138000" })),
        (5, "scroll", json!({ "ref": 2, "direction": "down" })),
        (6, "goto", json!({ "route": "/pages/detail/index" })),
        (7, "back", json!({})),
        (8, "reload", json!({})),
    ] {
        let result = dispatch_request(
            &mut state,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            }),
        )
        .await
        .expect("action request");
        assert_eq!(result["result"]["ok"], true, "{method} should succeed");
    }

    let payloads = seen_payloads.lock().expect("lock payloads");
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("navigateTo")),
        "goto should forward navigateTo"
    );
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("navigateBack")),
        "back should forward navigateBack"
    );
    assert!(
        payloads.iter().any(|payload| payload.contains("reLaunch")),
        "reload should forward reLaunch"
    );
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("\"method\":\"Element.tap\"")),
        "tap should call Element.tap"
    );
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("input.input")
                || payload.contains("scroll-view.scrollTo")),
        "type/scroll should call Element.callFunction"
    );
}

#[tokio::test]
async fn screenshot_run_and_wx_actions_forward_to_automator() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let seen_payloads = Arc::new(Mutex::new(Vec::<String>::new()));
    let server_payloads = seen_payloads.clone();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        while let Some(message) = ws.next().await {
            let payload = message.expect("message").into_text().expect("text");
            server_payloads
                .lock()
                .expect("lock payloads")
                .push(payload.to_string());
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
                "App.captureScreenshot" => {
                    json!({ "id": id, "result": { "path": "/tmp/wxp-shot.png", "base64": "ZmFrZQ==" } })
                }
                "App.callFunction" => {
                    json!({ "id": id, "result": { "value": 2 } })
                }
                "App.callWxMethod" => {
                    json!({ "id": id, "result": { "storage": "ok" } })
                }
                _ => json!({ "id": id, "result": { "ok": true } }),
            };

            ws.send(Message::Text(response.to_string().into()))
                .await
                .expect("send");
        }
    });

    let mut state = ServerState::new(test_paths("extended-actions")).expect("server state");
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

    let shot = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shot",
            "params": { "base64": true },
        }),
    )
    .await
    .expect("shot");
    assert_eq!(shot["result"]["ok"], true);
    assert_eq!(shot["result"]["path"], "/tmp/wxp-shot.png");
    assert_eq!(shot["result"]["base64"], "ZmFrZQ==");

    let run = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "run",
            "params": { "js": "1+1" },
        }),
    )
    .await
    .expect("run");
    assert_eq!(run["result"]["ok"], true);
    assert_eq!(run["result"]["result"]["value"], 2);

    let run_function = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "run",
            "params": { "js": "   function() { return 3; }" },
        }),
    )
    .await
    .expect("run function");
    assert_eq!(run_function["result"]["ok"], true);

    let wx = dispatch_request(
        &mut state,
        &json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "wx",
            "params": { "method": "getStorageSync", "args": ["foo"] },
        }),
    )
    .await
    .expect("wx");
    assert_eq!(wx["result"]["ok"], true);
    assert_eq!(wx["result"]["result"]["storage"], "ok");

    let payloads = seen_payloads.lock().expect("lock payloads");
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("\"method\":\"App.captureScreenshot\"")),
        "shot should call App.captureScreenshot"
    );
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("\"method\":\"App.callFunction\"")),
        "run should call App.callFunction"
    );
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("\"method\":\"App.callWxMethod\"")),
        "wx should call App.callWxMethod"
    );

    let call_function_payloads = payloads
        .iter()
        .filter_map(|payload| serde_json::from_str::<Value>(payload).ok())
        .filter(|payload| payload["method"] == "App.callFunction")
        .collect::<Vec<_>>();
    assert_eq!(call_function_payloads.len(), 2, "should record two run calls");
    assert_eq!(
        call_function_payloads[0]["params"]["functionDeclaration"],
        "function() { return (1+1); }"
    );
    assert_eq!(
        call_function_payloads[1]["params"]["functionDeclaration"],
        "   function() { return 3; }"
    );
}

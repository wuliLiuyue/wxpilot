use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use wxp_automator_client::{normalize_input_call, AutomatorClientError, AutomatorConnection};

#[tokio::test]
async fn connection_send_round_trips_a_response() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        let message = ws.next().await.expect("message").expect("message ok");
        let payload = message.into_text().expect("text");
        assert!(payload.contains("\"method\":\"Tool.getInfo\""));
        ws.send(Message::Text(
            r#"{"id":"req-1","result":{"SDKVersion":"3.3.5"}}"#.into(),
        ))
        .await
        .expect("send");
    });

    let connection = AutomatorConnection::connect(&format!("ws://{addr}"))
        .await
        .expect("connect");
    let response = connection
        .send_with_id("req-1".into(), "Tool.getInfo", serde_json::json!({}))
        .await
        .expect("send");

    assert_eq!(response["SDKVersion"], "3.3.5");
}

#[tokio::test]
async fn connection_returns_protocol_error_when_server_replies_with_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        let _ = ws.next().await.expect("message").expect("message ok");
        ws.send(Message::Text(
            r#"{"id":"req-error","error":{"message":"boom"}}"#.into(),
        ))
        .await
        .expect("send");
    });

    let connection = AutomatorConnection::connect(&format!("ws://{addr}"))
        .await
        .expect("connect");
    let error = connection
        .send_with_id("req-error".into(), "Tool.getInfo", serde_json::json!({}))
        .await
        .expect_err("error response should fail");

    assert_eq!(error, AutomatorClientError::Protocol("boom".into()));
}

#[tokio::test]
async fn connection_rejects_duplicate_request_ids() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        let _ = ws.next().await.expect("first").expect("first ok");
        let _ = ws
            .send(Message::Text(
                r#"{"id":"duplicate","result":{"ok":true}}"#.into(),
            ))
            .await;
    });

    let connection = AutomatorConnection::connect(&format!("ws://{addr}"))
        .await
        .expect("connect");
    let first = connection.clone();
    let second = connection.clone();

    let first_task = tokio::spawn(async move {
        first
            .send_with_id("duplicate".into(), "Tool.getInfo", serde_json::json!({}))
            .await
    });

    tokio::task::yield_now().await;

    let second_result = second
        .send_with_id("duplicate".into(), "Tool.getInfo", serde_json::json!({}))
        .await;
    let first_result = first_task.await.expect("join");

    assert_eq!(
        second_result.expect_err("duplicate id should fail"),
        AutomatorClientError::DuplicateRequestId("duplicate".into())
    );
    assert_eq!(
        first_result.expect("first request"),
        serde_json::json!({"ok": true})
    );
}

#[tokio::test]
async fn connection_times_out_when_server_never_replies() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        let _ = ws.next().await.expect("message").expect("message ok");
        tokio::time::sleep(Duration::from_millis(250)).await;
    });

    let connection = AutomatorConnection::connect(&format!("ws://{addr}"))
        .await
        .expect("connect")
        .with_request_timeout(Duration::from_millis(50));
    let error = connection
        .send_with_id("req-timeout".into(), "Tool.getInfo", serde_json::json!({}))
        .await
        .expect_err("missing response should time out");

    assert_eq!(
        error,
        AutomatorClientError::RequestTimedOut("req-timeout".into())
    );
}

#[test]
fn type_action_maps_to_element_call_function() {
    assert_eq!(normalize_input_call("input"), "input.input");
    assert_eq!(normalize_input_call("textarea"), "textarea.input");
}

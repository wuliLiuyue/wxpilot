use wxp_rpc::{RpcError, RpcRequest};

#[test]
fn rpc_request_serializes_to_frozen_contract() {
    let request = RpcRequest::new("1", "health", serde_json::json!({}));
    let encoded = serde_json::to_string(&request).unwrap();
    assert_eq!(
        encoded,
        r#"{"jsonrpc":"2.0","id":"1","method":"health","params":{}}"#
    );
}

#[test]
fn rpc_error_codes_match_ts_contract() {
    assert_eq!(RpcError::ref_expired().code, -32000);
    assert_eq!(RpcError::not_connected().code, -32001);
}

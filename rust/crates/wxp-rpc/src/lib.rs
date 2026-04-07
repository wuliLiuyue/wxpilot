use serde::{Deserialize, Serialize};
use wxp_common::{
    ERROR_INTERNAL, ERROR_NOT_CONNECTED, ERROR_NOT_FOUND, ERROR_REF_EXPIRED, ERROR_TIMEOUT,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

impl RpcRequest {
    pub fn new(
        id: impl Into<String>,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> Self {
        Self {
            jsonrpc: "2.0",
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<RpcErrorPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcErrorPayload {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: &'static str,
}

impl RpcError {
    pub fn ref_expired() -> Self {
        Self {
            code: ERROR_REF_EXPIRED,
            message: "ref_expired",
        }
    }

    pub fn not_connected() -> Self {
        Self {
            code: ERROR_NOT_CONNECTED,
            message: "not_connected",
        }
    }

    pub fn timeout() -> Self {
        Self {
            code: ERROR_TIMEOUT,
            message: "timeout",
        }
    }

    pub fn not_found() -> Self {
        Self {
            code: ERROR_NOT_FOUND,
            message: "not_found",
        }
    }

    pub fn internal() -> Self {
        Self {
            code: ERROR_INTERNAL,
            message: "internal",
        }
    }
}

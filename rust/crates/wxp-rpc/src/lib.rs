use serde::{Deserialize, Serialize};

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
pub struct RpcError {
    pub code: i32,
    pub message: &'static str,
}

impl RpcError {
    pub fn ref_expired() -> Self {
        Self {
            code: -32000,
            message: "ref_expired",
        }
    }

    pub fn not_connected() -> Self {
        Self {
            code: -32001,
            message: "not_connected",
        }
    }

    pub fn timeout() -> Self {
        Self {
            code: -32002,
            message: "timeout",
        }
    }

    pub fn not_found() -> Self {
        Self {
            code: -32003,
            message: "not_found",
        }
    }

    pub fn internal() -> Self {
        Self {
            code: -32099,
            message: "internal",
        }
    }
}

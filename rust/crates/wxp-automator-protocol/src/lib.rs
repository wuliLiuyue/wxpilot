use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatorRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatorError {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatorResponse {
    pub id: Option<String>,
    pub method: Option<String>,
    pub result: Option<Value>,
    pub error: Option<AutomatorError>,
    pub params: Option<Value>,
}

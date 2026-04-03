use std::io;

use serde_json::{json, Map, Value};

pub fn handle_probe(method: &str) -> Value {
    match method {
        "health" => json!({ "status": "ok", "daemonKind": "rust" }),
        "version" => json!({
            "status": "ok",
            "daemonKind": "rust",
            "version": env!("CARGO_PKG_VERSION"),
        }),
        "capabilities" => json!({
            "status": "ok",
            "daemonKind": "rust",
            "methods": crate::PUBLIC_METHODS,
            "find": true,
            "view": true,
            "state": true,
            "shot": true,
            "run": true,
            "wx": true,
        }),
        _ => json!({ "status": "error" }),
    }
}

pub fn format_find_output(
    page_path: &str,
    query: &str,
    rendered: &str,
    shown: usize,
    total: usize,
) -> String {
    let suffix = if total > shown {
        format!(" [显示 {shown}/{total} 条]")
    } else {
        String::new()
    };
    format!(
        "页面: {page_path}\n查找: {query:?}{suffix}\n──────────────────────────────────\n{rendered}"
    )
}

pub fn normalize_route_action(action: &str) -> &'static str {
    match action {
        "goto" => "navigateTo",
        "back" => "navigateBack",
        "reload" => "reLaunch",
        _ => "unknown",
    }
}

pub fn element_request_params(page_id: Value, element: &Value) -> io::Result<Map<String, Value>> {
    let element_id = element
        .get("elementId")
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "element_id_missing"))?;

    let mut params = Map::new();
    params.insert("pageId".into(), page_id);
    params.insert("elementId".into(), element_id);

    if let Some(node_id) = element.get("nodeId").cloned() {
        params.insert("nodeId".into(), node_id);
    }
    if let Some(video_id) = element.get("videoId").cloned() {
        params.insert("videoId".into(), video_id);
    }

    Ok(params)
}

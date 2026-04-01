use serde_json::{json, Value};

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
            "find": true,
            "view": true,
            "state": true,
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

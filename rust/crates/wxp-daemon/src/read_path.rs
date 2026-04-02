use std::{
    io,
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use tokio::time::sleep;
use wxp_snapshot::{find_nodes, parse_wxml, render_find_matches, render_tree, FindMatch};

use crate::{
    handlers::{element_request_params, format_find_output},
    server::ServerState,
};

pub async fn handle_view(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let full = params.get("full").and_then(Value::as_bool).unwrap_or(false);
    let compact = if full {
        false
    } else {
        params
            .get("compact")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    };
    let depth = params
        .get("depth")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(if full { 20 } else { 5 });
    let limit = if full {
        None
    } else {
        Some(
            params
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(50),
        )
    };
    let text_limit = if full { 40 } else { 30 };

    let snapshot = build_snapshot(state, &id, depth).await?;
    let snapshot_version = state
        .runtime
        .replace_snapshot(&snapshot.page_path, &snapshot.tree);
    let rendered = render_tree(&snapshot.tree, compact, depth, limit, text_limit);
    let truncated = limit.is_some_and(|value| rendered.total_count > value);
    let header = if truncated {
        format!(
            "页面: {} [显示 {}/{} 个交互节点]",
            snapshot.page_path, rendered.shown_count, rendered.total_count
        )
    } else {
        format!("页面: {}", snapshot.page_path)
    };
    let output = format!("{header}\n{}\n{}", "─".repeat(34), rendered.output);

    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "ok": true,
            "pagePath": snapshot.page_path,
            "output": output,
            "snapshotVersion": snapshot_version,
            "shownCount": rendered.shown_count,
            "totalCount": rendered.total_count,
            "truncated": truncated,
        }
    }))
}

pub async fn handle_find(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let query = params
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    if query.is_empty() {
        return Ok(error_response(id, -32099, "query_required", None));
    }

    let interactive_only = !params.get("all").and_then(Value::as_bool).unwrap_or(false);
    let limit = params
        .get("limit")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(10);
    let snapshot = build_snapshot(state, &id, 20).await?;
    let snapshot_version = state
        .runtime
        .replace_snapshot(&snapshot.page_path, &snapshot.tree);
    let result = find_nodes(&snapshot.tree, &query, interactive_only, Some(limit));
    let rendered = if result.matches.is_empty() {
        "未找到匹配项".to_string()
    } else {
        render_find_matches(&result.matches, 30)
    };
    let output = format_find_output(
        &snapshot.page_path,
        &query,
        &rendered,
        result.shown_count,
        result.total_count,
    );

    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "ok": true,
            "pagePath": snapshot.page_path,
            "query": query,
            "matches": serialize_matches(&result.matches),
            "output": output,
            "shownCount": result.shown_count,
            "totalCount": result.total_count,
            "snapshotVersion": snapshot_version,
            "truncated": result.total_count > result.shown_count,
        }
    }))
}

pub async fn handle_wait(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let ref_id = params
        .get("ref")
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing ref"))?;
    let timeout_ms = params
        .get("timeout")
        .and_then(Value::as_u64)
        .unwrap_or(10_000);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);

    while Instant::now() <= deadline {
        if state.runtime.has_ref(ref_id) {
            return Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "ok": true,
                    "ref": ref_id,
                }
            }));
        }
        sleep(Duration::from_millis(200)).await;
    }

    Ok(error_response(
        id,
        -32002,
        "timeout",
        Some(json!({ "timeout": timeout_ms })),
    ))
}

pub async fn handle_read(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let ref_id = params
        .get("ref")
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing ref"))?;

    let Some(text) = state.runtime.read_ref_text(ref_id) else {
        return Ok(error_response(
            id,
            -32000,
            "ref_expired",
            Some(json!({ "ref": ref_id })),
        ));
    };

    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "ok": true,
            "ref": ref_id,
            "text": text,
        }
    }))
}

pub async fn handle_assert(
    state: &mut ServerState,
    id: Value,
    params: &Value,
) -> io::Result<Value> {
    let ref_id = params
        .get("ref")
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing ref"))?;
    let expected = params
        .get("expected")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let Some(actual) = state.runtime.read_ref_text(ref_id) else {
        return Ok(error_response(
            id,
            -32000,
            "ref_expired",
            Some(json!({ "ref": ref_id })),
        ));
    };

    if actual != expected {
        return Ok(error_response(
            id,
            -32099,
            "assertion_failed",
            Some(json!({
                "ref": ref_id,
                "expected": expected,
                "actual": actual,
            })),
        ));
    }

    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "ok": true,
            "ref": ref_id,
            "expected": expected,
            "actual": actual,
        }
    }))
}

struct SnapshotPayload {
    page_path: String,
    tree: Vec<wxp_snapshot::ElementNode>,
}

async fn build_snapshot(
    state: &mut ServerState,
    request_id: &Value,
    depth: usize,
) -> io::Result<SnapshotPayload> {
    let current_page = state
        .call_automator(request_id, "current_page", "App.getCurrentPage", json!({}))
        .await?;
    let page_id = current_page
        .get("pageId")
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "page_id_missing"))?;
    let page_path = current_page
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "page_path_missing"))?
        .to_string();
    let root_element = state
        .call_automator(
            request_id,
            "root_element",
            "Page.getElement",
            json!({
                "pageId": page_id,
                "selector": "page",
            }),
        )
        .await?;
    let mut wxml_params = element_request_params(page_id, &root_element)?;
    wxml_params.insert("type".into(), Value::String("outer".to_string()));
    let wxml = state
        .call_automator(
            request_id,
            "outer_wxml",
            "Element.getWXML",
            Value::Object(wxml_params),
        )
        .await?;
    let outer_wxml = wxml
        .get("wxml")
        .and_then(Value::as_str)
        .or_else(|| wxml.get("outerWxml").and_then(Value::as_str))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "outer_wxml_missing"))?;

    Ok(SnapshotPayload {
        page_path,
        tree: parse_wxml(outer_wxml, depth, None),
    })
}

fn serialize_matches(matches: &[FindMatch]) -> Vec<Value> {
    matches
        .iter()
        .map(|item| {
            json!({
                "ref": item.ref_id,
                "tag": item.tag,
                "classes": item.classes,
                "text": item.text,
                "placeholder": item.placeholder,
                "selector": item.selector,
                "interactive": item.interactive,
            })
        })
        .collect()
}

fn error_response(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
            "data": data,
        }
    })
}

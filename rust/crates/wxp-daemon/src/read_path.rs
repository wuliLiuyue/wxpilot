// SPDX-License-Identifier: AGPL-3.0-or-later
// wxpilot - 面向 AI Agent 的微信小程序自动化 CLI
// Copyright (C) 2026  wuliLiuyue
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::Path,
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use tokio::time::sleep;
use wxp_snapshot::{
    extract_event_fingerprints, find_nodes, parse_wxml, render_find_matches, render_tree,
    FindMatch, InteractiveHints,
};

use crate::{
    handlers::{element_request_params, format_find_output},
    server::ServerState,
};

pub async fn handle_state(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let data = read_state_data(state, &id).await?;
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let full = params.get("full").and_then(Value::as_bool).unwrap_or(false);

    if let Some(path) = path {
        return Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "ok": true,
                "data": read_data_path(&data, path).cloned().unwrap_or(Value::Null),
            }
        }));
    }

    if !full && should_return_summary(&data) {
        return Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "ok": true,
                "summary": summarize_top_level_object(&data),
                "hint": "use --full or specify a path to get full data",
            }
        }));
    }

    if full {
        return Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "ok": true,
                "data": data,
            }
        }));
    }

    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "ok": true,
            "data": data,
        }
    }))
}

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

async fn read_state_data(state: &mut ServerState, request_id: &Value) -> io::Result<Value> {
    let current_page = state
        .call_automator(request_id, "current_page", "App.getCurrentPage", json!({}))
        .await?;
    let page_id = current_page
        .get("pageId")
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "page_id_missing"))?;
    let data = state
        .call_automator(
            request_id,
            "page_data",
            "Page.getData",
            json!({ "pageId": page_id }),
        )
        .await?;

    Ok(data.get("data").cloned().unwrap_or(Value::Null))
}

// DevTools 2.02+ (Electron core) no longer exposes the page root as a
// queryable element; selector "page" returns no such element. Fall back to
// "*" to take the first top-level node in document order.
async fn query_root_element(
    state: &mut ServerState,
    request_id: &Value,
    page_id: Value,
) -> io::Result<Value> {
    const ROOT_SELECTORS: [&str; 2] = ["page", "*"];
    let mut last_error = None;
    for (index, selector) in ROOT_SELECTORS.iter().enumerate() {
        let suffix = if index == 0 {
            "root_element"
        } else {
            "root_element_fallback"
        };
        match state
            .call_automator(
                request_id,
                suffix,
                "Page.getElement",
                json!({
                    "pageId": page_id.clone(),
                    "selector": selector,
                }),
            )
            .await
        {
            Ok(element) => return Ok(element),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| io::Error::other("root_element_query_failed")))
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
    let page_data = state
        .call_automator(
            request_id,
            "page_data",
            "Page.getData",
            json!({ "pageId": page_id.clone() }),
        )
        .await
        .ok();
    let root_element = query_root_element(state, request_id, page_id.clone()).await?;
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
    let interactive_hints = build_interactive_hints(
        state.runtime.saved.project_path.as_deref(),
        &page_path,
        page_data.as_ref().and_then(|value| value.get("data")),
    );

    Ok(SnapshotPayload {
        page_path,
        tree: parse_wxml(outer_wxml, depth, Some(&interactive_hints)),
    })
}

fn build_interactive_hints(
    project_path: Option<&str>,
    page_path: &str,
    page_data: Option<&Value>,
) -> InteractiveHints {
    let Some(project_path) = project_path else {
        return InteractiveHints::default();
    };

    let project_root = Path::new(project_path);
    let mut hints = InteractiveHints::default();

    let source_path = project_root.join(format!("{page_path}.wxml"));
    if let Ok(source_wxml) = fs::read_to_string(&source_path) {
        hints.event_fingerprints = extract_event_fingerprints(&source_wxml);
    }

    let base_path = resolve_taro_base_wxml_path(project_root, &source_path);
    if let (Some(base_path), Some(page_data)) = (base_path, page_data) {
        if let Ok(base_wxml) = fs::read_to_string(&base_path) {
            let templates = extract_taro_interactive_templates(&base_wxml);
            if !templates.is_empty() {
                let template_ids = templates.keys().cloned().collect::<HashSet<_>>();
                collect_taro_interactive_sids(page_data, &template_ids, &mut hints.sid_hints);
                collect_taro_runtime_fingerprints(
                    page_data,
                    &templates,
                    &mut hints.event_fingerprints,
                );
            }
        }
    }

    hints
}

fn resolve_taro_base_wxml_path(
    project_root: &Path,
    source_path: &Path,
) -> Option<std::path::PathBuf> {
    let mut current = source_path.parent()?;
    loop {
        let candidate = current.join("base.wxml");
        if candidate.exists() {
            return Some(candidate);
        }
        if current == project_root {
            return None;
        }
        current = current.parent()?;
    }
}

fn extract_taro_interactive_templates(base_wxml: &str) -> HashMap<String, String> {
    let mut templates = HashMap::new();
    let mut current_id = None::<String>;
    let mut current_tag = None::<String>;
    let mut is_interactive = false;

    for line in base_wxml.lines() {
        if let Some(next_id) = parse_taro_template_id(line) {
            current_id = Some(next_id);
            current_tag = None;
            is_interactive = line_has_interactive_signal(line);
            continue;
        }

        if current_tag.is_none() {
            current_tag = parse_taro_template_tag(line);
        }

        if current_id.is_some() && line_has_interactive_signal(line) {
            is_interactive = true;
        }

        if line.contains("</template>") {
            if is_interactive {
                if let (Some(template_id), Some(tag)) =
                    (current_id.as_deref(), current_tag.as_deref())
                {
                    templates.insert(template_id.to_string(), tag.to_string());
                    if let Some((base_id, _)) = template_id.rsplit_once('_') {
                        if matches!(template_id.split('_').next_back(), Some("focus" | "blur")) {
                            templates.insert(base_id.to_string(), tag.to_string());
                        }
                    }
                }
            }
            current_id = None;
            current_tag = None;
            is_interactive = false;
        }
    }

    templates
}

fn parse_taro_template_id(line: &str) -> Option<String> {
    let start = line.find(r#"name="tmpl_"#)?;
    let remainder = &line[start + r#"name="tmpl_"#.len()..];
    let end = remainder.find('"')?;
    let template_name = &remainder[..end];
    let (_, template_id) = template_name.split_once('_')?;
    Some(template_id.to_string())
}

fn parse_taro_template_tag(line: &str) -> Option<String> {
    let start = line.find('<')?;
    let remainder = &line[start + 1..];
    if remainder.starts_with('/') || remainder.starts_with("template") {
        return None;
    }

    let tag = remainder
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect::<String>();
    let tag = tag.to_ascii_lowercase();
    if tag.is_empty() || matches!(tag.as_str(), "block") {
        None
    } else {
        Some(tag)
    }
}

fn line_has_interactive_signal(line: &str) -> bool {
    const INTERACTIVE_ATTRS: &[&str] = &[
        "bindtap",
        "catchtap",
        "bind:tap",
        "catch:tap",
        "bindchange",
        "bind:change",
        "bindinput",
        "bind:input",
        "bindsubmit",
        "bind:submit",
        "bindlongpress",
        "bind:longpress",
        "bindconfirm",
        "bind:confirm",
        "bindfocus",
        "bind:focus",
        "bindblur",
        "bind:blur",
        "bindscroll",
        "bind:scroll",
        "bindscrolltoupper",
        "bind:scrolltoupper",
        "bindscrolltolower",
        "bind:scrolltolower",
        "hover-start-time",
        "hover-stay-time",
    ];
    const INTERACTIVE_TAGS: &[&str] = &[
        "button",
        "input",
        "textarea",
        "picker",
        "picker-view",
        "switch",
        "slider",
        "checkbox",
        "radio",
        "scroll-view",
    ];

    let line = line.trim_start().to_ascii_lowercase();
    INTERACTIVE_ATTRS
        .iter()
        .any(|attr| line_has_attr_assignment(&line, attr))
        || INTERACTIVE_TAGS
            .iter()
            .any(|tag| line_starts_with_tag(&line, tag))
}

fn line_has_attr_assignment(line: &str, attr: &str) -> bool {
    let mut start = 0;
    while let Some(offset) = line[start..].find(attr) {
        let position = start + offset;
        let before = position
            .checked_sub(1)
            .and_then(|index| line.as_bytes().get(index));
        let after = line.as_bytes().get(position + attr.len());
        if matches!(before, Some(b' ' | b'\t' | b'\r' | b'\n')) && matches!(after, Some(b'=')) {
            return true;
        }
        start = position + attr.len();
    }

    false
}

fn line_starts_with_tag(line: &str, tag: &str) -> bool {
    let Some(remainder) = line.strip_prefix('<') else {
        return false;
    };
    let Some(after_tag) = remainder.strip_prefix(tag) else {
        return false;
    };

    matches!(after_tag.as_bytes().first(), Some(b' ' | b'>' | b'/'))
}

fn collect_taro_interactive_sids(
    value: &Value,
    template_ids: &HashSet<String>,
    sid_hints: &mut HashSet<String>,
) {
    match value {
        Value::Object(map) => {
            if let (Some(sid), Some(template_id)) = (
                map.get("sid").and_then(value_as_str),
                map.get("nn").and_then(value_as_str),
            ) {
                if template_ids.contains(template_id) {
                    sid_hints.insert(sid.to_string());
                }
            }

            for child in map.values() {
                collect_taro_interactive_sids(child, template_ids, sid_hints);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_taro_interactive_sids(item, template_ids, sid_hints);
            }
        }
        _ => {}
    }
}

fn collect_taro_runtime_fingerprints(
    value: &Value,
    templates: &HashMap<String, String>,
    event_fingerprints: &mut HashSet<String>,
) {
    match value {
        Value::Object(map) => {
            if let (Some(template_id), Some(class_attr)) = (
                map.get("nn").and_then(value_as_str),
                map.get("cl").and_then(value_as_str),
            ) {
                if let Some(tag) = templates.get(template_id) {
                    let normalized = class_attr
                        .split_whitespace()
                        .filter(|value| !value.is_empty())
                        .collect::<Vec<_>>();
                    if !normalized.is_empty() {
                        let mut classes = normalized
                            .into_iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        classes.sort();
                        event_fingerprints.insert(format!("{tag}:{}", classes.join(",")));
                    }
                }
            }

            for child in map.values() {
                collect_taro_runtime_fingerprints(child, templates, event_fingerprints);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_taro_runtime_fingerprints(item, templates, event_fingerprints);
            }
        }
        _ => {}
    }
}

fn value_as_str(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text.as_str()),
        _ => None,
    }
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

fn read_data_path<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .filter(|segment| !segment.is_empty())
        .try_fold(data, |current, segment| match current {
            Value::Object(map) => map.get(segment),
            Value::Array(items) => segment
                .parse::<usize>()
                .ok()
                .and_then(|index| items.get(index)),
            _ => None,
        })
}

fn should_return_summary(data: &Value) -> bool {
    matches!(data, Value::Object(_))
}

fn summarize_top_level_object(data: &Value) -> Value {
    let Value::Object(map) = data else {
        return Value::Null;
    };

    Value::Object(
        map.iter()
            .map(|(key, value)| (key.clone(), Value::String(summarize_value(value))))
            .collect(),
    )
}

fn summarize_value(value: &Value) -> String {
    let json = serde_json::to_string(value).unwrap_or_else(|_| "null".to_string());
    format!("{}({})", js_typeof(value), json.encode_utf16().count())
}

fn js_typeof(value: &Value) -> &'static str {
    match value {
        Value::Null => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) | Value::Object(_) => "object",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taro_template_scan_matches_interactive_attrs_and_tags() {
        let templates = extract_taro_interactive_templates(
            r#"
<template name="tmpl_0_1">
  <view class="{{i.cl}}" catchtap="eh"></view>
</template>
<template name="tmpl_0_2">
  <view class="{{i.cl}}" bind:tap="eh"></view>
</template>
<template name="tmpl_0_3">
  <input class="{{i.cl}}" bind:input="eh" />
</template>
<template name="tmpl_0_4">
  <picker class="{{i.cl}}"></picker>
</template>
<template name="tmpl_0_5">
  <view class="{{i.cl}}" bind:submit="eh"></view>
</template>
<template name="tmpl_0_6">
  <view class="{{i.cl}}" bind:longpress="eh"></view>
</template>
<template name="tmpl_0_7">
  <view class="{{i.cl}}" bind:change="eh"></view>
</template>
"#,
        );

        assert_eq!(templates.get("1").map(String::as_str), Some("view"));
        assert_eq!(templates.get("2").map(String::as_str), Some("view"));
        assert_eq!(templates.get("3").map(String::as_str), Some("input"));
        assert_eq!(templates.get("4").map(String::as_str), Some("picker"));
        assert_eq!(templates.get("5").map(String::as_str), Some("view"));
        assert_eq!(templates.get("6").map(String::as_str), Some("view"));
        assert_eq!(templates.get("7").map(String::as_str), Some("view"));
    }

    #[test]
    fn taro_template_scan_skips_block_wrappers_for_runtime_fingerprints() {
        let templates = extract_taro_interactive_templates(
            r#"
<template name="tmpl_0_9">
  <block wx:if="{{visible}}">
    <text class="{{i.cl}}" bind:tap="eh"></text>
  </block>
</template>
"#,
        );

        assert_eq!(templates.get("9").map(String::as_str), Some("text"));
    }
}

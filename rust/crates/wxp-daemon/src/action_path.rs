use std::{
    fs, io,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use wxp_automator_client::normalize_input_call;

use crate::{
    handlers::{element_request_params, normalize_route_action},
    server::ServerState,
};

pub async fn handle_action(
    state: &mut ServerState,
    id: Value,
    method: &str,
    params: &Value,
) -> io::Result<Value> {
    match method {
        "goto" => goto(state, id, params).await,
        "back" => back(state, id).await,
        "reload" => reload(state, id).await,
        "tap" => tap(state, id, params).await,
        "type" => type_text(state, id, params).await,
        "scroll" => scroll(state, id, params).await,
        "shot" => shot(state, id, params).await,
        "run" => run_script(state, id, params).await,
        "wx" => wx_call(state, id, params).await,
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported action method: {method}"),
        )),
    }
}

async fn goto(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let route = params
        .get("route")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing route"))?;
    state
        .call_automator(
            &id,
            "goto",
            "App.callWxMethod",
            json!({
                "method": normalize_route_action("goto"),
                "args": [{ "url": route }],
            }),
        )
        .await?;
    state.runtime.invalidate_refs();

    Ok(ok_result(id, json!({ "ok": true, "route": route })))
}

async fn back(state: &mut ServerState, id: Value) -> io::Result<Value> {
    state
        .call_automator(
            &id,
            "back",
            "App.callWxMethod",
            json!({
                "method": normalize_route_action("back"),
                "args": [],
            }),
        )
        .await?;
    state.runtime.invalidate_refs();

    Ok(ok_result(id, json!({ "ok": true })))
}

async fn reload(state: &mut ServerState, id: Value) -> io::Result<Value> {
    let current_page = current_page(state, &id).await?;
    let page_path = current_page
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "page_path_missing"))?;
    let route = normalize_reload_route(page_path, current_page.get("query"));
    state
        .call_automator(
            &id,
            "reload",
            "App.callWxMethod",
            json!({
                "method": normalize_route_action("reload"),
                "args": [{ "url": route }],
            }),
        )
        .await?;
    state.runtime.invalidate_refs();

    Ok(ok_result(id, json!({ "ok": true, "route": route })))
}

async fn tap(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let ref_id = parse_ref(params)?;
    let element = resolve_element_request(state, &id, ref_id).await?;
    state
        .call_automator(&id, "tap", "Element.tap", Value::Object(element.clone()))
        .await?;

    Ok(ok_result(
        id,
        json!({ "ok": true, "action": "tap", "ref": ref_id }),
    ))
}

async fn type_text(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let ref_id = parse_ref(params)?;
    let text = params
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let mut element = resolve_element_request(state, &id, ref_id).await?;
    let call_name = state
        .runtime
        .snapshot_refs
        .get(&ref_id)
        .map(|entry| {
            normalize_input_call(
                entry
                    .selector
                    .split('>')
                    .next_back()
                    .unwrap_or("input")
                    .split('.')
                    .next()
                    .unwrap_or("input")
                    .trim(),
            )
        })
        .unwrap_or("input.input");
    element.insert("functionName".into(), Value::String(call_name.to_string()));
    element.insert("args".into(), json!([text]));
    state
        .call_automator(&id, "type", "Element.callFunction", Value::Object(element))
        .await?;

    Ok(ok_result(
        id,
        json!({ "ok": true, "action": "type", "ref": ref_id, "text": text }),
    ))
}

async fn scroll(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let ref_id = parse_ref(params)?;
    let direction = params
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("down");
    let element = resolve_element_request(state, &id, ref_id).await?;
    let mut scroll_top_request = element.clone();
    scroll_top_request.insert(
        "functionName".into(),
        Value::String("scroll-view.scrollTop".to_string()),
    );
    scroll_top_request.insert("args".into(), json!([]));
    let current_top = state
        .call_automator(
            &id,
            "scroll_top",
            "Element.callFunction",
            Value::Object(scroll_top_request),
        )
        .await
        .ok()
        .and_then(as_scroll_top)
        .unwrap_or(0.0);
    let delta = if direction == "up" { -300.0 } else { 300.0 };
    let mut scroll_request = element;
    scroll_request.insert(
        "functionName".into(),
        Value::String("scroll-view.scrollTo".to_string()),
    );
    scroll_request.insert("args".into(), json!([0, current_top + delta]));
    state
        .call_automator(
            &id,
            "scroll",
            "Element.callFunction",
            Value::Object(scroll_request),
        )
        .await?;

    Ok(ok_result(
        id,
        json!({ "ok": true, "action": "scroll", "ref": ref_id, "direction": direction }),
    ))
}

async fn shot(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let base64 = params
        .get("base64")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let save_path = params
        .get("path")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(default_screenshot_path);

    let result = state
        .call_automator(&id, "shot", "App.captureScreenshot", json!({}))
        .await?;

    let screenshot_base64 = result
        .get("data")
        .or_else(|| result.get("base64"))
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "screenshot_data_missing"))?;
    let bytes = decode_base64(screenshot_base64)?;
    fs::write(&save_path, bytes)?;

    let mut response = serde_json::Map::new();
    response.insert("ok".into(), Value::Bool(true));
    response.insert("path".into(), Value::String(save_path));
    if base64 {
        response.insert(
            "base64".into(),
            Value::String(screenshot_base64.to_string()),
        );
    }

    Ok(ok_result(id, Value::Object(response)))
}

fn decode_base64(input: &str) -> io::Result<Vec<u8>> {
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0u8;

    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid_screenshot_base64",
                ))
            }
        };

        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
    }

    Ok(output)
}

fn default_screenshot_path() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    std::env::temp_dir()
        .join(format!("wxpilot-shot-{millis}.png"))
        .display()
        .to_string()
}

async fn run_script(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let js = params
        .get("js")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing js"))?;
    let function_declaration = normalize_function_declaration(js);
    let result = state
        .call_automator(
            &id,
            "run",
            "App.callFunction",
            json!({ "functionDeclaration": function_declaration }),
        )
        .await?;

    Ok(ok_result(id, json!({ "ok": true, "result": result })))
}

async fn wx_call(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let method = params
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing method"))?;
    let args = params.get("args").cloned().unwrap_or_else(|| json!([]));
    if !args.is_array() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "args must be an array",
        ));
    }

    let result = state
        .call_automator(
            &id,
            "wx",
            "App.callWxMethod",
            json!({ "method": method, "args": args }),
        )
        .await?;

    Ok(ok_result(id, json!({ "ok": true, "result": result })))
}

async fn resolve_element_request(
    state: &mut ServerState,
    request_id: &Value,
    ref_id: u32,
) -> io::Result<serde_json::Map<String, Value>> {
    let selector = state.runtime.resolve_ref_selector(ref_id).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("ref_expired:{ref_id}"))
    })?;
    let current_page = current_page(state, request_id).await?;
    let page_id = current_page
        .get("pageId")
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "page_id_missing"))?;
    let element = state
        .call_automator(
            request_id,
            "element_lookup",
            "Page.getElement",
            json!({
                "pageId": page_id,
                "selector": selector,
            }),
        )
        .await?;

    element_request_params(page_id, &element)
}

async fn current_page(state: &mut ServerState, request_id: &Value) -> io::Result<Value> {
    state
        .call_automator(request_id, "current_page", "App.getCurrentPage", json!({}))
        .await
}

fn parse_ref(params: &Value) -> io::Result<u32> {
    params
        .get("ref")
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing ref"))
}

fn normalize_function_declaration(js: &str) -> String {
    if js.trim_start().starts_with("function") {
        js.to_string()
    } else {
        format!("function() {{ return ({}); }}", js)
    }
}

fn normalize_reload_route(page_path: &str, query: Option<&Value>) -> String {
    let mut route = if page_path.starts_with('/') {
        page_path.to_string()
    } else {
        format!("/{page_path}")
    };

    if let Some(query) = query.and_then(Value::as_object) {
        let pairs = query
            .iter()
            .map(|(key, value)| {
                let value = value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string());
                format!("{key}={value}")
            })
            .collect::<Vec<_>>();
        if !pairs.is_empty() {
            route.push('?');
            route.push_str(&pairs.join("&"));
        }
    }

    route
}

fn as_scroll_top(value: Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|value| value as f64))
        .or_else(|| value.as_u64().map(|value| value as f64))
        .or_else(|| {
            value
                .as_str()
                .and_then(|value| value.trim().parse::<f64>().ok())
        })
}

fn ok_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

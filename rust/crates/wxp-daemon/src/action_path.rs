use std::io;

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
                "args": { "url": route },
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
                "args": {},
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
    state
        .call_automator(
            &id,
            "reload",
            "App.callWxMethod",
            json!({
                "method": normalize_route_action("reload"),
                "args": { "url": page_path },
            }),
        )
        .await?;
    state.runtime.invalidate_refs();

    Ok(ok_result(id, json!({ "ok": true, "route": page_path })))
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
    element.insert("name".into(), Value::String(call_name.to_string()));
    element.insert("args".into(), json!({ "value": text }));
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
    let mut element = resolve_element_request(state, &id, ref_id).await?;
    element.insert(
        "name".into(),
        Value::String("scroll-view.scrollTo".to_string()),
    );
    element.insert("args".into(), json!({ "direction": direction }));
    state
        .call_automator(
            &id,
            "scroll",
            "Element.callFunction",
            Value::Object(element),
        )
        .await?;

    Ok(ok_result(
        id,
        json!({ "ok": true, "action": "scroll", "ref": ref_id, "direction": direction }),
    ))
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

fn ok_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

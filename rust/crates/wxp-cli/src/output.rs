use serde_json::{Map, Value};
use wxp_common::MAX_OUTPUT_CHARS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Json,
}

pub fn render_ok(payload: &Value, mode: OutputMode) -> String {
    match mode {
        OutputMode::Json => {
            let rendered = match payload {
                Value::Object(object) => ordered_json(true, Some(object), None),
                value => ordered_json(
                    true,
                    Some(&Map::from_iter([(String::from("data"), value.clone())])),
                    None,
                ),
            };
            safe_truncate(rendered)
        }
        OutputMode::Human => render_human_ok(payload),
    }
}

pub fn render_error(message: &str, extra: Option<Value>, mode: OutputMode) -> String {
    match mode {
        OutputMode::Json => ordered_error_json(message, extra),
        OutputMode::Human => format!("ERR: {message}"),
    }
}

pub fn render_ref_expired(reference: u64, mode: OutputMode) -> String {
    match mode {
        OutputMode::Json => render_error(
            "ref_expired",
            Some(serde_json::json!({ "ref": reference })),
            mode,
        ),
        OutputMode::Human => format!("ERR: %{reference} expired, run 'wxpilot view' to refresh"),
    }
}

fn render_human_ok(payload: &Value) -> String {
    if let Some(output) = payload.get("output").and_then(Value::as_str) {
        return safe_truncate(output.to_string());
    }

    if let Some(rendered) = render_action(payload) {
        return rendered;
    }

    if let Some(rendered) = render_read(payload) {
        return rendered;
    }

    if let Some(rendered) = render_base64(payload) {
        return rendered;
    }

    if let Some(rendered) = render_path(payload) {
        return rendered;
    }

    if let Some(rendered) = render_summary(payload) {
        return rendered;
    }

    if let Some(result) = payload.get("result") {
        return safe_truncate(format!("OK: {result}"));
    }

    if let Some(rendered) = render_data(payload) {
        return rendered;
    }

    let parts = payload
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter(|(key, _)| key.as_str() != "ok")
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();

    if parts.is_empty() {
        String::from("OK")
    } else {
        safe_truncate(format!("OK: {parts}"))
    }
}

fn render_action(payload: &Value) -> Option<String> {
    let action = payload.get("action")?.as_str()?;
    let reference = payload.get("ref")?;

    match action {
        "tap" => Some(format!("OK: tap %{reference}")),
        "type" => Some(format!(
            "OK: type %{reference} \"{}\"",
            payload.get("text")?.as_str()?
        )),
        "scroll" => Some(format!(
            "OK: scroll %{reference} {}",
            payload.get("direction")?.as_str()?
        )),
        _ => None,
    }
}

fn render_read(payload: &Value) -> Option<String> {
    Some(format!(
        "OK: read %{} \"{}\"",
        payload.get("ref")?,
        payload.get("text")?.as_str()?
    ))
}

fn render_base64(payload: &Value) -> Option<String> {
    let base64 = payload.get("base64")?.as_str()?;
    Some(safe_truncate(format!("OK: screenshot captured (base64)\n{base64}")))
}

fn render_path(payload: &Value) -> Option<String> {
    if payload.get("action").is_some() {
        return None;
    }

    Some(format!(
        "OK: screenshot saved → {}",
        payload.get("path")?.as_str()?
    ))
}

fn render_summary(payload: &Value) -> Option<String> {
    let summary = payload.get("summary")?;
    let rendered = serde_json::to_string_pretty(summary).ok()?;
    let hint = payload
        .get("hint")
        .and_then(Value::as_str)
        .map(|value| format!("\n[{value}]"))
        .unwrap_or_default();
    Some(safe_truncate(rendered) + &hint)
}

fn render_data(payload: &Value) -> Option<String> {
    let data = payload.get("data")?;
    let mut rendered = safe_truncate(serde_json::to_string_pretty(data).ok()?);
    if payload
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        rendered.push('\n');
        rendered.push_str(&format!(
            "[truncated, originalLength={}]",
            payload.get("originalLength")?
        ));
    }
    Some(rendered)
}

fn safe_truncate(text: String) -> String {
    if text.len() <= MAX_OUTPUT_CHARS {
        return text;
    }

    format!(
        "{}\n[truncated, originalLength={}]",
        &text[..MAX_OUTPUT_CHARS],
        text.len()
    )
}

fn ordered_json(ok: bool, object: Option<&Map<String, Value>>, error: Option<&str>) -> String {
    let mut parts = vec![format!("\"ok\":{}", if ok { "true" } else { "false" })];
    if let Some(message) = error {
        parts.push(format!(
            "\"error\":{}",
            serde_json::to_string(message).expect("serialize error message")
        ));
    }
    if let Some(object) = object {
        for (key, value) in object {
            parts.push(format!(
                "{}:{}",
                serde_json::to_string(key).expect("serialize key"),
                serde_json::to_string(value).expect("serialize value")
            ));
        }
    }
    format!("{{{}}}", parts.join(","))
}

fn ordered_error_json(message: &str, extra: Option<Value>) -> String {
    match extra {
        Some(Value::Object(object)) => ordered_json(false, Some(&object), Some(message)),
        _ => ordered_json(false, None, Some(message)),
    }
}

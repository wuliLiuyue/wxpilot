use serde_json::{Map, Value};
use wxp_common::MAX_OUTPUT_CHARS;

use crate::command::CliCommand;

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

pub fn render_command_ok(command: &CliCommand, payload: &Value, mode: OutputMode) -> String {
    if matches!(mode, OutputMode::Json) {
        return render_ok(payload, mode);
    }

    match command {
        CliCommand::Activate { .. } => render_activate(payload),
        CliCommand::NetStart { .. } => render_net_start(payload),
        CliCommand::NetLog { .. } => render_net_log(payload),
        CliCommand::NetInstallCa => render_net_install_ca(payload),
        _ => render_ok(payload, mode),
    }
}

fn render_activate(payload: &Value) -> String {
    let Some(path) = payload.get("path").and_then(Value::as_str) else {
        return render_ok(payload, OutputMode::Human);
    };
    let license_id = payload
        .get("licenseId")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let machine_fingerprint = payload
        .get("machineFingerprint")
        .and_then(Value::as_str)
        .unwrap_or("-");

    format!("OK: 激活成功\nlicense: {license_id}\nmachine: {machine_fingerprint}\npath: {path}")
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

fn render_net_start(payload: &Value) -> String {
    let Some(port) = payload.get("port") else {
        return render_ok(payload, OutputMode::Human);
    };
    let https_enabled = payload
        .get("https")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut lines = vec![format!(
        "OK: proxy listening on 127.0.0.1:{port}{}",
        if https_enabled {
            " (HTTPS enabled)"
        } else {
            ""
        }
    )];
    lines.push(format!("请在微信开发者工具设置代理 → 127.0.0.1:{port}"));
    if let Some(ca_path) = payload.get("caPath").and_then(Value::as_str) {
        lines.push(format!("CA 证书已生成: {ca_path}"));
    }
    lines.join("\n")
}

fn render_net_log(payload: &Value) -> String {
    let Some(logs) = payload.get("logs").and_then(Value::as_array) else {
        return render_ok(payload, OutputMode::Human);
    };

    let mut lines = Vec::new();
    if payload
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let shown = payload.get("shownCount").cloned().unwrap_or(Value::Null);
        let total = payload.get("totalCount").cloned().unwrap_or(Value::Null);
        lines.push(format!("[显示 {shown}/{total} 条，使用 --limit 调整]"));
    }

    for log in logs {
        let ts = log
            .get("ts")
            .and_then(Value::as_i64)
            .map(format_ts_millis)
            .unwrap_or_else(|| String::from("invalid-ts"));
        let method = log.get("method").and_then(Value::as_str).unwrap_or("-");
        let url = log.get("url").and_then(Value::as_str).unwrap_or("-");
        let status = log
            .get("status")
            .map(stringify_json_value)
            .unwrap_or_else(|| String::from("-"));
        let mocked = if log
            .get("mocked")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            != 0
        {
            " [MOCK]"
        } else {
            ""
        };
        let duration = log
            .get("duration")
            .and_then(Value::as_i64)
            .map(|value| format!(" {value}ms"))
            .unwrap_or_default();
        lines.push(format!("{ts} {method} {url} {status}{mocked}{duration}"));

        if let Some(req_body) = log.get("req_body").and_then(Value::as_str) {
            lines.push(format!("  req: {}", normalize_body(req_body)));
        }
        if let Some(res_body) = log.get("res_body").and_then(Value::as_str) {
            lines.push(format!("  res: {}", normalize_body(res_body)));
        }
    }

    if lines.is_empty() {
        String::from("OK")
    } else {
        safe_truncate(lines.join("\n"))
    }
}

fn render_net_install_ca(payload: &Value) -> String {
    let Some(cert_file) = payload.get("certFile").and_then(Value::as_str) else {
        return render_ok(payload, OutputMode::Human);
    };

    format!("OK: CA 证书已安装到系统钥匙串\ncert: {cert_file}\n请完全重启微信开发者工具后生效")
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
    Some(safe_truncate(format!(
        "OK: screenshot captured (base64)\n{base64}"
    )))
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

fn stringify_json_value(value: &Value) -> String {
    match value {
        Value::Null => String::from("-"),
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

fn normalize_body(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .map(|value| serde_json::to_string(&value).unwrap_or_else(|_| body.to_string()))
        .unwrap_or_else(|_| body.to_string())
}

fn format_ts_millis(ts: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let ts = if ts < 0 { 0 } else { ts as u64 };
    let time = UNIX_EPOCH + Duration::from_millis(ts);
    let datetime: chrono::DateTime<chrono::Utc> = time.into();
    datetime.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

use std::io;

use serde_json::{json, Value};

use crate::server::ServerState;

pub async fn handle_net_request(
    state: &mut ServerState,
    id: Value,
    method: &str,
    params: &Value,
) -> io::Result<Value> {
    match method {
        "net.mock" => {
            let http_method = params
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("GET");
            let url = params
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing url"))?;
            let body = params
                .get("body")
                .cloned()
                .unwrap_or(Value::Null)
                .to_string();
            state.runtime.mock_registry.upsert(http_method, url, &body);
            Ok(ok_result(
                id,
                json!({ "ok": true, "method": http_method, "url": url }),
            ))
        }
        "net.unmock" => {
            let url = params
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing url"))?;
            state.runtime.mock_registry.remove(url);
            Ok(ok_result(id, json!({ "ok": true, "url": url })))
        }
        "net.log" => {
            let limit = params
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(20);
            let with_body = params
                .get("withBody")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let logs = state
                .runtime
                .request_store
                .list_requests(limit, with_body)
                .map_err(io::Error::other)?;
            let logs = logs
                .into_iter()
                .map(|item| {
                    json!({
                        "method": item.method,
                        "url": item.url,
                        "reqBody": item.req_body,
                        "resBody": item.res_body,
                    })
                })
                .collect::<Vec<_>>();

            Ok(ok_result(id, json!({ "ok": true, "logs": logs })))
        }
        "net.clear" => {
            state
                .runtime
                .request_store
                .clear_requests()
                .map_err(io::Error::other)?;
            Ok(ok_result(id, json!({ "ok": true })))
        }
        "net.start" | "net.stop" | "net.install-ca" => Ok(ok_result(id, json!({ "ok": true }))),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported net method: {method}"),
        )),
    }
}

fn ok_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

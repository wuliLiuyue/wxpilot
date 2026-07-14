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
            let body = if let Some(json_file) = params.get("jsonFile").and_then(Value::as_str) {
                std::fs::read_to_string(json_file)?
            } else {
                params
                    .get("body")
                    .cloned()
                    .unwrap_or(Value::Null)
                    .to_string()
            };
            state.runtime.mock_registry.upsert(http_method, url, &body);
            Ok(ok_result(id, json!({ "ok": true, "url": url })))
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
            let filter = params.get("filter").and_then(Value::as_str);
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
                .list_requests_filtered(limit, with_body, filter)
                .map_err(io::Error::other)?;
            let logs = logs
                .into_iter()
                .map(|item| {
                    let mut entry = serde_json::Map::from_iter([
                        ("method".to_string(), Value::String(item.method)),
                        ("url".to_string(), Value::String(item.url)),
                        (
                            "status".to_string(),
                            item.status.map(Value::from).unwrap_or(Value::Null),
                        ),
                        ("ts".to_string(), json!(item.ts)),
                        ("mocked".to_string(), json!(item.mocked)),
                        ("duration".to_string(), Value::Null),
                    ]);
                    if with_body {
                        entry.insert(
                            "req_body".to_string(),
                            item.req_body.map(Value::String).unwrap_or(Value::Null),
                        );
                        entry.insert(
                            "res_body".to_string(),
                            item.res_body.map(Value::String).unwrap_or(Value::Null),
                        );
                    }
                    Value::Object(entry)
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
        "net.start" => {
            let port = params
                .get("port")
                .and_then(Value::as_u64)
                .map(u16::try_from)
                .transpose()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "port_out_of_range"))?
                .unwrap_or(state.default_proxy_port);
            let https = params
                .get("https")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let ca_path = if https {
                let cert_path = crate::ca::ensure_ca_cert(&state.paths.ca_dir)?;
                let key_path = state.paths.ca_dir.join("key.pem");
                state.runtime.proxy.configure_https(&cert_path, &key_path)?;
                Some(cert_path)
            } else {
                state.runtime.proxy.disable_https();
                None
            };
            if state.runtime.proxy.is_running() {
                state.runtime.proxy.stop()?;
            }
            let bound_port = state.runtime.proxy.start(port)?;
            Ok(ok_result(
                id,
                json!({
                    "ok": true,
                    "port": bound_port,
                    "https": https,
                    "caPath": ca_path.map(|path| path.display().to_string()),
                }),
            ))
        }
        "net.stop" => {
            state.runtime.proxy.stop()?;
            Ok(ok_result(id, json!({ "ok": true })))
        }
        "net.install-ca" => {
            let cert_file = crate::ca::ensure_ca_cert(&state.paths.ca_dir)?;
            crate::ca::install_ca_cert(&cert_file)?;
            Ok(ok_result(
                id,
                json!({
                    "ok": true,
                    "certFile": cert_file.display().to_string(),
                }),
            ))
        }
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

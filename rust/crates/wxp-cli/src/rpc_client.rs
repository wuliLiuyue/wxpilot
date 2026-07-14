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

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::Value;
use wxp_common::DEFAULT_TIMEOUT_MS;
use wxp_rpc::{RpcRequest, RpcResponse};

pub fn call(socket_path: &Path, method: &str, params: Value) -> Result<RpcResponse, String> {
    call_with_timeout(socket_path, method, params, DEFAULT_TIMEOUT_MS)
}

pub fn call_with_timeout(
    socket_path: &Path,
    method: &str,
    params: Value,
    timeout_ms: u64,
) -> Result<RpcResponse, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|error| error.to_string())?;
    let timeout = Some(Duration::from_millis(timeout_ms));
    stream
        .set_read_timeout(timeout)
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(timeout)
        .map_err(|error| error.to_string())?;

    let request = RpcRequest::new("1", method, params);
    let payload = serde_json::to_string(&request).map_err(|error| error.to_string())? + "\n";
    stream
        .write_all(payload.as_bytes())
        .map_err(|error| error.to_string())?;

    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;

    serde_json::from_str(&line).map_err(|error| error.to_string())
}

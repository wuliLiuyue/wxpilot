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

use std::io::Write;
use std::os::unix::net::UnixListener;
use std::thread;

use tempfile::tempdir;

#[test]
fn rpc_call_returns_error_when_response_is_not_json() {
    let temp = tempdir().expect("tempdir");
    let socket_path = temp.path().join("rpc.sock");
    let listener = UnixListener::bind(&socket_path).expect("bind");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream.write_all(b"not json\n").expect("write");
    });

    let error = wxp_cli::rpc_client::call(&socket_path, "health", serde_json::json!({}))
        .expect_err("invalid json should fail");
    server.join().expect("join");

    assert!(!error.is_empty());
}

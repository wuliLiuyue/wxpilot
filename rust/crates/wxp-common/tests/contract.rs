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

use wxp_common::{
    DAEMON_KIND, DEFAULT_TIMEOUT_MS, ERROR_INTERNAL, ERROR_NOT_CONNECTED, ERROR_NOT_FOUND,
    ERROR_REF_EXPIRED, ERROR_TIMEOUT, MAX_OUTPUT_CHARS, RUST_PID_FILE, RUST_SOCKET_FILE,
};

#[test]
fn freezes_public_contract_constants_in_rust() {
    assert_eq!(DAEMON_KIND, "rust");
    assert_eq!(RUST_SOCKET_FILE, "rust-daemon.sock");
    assert_eq!(RUST_PID_FILE, "rust-daemon.pid");
    assert_eq!(DEFAULT_TIMEOUT_MS, 10_000);
    assert_eq!(MAX_OUTPUT_CHARS, 4_000);
    assert_eq!(ERROR_REF_EXPIRED, -32000);
    assert_eq!(ERROR_NOT_CONNECTED, -32001);
    assert_eq!(ERROR_TIMEOUT, -32002);
    assert_eq!(ERROR_NOT_FOUND, -32003);
    assert_eq!(ERROR_INTERNAL, -32099);
}

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

use wxp_rpc::{RpcError, RpcRequest};

#[test]
fn rpc_request_serializes_to_frozen_contract() {
    let request = RpcRequest::new("1", "health", serde_json::json!({}));
    let encoded = serde_json::to_string(&request).unwrap();
    assert_eq!(
        encoded,
        r#"{"jsonrpc":"2.0","id":"1","method":"health","params":{}}"#
    );
}

#[test]
fn rpc_error_codes_match_ts_contract() {
    assert_eq!(RpcError::ref_expired().code, -32000);
    assert_eq!(RpcError::not_connected().code, -32001);
}

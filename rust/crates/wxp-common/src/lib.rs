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

pub const DAEMON_KIND: &str = "rust";
pub const RUST_SOCKET_FILE: &str = "rust-daemon.sock";
pub const RUST_PID_FILE: &str = "rust-daemon.pid";
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
pub const MAX_OUTPUT_CHARS: usize = 4_000;

pub const ERROR_REF_EXPIRED: i32 = -32000;
pub const ERROR_NOT_CONNECTED: i32 = -32001;
pub const ERROR_TIMEOUT: i32 = -32002;
pub const ERROR_NOT_FOUND: i32 = -32003;
pub const ERROR_INTERNAL: i32 = -32099;

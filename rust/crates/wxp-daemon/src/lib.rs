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

use serde::Serialize;

pub mod action_path;
pub mod bootstrap_guard;
pub mod ca;
pub mod devtools;
pub mod handlers;
pub mod net_path;
pub mod read_path;
pub mod runtime;
pub mod server;

pub use devtools::DevtoolsLauncher;
pub use handlers::format_find_output;
pub use runtime::{DaemonState, StateStore};
pub use server::{dispatch_local_request, RuntimePaths, ServerState};

pub const PUBLIC_METHODS: &[&str] = &[
    "start",
    "connect",
    "disconnect",
    "status",
    "goto",
    "back",
    "reload",
    "view",
    "find",
    "wait",
    "tap",
    "type",
    "scroll",
    "read",
    "assert",
    "state",
    "shot",
    "run",
    "wx",
    "net.start",
    "net.stop",
    "net.log",
    "net.mock",
    "net.unmock",
    "net.clear",
    "net.install-ca",
    "daemon.stop",
];

#[derive(Debug, Serialize)]
pub struct ProbePayload {
    pub daemon_kind: &'static str,
    pub status: &'static str,
}

pub fn build_probe_payload() -> ProbePayload {
    ProbePayload {
        daemon_kind: wxp_common::DAEMON_KIND,
        status: "ok",
    }
}

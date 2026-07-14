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

use wxp_daemon::bootstrap_guard::{run_bootstrap_guard, AllowAllBootstrapGuard};
use wxp_daemon::server::{serve_forever, RuntimePaths, ServerState};

fn main() {
    let decision = run_bootstrap_guard(&AllowAllBootstrapGuard);
    if !decision.allowed {
        eprintln!(
            "{}",
            decision
                .reason
                .as_deref()
                .unwrap_or("bootstrap guard rejected startup")
        );
        std::process::exit(1);
    }

    let state = match ServerState::new(RuntimePaths::default()) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("rust daemon init error: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = serve_forever(state) {
        eprintln!("rust daemon error: {error}");
        std::process::exit(1);
    }
}

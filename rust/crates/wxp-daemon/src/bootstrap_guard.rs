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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapDecision {
    pub allowed: bool,
    pub reason: Option<String>,
}

impl BootstrapDecision {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
        }
    }
}

pub trait BootstrapGuard {
    fn check(&self) -> BootstrapDecision;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AllowAllBootstrapGuard;

impl BootstrapGuard for AllowAllBootstrapGuard {
    fn check(&self) -> BootstrapDecision {
        BootstrapDecision::allow()
    }
}

pub fn run_bootstrap_guard(guard: &impl BootstrapGuard) -> BootstrapDecision {
    guard.check()
}

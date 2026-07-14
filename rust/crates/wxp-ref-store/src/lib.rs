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

use std::collections::HashMap;

#[derive(Default)]
pub struct RefStore {
    current_version: u64,
    refs: HashMap<(u64, u32), String>,
}

impl RefStore {
    pub fn replace_snapshot<S>(&mut self, _page_path: &str, version: u64, refs: Vec<(u32, S)>)
    where
        S: Into<String>,
    {
        self.current_version = version;
        self.refs
            .retain(|(existing_version, _), _| *existing_version == version);
        for (ref_id, selector) in refs {
            self.refs.insert((version, ref_id), selector.into());
        }
    }

    pub fn resolve(&self, ref_id: u32, version: u64) -> Option<String> {
        if version != self.current_version {
            return None;
        }
        self.refs.get(&(version, ref_id)).cloned()
    }
}

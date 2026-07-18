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

use wxp_ref_store::RefStore;

#[test]
fn replacing_snapshot_invalidates_previous_refs() {
    let mut store = RefStore::default();
    store.replace_snapshot("pages/index/index", 1, vec![(1, "page > input")]);
    store.replace_snapshot("pages/index/index", 2, vec![(1, "page > button")]);

    assert!(store.resolve(1, 1).is_none());
    assert_eq!(store.resolve(1, 2).as_deref(), Some("page > button"));
}

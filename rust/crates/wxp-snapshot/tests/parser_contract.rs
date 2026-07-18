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

use wxp_snapshot::{find_nodes, parse_wxml, render_find_matches, render_tree};

#[test]
fn parse_render_and_find_match_ts_behavior() {
    let tree = parse_wxml(
        r#"<page><view class="card"><input class="PhoneInput" placeholder="手机号" /></view></page>"#,
        20,
        None,
    );
    let rendered = render_tree(&tree, true, 5, Some(50), 30);
    assert!(rendered.output.contains("placeholder=\"手机号\""));

    let result = find_nodes(&tree, "phone", true, Some(10));
    assert_eq!(result.total_count, 1);
    assert_eq!(result.matches[0].placeholder.as_deref(), Some("手机号"));
    let summary = render_find_matches(&result.matches, 30);
    assert!(summary.contains("%1 [input].PhoneInput"));
}

#[test]
fn parse_ignores_devtools_overlay_nodes_and_styles() {
    let tree = parse_wxml(
        r#"<page>.luna-dom-highlighter{position:fixed}#luna-dom-highlighter-element-title{position:absolute}<view class="luna-dom-highlighter-fill"></view><view class="content">余额</view></page>"#,
        20,
        None,
    );

    let rendered = render_tree(&tree, false, 5, None, 30);
    assert!(!rendered.output.contains("luna-dom-highlighter"));

    let result = find_nodes(&tree, "余额", false, Some(10));
    assert!(result.total_count > 0);
    for item in &result.matches {
        assert!(!item.tag.contains("luna-dom-highlighter"));
        assert!(item
            .text
            .as_deref()
            .is_none_or(|text| !text.contains("luna-dom-highlighter")));
        assert!(!item
            .classes
            .iter()
            .any(|class_name| class_name.contains("luna-dom-highlighter")));
    }
}

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

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn prints_help_from_wxpilot_binary() {
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI Agent 微信小程序自动化 CLI"))
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("net"));
}

#[test]
fn prints_semver_version() {
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("wxpilot 0.1.0"));
}

#[test]
fn fails_without_subcommand() {
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Usage: wxpilot [OPTIONS] <COMMAND>",
        ))
        .stderr(predicate::str::contains("Commands:"));
}

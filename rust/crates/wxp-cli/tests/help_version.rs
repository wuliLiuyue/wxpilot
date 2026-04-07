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
        .stderr(predicate::str::contains("Usage: wxpilot [OPTIONS] <COMMAND>"))
        .stderr(predicate::str::contains("Commands:"));
}

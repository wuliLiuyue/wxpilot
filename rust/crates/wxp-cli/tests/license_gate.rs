use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

mod support;

#[test]
fn status_requires_activation_when_license_file_is_missing() {
    let temp = tempdir().expect("tempdir");

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_MACHINE_ID", "machine-a")
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("当前发布版需要先激活"));
}

#[test]
fn status_rejects_tampered_license_certificate() {
    let temp = tempdir().expect("tempdir");
    support::write_license(temp.path(), "machine-a");
    let license_path = temp.path().join(".wxpilot").join("license.json");
    let raw = fs::read_to_string(&license_path).expect("read license");
    let tampered = raw.replace("lic_test", "lic_other");
    fs::write(&license_path, tampered).expect("write tampered license");

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_MACHINE_ID", "machine-a")
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("本地激活凭证无效"));
}

#[test]
fn status_allows_valid_offline_license() {
    let temp = tempdir().expect("tempdir");
    support::write_license(temp.path(), "machine-a");
    let server = support::spawn_status_daemon(temp.path());

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_MACHINE_ID", "machine-a")
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("daemonKind=\"rust\""));

    server.join().expect("join daemon");
}

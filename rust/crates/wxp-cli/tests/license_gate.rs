use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;
use wxp_license::{derive_public_key_hex, test_keys::PRIVATE_KEY_HEX};

mod support;

#[test]
fn status_requires_activation_when_license_file_is_missing() {
    let temp = tempdir().expect("tempdir");

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("当前发布版需要先激活"));
}

#[test]
fn status_rejects_tampered_license_certificate() {
    let machine_id = "tampered-machine";
    let temp = tempdir().expect("tempdir");
    support::write_license(temp.path(), machine_id);
    let license_path = temp.path().join(".wxpilot").join("license.json");
    let raw = fs::read_to_string(&license_path).expect("read license");
    let tampered = raw.replace("lic_test", "lic_other");
    fs::write(&license_path, tampered).expect("write tampered license");

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("本地激活凭证无效"));
}

#[test]
fn status_allows_valid_offline_license() {
    let machine_id = "offline-machine";
    let temp = tempdir().expect("tempdir");
    support::write_license(temp.path(), machine_id);
    let server = support::spawn_status_daemon(temp.path());

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("daemonKind=\"rust\""));

    server.join().expect("join daemon");
}

#[test]
fn status_ignores_runtime_public_key_override() {
    const ALT_PRIVATE_KEY_HEX: &str =
        "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";

    let machine_id = "override-machine";
    let temp = tempdir().expect("tempdir");
    support::write_license_signed_by(temp.path(), machine_id, ALT_PRIVATE_KEY_HEX);
    let alt_public_key = derive_public_key_hex(ALT_PRIVATE_KEY_HEX).expect("derive public key");
    assert_ne!(
        alt_public_key,
        derive_public_key_hex(PRIVATE_KEY_HEX).expect("derive embedded public key")
    );

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .env("WXP_LICENSE_PUBLIC_KEY_HEX", alt_public_key)
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("本地激活凭证无效"));
}

#[test]
fn status_ignores_runtime_machine_id_override() {
    let temp = tempdir().expect("tempdir");
    support::write_license(temp.path(), "trusted-machine");

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", "other-machine")
        .env("WXP_LICENSE_MACHINE_ID", "trusted-machine")
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("本地激活凭证无效"));
}

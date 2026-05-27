use std::thread;
use std::time::Duration;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;
use tiny_http::StatusCode;
use wxp_license::{ActivationErrorResponse, ActivationResponse};

mod support;

#[test]
fn activate_persists_certificate_and_status_works_offline_afterward() {
    let machine_id = "activation-machine";
    let (addr, handle) = support::spawn_activation_server_with_certificate(machine_id);

    let temp = tempdir().expect("tempdir");
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .env("WXP_LICENSE_SERVER_URL", format!("http://{addr}"))
        .arg("activate")
        .arg("lic-live-001")
        .assert()
        .success()
        .stdout(predicate::str::contains("激活成功"));

    handle.join().expect("join activation server");
    assert!(temp.path().join(".wxpilot").join("license.json").exists());

    let daemon = support::spawn_status_daemon(temp.path());
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("daemonKind=\"rust\""));
    daemon.join().expect("join daemon");
}

#[test]
fn activate_surfaces_backend_rejection() {
    let machine_id = "rejection-machine";
    let (addr, handle) = support::spawn_activation_server_once(
        StatusCode(409),
        ActivationErrorResponse {
            error: "already_bound".into(),
            message: "license key already bound to another machine".into(),
        },
    );

    let temp = tempdir().expect("tempdir");
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .env("WXP_LICENSE_SERVER_URL", format!("http://{addr}"))
        .arg("activate")
        .arg("lic-live-001")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "license key already bound to another machine",
        ));
    assert!(!temp.path().join(".wxpilot").join("license.json").exists());

    handle.join().expect("join activation server");
    thread::sleep(Duration::from_millis(50));
}

#[test]
fn activate_rejects_mismatched_machine_fingerprint_from_server() {
    let machine_id = "expected-machine";
    let (addr, handle) = support::spawn_activation_server_once(
        StatusCode(200),
        ActivationResponse {
            certificate: support::signed_certificate_for_machine_id("other-machine"),
        },
    );

    let temp = tempdir().expect("tempdir");
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .env("WXP_LICENSE_SERVER_URL", format!("http://{addr}"))
        .arg("activate")
        .arg("lic-live-001")
        .assert()
        .failure()
        .stderr(predicate::str::contains("服务端返回的证书与当前机器不匹配"));
    assert!(!temp.path().join(".wxpilot").join("license.json").exists());

    handle.join().expect("join activation server");
}

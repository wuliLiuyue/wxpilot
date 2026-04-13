use std::thread;
use std::time::Duration;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;
use tiny_http::{Header, Method, Response, Server, StatusCode};
use wxp_license::{
    sign_certificate, ActivationErrorResponse, ActivationResponse, LicenseCertificate,
    DEFAULT_DEV_PRIVATE_KEY_HEX,
};

mod support;

#[test]
fn activate_persists_certificate_and_status_works_offline_afterward() {
    let addr = support::free_tcp_addr();
    let server = Server::http(&addr).expect("bind activation server");
    let handle = thread::spawn(move || {
        let request = server.recv().expect("receive request");
        assert_eq!(request.method(), &Method::Post);
        assert_eq!(request.url(), "/v1/activate");

        let payload = LicenseCertificate {
            version: 1,
            license_id: "lic_test".into(),
            key_id: "dev".into(),
            machine_fingerprint: support::machine_fingerprint("machine-a"),
            issued_at: "2026-04-13T00:00:00Z".into(),
            client_version: "0.1.0".into(),
        };
        let certificate =
            sign_certificate(&payload, DEFAULT_DEV_PRIVATE_KEY_HEX).expect("sign certificate");
        let response = ActivationResponse { certificate };
        let header = Header::from_bytes("Content-Type", "application/json").expect("header");
        request
            .respond(
                Response::from_string(serde_json::to_string(&response).expect("serialize"))
                    .with_status_code(StatusCode(200))
                    .with_header(header),
            )
            .expect("respond");
    });

    let temp = tempdir().expect("tempdir");
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_MACHINE_ID", "machine-a")
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
        .env("WXP_LICENSE_MACHINE_ID", "machine-a")
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("daemonKind=\"rust\""));
    daemon.join().expect("join daemon");
}

#[test]
fn activate_surfaces_backend_rejection() {
    let addr = support::free_tcp_addr();
    let server = Server::http(&addr).expect("bind activation server");
    let handle = thread::spawn(move || {
        let request = server.recv().expect("receive request");
        let response = ActivationErrorResponse {
            error: "already_bound".into(),
            message: "license key already bound to another machine".into(),
        };
        let header = Header::from_bytes("Content-Type", "application/json").expect("header");
        request
            .respond(
                Response::from_string(serde_json::to_string(&response).expect("serialize"))
                    .with_status_code(StatusCode(409))
                    .with_header(header),
            )
            .expect("respond");
    });

    let temp = tempdir().expect("tempdir");
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .env("WXP_LICENSE_MACHINE_ID", "machine-b")
        .env("WXP_LICENSE_SERVER_URL", format!("http://{addr}"))
        .arg("activate")
        .arg("lic-live-001")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "license key already bound to another machine",
        ));

    handle.join().expect("join activation server");
    thread::sleep(Duration::from_millis(50));
}

use std::collections::BTreeSet;
use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;
use tiny_http::StatusCode;
use wxp_license::ActivationErrorResponse;

mod support;

#[derive(Debug)]
struct ScenarioSpec {
    name: &'static str,
    acceptance: &'static [u8],
    verification: &'static str,
}

const PRD_ACCEPTANCE_COUNT: u8 = 8;

const SCENARIOS: &[ScenarioSpec] = &[
    ScenarioSpec {
        name: "license_help_version_activate_whitelist",
        acceptance: &[2, 3],
        verification: "未激活时白名单命令仍可执行，受限命令统一进入授权门禁。",
    },
    ScenarioSpec {
        name: "license_activate_success_persists_certificate",
        acceptance: &[1],
        verification: "激活成功后写入本地可离线校验的证书。",
    },
    ScenarioSpec {
        name: "license_offline_after_activation",
        acceptance: &[4],
        verification: "已激活实例在断网情况下继续执行受限命令。",
    },
    ScenarioSpec {
        name: "license_second_machine_rejected",
        acceptance: &[5],
        verification: "同一 key 在第二台机器上激活时被后端拒绝。",
    },
    ScenarioSpec {
        name: "license_tampered_or_deleted_certificate_is_rejected",
        acceptance: &[6, 7],
        verification: "篡改或删除本地证书后，CLI 重新回到拒绝状态。",
    },
    ScenarioSpec {
        name: "license_release_docs_call_out_activation_requirement",
        acceptance: &[8],
        verification: "README/安装文档明确说明发布版需要激活。",
    },
];

#[test]
fn prd_acceptance_matrix_covers_all_license_requirements() {
    let covered = SCENARIOS
        .iter()
        .flat_map(|scenario| scenario.acceptance.iter().copied())
        .collect::<BTreeSet<_>>();

    let expected = (1..=PRD_ACCEPTANCE_COUNT).collect::<BTreeSet<_>>();
    assert_eq!(
        covered, expected,
        "license activation acceptance coverage drifted"
    );

    for scenario in SCENARIOS {
        assert!(
            !scenario.verification.trim().is_empty(),
            "scenario {} must describe its verification intent",
            scenario.name
        );
    }
}

#[test]
fn allows_help_version_and_activate_without_local_license() {
    let home = tempdir().expect("tempdir");
    let machine_id = "plan-activate-machine";
    let (addr, handle) = support::spawn_activation_server_with_certificate(machine_id);

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .arg("--help")
        .assert()
        .success();

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .arg("--version")
        .assert()
        .success();

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .env("WXP_LICENSE_SERVER_URL", format!("http://{addr}"))
        .args(["activate", "demo-license-key"])
        .assert()
        .success()
        .stdout(predicate::str::contains("激活成功"));

    handle.join().expect("join activation server");
}

#[test]
fn blocks_restricted_commands_when_license_is_missing() {
    let home = tempdir().expect("tempdir");

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .env("USER", "plan-missing-user")
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("当前发布版需要先激活"));
}

#[test]
fn persists_license_after_activation_and_supports_offline_use() {
    let machine_id = "plan-offline-machine";
    let (addr, handle) = support::spawn_activation_server_with_certificate(machine_id);

    let home = tempdir().expect("tempdir");
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .env("WXP_LICENSE_SERVER_URL", format!("http://{addr}"))
        .args(["activate", "demo-license-key"])
        .assert()
        .success();
    handle.join().expect("join activation server");

    let license_path = home.path().join(".wxpilot").join("license.json");
    assert!(license_path.exists(), "license file should exist after activation");

    let daemon = support::spawn_status_daemon(home.path());
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("daemonKind=\"rust\""));
    daemon.join().expect("join daemon");
}

#[test]
fn rejects_second_machine_activation_for_same_key() {
    let (addr, handle) = support::spawn_activation_server_once(
        StatusCode(409),
        ActivationErrorResponse {
            error: "already_bound".into(),
            message: "license key already bound to another machine".into(),
        },
    );

    let home = tempdir().expect("tempdir");
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", "plan-second-machine")
        .env("WXP_LICENSE_SERVER_URL", format!("http://{addr}"))
        .args(["activate", "demo-license-key"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "license key already bound to another machine",
        ));
    assert!(!home.path().join(".wxpilot").join("license.json").exists());

    handle.join().expect("join activation server");
}

#[test]
fn rejects_tampered_or_deleted_local_license() {
    let machine_id = "plan-tampered-machine";
    let home = tempdir().expect("tempdir");
    support::write_license(home.path(), machine_id);
    let license_path = home.path().join(".wxpilot").join("license.json");

    let raw = fs::read_to_string(&license_path).expect("read license");
    fs::write(&license_path, raw.replace("lic_test", "lic_other")).expect("write tampered");

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("本地激活凭证无效"));

    fs::remove_file(&license_path).expect("delete license");
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .env("WXP_LICENSE_TEST_MACHINE_ID", machine_id)
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("当前发布版需要先激活"));
}

#[test]
fn release_docs_explicitly_call_out_activation_requirement() {
    let readme = fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../../README.md"),
    )
    .expect("read README");

    assert!(readme.contains("发布版**不是安装即用**"));
    assert!(readme.contains("开发者验证"));
    assert!(readme.contains("不承担正式授权发放职责"));
    assert!(readme.contains("WXP_LICENSE_SERVER_URL"));
    assert!(readme.contains("wxpilot activate <key>"));
}

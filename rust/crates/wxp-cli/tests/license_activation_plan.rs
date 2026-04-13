use std::collections::BTreeSet;

use assert_cmd::Command;
use tempfile::tempdir;

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
#[ignore = "等待 worker-1 落地授权实现后启用"]
fn allows_help_version_and_activate_without_local_license() {
    let home = tempdir().expect("tempdir");

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
        .args(["activate", "demo-license-key"])
        .assert()
        .success();
}

#[test]
#[ignore = "等待 worker-1 落地授权实现后启用"]
fn blocks_restricted_commands_when_license_is_missing() {
    let home = tempdir().expect("tempdir");

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", home.path())
        .arg("status")
        .assert()
        .failure();
}

#[test]
#[ignore = "等待 worker-1 落地授权实现后启用"]
fn persists_license_after_activation_and_supports_offline_use() {
    // 预期流程：
    // 1. 启动本地激活桩服务，返回签名证书。
    // 2. 执行 `wxpilot activate <key>` 成功。
    // 3. 验证 ~/.wxpilot/license.json 存在且包含可离线校验字段。
    // 4. 关闭激活服务后再次执行受限命令，命令仍可通过门禁。
    todo!("activation happy path pending implementation");
}

#[test]
#[ignore = "等待 worker-1 落地授权实现后启用"]
fn rejects_second_machine_activation_for_same_key() {
    // 预期流程：
    // 1. machine-A 首次激活成功。
    // 2. machine-B 使用同一 key 再次激活。
    // 3. CLI 显示后端拒绝信息且不落地本地证书。
    todo!("single-machine binding pending implementation");
}

#[test]
#[ignore = "等待 worker-1 落地授权实现后启用"]
fn rejects_tampered_or_deleted_local_license() {
    // 预期流程：
    // 1. 先准备一个有效激活态。
    // 2. 篡改证书任意字段或删除证书文件。
    // 3. `status` / `start` / `connect` 等受限命令重新被门禁拦截。
    todo!("tamper/delete certificate pending implementation");
}

#[test]
#[ignore = "等待 worker-3 更新文档后启用"]
fn release_docs_explicitly_call_out_activation_requirement() {
    // 预期校验 README 和安装说明：
    // - 发布版需要先激活
    // - 安装完成不等于可直接使用
    todo!("release docs activation note pending documentation update");
}

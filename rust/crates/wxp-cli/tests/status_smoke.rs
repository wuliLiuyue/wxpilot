use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

mod support;

#[test]
fn status_command_round_trips_against_stub_daemon() {
    let machine_id = "status-smoke-machine";
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
        .stdout(contains(r#"daemonKind="rust""#));

    server.join().expect("join");
}

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

mod support;

#[test]
fn status_command_round_trips_against_stub_daemon() {
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
        .stdout(contains(r#"daemonKind="rust""#));

    server.join().expect("join");
}

use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use wxp_daemon::DevtoolsLauncher;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn unique_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("wxp-devtools-{name}-{nanos}"))
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("set executable");
}

fn write_cli_script(path: &Path, body: &str) {
    let mut script = fs::File::create(path).expect("create cli script");
    writeln!(script, "#!/bin/sh\nset -eu\n{body}").expect("write cli script");
    #[cfg(unix)]
    make_executable(path);
}

fn write_success_script(path: &Path, with_appid: bool, marker: &str) {
    let body = if with_appid {
        format!(
            r#"
if [ "$1" != "auto" ]; then exit 2; fi
if [ "$2" != "--project" ]; then exit 3; fi
if [ "$4" != "--auto-port" ]; then exit 4; fi
if [ "${{6:-}}" != "--appid" ]; then exit 5; fi
if [ "${{7:-}}" != "wx1234567890" ]; then exit 6; fi
printf '{marker}\n'
"#
        )
    } else {
        format!(
            r#"
if [ "$1" != "auto" ]; then exit 2; fi
if [ "$2" != "--project" ]; then exit 3; fi
if [ "$4" != "--auto-port" ]; then exit 4; fi
if [ "${{6:-}}" != "" ]; then exit 5; fi
printf '{marker}\n'
"#
        )
    };
    write_cli_script(path, &body);
}

fn write_failure_script(path: &Path, exit_code: i32) {
    let body = format!(
        r#"
if [ "$1" != "auto" ]; then exit 2; fi
printf 'failed on purpose\n' >&2
exit {exit_code}
"#
    );
    write_cli_script(path, &body);
}

fn write_timeout_script(path: &Path) {
    let body = r#"
if [ "$1" != "auto" ]; then exit 2; fi
sleep 5
printf 'auto success\n'
"#;
    write_cli_script(path, body);
}

#[test]
fn find_cli_returns_not_found_for_missing_explicit_path() {
    let launcher = DevtoolsLauncher::new(vec![]);
    let missing = unique_test_dir("missing").join("cli");

    let error = launcher
        .find_cli(Some(missing))
        .expect_err("missing explicit path should fail");

    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[test]
fn find_cli_rejects_non_file_explicit_path() {
    let root = unique_test_dir("dir");
    fs::create_dir_all(&root).expect("create temp root");
    let launcher = DevtoolsLauncher::new(vec![]);

    let error = launcher
        .find_cli(Some(root))
        .expect_err("directory should not be accepted");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
}

#[cfg(unix)]
#[test]
fn find_cli_rejects_non_executable_explicit_path() {
    let root = unique_test_dir("non-exec");
    fs::create_dir_all(&root).expect("create temp root");
    let cli_path = root.join("cli");
    fs::write(&cli_path, "#!/bin/sh\nexit 0\n").expect("write cli");
    let launcher = DevtoolsLauncher::new(vec![]);

    let error = launcher
        .find_cli(Some(cli_path))
        .expect_err("non-executable file should not be accepted");

    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
}

#[cfg(unix)]
#[test]
fn run_auto_returns_ws_endpoint_when_cli_reports_success_without_appid() {
    let root = unique_test_dir("run-auto-no-appid");
    fs::create_dir_all(&root).expect("create temp root");
    let cli_path = root.join("cli");
    write_success_script(&cli_path, false, "auto success");

    let launcher = DevtoolsLauncher::new(vec![]);
    let endpoint = launcher
        .run_auto("/tmp/project", 9420, Some(cli_path), None)
        .expect("run auto");

    assert_eq!(endpoint, "ws://127.0.0.1:9420");
}

#[cfg(unix)]
#[test]
fn run_auto_returns_error_when_cli_exits_non_zero() {
    let root = unique_test_dir("run-auto-failure");
    fs::create_dir_all(&root).expect("create temp root");
    let cli_path = root.join("cli");
    write_failure_script(&cli_path, 17);

    let launcher = DevtoolsLauncher::new(vec![]);
    let error = launcher
        .run_auto("/tmp/project", 9420, Some(cli_path), Some("wx1234567890"))
        .expect_err("non-zero exit should fail");

    assert_eq!(error.kind(), ErrorKind::Other);
}

#[cfg(unix)]
#[test]
fn run_auto_times_out_when_cli_hangs() {
    let root = unique_test_dir("run-auto-timeout");
    fs::create_dir_all(&root).expect("create temp root");
    let cli_path = root.join("cli");
    write_timeout_script(&cli_path);

    let launcher = DevtoolsLauncher::new(vec![]).with_run_auto_timeout(Duration::from_millis(100));
    let error = launcher
        .run_auto("/tmp/project", 9420, Some(cli_path), Some("wx1234567890"))
        .expect_err("hanging cli should time out");

    assert_eq!(error.kind(), ErrorKind::TimedOut);
}

#[cfg(unix)]
#[test]
fn run_auto_returns_ws_endpoint_when_cli_reports_success_with_appid() {
    let root = unique_test_dir("run-auto-appid");
    fs::create_dir_all(&root).expect("create temp root");
    let cli_path = root.join("cli");
    write_success_script(&cli_path, true, "自动化已开启");

    let launcher = DevtoolsLauncher::new(vec![]);
    let endpoint = launcher
        .run_auto("/tmp/project", 9420, Some(cli_path), Some("wx1234567890"))
        .expect("run auto");

    assert_eq!(endpoint, "ws://127.0.0.1:9420");
}

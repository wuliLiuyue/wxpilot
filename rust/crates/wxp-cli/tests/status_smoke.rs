use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::thread;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn status_command_round_trips_against_stub_daemon() {
    let temp = tempdir().expect("tempdir");
    let runtime = temp.path().join(".wxpilot");
    std::fs::create_dir_all(&runtime).expect("mkdir runtime");
    let socket_path = runtime.join("rust-daemon.sock");
    let listener = UnixListener::bind(&socket_path).expect("bind");

    let server = thread::spawn(move || {
        let (_probe, _) = listener.accept().expect("accept probe");
        let (mut stream, _) = listener.accept().expect("accept rpc");
        let mut line = String::new();
        BufReader::new(stream.try_clone().expect("clone"))
            .read_line(&mut line)
            .expect("read");
        assert!(line.contains("\"method\":\"status\""));
        writeln!(
            stream,
            "{{\"jsonrpc\":\"2.0\",\"id\":\"1\",\"result\":{{\"ok\":true,\"connected\":false,\"projectPath\":null,\"proxy\":null}}}}"
        )
        .expect("write");
    });

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(contains(r#"daemonKind="rust""#));

    server.join().expect("join");
}

#![allow(dead_code)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::thread::{self, JoinHandle};

use sha2::{Digest, Sha256};
use wxp_license::{sign_certificate, LicenseCertificate, DEFAULT_DEV_PRIVATE_KEY_HEX};

pub fn machine_fingerprint(machine_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(machine_id.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn write_license(home: &Path, machine_id: &str) {
    let runtime = home.join(".wxpilot");
    fs::create_dir_all(&runtime).expect("create runtime dir");
    let payload = LicenseCertificate {
        version: 1,
        license_id: "lic_test".into(),
        key_id: "dev".into(),
        machine_fingerprint: machine_fingerprint(machine_id),
        issued_at: "2026-04-13T00:00:00Z".into(),
        client_version: "0.1.0".into(),
    };
    let certificate =
        sign_certificate(&payload, DEFAULT_DEV_PRIVATE_KEY_HEX).expect("sign certificate");
    let body = serde_json::json!({ "certificate": certificate });
    fs::write(
        runtime.join("license.json"),
        serde_json::to_string_pretty(&body).expect("serialize license"),
    )
    .expect("write license");
}

pub fn spawn_status_daemon(home: &Path) -> JoinHandle<()> {
    let runtime = home.join(".wxpilot");
    fs::create_dir_all(&runtime).expect("create runtime dir");
    let socket_path = runtime.join("rust-daemon.sock");
    if socket_path.exists() {
        fs::remove_file(&socket_path).expect("remove stale socket");
    }
    let listener = UnixListener::bind(&socket_path).expect("bind unix listener");
    thread::spawn(move || {
        let (_probe, _) = listener.accept().expect("accept probe");
        let (mut stream, _) = listener.accept().expect("accept rpc");
        let mut line = String::new();
        BufReader::new(stream.try_clone().expect("clone stream"))
            .read_line(&mut line)
            .expect("read request");
        assert!(line.contains("\"method\":\"status\""));
        writeln!(
            stream,
            "{{\"jsonrpc\":\"2.0\",\"id\":\"1\",\"result\":{{\"ok\":true,\"connected\":false,\"projectPath\":null,\"proxy\":null}}}}"
        )
        .expect("write response");
    })
}

pub fn free_tcp_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind free port");
    let addr = listener.local_addr().expect("read local addr");
    drop(listener);
    format!("127.0.0.1:{}", addr.port())
}

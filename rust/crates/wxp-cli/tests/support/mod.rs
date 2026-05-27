#![allow(dead_code)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::thread::{self, JoinHandle};

use sha2::{Digest, Sha256};
use tiny_http::{Header, Method, Response, Server, StatusCode};
use wxp_license::{
    sign_certificate, test_keys::PRIVATE_KEY_HEX, ActivationResponse, LicenseCertificate,
    SignedLicenseCertificate,
};

pub fn machine_fingerprint(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn runtime_machine_fingerprint(machine_id: &str) -> String {
    machine_fingerprint(machine_id)
}

pub fn write_license(home: &Path, machine_id: &str) {
    write_license_signed_by(home, machine_id, PRIVATE_KEY_HEX)
}

pub fn write_license_signed_by(home: &Path, machine_id: &str, private_key_hex: &str) {
    let runtime = home.join(".wxpilot");
    fs::create_dir_all(&runtime).expect("create runtime dir");
    let payload = LicenseCertificate {
        version: 1,
        license_id: "lic_test".into(),
        key_id: "dev".into(),
        machine_fingerprint: runtime_machine_fingerprint(machine_id),
        issued_at: "2026-04-13T00:00:00Z".into(),
        client_version: "0.1.0".into(),
    };
    let certificate = sign_certificate(&payload, private_key_hex).expect("sign certificate");
    let body = serde_json::json!({ "certificate": certificate });
    fs::write(
        runtime.join("license.json"),
        serde_json::to_string_pretty(&body).expect("serialize license"),
    )
    .expect("write license");
}

pub fn signed_certificate_for_machine_id(machine_id: &str) -> SignedLicenseCertificate {
    signed_certificate_for_machine_fingerprint(&runtime_machine_fingerprint(machine_id))
}

pub fn signed_certificate_for_machine_fingerprint(machine_fingerprint: &str) -> SignedLicenseCertificate {
    let payload = LicenseCertificate {
        version: 1,
        license_id: "lic_test".into(),
        key_id: "dev".into(),
        machine_fingerprint: machine_fingerprint.into(),
        issued_at: "2026-04-13T00:00:00Z".into(),
        client_version: "0.1.0".into(),
    };
    sign_certificate(&payload, PRIVATE_KEY_HEX).expect("sign certificate")
}

pub fn spawn_activation_server_once<T>(status: StatusCode, body: T) -> (String, JoinHandle<()>)
where
    T: serde::Serialize + Send + 'static,
{
    for _ in 0..10 {
        let addr = free_tcp_addr();
        match Server::http(&addr) {
            Ok(server) => {
                let handle = thread::spawn(move || {
                    let request = server.recv().expect("receive request");
                    assert_eq!(request.method(), &Method::Post);
                    assert_eq!(request.url(), "/v1/activate");
                    let header =
                        Header::from_bytes("Content-Type", "application/json").expect("header");
                    request
                        .respond(
                            Response::from_string(serde_json::to_string(&body).expect("serialize"))
                                .with_status_code(status)
                                .with_header(header),
                        )
                        .expect("respond");
                });
                return (addr, handle);
            }
            Err(error) if error.to_string().contains("Address already in use") => continue,
            Err(error) => panic!("bind activation server: {error}"),
        }
    }

    panic!("bind activation server: retry budget exhausted");
}

pub fn spawn_activation_server_with_certificate(machine_id: &str) -> (String, JoinHandle<()>) {
    spawn_activation_server_once(
        StatusCode(200),
        ActivationResponse {
            certificate: signed_certificate_for_machine_id(machine_id),
        },
    )
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

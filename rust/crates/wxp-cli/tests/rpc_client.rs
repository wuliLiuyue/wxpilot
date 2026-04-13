use std::io::Write;
use std::os::unix::net::UnixListener;
use std::thread;

use tempfile::tempdir;

#[test]
fn rpc_call_returns_error_when_response_is_not_json() {
    let temp = tempdir().expect("tempdir");
    let socket_path = temp.path().join("rpc.sock");
    let listener = UnixListener::bind(&socket_path).expect("bind");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream.write_all(b"not json\n").expect("write");
    });

    let error = wxp_cli::rpc_client::call(&socket_path, "health", serde_json::json!({}))
        .expect_err("invalid json should fail");
    server.join().expect("join");

    assert!(!error.is_empty());
}

use wxp_common::{
    DAEMON_KIND, DEFAULT_TIMEOUT_MS, ERROR_INTERNAL, ERROR_NOT_CONNECTED, ERROR_NOT_FOUND,
    ERROR_REF_EXPIRED, ERROR_TIMEOUT, MAX_OUTPUT_CHARS, RUST_PID_FILE, RUST_SOCKET_FILE,
};

#[test]
fn freezes_public_contract_constants_in_rust() {
    assert_eq!(DAEMON_KIND, "rust");
    assert_eq!(RUST_SOCKET_FILE, "rust-daemon.sock");
    assert_eq!(RUST_PID_FILE, "rust-daemon.pid");
    assert_eq!(DEFAULT_TIMEOUT_MS, 10_000);
    assert_eq!(MAX_OUTPUT_CHARS, 4_000);
    assert_eq!(ERROR_REF_EXPIRED, -32000);
    assert_eq!(ERROR_NOT_CONNECTED, -32001);
    assert_eq!(ERROR_TIMEOUT, -32002);
    assert_eq!(ERROR_NOT_FOUND, -32003);
    assert_eq!(ERROR_INTERNAL, -32099);
}

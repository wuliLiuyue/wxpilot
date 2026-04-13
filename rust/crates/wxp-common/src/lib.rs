pub const DAEMON_KIND: &str = "rust";
pub const RUST_SOCKET_FILE: &str = "rust-daemon.sock";
pub const RUST_PID_FILE: &str = "rust-daemon.pid";
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
pub const MAX_OUTPUT_CHARS: usize = 4_000;

pub const ERROR_REF_EXPIRED: i32 = -32000;
pub const ERROR_NOT_CONNECTED: i32 = -32001;
pub const ERROR_TIMEOUT: i32 = -32002;
pub const ERROR_NOT_FOUND: i32 = -32003;
pub const ERROR_INTERNAL: i32 = -32099;

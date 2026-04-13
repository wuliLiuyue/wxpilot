use serde::Serialize;

pub mod action_path;
pub mod bootstrap_guard;
pub mod ca;
pub mod devtools;
pub mod handlers;
pub mod net_path;
pub mod read_path;
pub mod runtime;
pub mod server;

pub use devtools::DevtoolsLauncher;
pub use handlers::format_find_output;
pub use runtime::{DaemonState, StateStore};
pub use server::{dispatch_local_request, RuntimePaths, ServerState};

pub const PUBLIC_METHODS: &[&str] = &[
    "start",
    "connect",
    "disconnect",
    "status",
    "goto",
    "back",
    "reload",
    "view",
    "find",
    "wait",
    "tap",
    "type",
    "scroll",
    "read",
    "assert",
    "state",
    "shot",
    "run",
    "wx",
    "net.start",
    "net.stop",
    "net.log",
    "net.mock",
    "net.unmock",
    "net.clear",
    "net.install-ca",
    "daemon.stop",
];

#[derive(Debug, Serialize)]
pub struct ProbePayload {
    pub daemon_kind: &'static str,
    pub status: &'static str,
}

pub fn build_probe_payload() -> ProbePayload {
    ProbePayload {
        daemon_kind: wxp_common::DAEMON_KIND,
        status: "ok",
    }
}

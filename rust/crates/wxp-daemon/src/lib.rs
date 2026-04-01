use serde::Serialize;

pub mod bootstrap_guard;
pub mod devtools;
pub mod handlers;
pub mod runtime;
pub mod server;

pub use devtools::DevtoolsLauncher;
pub use handlers::format_find_output;
pub use runtime::{DaemonState, StateStore};
pub use server::{dispatch_local_request, RuntimePaths, ServerState};

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

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use wxp_common::{DAEMON_KIND, RUST_PID_FILE, RUST_SOCKET_FILE};

pub struct ProbeResult {
    pub socket_path: PathBuf,
    pub daemon_kind: String,
}

pub type ConnectFuture = Pin<Box<dyn Future<Output = Result<ProbeResult, String>> + Send>>;
pub type WaitFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
pub type StopFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
pub type ProbeFuture = Pin<Box<dyn Future<Output = bool> + Send>>;

pub struct ConnectDeps {
    pub connect: Box<dyn Fn(PathBuf) -> ConnectFuture + Send + Sync>,
    pub spawn_daemon: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
    pub wait: Box<dyn Fn(u64) -> WaitFuture + Send + Sync>,
}

pub struct StopResult {
    pub forced: bool,
    pub killed_pids: Vec<u32>,
}

pub struct StopDeps {
    pub rpc_stop: Box<dyn Fn() -> StopFuture + Send + Sync>,
    pub probe_running: Box<dyn Fn() -> ProbeFuture + Send + Sync>,
    pub pid_from_file: Option<u32>,
    pub listed_pids: Vec<u32>,
    pub kill_pid: Box<dyn Fn(u32) -> Result<(), String> + Send + Sync>,
}

fn runtime_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".wxpilot")
}

pub async fn connect_to_daemon(deps: ConnectDeps) -> Result<ProbeResult, String> {
    let socket = runtime_dir().join(RUST_SOCKET_FILE);
    if let Ok(result) = (deps.connect)(socket.clone()).await {
        return Ok(result);
    }

    (deps.spawn_daemon)()?;

    for attempt in 0..5 {
        if attempt > 0 {
            (deps.wait)(500).await;
        }

        if let Ok(result) = (deps.connect)(socket.clone()).await {
            return Ok(result);
        }
    }

    Err(format!(
        "daemon_unavailable: failed to connect to {DAEMON_KIND} daemon"
    ))
}

pub async fn stop_daemon(deps: StopDeps) -> Result<StopResult, String> {
    let _socket_path = runtime_dir().join(RUST_SOCKET_FILE);
    let _pid_path = runtime_dir().join(RUST_PID_FILE);
    let mut target_pids = Vec::new();

    let _ = (deps.rpc_stop)().await;

    if let Some(pid) = deps.pid_from_file {
        target_pids.push(pid);
    }

    if !(deps.probe_running)().await {
        target_pids.extend(deps.listed_pids);
    }

    let mut killed_pids = Vec::new();
    for pid in target_pids {
        if (deps.kill_pid)(pid).is_ok() {
            killed_pids.push(pid);
        }
    }

    Ok(StopResult {
        forced: !killed_pids.is_empty(),
        killed_pids,
    })
}

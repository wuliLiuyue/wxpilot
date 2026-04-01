use std::{
    fs, io,
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    process,
};

use serde_json::{json, Value};

use crate::{
    devtools::DevtoolsLauncher,
    runtime::{DaemonRuntime, StateStore},
};

#[derive(Debug, Clone)]
pub struct RuntimePaths {
    pub root_dir: PathBuf,
    pub rust_socket: PathBuf,
    pub rust_pid: PathBuf,
    pub state_file: PathBuf,
    pub db_file: PathBuf,
    pub ca_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl Default for RuntimePaths {
    fn default() -> Self {
        let root_dir = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".wxpilot")
            .join("rust");
        let rust_socket = root_dir.join("rust-daemon.sock");
        let rust_pid = root_dir.join("rust-daemon.pid");
        let state_file = root_dir.join("rust-daemon-state.json");
        let db_file = root_dir.join("rust-daemon.db");
        let ca_dir = root_dir.join("ca");
        let log_dir = root_dir.join("logs");

        Self {
            root_dir,
            rust_socket,
            rust_pid,
            state_file,
            db_file,
            ca_dir,
            log_dir,
        }
    }
}

#[derive(Debug)]
pub struct ServerState {
    pub daemon_kind: &'static str,
    pub paths: RuntimePaths,
    pub store: StateStore,
    pub launcher: DevtoolsLauncher,
    pub runtime: DaemonRuntime,
}

impl ServerState {
    pub fn new(paths: RuntimePaths) -> Self {
        let store = StateStore::new(paths.state_file.clone());
        let mut saved = store.load().unwrap_or_default();
        if saved.connected {
            saved.connected = false;
            let _ = store.save(&saved);
        }
        Self {
            daemon_kind: wxp_common::DAEMON_KIND,
            paths,
            launcher: DevtoolsLauncher::new(DevtoolsLauncher::default_candidates()),
            runtime: DaemonRuntime::new(saved),
            store,
        }
    }

    fn persist_runtime_state(&mut self, saved: crate::runtime::DaemonState) -> io::Result<()> {
        self.store.save(&saved)?;
        self.runtime.saved = saved;
        Ok(())
    }

    fn clear_connection_state(&mut self) -> io::Result<()> {
        self.runtime.connection = None;
        let mut saved = self.runtime.saved.clone();
        saved.connected = false;
        self.persist_runtime_state(saved)
    }

    async fn read_state_data(&mut self, request_id: &Value) -> io::Result<Value> {
        let connection = self.runtime.connection.clone().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "automator_not_connected")
        })?;
        let request_tag = request_id.to_string();
        let current_page = connection
            .send_with_id(
                format!("{request_tag}:current_page"),
                "App.getCurrentPage",
                json!({}),
            )
            .await
            .map_err(|error| {
                let _ = self.clear_connection_state();
                io::Error::other(error)
            })?;
        let page_id = current_page
            .get("pageId")
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "page_id_missing"))?;
        let data = connection
            .send_with_id(
                format!("{request_tag}:page_data"),
                "Page.getData",
                json!({ "pageId": page_id }),
            )
            .await
            .map_err(|error| {
                let _ = self.clear_connection_state();
                io::Error::other(error)
            })?;

        Ok(data.get("data").cloned().unwrap_or(Value::Null))
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new(RuntimePaths::default())
    }
}

struct RuntimeGuard {
    socket_path: PathBuf,
    pid_path: PathBuf,
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
        let _ = fs::remove_file(&self.pid_path);
    }
}

pub fn prepare_rust_socket(socket_path: &Path) -> io::Result<()> {
    if !socket_path.exists() {
        return Ok(());
    }

    match UnixStream::connect(socket_path) {
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "rust daemon socket is already active",
        )),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
            ) =>
        {
            fs::remove_file(socket_path)?;
            Ok(())
        }
        Err(error) => Err(error),
    }
}

pub fn dispatch_local_request(state: &ServerState, request: &Value) -> io::Result<Value> {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing method"))?;

    let id = request.get("id").cloned().unwrap_or(Value::Null);

    match method {
        "status" => {
            let effective_connected =
                state.runtime.connection.is_some() && state.runtime.saved.connected;
            let saved = &state.runtime.saved;
            Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "ok": true,
                    "connected": effective_connected,
                    "projectPath": saved.project_path.clone(),
                    "proxy": Value::Null,
                    "daemonKind": state.daemon_kind,
                }
            }))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported method: {method}"),
        )),
    }
}

pub async fn dispatch_request(state: &mut ServerState, request: &Value) -> io::Result<Value> {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing method"))?;
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    match method {
        "start" => {
            let project_path = params
                .get("projectPath")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing projectPath"))?
                .to_string();
            let port = params
                .get("port")
                .and_then(Value::as_u64)
                .map(u16::try_from)
                .transpose()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "port_out_of_range"))?
                .unwrap_or(9420);
            let endpoint = state.launcher.run_auto(
                &project_path,
                port,
                None,
                params.get("appid").and_then(Value::as_str),
            )?;

            let mut next_saved = state.runtime.saved.clone();
            next_saved.project_path = Some(project_path);
            next_saved.ws_endpoint = Some(endpoint.clone());
            next_saved.connected = false;
            state.persist_runtime_state(next_saved)?;
            state.runtime.connection = None;

            Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "ok": true,
                    "wsEndpoint": endpoint,
                    "port": port,
                }
            }))
        }
        "connect" => {
            let endpoint =
                state.runtime.saved.ws_endpoint.clone().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "ws_endpoint_missing")
                })?;
            let connection = wxp_automator_client::AutomatorConnection::connect(&endpoint)
                .await
                .map_err(io::Error::other)?;
            let mut next_saved = state.runtime.saved.clone();
            next_saved.connected = true;
            state.persist_runtime_state(next_saved)?;
            state.runtime.connection = Some(connection);

            Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "ok": true,
                    "wsEndpoint": endpoint,
                }
            }))
        }
        "state" => {
            let data = state.read_state_data(&id).await?;
            Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "ok": true,
                    "data": data,
                }
            }))
        }
        _ => dispatch_local_request(state, request),
    }
}

pub fn serve_forever(state: ServerState) -> io::Result<()> {
    fs::create_dir_all(&state.paths.root_dir)?;
    fs::create_dir_all(&state.paths.ca_dir)?;
    fs::create_dir_all(&state.paths.log_dir)?;
    if let Some(parent) = state.paths.state_file.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = state.paths.db_file.parent() {
        fs::create_dir_all(parent)?;
    }
    prepare_rust_socket(&state.paths.rust_socket)?;
    fs::write(&state.paths.rust_pid, process::id().to_string())?;

    let _guard = RuntimeGuard {
        socket_path: state.paths.rust_socket.clone(),
        pid_path: state.paths.rust_pid.clone(),
    };
    let listener = UnixListener::bind(&state.paths.rust_socket)?;

    for incoming in listener.incoming() {
        let _stream = incoming?;
        let _ = &state;
    }

    Ok(())
}

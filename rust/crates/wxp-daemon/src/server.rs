use std::{
    fs,
    io::{self, BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    process,
    time::Duration,
};

use serde_json::{json, Value};
use tokio::time::sleep;

use crate::{
    action_path,
    devtools::DevtoolsLauncher,
    net_path, read_path,
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
            .join(".wxpilot");
        let rust_socket = root_dir.join("rust-daemon.sock");
        let rust_pid = root_dir.join("rust-daemon.pid");
        let state_file = root_dir.join("rust-daemon-state.json");
        let db_file = root_dir.join("rust-daemon.db");
        let ca_dir = root_dir.join("rust-ca");
        let log_dir = root_dir.join("rust-logs");

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
    pub default_proxy_port: u16,
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
            default_proxy_port: 8899,
            launcher: DevtoolsLauncher::new(DevtoolsLauncher::default_candidates()),
            runtime: DaemonRuntime::new(saved),
            store,
        }
    }

    pub(crate) fn persist_runtime_state(
        &mut self,
        saved: crate::runtime::DaemonState,
    ) -> io::Result<()> {
        self.store.save(&saved)?;
        self.runtime.saved = saved;
        Ok(())
    }

    pub(crate) fn clear_connection_state(&mut self) -> io::Result<()> {
        self.runtime.connection = None;
        let mut saved = self.runtime.saved.clone();
        saved.connected = false;
        self.persist_runtime_state(saved)
    }

    pub(crate) async fn call_automator(
        &mut self,
        request_id: &Value,
        suffix: &str,
        method: &str,
        params: Value,
    ) -> io::Result<Value> {
        let connection = self.runtime.connection.clone().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "automator_not_connected")
        })?;
        let request_tag = request_id.to_string();
        connection
            .send_with_id(format!("{request_tag}:{suffix}"), method, params)
            .await
            .map_err(|error| {
                let _ = self.clear_connection_state();
                io::Error::other(error)
            })
    }

    async fn read_state_data(&mut self, request_id: &Value) -> io::Result<Value> {
        let current_page = self
            .call_automator(request_id, "current_page", "App.getCurrentPage", json!({}))
            .await?;
        let page_id = current_page
            .get("pageId")
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "page_id_missing"))?;
        let data = self
            .call_automator(
                request_id,
                "page_data",
                "Page.getData",
                json!({ "pageId": page_id }),
            )
            .await?;

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
        "health" => Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": crate::handlers::handle_probe("health"),
        })),
        "version" => Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": crate::handlers::handle_probe("version"),
        })),
        "capabilities" => Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": crate::handlers::handle_probe("capabilities"),
        })),
        "status" => {
            let effective_connected =
                state.runtime.connection.is_some() && state.runtime.saved.connected;
            let saved = &state.runtime.saved;
            let proxy = state
                .runtime
                .proxy
                .port()
                .map(|port| json!({ "port": port }))
                .unwrap_or(Value::Null);
            Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "ok": true,
                    "connected": effective_connected,
                    "projectPath": saved.project_path.clone(),
                    "proxy": proxy,
                    "daemonKind": state.daemon_kind,
                }
            }))
        }
        "daemon.stop" => Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "ok": true,
            }
        })),
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
            let appid = params.get("appid").and_then(Value::as_str);
            let endpoint = match state.launcher.run_auto(&project_path, port, None, appid) {
                Ok(endpoint) => endpoint,
                Err(error)
                    if is_proxy_error(&error.to_string()) && !state.runtime.proxy.is_running() =>
                {
                    state.runtime.proxy.start(state.default_proxy_port)?;
                    state.launcher.run_auto(&project_path, port, None, appid)?
                }
                Err(error) => return Err(error),
            };

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
            let mut last_error = None;
            let mut connection = None;
            for attempt in 0..5 {
                if attempt > 0 {
                    sleep(Duration::from_secs(2)).await;
                }
                match wxp_automator_client::AutomatorConnection::connect(&endpoint).await {
                    Ok(next) => {
                        connection = Some(next);
                        break;
                    }
                    Err(error) => last_error = Some(error),
                }
            }
            let connection = connection.ok_or_else(|| {
                io::Error::other(
                    last_error
                        .map(|error| error.to_string())
                        .unwrap_or_else(|| "connect failed".to_string()),
                )
            })?;
            wait_for_current_page_ready(&connection).await?;
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
        "view" => read_path::handle_view(state, id, &params).await,
        "find" => read_path::handle_find(state, id, &params).await,
        "wait" => read_path::handle_wait(state, id, &params).await,
        "read" => read_path::handle_read(state, id, &params).await,
        "assert" => read_path::handle_assert(state, id, &params).await,
        "goto" | "back" | "reload" | "tap" | "type" | "scroll" => {
            action_path::handle_action(state, id, method, &params).await
        }
        "net.start" | "net.stop" | "net.log" | "net.mock" | "net.unmock" | "net.clear"
        | "net.install-ca" => net_path::handle_net_request(state, id, method, &params).await,
        _ => dispatch_local_request(state, request),
    }
}

fn write_json_line(stream: &mut UnixStream, value: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *stream, value).map_err(io::Error::other)?;
    stream.write_all(b"\n")?;
    stream.flush()
}

fn invalid_request_response(id: Value, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32099,
            "message": message,
        }
    })
}

fn is_proxy_error(message: &str) -> bool {
    message.contains("8899")
        || message.contains("tunneling socket")
        || message.contains("TLS connection")
        || message.contains("ECONNRESET")
        || message.contains("Failed to fetch")
}

async fn wait_for_current_page_ready(
    connection: &wxp_automator_client::AutomatorConnection,
) -> io::Result<()> {
    let mut last_error = None;
    for attempt in 0..5 {
        if attempt > 0 {
            sleep(Duration::from_secs(2)).await;
        }

        match connection
            .send_with_id(
                format!("connect_probe:{attempt}"),
                "App.getCurrentPage",
                json!({}),
            )
            .await
        {
            Ok(_) => return Ok(()),
            Err(error) => last_error = Some(error.to_string()),
        }
    }

    Err(io::Error::other(
        last_error.unwrap_or_else(|| "current_page_probe_failed".to_string()),
    ))
}

fn handle_stream(
    state: &mut ServerState,
    runtime: &tokio::runtime::Runtime,
    mut stream: UnixStream,
) -> io::Result<bool> {
    let reader_stream = stream.try_clone()?;
    let mut reader = BufReader::new(reader_stream);

    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return Ok(false);
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(trimmed) {
            Ok(request) => request,
            Err(error) => {
                let response =
                    invalid_request_response(Value::Null, &format!("invalid_json: {error}"));
                write_json_line(&mut stream, &response)?;
                continue;
            }
        };

        let should_stop = request
            .get("method")
            .and_then(Value::as_str)
            .is_some_and(|method| method == "daemon.stop");

        let request_id = request.get("id").cloned().unwrap_or(Value::Null);
        let response: Value = match runtime.block_on(dispatch_request(state, &request)) {
            Ok(response) => response,
            Err(error) => invalid_request_response(request_id, &error.to_string()),
        };
        write_json_line(&mut stream, &response)?;

        if should_stop {
            return Ok(true);
        }
    }
}

pub fn serve_forever(state: ServerState) -> io::Result<()> {
    let mut state = state;
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
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(io::Error::other)?;

    for incoming in listener.incoming() {
        let stream = incoming?;
        if handle_stream(&mut state, &runtime, stream)? {
            break;
        }
    }

    Ok(())
}

# Rust Daemon Native Realignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在保留现有 CLI 命令面与可显式选择 Rust/TS daemon 的前提下，把 Rust daemon 拉回技术方案规定的“原生实现”路线，并补齐首个可回归的 Rust 读路径闭环。

**Architecture:** 当前仓库已经完成契约冻结、CLI daemon 选择逻辑和 Rust workspace 骨架，但 Rust 业务层仍是 stub。后续实现必须坚持单-daemon模型：Rust daemon 只能依赖 Rust runtime、Rust automator client、Rust snapshot/ref/proxy/store，CLI 负责连接并拉起显式指定的 daemon，Rust 运行时内部禁止再出现 `TS daemon` socket、进程拉起或桥接转发。

**Tech Stack:** TypeScript CLI, Rust workspace, Tokio, tokio-tungstenite, Serde, rusqlite, tracing, html parser (`tl`)

---

## Current Verified Baseline

以下内容已经在当前分支验证通过，不要重复推翻：

- `pnpm test` 通过
- `pnpm lint` 通过
- `docs/contracts/*`、`tests/contracts/*`、CLI daemon 选择逻辑、Rust workspace 骨架已存在
- 错误的 `Rust -> TS daemon` bridge 已移除，`rust/crates/wxp-daemon/src/server.rs` 现在回到空骨架

以下内容明确禁止重新引入：

- Rust daemon 运行时内部连接 `~/.wxpilot/daemon.sock`
- Rust daemon 自动拉起 `packages/daemon/dist/index.js`
- 任何 `helper.js` / Node 子进程作为 Rust daemon 的长期运行依赖

## Scope

这份计划只覆盖“未完成且必须按技术方案原生实现”的部分，不重做已完成的契约冻结和 CLI daemon 选择逻辑：

1. Rust native runtime + state store + probe/status
2. Rust native devtools launcher
3. Rust native automator websocket client
4. Rust native `start/connect/status/state`
5. Rust native `view/find/wait/read/assert`
6. Rust native `goto/back/reload/tap/type/scroll/shot/run/wx`
7. Rust native `net.*`、回归文档与真实项目对照测试

## File Structure

### Existing files to modify

- Modify: `rust/crates/wxp-daemon/Cargo.toml` - 引入 `tokio`、`tracing`、`tokio-tungstenite`、`rusqlite`、`tl` 等 Rust 原生依赖。
- Modify: `rust/crates/wxp-daemon/src/main.rs` - 启动单实例 Rust daemon，不接 TS backend。
- Modify: `rust/crates/wxp-daemon/src/lib.rs` - 导出 runtime/server/handler/public types。
- Modify: `rust/crates/wxp-daemon/src/runtime.rs` - 实现 `DaemonState`、`StateStore`、`RequestContext`、连接状态管理。
- Modify: `rust/crates/wxp-daemon/src/server.rs` - 实现 UDS listener、JSON-RPC dispatch、本地 probe/status/start/connect/view/find/state/action/net 路由。
- Modify: `rust/crates/wxp-daemon/src/handlers/mod.rs` - 收敛 probe、输出格式、action 映射等兼容逻辑。
- Modify: `rust/crates/wxp-automator-protocol/src/lib.rs` - 定义 automator request/response/page/element 协议结构。
- Modify: `rust/crates/wxp-automator-client/src/lib.rs` - 实现 websocket 连接、request/response、`MiniProgramClient/PageHandle/ElementHandle`。
- Modify: `rust/crates/wxp-snapshot/src/lib.rs` - 用真实 WXML parser 替换当前玩具实现。
- Modify: `rust/crates/wxp-ref-store/src/lib.rs` - 实现带 `snapshot_version` 的 ref store。
- Modify: `rust/crates/wxp-proxy/src/lib.rs` - 用真实 HTTP/HTTPS 代理与 mock registry 替换当前内存 stub。
- Modify: `rust/crates/wxp-store/src/lib.rs` - 用 `rusqlite` 实现真实 store。
- Modify: `README.md` - 更新 Rust daemon 原生能力里程碑与真实回归说明。

### New Rust daemon support files

- Create: `rust/crates/wxp-daemon/src/devtools.rs` - 微信开发者工具 CLI 查找与 `auto` 调起。
- Create: `rust/crates/wxp-daemon/src/read_path.rs` - `status/state/view/find/wait/read/assert` 读路径路由。
- Create: `rust/crates/wxp-daemon/src/action_path.rs` - `goto/back/reload/tap/type/scroll/shot/run/wx` 写路径路由。
- Create: `rust/crates/wxp-daemon/src/net_path.rs` - `net.*` 路由。
- Create: `rust/crates/wxp-daemon/tests/runtime_state.rs`
- Create: `rust/crates/wxp-daemon/tests/devtools_launcher.rs`
- Create: `rust/crates/wxp-daemon/tests/read_path.rs`
- Create: `rust/crates/wxp-daemon/tests/action_path.rs`
- Create: `rust/crates/wxp-daemon/tests/net_path.rs`

### Existing contract tests to keep green

- Keep: `tests/contracts/daemon-contract.spec.ts`
- Keep: `tests/contracts/cli-fallback.spec.ts`
- Keep: `tests/contracts/find-output.spec.ts`
- Keep: `tests/contracts/cross-implementation.spec.ts`

## Task 1: 建立单-daemon runtime 与持久状态

**Files:**
- Modify: `rust/crates/wxp-daemon/src/runtime.rs`
- Modify: `rust/crates/wxp-daemon/src/server.rs`
- Modify: `rust/crates/wxp-daemon/src/lib.rs`
- Create: `rust/crates/wxp-daemon/tests/runtime_state.rs`

- [ ] **Step 1: 写失败中的 runtime/state 测试**

```rust
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use wxp_daemon::runtime::{DaemonState, StateStore};
use wxp_daemon::server::{dispatch_local_request, RuntimePaths, ServerState};

fn unique_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("wxp-daemon-{name}-{nanos}"))
}

fn test_paths(name: &str) -> RuntimePaths {
    let root = unique_test_dir(name);
    fs::create_dir_all(&root).expect("create temp root");
    RuntimePaths {
        root_dir: root.clone(),
        rust_socket: root.join("rust-daemon.sock"),
        rust_pid: root.join("rust-daemon.pid"),
        state_file: root.join("rust-daemon-state.json"),
        db_file: root.join("rust-daemon.db"),
        ca_dir: root.join("ca"),
        log_dir: root.join("logs"),
    }
}

#[test]
fn state_store_round_trips_project_and_endpoint() {
    let paths = test_paths("state-store");
    let store = StateStore::new(paths.state_file.clone());
    let state = DaemonState {
        project_path: Some("/tmp/project".into()),
        ws_endpoint: Some("ws://127.0.0.1:9420".into()),
        connected: true,
    };

    store.save(&state).expect("save state");
    let loaded = store.load().expect("load state");

    assert_eq!(loaded.project_path, state.project_path);
    assert_eq!(loaded.ws_endpoint, state.ws_endpoint);
    assert!(loaded.connected);
}

#[test]
fn local_status_reports_rust_daemon_without_saved_connection() {
    let state = ServerState::new(test_paths("status"));

    let response = dispatch_local_request(
        &state,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "status",
            "params": {},
        }),
    )
    .expect("dispatch status");

    assert_eq!(response["result"]["ok"], true);
    assert_eq!(response["result"]["connected"], false);
    assert_eq!(response["result"]["projectPath"], serde_json::Value::Null);
    assert_eq!(response["result"]["proxy"], serde_json::Value::Null);
    assert_eq!(response["result"]["daemonKind"], "rust");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-daemon --test runtime_state`

Expected: FAIL，提示 `DaemonState` / `StateStore` / `dispatch_local_request` 或 `RuntimePaths.state_file` 不存在

- [ ] **Step 3: 写最小 runtime/state 实现**

```rust
// rust/crates/wxp-daemon/src/runtime.rs
use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonState {
    pub project_path: Option<String>,
    pub ws_endpoint: Option<String>,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> io::Result<DaemonState> {
        if !self.path.exists() {
            return Ok(DaemonState::default());
        }
        let content = fs::read_to_string(&self.path)?;
        serde_json::from_str(&content).map_err(io::Error::other)
    }

    pub fn save(&self, state: &DaemonState) -> io::Result<()> {
        let content = serde_json::to_string_pretty(state).map_err(io::Error::other)?;
        fs::write(&self.path, content)
    }

    pub fn clear(&self) -> io::Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub method: String,
    pub daemon_kind: &'static str,
}
```

```rust
// rust/crates/wxp-daemon/src/server.rs
use std::{fs, io, path::PathBuf, process};

use serde_json::{json, Value};

use crate::runtime::StateStore;

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

        Self {
            rust_socket: root_dir.join("daemon.sock"),
            rust_pid: root_dir.join("daemon.pid"),
            state_file: root_dir.join("daemon-state.json"),
            db_file: root_dir.join("daemon.db"),
            ca_dir: root_dir.join("ca"),
            log_dir: root_dir.join("logs"),
            root_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerState {
    pub daemon_kind: &'static str,
    pub paths: RuntimePaths,
    pub store: StateStore,
}

impl ServerState {
    pub fn new(paths: RuntimePaths) -> Self {
        let store = StateStore::new(paths.state_file.clone());
        Self {
            daemon_kind: wxp_common::DAEMON_KIND,
            paths,
            store,
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new(RuntimePaths::default())
    }
}

pub fn dispatch_local_request(state: &ServerState, request: &Value) -> io::Result<Value> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request["method"].as_str().unwrap_or_default();
    let saved = state.store.load()?;

    let result = match method {
        "health" => json!({ "status": "ok", "daemonKind": state.daemon_kind }),
        "version" => json!({
            "status": "ok",
            "daemonKind": state.daemon_kind,
            "version": env!("CARGO_PKG_VERSION"),
        }),
        "capabilities" => json!({
            "status": "ok",
            "start": true,
            "connect": true,
            "status": true,
            "view": true,
            "find": true,
            "state": true,
        }),
        "status" => json!({
            "ok": true,
            "connected": saved.connected,
            "projectPath": saved.project_path,
            "proxy": Value::Null,
            "daemonKind": state.daemon_kind,
        }),
        _ => json!({ "ok": false, "message": "method_not_ready" }),
    };

    Ok(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

pub fn prepare_runtime(paths: &RuntimePaths) -> io::Result<()> {
    fs::create_dir_all(&paths.root_dir)?;
    fs::create_dir_all(&paths.ca_dir)?;
    fs::create_dir_all(&paths.log_dir)?;
    let _ = fs::remove_file(&paths.rust_socket);
    fs::write(&paths.rust_pid, process::id().to_string())?;
    Ok(())
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-daemon --test runtime_state`

Expected: PASS，2 tests passed

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon/src/runtime.rs rust/crates/wxp-daemon/src/server.rs rust/crates/wxp-daemon/src/lib.rs rust/crates/wxp-daemon/tests/runtime_state.rs
git commit -m "feat: add rust daemon runtime state store"
```

## Task 2: 实现微信开发者工具 CLI launcher

**Files:**
- Create: `rust/crates/wxp-daemon/src/devtools.rs`
- Modify: `rust/crates/wxp-daemon/src/lib.rs`
- Create: `rust/crates/wxp-daemon/tests/devtools_launcher.rs`

- [ ] **Step 1: 写失败中的 devtools launcher 测试**

```rust
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

use wxp_daemon::devtools::DevtoolsLauncher;

fn temp_script(contents: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("wxp-devtools-{nanos}.sh"));
    fs::write(&path, contents).expect("write script");
    let mut perms = fs::metadata(&path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).expect("chmod");
    path
}

#[test]
fn launcher_returns_ws_endpoint_when_cli_auto_succeeds() {
    let script = temp_script(
        "#!/bin/sh\n\
         echo 'auto success'\n\
         exit 0\n",
    );

    let launcher = DevtoolsLauncher::new(vec![script.clone()]);
    let endpoint = launcher
        .run_auto("/tmp/project", 9420, Some(script), None)
        .expect("run auto");

    assert_eq!(endpoint, "ws://127.0.0.1:9420");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-daemon --test devtools_launcher`

Expected: FAIL，提示 `DevtoolsLauncher` 不存在

- [ ] **Step 3: 写最小 launcher 实现**

```rust
// rust/crates/wxp-daemon/src/devtools.rs
use std::{
    io,
    path::PathBuf,
    process::Command,
};

#[derive(Debug, Clone)]
pub struct DevtoolsLauncher {
    candidates: Vec<PathBuf>,
}

impl DevtoolsLauncher {
    pub fn new(candidates: Vec<PathBuf>) -> Self {
        Self { candidates }
    }

    pub fn default_candidates() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/Applications/wechatwebdevtools.app/Contents/MacOS/cli"),
            PathBuf::from("/Applications/Wechat Devtools Stable.app/Contents/MacOS/cli"),
        ]
    }

    pub fn find_cli(&self, explicit: Option<PathBuf>) -> io::Result<PathBuf> {
        if let Some(path) = explicit {
            return Ok(path);
        }
        self.candidates
            .iter()
            .find(|path| path.exists())
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "wechat_devtools_cli_not_found"))
    }

    pub fn run_auto(
        &self,
        project_path: &str,
        port: u16,
        explicit: Option<PathBuf>,
        appid: Option<&str>,
    ) -> io::Result<String> {
        let cli = self.find_cli(explicit)?;
        let mut command = Command::new(cli);
        command.args(["auto", "--project", project_path, "--auto-port", &port.to_string()]);
        if let Some(appid) = appid {
            command.args(["--appid", appid]);
        }
        let output = command.output()?;
        let merged = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        if output.status.success()
            && (merged.contains("auto success") || merged.contains("自动化已开启") || merged.contains("✔ auto"))
        {
            return Ok(format!("ws://127.0.0.1:{port}"));
        }
        Err(io::Error::other(format!("cli auto failed: {merged}")))
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-daemon --test devtools_launcher`

Expected: PASS，1 test passed

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon/src/devtools.rs rust/crates/wxp-daemon/src/lib.rs rust/crates/wxp-daemon/tests/devtools_launcher.rs
git commit -m "feat: add rust devtools cli launcher"
```

## Task 3: 实现 Rust automator protocol 与 websocket client

**Files:**
- Modify: `rust/crates/wxp-automator-protocol/src/lib.rs`
- Modify: `rust/crates/wxp-automator-client/Cargo.toml`
- Modify: `rust/crates/wxp-automator-client/src/lib.rs`
- Modify: `rust/crates/wxp-automator-client/tests/ws_client.rs`

- [ ] **Step 1: 写失败中的 websocket client 测试**

```rust
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use wxp_automator_client::AutomatorConnection;

#[tokio::test]
async fn connection_send_round_trips_a_response() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        let message = ws.next().await.expect("message").expect("message ok");
        let payload = message.into_text().expect("text");
        assert!(payload.contains("\"method\":\"Tool.getInfo\""));
        ws.send(Message::Text(
            r#"{"id":"req-1","result":{"SDKVersion":"3.3.5"}}"#.into(),
        ))
        .await
        .expect("send");
    });

    let connection = AutomatorConnection::connect(&format!("ws://{addr}"))
        .await
        .expect("connect");
    let response = connection
        .send_with_id("req-1".into(), "Tool.getInfo", serde_json::json!({}))
        .await
        .expect("send");

    assert_eq!(response["SDKVersion"], "3.3.5");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-automator-client --test ws_client`

Expected: FAIL，提示 `AutomatorConnection` 不存在或缺少 websocket 依赖

- [ ] **Step 3: 写最小 automator protocol/client 实现**

```rust
// rust/crates/wxp-automator-protocol/src/lib.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatorRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatorError {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatorResponse {
    pub id: Option<String>,
    pub method: Option<String>,
    pub result: Option<Value>,
    pub error: Option<AutomatorError>,
    pub params: Option<Value>,
}
```

```rust
// rust/crates/wxp-automator-client/src/lib.rs
use std::{collections::HashMap, sync::Arc};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use wxp_automator_protocol::{AutomatorRequest, AutomatorResponse};

#[derive(Clone)]
pub struct AutomatorConnection {
    writer: Arc<Mutex<futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>>,
}

impl AutomatorConnection {
    pub async fn connect(url: &str) -> Result<Self, String> {
        let (stream, _) = connect_async(url).await.map_err(|e| e.to_string())?;
        let (writer, mut reader) = stream.split();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let pending_reader = pending.clone();

        tokio::spawn(async move {
            while let Some(message) = reader.next().await {
                let Ok(message) = message else { break };
                let Ok(text) = message.into_text() else { continue };
                let Ok(response) = serde_json::from_str::<AutomatorResponse>(&text) else { continue };
                if let Some(id) = response.id {
                    if let Some(tx) = pending_reader.lock().await.remove(&id) {
                        let value = response.result.unwrap_or(serde_json::Value::Null);
                        let _ = tx.send(value);
                    }
                }
            }
        });

        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            pending,
        })
    }

    pub async fn send_with_id(
        &self,
        id: String,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let request = AutomatorRequest {
            id: id.clone(),
            method: method.to_string(),
            params,
        };
        let payload = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        self.writer
            .lock()
            .await
            .send(Message::Text(payload.into()))
            .await
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|_| "connection_closed".to_string())
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-automator-client --test ws_client`

Expected: PASS，websocket request/response 流程通过

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-automator-protocol/src/lib.rs rust/crates/wxp-automator-client/Cargo.toml rust/crates/wxp-automator-client/src/lib.rs rust/crates/wxp-automator-client/tests/ws_client.rs
git commit -m "feat: add rust automator websocket client"
```

## Task 4: 打通 Rust native `start/connect/status/state`

**Files:**
- Modify: `rust/crates/wxp-daemon/src/runtime.rs`
- Modify: `rust/crates/wxp-daemon/src/server.rs`
- Modify: `rust/crates/wxp-daemon/src/devtools.rs`
- Create: `rust/crates/wxp-daemon/tests/read_path.rs`

- [ ] **Step 1: 写失败中的 `start/connect/status/state` 测试**

```rust
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use wxp_daemon::server::{dispatch_request, RuntimePaths, ServerState};

#[tokio::test]
async fn start_connect_status_and_state_work_on_rust_daemon() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        while let Some(message) = ws.next().await {
            let payload = message.expect("message").into_text().expect("text");
            if payload.contains("\"method\":\"App.getCurrentPage\"") {
                ws.send(Message::Text(
                    r#"{"id":"2","result":{"pageId":1,"path":"pages/index/index","query":{}}}"#.into(),
                ))
                .await
                .expect("send");
            } else if payload.contains("\"method\":\"Page.getData\"") {
                ws.send(Message::Text(
                    r#"{"id":"3","result":{"data":{"title":"首页","count":3}}}"#.into(),
                ))
                .await
                .expect("send");
            } else {
                ws.send(Message::Text(r#"{"id":"1","result":{}}"#.into()))
                    .await
                    .expect("send");
            }
        }
    });

    let mut state = ServerState::new(RuntimePaths::default());
    state.runtime.set_ws_endpoint_for_test(format!("ws://{addr}"));

    let connect = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "connect",
        "params": {},
    }))
    .await
    .expect("connect");
    assert_eq!(connect["result"]["ok"], true);

    let status = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "status",
        "params": {},
    }))
    .await
    .expect("status");
    assert_eq!(status["result"]["connected"], true);
    assert_eq!(status["result"]["daemonKind"], "rust");

    let state_result = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "state",
        "params": { "full": true },
    }))
    .await
    .expect("state");
    assert_eq!(state_result["result"]["data"]["title"], "首页");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-daemon --test read_path`

Expected: FAIL，提示 `dispatch_request` 不是 async，或 `connect/state` 尚未实现

- [ ] **Step 3: 写最小 `start/connect/status/state` 实现**

```rust
// rust/crates/wxp-daemon/src/runtime.rs
use std::sync::Arc;

use tokio::sync::Mutex;
use wxp_automator_client::AutomatorConnection;

pub struct DaemonRuntime {
    pub saved: DaemonState,
    pub connection: Option<AutomatorConnection>,
}

impl DaemonRuntime {
    pub fn new(saved: DaemonState) -> Self {
        Self {
            saved,
            connection: None,
        }
    }

    pub fn set_ws_endpoint_for_test(&mut self, endpoint: String) {
        self.saved.ws_endpoint = Some(endpoint);
    }
}

pub type SharedRuntime = Arc<Mutex<DaemonRuntime>>;
```

```rust
// rust/crates/wxp-daemon/src/server.rs
pub struct ServerState {
    pub daemon_kind: &'static str,
    pub paths: RuntimePaths,
    pub store: StateStore,
    pub launcher: DevtoolsLauncher,
    pub runtime: DaemonRuntime,
}

pub async fn dispatch_request(state: &mut ServerState, request: &Value) -> io::Result<Value> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request["method"].as_str().unwrap_or_default();
    let params = request["params"].clone();

    match method {
        "start" => {
            let project_path = params["projectPath"].as_str().unwrap_or_default().to_string();
            let port = params["port"].as_u64().unwrap_or(9420) as u16;
            let endpoint = state
                .launcher
                .run_auto(&project_path, port, None, params["appid"].as_str())
                .map_err(io::Error::other)?;
            state.runtime.saved.project_path = Some(project_path);
            state.runtime.saved.ws_endpoint = Some(endpoint.clone());
            state.store.save(&state.runtime.saved)?;
            Ok(json!({ "jsonrpc": "2.0", "id": id, "result": { "ok": true, "wsEndpoint": endpoint, "port": port } }))
        }
        "connect" => {
            let endpoint = state.runtime.saved.ws_endpoint.clone().ok_or_else(|| io::Error::other("ws_endpoint_missing"))?;
            state.runtime.connection = Some(
                wxp_automator_client::AutomatorConnection::connect(&endpoint)
                    .await
                    .map_err(io::Error::other)?,
            );
            state.runtime.saved.connected = true;
            state.store.save(&state.runtime.saved)?;
            Ok(json!({ "jsonrpc": "2.0", "id": id, "result": { "ok": true, "wsEndpoint": endpoint } }))
        }
        "status" => dispatch_local_request(state, request),
        "state" => {
            let data = state.read_full_data(params["path"].as_str()).await?;
            Ok(json!({ "jsonrpc": "2.0", "id": id, "result": { "ok": true, "data": data } }))
        }
        _ => dispatch_local_request(state, request),
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-daemon --test read_path`

Expected: PASS，`start/connect/status/state` 通过

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon/src/runtime.rs rust/crates/wxp-daemon/src/server.rs rust/crates/wxp-daemon/src/devtools.rs rust/crates/wxp-daemon/tests/read_path.rs
git commit -m "feat: wire rust native start connect status state"
```

## Task 5: 实现 Rust native snapshot/ref 与 `view/find/wait/read/assert`

**Files:**
- Modify: `rust/crates/wxp-snapshot/src/lib.rs`
- Modify: `rust/crates/wxp-ref-store/src/lib.rs`
- Modify: `rust/crates/wxp-daemon/src/read_path.rs`
- Modify: `rust/crates/wxp-daemon/src/handlers/mod.rs`
- Modify: `rust/crates/wxp-daemon/tests/read_path.rs`
- Modify: `rust/crates/wxp-snapshot/tests/parser_contract.rs`
- Modify: `rust/crates/wxp-ref-store/tests/ref_versioning.rs`

- [ ] **Step 1: 写失败中的 snapshot/ref/read-path 测试**

```rust
use wxp_snapshot::{parse_wxml, render_find_matches, render_tree};

#[test]
fn parse_render_and_find_match_ts_behavior() {
    let tree = parse_wxml(
        r#"<page><view class="card"><input class="PhoneInput" placeholder="手机号" /></view></page>"#,
        20,
        None,
    );
    let rendered = render_tree(&tree, true, 5, Some(50), 30);
    assert!(rendered.output.contains("placeholder=\"手机号\""));

    let result = wxp_snapshot::find_nodes(&tree, "phone", true, Some(10));
    let summary = render_find_matches(&result.matches, 30);
    assert!(summary.contains("%1 [input].PhoneInput"));
}
```

```rust
use wxp_ref_store::RefStore;

#[test]
fn replacing_snapshot_invalidates_previous_refs() {
    let mut store = RefStore::default();
    store.replace_snapshot("pages/index/index", 1, vec![(1, "page > input")]);
    store.replace_snapshot("pages/index/index", 2, vec![(1, "page > button")]);

    assert!(store.resolve(1, 1).is_none());
    assert_eq!(store.resolve(1, 2).as_deref(), Some("page > button"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-snapshot -p wxp-ref-store`

Expected: FAIL，提示真实 parser/render API 不存在或行为不对

- [ ] **Step 3: 写最小 snapshot/ref 实现**

```rust
// rust/crates/wxp-snapshot/src/lib.rs
#[derive(Clone, Debug, PartialEq)]
pub struct ElementNode {
    pub tag: String,
    pub classes: Vec<String>,
    pub text: Option<String>,
    pub placeholder: Option<String>,
    pub interactive: bool,
    pub selector: String,
    pub ref_id: Option<u32>,
    pub children: Vec<ElementNode>,
}

pub struct RenderResult {
    pub output: String,
    pub shown_count: usize,
    pub total_count: usize,
}

pub fn parse_wxml(source: &str, max_depth: usize, fingerprints: Option<&std::collections::HashSet<String>>) -> Vec<ElementNode> {
    let dom = tl::parse(source, tl::ParserOptions::default()).expect("parse wxml");
    walk_dom(dom.parser(), dom.children(), "", 0, max_depth, fingerprints)
}

pub fn render_tree(
    nodes: &[ElementNode],
    compact: bool,
    depth: usize,
    limit: Option<usize>,
    text_limit: usize,
) -> RenderResult {
    fn render_node(
        node: &ElementNode,
        prefix: &str,
        compact: bool,
        depth: usize,
        current_depth: usize,
        limit: Option<usize>,
        text_limit: usize,
        shown: &mut usize,
        total: &mut usize,
        lines: &mut Vec<String>,
    ) {
        if node.interactive {
            *total += 1;
        }
        if compact && !node.interactive && node.children.iter().all(|child| !child.interactive) {
            return;
        }
        if current_depth > depth {
            return;
        }
        if node.interactive && limit.is_some_and(|value| *shown >= value) {
            return;
        }
        if node.interactive {
            *shown += 1;
        }
        let tag = if node.interactive {
            format!("[{}]", node.tag)
        } else {
            node.tag.clone()
        };
        let cls = node.classes.first().map(|value| format!(".{value}")).unwrap_or_default();
        let text = node.text.clone().unwrap_or_default();
        let text = if text.is_empty() {
            String::new()
        } else {
            format!(" \"{}\"", text.chars().take(text_limit).collect::<String>())
        };
        let placeholder = node
            .placeholder
            .as_ref()
            .map(|value| format!(" placeholder=\"{}\"", value))
            .unwrap_or_default();
        let ref_part = node
            .ref_id
            .map(|value| format!("  → %{value}"))
            .unwrap_or_default();
        lines.push(format!("{prefix}{tag}{cls}{text}{placeholder}{ref_part}"));
        let child_prefix = format!("{prefix}  ");
        for child in &node.children {
            render_node(
                child,
                &child_prefix,
                compact,
                depth,
                current_depth + 1,
                limit,
                text_limit,
                shown,
                total,
                lines,
            );
        }
    }

    let mut lines = Vec::new();
    let mut shown = 0;
    let mut total = 0;
    for node in nodes {
        render_node(
            node,
            "",
            compact,
            depth,
            0,
            limit,
            text_limit,
            &mut shown,
            &mut total,
            &mut lines,
        );
    }
    RenderResult {
        output: lines.join("\n"),
        shown_count: shown,
        total_count: total,
    }
}
```

```rust
// rust/crates/wxp-ref-store/src/lib.rs
use std::collections::HashMap;

#[derive(Default)]
pub struct RefStore {
    current_version: u64,
    refs: HashMap<(u64, u32), String>,
}

impl RefStore {
    pub fn replace_snapshot(&mut self, _page_path: &str, version: u64, refs: Vec<(u32, &str)>) {
        self.current_version = version;
        self.refs.retain(|(snapshot_version, _), _| *snapshot_version == version);
        for (ref_id, selector) in refs {
            self.refs.insert((version, ref_id), selector.to_string());
        }
    }

    pub fn resolve(&self, ref_id: u32, version: u64) -> Option<String> {
        if version != self.current_version {
            return None;
        }
        self.refs.get(&(version, ref_id)).cloned()
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-snapshot -p wxp-ref-store`

Expected: PASS，parser/render/find/ref versioning 基础行为通过

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-snapshot/src/lib.rs rust/crates/wxp-ref-store/src/lib.rs rust/crates/wxp-snapshot/tests/parser_contract.rs rust/crates/wxp-ref-store/tests/ref_versioning.rs rust/crates/wxp-daemon/src/read_path.rs rust/crates/wxp-daemon/src/handlers/mod.rs rust/crates/wxp-daemon/tests/read_path.rs
git commit -m "feat: add rust native view find ref store"
```

## Task 6: 实现 Rust native action path

**Files:**
- Modify: `rust/crates/wxp-daemon/src/action_path.rs`
- Modify: `rust/crates/wxp-daemon/src/handlers/mod.rs`
- Modify: `rust/crates/wxp-daemon/tests/action_path.rs`
- Modify: `rust/crates/wxp-automator-client/src/lib.rs`
- Modify: `rust/crates/wxp-automator-client/tests/ws_client.rs`

- [ ] **Step 1: 写失败中的 action path 测试**

```rust
use serde_json::json;
use wxp_daemon::handlers::normalize_route_action;

#[test]
fn goto_back_reload_map_to_ts_compatible_route_methods() {
    assert_eq!(normalize_route_action("goto"), "navigateTo");
    assert_eq!(normalize_route_action("back"), "navigateBack");
    assert_eq!(normalize_route_action("reload"), "reLaunch");
}

#[test]
fn input_and_textarea_map_to_expected_element_calls() {
    assert_eq!(wxp_automator_client::normalize_input_call("input"), "input.input");
    assert_eq!(wxp_automator_client::normalize_input_call("textarea"), "textarea.input");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-daemon --test action_path -p wxp-automator-client --test ws_client`

Expected: FAIL，真实 action forwarding 尚未接通

- [ ] **Step 3: 写最小 action path 实现**

```rust
// rust/crates/wxp-daemon/src/action_path.rs
pub async fn goto(runtime: &mut DaemonRuntime, route: &str) -> io::Result<()> {
    runtime
        .mini_program()?
        .call_wx_method("navigateTo", serde_json::json!({ "url": route }))
        .await
        .map_err(io::Error::other)?;
    runtime.invalidate_refs();
    Ok(())
}

pub async fn reload(runtime: &mut DaemonRuntime) -> io::Result<()> {
    let page = runtime.current_page().await?;
    let url = build_page_url(&page.path, &page.query);
    runtime
        .mini_program()?
        .call_wx_method("reLaunch", serde_json::json!({ "url": url }))
        .await
        .map_err(io::Error::other)?;
    runtime.invalidate_refs();
    Ok(())
}
```

```rust
// rust/crates/wxp-automator-client/src/lib.rs
pub fn normalize_input_call(tag: &str) -> &'static str {
    match tag {
        "input" => "input.input",
        "textarea" => "textarea.input",
        _ => "unknown",
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-daemon --test action_path -p wxp-automator-client --test ws_client`

Expected: PASS，动作映射和 action forwarding 通过

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon/src/action_path.rs rust/crates/wxp-daemon/src/handlers/mod.rs rust/crates/wxp-daemon/tests/action_path.rs rust/crates/wxp-automator-client/src/lib.rs rust/crates/wxp-automator-client/tests/ws_client.rs
git commit -m "feat: add rust native action path"
```

## Task 7: 实现 Rust native proxy/store 与 `net.*`

**Files:**
- Modify: `rust/crates/wxp-proxy/Cargo.toml`
- Modify: `rust/crates/wxp-store/Cargo.toml`
- Modify: `rust/crates/wxp-proxy/src/lib.rs`
- Modify: `rust/crates/wxp-store/src/lib.rs`
- Modify: `rust/crates/wxp-daemon/src/net_path.rs`
- Modify: `rust/crates/wxp-daemon/tests/net_path.rs`
- Modify: `rust/crates/wxp-proxy/tests/proxy_flow.rs`
- Modify: `rust/crates/wxp-store/tests/sqlite_store.rs`

- [ ] **Step 1: 写失败中的 proxy/store/net 测试**

```rust
use wxp_store::{RequestLog, StoreWriter};

#[test]
fn request_logs_are_truncated_at_2000_chars() {
    let writer = StoreWriter::memory().expect("memory store");
    let oversized = "x".repeat(2100);
    let log = RequestLog::new("GET", "https://wxpilot.test", Some(oversized), None);
    writer.insert_request(log).expect("insert");
    let stored = writer.list_requests(20, true).expect("list");
    assert_eq!(stored[0].req_body.as_ref().unwrap().len(), 2001);
}
```

```rust
use wxp_proxy::MockRegistry;

#[test]
fn exact_url_mock_can_be_added_and_removed() {
    let mut registry = MockRegistry::default();
    registry.upsert("GET", "https://wxpilot.test/api", "{\"ok\":true}");
    assert!(registry.match_url("GET", "https://wxpilot.test/api").is_some());
    registry.remove("https://wxpilot.test/api");
    assert!(registry.match_url("GET", "https://wxpilot.test/api").is_none());
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-store -p wxp-proxy`

Expected: FAIL，真实 SQLite/proxy 行为与 API 不对齐

- [ ] **Step 3: 写最小 proxy/store 实现**

```rust
// rust/crates/wxp-store/src/lib.rs
use rusqlite::{params, Connection};

pub struct StoreWriter {
    conn: Connection,
}

impl StoreWriter {
    pub fn memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE requests (
                id INTEGER PRIMARY KEY,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                req_body TEXT,
                res_body TEXT
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn insert_request(&self, log: RequestLog) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO requests (method, url, req_body, res_body) VALUES (?1, ?2, ?3, ?4)",
            params![log.method, log.url, log.req_body, log.res_body],
        )?;
        Ok(())
    }
}
```

```rust
// rust/crates/wxp-proxy/src/lib.rs
use std::collections::HashMap;

#[derive(Default)]
pub struct MockRegistry {
    items: HashMap<String, String>,
}

impl MockRegistry {
    pub fn upsert(&mut self, method: &str, url: &str, body: &str) {
        self.items.insert(format!("{method}:{url}"), body.to_string());
    }

    pub fn remove(&mut self, url: &str) {
        self.items.retain(|key, _| !key.ends_with(url));
    }

    pub fn match_url(&self, method: &str, url: &str) -> Option<&String> {
        self.items.get(&format!("{method}:{url}"))
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-store -p wxp-proxy`

Expected: PASS，net body 截断与 mock 行为通过

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-store/Cargo.toml rust/crates/wxp-store/src/lib.rs rust/crates/wxp-store/tests/sqlite_store.rs rust/crates/wxp-proxy/Cargo.toml rust/crates/wxp-proxy/src/lib.rs rust/crates/wxp-proxy/tests/proxy_flow.rs rust/crates/wxp-daemon/src/net_path.rs rust/crates/wxp-daemon/tests/net_path.rs
git commit -m "feat: add rust native proxy and store"
```

## Task 8: 真实项目回归、契约对照与文档收口

**Files:**
- Modify: `README.md`
- Modify: `docs/automator/protocol-manifest.md`
- Modify: `tests/contracts/cross-implementation.spec.ts`
- Modify: `tests/contracts/find-output.spec.ts`

- [ ] **Step 1: 写失败中的 cross-implementation 回归测试**

```ts
import { describe, expect, it } from 'vitest'
import { daemonContract } from '../../packages/daemon/src/__fixtures__/rpc-contract.js'

describe('cross implementation contract suite', () => {
  it('keeps rust daemon public method list aligned with frozen contract', () => {
    expect(daemonContract.methods).toContain('status')
    expect(daemonContract.methods).toContain('view')
    expect(daemonContract.methods).toContain('find')
    expect(daemonContract.methods).toContain('state')
  })
})
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run tests/contracts/cross-implementation.spec.ts`

Expected: FAIL，直到 Rust daemon 的真实回归输出补齐

- [ ] **Step 3: 更新文档与回归命令**

```md
<!-- README.md -->
## Rust Daemon Native Rollout

1. CLI 默认优先连接 Rust daemon，Rust 启动失败时才回退到 TS daemon。
2. Rust daemon 与 TS daemon 相互独立，运行时内部不得桥接。
3. 本地真实回归项目：
   `/Users/liaochengxing/Documents/workspace/frontend/fe-casamiel-miniprogram/main/dist`
4. 回归命令：
   - `node packages/cli/dist/index.js status --json`
   - `node packages/cli/dist/index.js start /Users/liaochengxing/Documents/workspace/frontend/fe-casamiel-miniprogram/main/dist --json`
   - `node packages/cli/dist/index.js connect --json`
   - `node packages/cli/dist/index.js view --json`
   - `node packages/cli/dist/index.js find 余额 --all --json`
   - `node packages/cli/dist/index.js state --json`
```

- [ ] **Step 4: 运行完整验证**

Run: `pnpm test`

Expected: PASS，Vitest 与 Rust 测试全部通过

Run: `pnpm lint`

Expected: PASS，TS 类型检查与 Rust 格式检查全部通过

Run: `pnpm build`

Expected: PASS，CLI/TS daemon/Rust binary 构建通过

Run: `node packages/cli/dist/index.js daemon stop --json || true`

Expected: 允许报错或返回 ok，用于清理旧 daemon

Run: `node packages/cli/dist/index.js status --json`

Expected: `daemonKind` 为 `rust`

Run: `node packages/cli/dist/index.js start /Users/liaochengxing/Documents/workspace/frontend/fe-casamiel-miniprogram/main/dist --json`

Expected: 返回 `ok=true`、`wsEndpoint`

Run: `node packages/cli/dist/index.js connect --json`

Expected: 返回 `ok=true`

Run: `node packages/cli/dist/index.js view --json`

Expected: 返回 `pagePath` 与 `output`

Run: `node packages/cli/dist/index.js find 余额 --all --json`

Expected: 返回 `pagePath`、`query`、`matches`、`output`

Run: `node packages/cli/dist/index.js state --json`

Expected: 返回 `summary` 或 `data`

- [ ] **Step 5: 提交**

```bash
git add README.md docs/automator/protocol-manifest.md tests/contracts/cross-implementation.spec.ts tests/contracts/find-output.spec.ts
git commit -m "chore: document rust native rollout regression"
```

## Execution Order

1. Task 1 先建立单-daemon runtime 边界，锁死不再桥接 TS daemon。
2. Task 2 完成 devtools launcher，给 `start` 提供原生入口。
3. Task 3 完成 websocket automator client，给 Rust 读/写路径提供真实后端。
4. Task 4 先打通 `start/connect/status/state`，形成第一个 Rust native 业务闭环。
5. Task 5 再落 `view/find/wait/read/assert`，把 snapshot/ref 接上真实页面。
6. Task 6 补动作链路，确保自动化主路径可回归。
7. Task 7 最后接 `net.*`，避免早期把失败面铺得太大。
8. Task 8 再做真实项目回归和文档收口。

## Verification Matrix

- `cd rust && cargo test -p wxp-daemon --test runtime_state`
- `cd rust && cargo test -p wxp-daemon --test devtools_launcher`
- `cd rust && cargo test -p wxp-automator-client --test ws_client`
- `cd rust && cargo test -p wxp-daemon --test read_path`
- `cd rust && cargo test -p wxp-snapshot -p wxp-ref-store`
- `cd rust && cargo test -p wxp-daemon --test action_path -p wxp-automator-client --test ws_client`
- `cd rust && cargo test -p wxp-store -p wxp-proxy`
- `pnpm test`
- `pnpm lint`
- `pnpm build`

## Spec Coverage Check

- 技术方案的 Phase 0-1 已在当前基线完成，本计划从 Phase 2 开始继续原生实现。
- `automator 由 Rust 直接接管` 对应 Task 3、Task 4、Task 6。
- `snapshot/ref` 对应 Task 5。
- `proxy/store` 对应 Task 7。
- `CLI 默认主连 Rust、失败回退 TS` 的运行方式保持不变，本计划不改 CLI 语义，只要求 Rust daemon 真正可用。
- 本计划不保留任何占位型步骤；每个任务都给出具体文件、测试、命令和最小实现骨架。

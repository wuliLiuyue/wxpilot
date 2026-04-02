# Rust Daemon Gap Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 补齐 Rust daemon 与当前公开 CLI/契约之间的剩余实现缺口，让 `state`、`shot/run/wx`、`net.*`、运行时 store 与 probe/cross-implementation 对齐到同一套可验证行为。

**Architecture:** 继续维持单 Rust daemon 架构，不重新引入自动回退或任何 Rust -> TS bridge。实现顺序按“先修公开契约错位，再补缺失 RPC，最后收口 runtime/net/probe 与文档”推进，每个任务都先写失败测试，再做最小实现。

**Tech Stack:** TypeScript CLI, Rust workspace, Tokio, tokio-tungstenite, rusqlite, rcgen, Vitest, cargo test

## Progress Update

**Last Updated:** 2026-04-02

- `Task 1` 已完成并验证：`state` 已对齐 `summary/path/full` 契约，`cargo test -p wxp-daemon --test read_path` 通过。已知残留：`state` 的 `truncated/originalLength` 仍未对齐 TS，留待后续统一处理。
- `Task 2` 已完成并验证：`shot/run/wx` 已接入 Rust action 路由，`run` 语义已对齐 TS 的函数包装规则，`cargo test -p wxp-daemon --test action_path` 通过。
- `Task 3` 已完成并验证：request store 已切到文件型 SQLite，daemon 初始化错误已改为显式 `Result` 传播，`cargo test -p wxp-store` 与 `cargo test -p wxp-daemon` 均通过。
- `Task 4` 未开始实现，仅完成现状与目标契约对照；明天从 `net.mock/net.log/net.start/net.install-ca` 和 `ca.rs` 开始。

---

## Scope Decision

本次缺口横跨 `state`、action RPC、`net.*`、store 与 probe，但它们都耦合在同一条公开 RPC 契约上：

1. `packages/cli/src/index.ts` 已公开这些命令面，不能只修其中一层。
2. Rust daemon 的 `server/read_path/action_path/net_path/runtime` 共同决定最终行为。
3. `tests/contracts/*`、Rust integration tests、真实 CLI 回归命令必须一起收口，才能避免“代码能编译、契约却继续漂移”。

因此保留一份总计划更合适，不再拆子计划。

## File Structure

### Existing files to modify

- Modify: `rust/crates/wxp-daemon/src/server.rs` - 把 `state` 分发到 `read_path`，补齐 `shot/run/wx` 路由，初始化文件型 store，并统一 probe 返回。
- Modify: `rust/crates/wxp-daemon/src/read_path.rs` - 新增 `handle_state`，实现摘要、子路径读取与 `full` 语义。
- Modify: `rust/crates/wxp-daemon/src/action_path.rs` - 新增 `shot/run/wx` 三个写路径 handler。
- Modify: `rust/crates/wxp-daemon/src/net_path.rs` - 对齐 `net.log/net.mock/net.install-ca/net.start --https` 的参数与返回结构。
- Modify: `rust/crates/wxp-daemon/src/runtime.rs` - 用 `db_file` 初始化真实 SQLite store，保留 runtime 状态管理。
- Modify: `rust/crates/wxp-daemon/src/handlers/mod.rs` - 把 Rust probe 能力整理成稳定方法清单与 capabilities 返回。
- Modify: `rust/crates/wxp-daemon/src/lib.rs` - 导出 probe manifest 辅助函数，给 Rust/TS contract test 复用。
- Modify: `rust/crates/wxp-daemon/Cargo.toml` - 为证书生成补 `rcgen` 依赖。
- Modify: `rust/crates/wxp-store/src/lib.rs` - 增加基于文件路径打开 SQLite 的能力。
- Modify: `rust/crates/wxp-store/tests/sqlite_store.rs` - 新增文件型 store 持久化测试。
- Modify: `rust/crates/wxp-daemon/tests/read_path.rs` - 补 `state` 摘要/子路径/全量行为测试。
- Modify: `rust/crates/wxp-daemon/tests/action_path.rs` - 补 `shot/run/wx` 行为测试。
- Modify: `rust/crates/wxp-daemon/tests/net_path.rs` - 补 `filter/jsonFile/caPath/certFile` 契约测试。
- Modify: `rust/crates/wxp-daemon/tests/health_probe.rs` - 校验 probe 返回的 `daemonKind/version/methods/capabilities`。
- Create: `rust-bindings/rust-capabilities.ts` - 给 Vitest contract suite 提供与 Rust probe 同步的公开方法清单。
- Modify: `tests/contracts/cross-implementation.spec.ts` - 让 cross-implementation 至少校验 Rust probe 导出的公开方法清单。
- Modify: `docs/contracts/cli-output.md` - 收口 `state`、`shot`、`net.log` 的最终输出契约。
- Modify: `docs/automator/protocol-manifest.md` - 补齐 `App.captureScreenshot`、`App.callFunction`、`App.callWxMethod` 用途说明。
- Modify: `README.md` - 更新 `state`、`shot/run/wx`、`net.*` 的真实行为与回归命令。

### New support file

- Create: `rust/crates/wxp-daemon/src/ca.rs` - 生成 `cert.pem/key.pem`，封装 macOS `security add-trusted-cert` 安装逻辑。

## Task 1: 对齐 `state` 读路径契约

**Files:**
- Modify: `rust/crates/wxp-daemon/src/server.rs`
- Modify: `rust/crates/wxp-daemon/src/read_path.rs`
- Test: `rust/crates/wxp-daemon/tests/read_path.rs`

- [x] **Step 1: 写失败中的 `state` 契约测试**

```rust
#[tokio::test]
async fn state_returns_summary_and_path_variants() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        while let Some(message) = ws.next().await {
            let payload = message.expect("message").into_text().expect("text");
            let request: Value = serde_json::from_str(&payload).expect("request json");
            let id = request.get("id").cloned().unwrap_or(Value::Null);
            let method = request["method"].as_str().expect("method");

            let response = match method {
                "App.getCurrentPage" => {
                    json!({ "id": id, "result": { "pageId": 1, "path": "pages/index/index", "query": {} } })
                }
                "Page.getData" => {
                    json!({
                        "id": id,
                        "result": {
                            "data": {
                                "title": "首页",
                                "count": 3,
                                "cardMap": { "vipCardPrice": "1,234,367.73" }
                            }
                        }
                    })
                }
                _ => json!({ "id": id, "result": {} }),
            };

            ws.send(Message::Text(response.to_string().into()))
                .await
                .expect("send");
        }
    });

    let mut state = ServerState::new(test_paths("state-contract"));
    state.runtime.set_ws_endpoint_for_test(format!("ws://{addr}"));

    dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0", "id": 1, "method": "connect", "params": {}
    })).await.expect("connect");

    let summary = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0", "id": 2, "method": "state", "params": {}
    })).await.expect("summary");
    assert_eq!(summary["result"]["summary"]["title"], "string(4)");
    assert_eq!(
        summary["result"]["hint"],
        "use --full or specify a path to get full data"
    );

    let path_result = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "state",
        "params": { "path": "cardMap.vipCardPrice" }
    })).await.expect("state path");
    assert_eq!(path_result["result"]["data"], "1,234,367.73");

    let full = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "state",
        "params": { "full": true }
    })).await.expect("state full");
    assert_eq!(full["result"]["data"]["count"], 3);
}
```

- [x] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-daemon --test read_path state_returns_summary_and_path_variants -- --exact`

Expected: FAIL，当前实现会直接返回完整 `data`，`summary` 为 `null`，`path` 参数也不会生效

- [x] **Step 3: 写最小实现**

```rust
// rust/crates/wxp-daemon/src/server.rs
match method {
    "state" => read_path::handle_state(state, id, &params).await,
    "view" => read_path::handle_view(state, id, &params).await,
    "find" => read_path::handle_find(state, id, &params).await,
    "wait" => read_path::handle_wait(state, id, &params).await,
    "read" => read_path::handle_read(state, id, &params).await,
    "assert" => read_path::handle_assert(state, id, &params).await,
    _ => dispatch_local_request(state, request),
}
```

```rust
// rust/crates/wxp-daemon/src/read_path.rs
pub async fn handle_state(
    state: &mut ServerState,
    id: Value,
    params: &Value,
) -> io::Result<Value> {
    let data = state.read_state_data(&id).await?;
    let full = params.get("full").and_then(Value::as_bool).unwrap_or(false);
    let data_path = params.get("path").and_then(Value::as_str);

    if let Some(path) = data_path.filter(|value| !value.is_empty()) {
        let resolved = resolve_data_path(&data, path).unwrap_or(Value::Null);
        return Ok(ok_result(id, json!({ "ok": true, "data": resolved })));
    }

    if !full {
        if let Some(summary) = summarize_top_level_data(&data) {
            return Ok(ok_result(id, json!({
                "ok": true,
                "summary": summary,
                "hint": "use --full or specify a path to get full data",
            })));
        }
    }

    Ok(ok_result(id, json!({ "ok": true, "data": data })))
}

fn summarize_top_level_data(data: &Value) -> Option<Value> {
    let object = data.as_object()?;
    let mut summary = serde_json::Map::new();
    for (key, value) in object {
        let encoded = serde_json::to_string(value).ok()?;
        summary.insert(
            key.clone(),
            Value::String(format!("{}({})", value_type_name(value), encoded.len())),
        );
    }
    Some(Value::Object(summary))
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn resolve_data_path(data: &Value, path: &str) -> Option<Value> {
    let mut current = data;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current.clone())
}

fn ok_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}
```

- [x] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-daemon --test read_path state_returns_summary_and_path_variants -- --exact`

Expected: PASS

Run: `cd rust && cargo test -p wxp-daemon --test read_path`

Expected: PASS，原有 `start/connect/status/view/find/read/assert` 测试继续通过

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon/src/server.rs rust/crates/wxp-daemon/src/read_path.rs rust/crates/wxp-daemon/tests/read_path.rs
git commit -m "feat: align rust state output contract"
```

### Task 2: 补齐 `shot/run/wx` Action RPC

**Files:**
- Modify: `rust/crates/wxp-daemon/src/server.rs`
- Modify: `rust/crates/wxp-daemon/src/action_path.rs`
- Test: `rust/crates/wxp-daemon/tests/action_path.rs`

- [x] **Step 1: 写失败中的 action parity 测试**

```rust
#[tokio::test]
async fn screenshot_run_and_wx_actions_forward_to_automator() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let seen_payloads = Arc::new(Mutex::new(Vec::<String>::new()));
    let server_payloads = seen_payloads.clone();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        while let Some(message) = ws.next().await {
            let payload = message.expect("message").into_text().expect("text");
            server_payloads.lock().expect("lock payloads").push(payload.to_string());
            let request: Value = serde_json::from_str(&payload).expect("request json");
            let id = request.get("id").cloned().unwrap_or(Value::Null);
            let method = request["method"].as_str().expect("method");

            let response = match method {
                "App.getCurrentPage" => {
                    json!({ "id": id, "result": { "pageId": 1, "path": "pages/index/index", "query": {} } })
                }
                "App.captureScreenshot" => {
                    json!({ "id": id, "result": { "path": "/tmp/wxp-shot.png", "base64": "ZmFrZQ==" } })
                }
                "App.callFunction" => {
                    json!({ "id": id, "result": { "value": 2 } })
                }
                "App.callWxMethod" => {
                    json!({ "id": id, "result": { "storage": "ok" } })
                }
                _ => json!({ "id": id, "result": {} }),
            };

            ws.send(Message::Text(response.to_string().into()))
                .await
                .expect("send");
        }
    });

    let mut state = ServerState::new(test_paths("extended-actions"));
    state.runtime.set_ws_endpoint_for_test(format!("ws://{addr}"));
    dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0", "id": 1, "method": "connect", "params": {}
    })).await.expect("connect");

    let shot = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0", "id": 2, "method": "shot", "params": { "base64": true }
    })).await.expect("shot");
    assert_eq!(shot["result"]["path"], "/tmp/wxp-shot.png");
    assert_eq!(shot["result"]["base64"], "ZmFrZQ==");

    let run = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0", "id": 3, "method": "run", "params": { "js": "1+1" }
    })).await.expect("run");
    assert_eq!(run["result"]["result"]["value"], 2);

    let wx = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "wx",
        "params": { "method": "getStorageSync", "args": ["foo"] }
    })).await.expect("wx");
    assert_eq!(wx["result"]["result"]["storage"], "ok");

    let payloads = seen_payloads.lock().expect("lock payloads");
    assert!(payloads.iter().any(|payload| payload.contains("\"method\":\"App.captureScreenshot\"")));
    assert!(payloads.iter().any(|payload| payload.contains("\"method\":\"App.callFunction\"")));
    assert!(payloads.iter().any(|payload| payload.contains("\"method\":\"App.callWxMethod\"")));
}
```

- [x] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-daemon --test action_path screenshot_run_and_wx_actions_forward_to_automator -- --exact`

Expected: FAIL，当前 `server.rs` 不分发 `shot/run/wx`

- [x] **Step 3: 写最小实现**

```rust
// rust/crates/wxp-daemon/src/server.rs
"goto" | "back" | "reload" | "tap" | "type" | "scroll" | "shot" | "run" | "wx" => {
    action_path::handle_action(state, id, method, &params).await
}
```

```rust
// rust/crates/wxp-daemon/src/action_path.rs
pub async fn handle_action(
    state: &mut ServerState,
    id: Value,
    method: &str,
    params: &Value,
) -> io::Result<Value> {
    match method {
        "goto" => goto(state, id, params).await,
        "back" => back(state, id).await,
        "reload" => reload(state, id).await,
        "tap" => tap(state, id, params).await,
        "type" => type_text(state, id, params).await,
        "scroll" => scroll(state, id, params).await,
        "shot" => shot(state, id, params).await,
        "run" => run_script(state, id, params).await,
        "wx" => wx_call(state, id, params).await,
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("unsupported action method: {method}"))),
    }
}

async fn shot(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let path = params.get("path").cloned().unwrap_or(Value::Null);
    let base64 = params.get("base64").and_then(Value::as_bool).unwrap_or(false);
    let result = state
        .call_automator(&id, "shot", "App.captureScreenshot", json!({ "path": path, "base64": base64 }))
        .await?;
    Ok(ok_result(id, json!({ "ok": true, "path": result["path"], "base64": result.get("base64").cloned() })))
}

async fn run_script(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let js = params.get("js").and_then(Value::as_str).unwrap_or_default();
    let result = state
        .call_automator(&id, "run", "App.callFunction", json!({ "functionDeclaration": js }))
        .await?;
    Ok(ok_result(id, json!({ "ok": true, "result": result })))
}

async fn wx_call(state: &mut ServerState, id: Value, params: &Value) -> io::Result<Value> {
    let method = params.get("method").and_then(Value::as_str).unwrap_or_default();
    let args = params.get("args").cloned().unwrap_or_else(|| json!([]));
    let result = state
        .call_automator(&id, "wx", "App.callWxMethod", json!({ "method": method, "args": args }))
        .await?;
    Ok(ok_result(id, json!({ "ok": true, "result": result })))
}
```

- [x] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-daemon --test action_path screenshot_run_and_wx_actions_forward_to_automator -- --exact`

Expected: PASS

Run: `cd rust && cargo test -p wxp-daemon --test action_path`

Expected: PASS，原有 `goto/back/reload/tap/type/scroll` 测试继续通过

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon/src/server.rs rust/crates/wxp-daemon/src/action_path.rs rust/crates/wxp-daemon/tests/action_path.rs
git commit -m "feat: add rust action parity for shot run wx"
```

### Task 3: 把运行时 request store 改成真实 SQLite 文件

**Files:**
- Modify: `rust/crates/wxp-store/src/lib.rs`
- Modify: `rust/crates/wxp-store/tests/sqlite_store.rs`
- Modify: `rust/crates/wxp-daemon/src/runtime.rs`
- Modify: `rust/crates/wxp-daemon/src/server.rs`

- [x] **Step 1: 写失败中的文件型 store 测试**

```rust
#[test]
fn file_store_persists_requests_across_reopen() {
    let root = std::env::temp_dir().join(format!("wxp-store-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("create temp root");
    let db_path = root.join("requests.db");

    {
        let store = StoreWriter::open(&db_path).expect("open file store");
        store
            .insert_request(RequestLog::new("GET", "https://wxpilot.test/api", None, None))
            .expect("insert request");
    }

    let reopened = StoreWriter::open(&db_path).expect("reopen file store");
    let logs = reopened.list_requests(10, false).expect("list requests");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].url, "https://wxpilot.test/api");
}
```

- [x] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-store file_store_persists_requests_across_reopen -- --exact`

Expected: FAIL，当前只有 `StoreWriter::memory()`

- [x] **Step 3: 写最小实现并接入 runtime**

```rust
// rust/crates/wxp-store/src/lib.rs
impl StoreWriter {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create store parent");
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                req_body TEXT,
                res_body TEXT
            );",
        )?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }
}
```

```rust
// rust/crates/wxp-daemon/src/runtime.rs
impl DaemonRuntime {
    pub fn new(saved: DaemonState, db_file: &std::path::Path) -> Self {
        Self {
            saved,
            connection: None,
            ref_store: RefStore::default(),
            snapshot_version: 0,
            snapshot_refs: HashMap::new(),
            request_store: StoreWriter::open(db_file).expect("open sqlite store"),
            mock_registry: MockRegistry::default(),
            proxy: LocalProxy::default(),
        }
    }
}
```

```rust
// rust/crates/wxp-daemon/src/server.rs
runtime: DaemonRuntime::new(saved, &paths.db_file),
```

- [x] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-store file_store_persists_requests_across_reopen -- --exact`

Expected: PASS

Run: `cd rust && cargo test -p wxp-daemon --test runtime_state`

Expected: PASS，daemon runtime 仍可正常初始化与持久化状态

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-store/src/lib.rs rust/crates/wxp-store/tests/sqlite_store.rs rust/crates/wxp-daemon/src/runtime.rs rust/crates/wxp-daemon/src/server.rs
git commit -m "feat: persist rust request store in sqlite"
```

### Task 4: 对齐 `net.*` 契约并补证书文件返回

**Files:**
- Modify: `rust/crates/wxp-daemon/Cargo.toml`
- Modify: `rust/crates/wxp-daemon/src/net_path.rs`
- Modify: `rust/crates/wxp-daemon/src/runtime.rs`
- Modify: `rust/crates/wxp-daemon/tests/net_path.rs`
- Modify or Create: `rust/crates/wxp-daemon/src/ca.rs`

- [ ] **Step 1: 写失败中的 `net.*` 契约测试**

```rust
#[tokio::test]
async fn net_log_mock_and_install_ca_match_cli_contract() {
    let root = test_paths("net-contract");
    let json_file = root.root_dir.join("mock.json");
    fs::write(&json_file, r#"{"ok":true}"#).expect("write mock json");

    let mut state = ServerState::new(root.clone());
    state
        .runtime
        .request_store
        .insert_request(RequestLog::new(
            "GET",
            "https://wxpilot.test/api",
            Some("{\"request\":true}".to_string()),
            Some("{\"response\":true}".to_string()),
        ))
        .expect("insert request");

    let mock = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "net.mock",
        "params": { "url": "https://wxpilot.test/api", "jsonFile": json_file }
    })).await.expect("net mock");
    assert_eq!(mock["result"]["ok"], true);

    let logs = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "net.log",
        "params": { "filter": "wxpilot.test", "limit": 20, "withBody": true }
    })).await.expect("net log");
    assert_eq!(logs["result"]["logs"][0]["url"], "https://wxpilot.test/api");
    assert_eq!(logs["result"]["logs"][0]["method"], "GET");
    assert!(logs["result"]["logs"][0].get("req_body").is_some());

    let started = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "net.start",
        "params": { "port": 0, "https": true }
    })).await.expect("net start https");
    assert!(started["result"]["caPath"].as_str().expect("ca path").ends_with("cert.pem"));

    let installed = dispatch_request(&mut state, &json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "net.install-ca",
        "params": {}
    })).await.expect("install ca");
    assert!(installed["result"]["certFile"].as_str().expect("cert file").ends_with("cert.pem"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-daemon --test net_path net_log_mock_and_install_ca_match_cli_contract -- --exact`

Expected: FAIL，当前 `net.mock` 读 `body` 不是 `jsonFile`，`net.log` 忽略 `filter` 且字段名不对，`net.start --https` 没有 `caPath`，`net.install-ca` 只返回 `ok`

- [ ] **Step 3: 写最小实现**

```rust
// rust/crates/wxp-daemon/src/net_path.rs
"net.mock" => {
    let url = params.get("url").and_then(Value::as_str).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing url"))?;
    let json_file = params.get("jsonFile").and_then(Value::as_str).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing jsonFile"))?;
    let body = std::fs::read_to_string(json_file)?;
    state.runtime.mock_registry.upsert("GET", url, &body);
    Ok(ok_result(id, json!({ "ok": true, "url": url })))
}
"net.log" => {
    let filter = params.get("filter").and_then(Value::as_str);
    let with_body = params.get("withBody").and_then(Value::as_bool).unwrap_or(false);
    let limit = params.get("limit").and_then(Value::as_u64).map(|value| value as usize).unwrap_or(20);
    let mut logs = state.runtime.request_store.list_requests(limit, with_body).map_err(io::Error::other)?;
    if let Some(filter) = filter {
        logs.retain(|item| item.url.contains(filter));
    }
    let logs = logs.into_iter().map(|item| json!({
        "method": item.method,
        "url": item.url,
        "status": Value::Null,
        "ts": Value::Null,
        "mocked": 0,
        "duration": Value::Null,
        "req_body": item.req_body,
        "res_body": item.res_body,
    })).collect::<Vec<_>>();
    Ok(ok_result(id, json!({ "ok": true, "logs": logs })))
}
```

```rust
// rust/crates/wxp-daemon/src/ca.rs
pub fn ensure_ca_cert(ca_dir: &std::path::Path) -> io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(ca_dir)?;
    let cert_file = ca_dir.join("cert.pem");
    let key_file = ca_dir.join("key.pem");
    if !cert_file.exists() || !key_file.exists() {
        let cert = rcgen::generate_simple_self_signed(vec!["wxpilot.local".into()])
            .map_err(io::Error::other)?;
        std::fs::write(&cert_file, cert.cert.pem())?;
        std::fs::write(&key_file, cert.signing_key.serialize_pem())?;
    }
    Ok(cert_file)
}

pub fn install_ca_cert(cert_file: &std::path::Path) -> io::Result<()> {
    let status = std::process::Command::new("security")
        .args([
            "add-trusted-cert",
            "-d",
            "-r",
            "trustRoot",
            "-k",
            &format!(
                "{}/Library/Keychains/login.keychain-db",
                std::env::var("HOME").unwrap_or_default()
            ),
            cert_file.to_str().unwrap_or_default(),
        ])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("security add-trusted-cert failed"))
    }
}
```

```rust
// rust/crates/wxp-daemon/src/net_path.rs
"net.start" => {
    let port = params.get("port").and_then(Value::as_u64).map(u16::try_from).transpose().map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "port_out_of_range"))?.unwrap_or(state.default_proxy_port);
    let https = params.get("https").and_then(Value::as_bool).unwrap_or(false);
    let bound_port = state.runtime.proxy.start(port)?;
    let ca_path = if https {
        Some(crate::ca::ensure_ca_cert(&state.paths.ca_dir)?)
    } else {
        None
    };
    Ok(ok_result(id, json!({
        "ok": true,
        "port": bound_port,
        "https": https,
        "caPath": ca_path.as_ref().map(|path| path.display().to_string()),
    })))
}
"net.install-ca" => {
    let cert_file = crate::ca::ensure_ca_cert(&state.paths.ca_dir)?;
    crate::ca::install_ca_cert(&cert_file)?;
    Ok(ok_result(id, json!({ "ok": true, "certFile": cert_file.display().to_string() })))
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-daemon --test net_path net_log_mock_and_install_ca_match_cli_contract -- --exact`

Expected: PASS

Run: `cd rust && cargo test -p wxp-daemon --test net_path`

Expected: PASS，原有 `net.start/net.stop/net.clear` 测试继续通过

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon/Cargo.toml rust/crates/wxp-daemon/src/net_path.rs rust/crates/wxp-daemon/src/runtime.rs rust/crates/wxp-daemon/tests/net_path.rs rust/crates/wxp-daemon/src/ca.rs
git commit -m "feat: align rust net contract and ca lifecycle"
```

### Task 5: 收口 probe 能力清单与 cross-implementation 对照

**Files:**
- Modify: `rust/crates/wxp-daemon/src/handlers/mod.rs`
- Modify: `rust/crates/wxp-daemon/src/lib.rs`
- Modify: `rust/crates/wxp-daemon/tests/health_probe.rs`
- Create: `rust-bindings/rust-capabilities.ts`
- Modify: `tests/contracts/cross-implementation.spec.ts`

- [ ] **Step 1: 写失败中的 probe/cross-implementation 测试**

```rust
#[test]
fn health_version_capabilities_publish_public_method_manifest() {
    let capabilities = handle_probe("capabilities");
    let methods = capabilities["methods"].as_array().expect("methods array");
    assert!(methods.iter().any(|item| item == "state"));
    assert!(methods.iter().any(|item| item == "shot"));
    assert!(methods.iter().any(|item| item == "run"));
    assert!(methods.iter().any(|item| item == "wx"));
    assert!(methods.iter().any(|item| item == "net.log"));
}
```

```ts
import { describe, expect, it } from 'vitest'
import { daemonContract } from '../../packages/daemon/src/__fixtures__/rpc-contract.js'
import { rustPublicMethods } from '../../rust-bindings/rust-capabilities.js'

describe('cross implementation contract suite', () => {
  it('keeps rust daemon public method list aligned with frozen contract', () => {
    for (const method of ['state', 'shot', 'run', 'wx', 'net.log']) {
      expect(daemonContract.methods).toContain(method)
      expect(rustPublicMethods).toContain(method)
    }
  })
})
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-daemon --test health_probe health_version_capabilities_publish_public_method_manifest -- --exact`

Expected: FAIL，当前 `capabilities` 没有 `methods`

Run: `pnpm vitest run tests/contracts/cross-implementation.spec.ts`

Expected: FAIL，当前没有 `rust-bindings/rust-capabilities.js`

- [ ] **Step 3: 写最小实现**

```rust
// rust/crates/wxp-daemon/src/lib.rs
pub const PUBLIC_METHODS: &[&str] = &[
    "start", "connect", "disconnect", "status",
    "goto", "back", "reload", "view", "find", "wait",
    "tap", "type", "scroll", "read", "assert", "state",
    "shot", "run", "wx",
    "net.start", "net.stop", "net.log", "net.mock", "net.unmock", "net.clear", "net.install-ca",
    "daemon.stop",
];
```

```rust
// rust/crates/wxp-daemon/src/handlers/mod.rs
"capabilities" => json!({
    "status": "ok",
    "daemonKind": "rust",
    "methods": crate::PUBLIC_METHODS,
    "find": true,
    "view": true,
    "state": true,
    "shot": true,
    "run": true,
    "wx": true,
}),
```

```ts
// rust-bindings/rust-capabilities.ts
export const rustPublicMethods = [
  'start', 'connect', 'disconnect', 'status',
  'goto', 'back', 'reload', 'view', 'find', 'wait',
  'tap', 'type', 'scroll', 'read', 'assert', 'state',
  'shot', 'run', 'wx',
  'net.start', 'net.stop', 'net.log', 'net.mock', 'net.unmock', 'net.clear', 'net.install-ca',
  'daemon.stop',
] as const
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-daemon --test health_probe health_version_capabilities_publish_public_method_manifest -- --exact`

Expected: PASS

Run: `pnpm vitest run tests/contracts/cross-implementation.spec.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon/src/lib.rs rust/crates/wxp-daemon/src/handlers/mod.rs rust/crates/wxp-daemon/tests/health_probe.rs rust-bindings/rust-capabilities.ts tests/contracts/cross-implementation.spec.ts
git commit -m "test: align rust probe manifest with public contract"
```

### Task 6: 文档收口与最终回归

**Files:**
- Modify: `README.md`
- Modify: `docs/contracts/cli-output.md`
- Modify: `docs/automator/protocol-manifest.md`

- [ ] **Step 1: 写失败中的文档对照检查清单**

```md
<!-- docs/contracts/cli-output.md -->
- `state` 默认返回 `summary`，`state [path]` 返回子路径值，`state --full` 返回完整 `data`
- `shot --base64` 返回 `path + base64`
- `run` / `wx` 返回 `result`
- `net.log` 支持 `filter/limit/withBody`
- `net.install-ca` 返回 `certFile`
```

- [ ] **Step 2: 运行相关命令确认旧文档已过时**

Run: `node packages/cli/dist/index.js state cardMap.vipCardPrice --json`

Expected: 在 Task 1 完成前，返回值仍与文档不一致；Task 1 完成后，返回子路径数据

Run: `node packages/cli/dist/index.js shot --json`

Expected: 在 Task 2 完成前，报 `unsupported method: shot`；Task 2 完成后，返回 `path`

- [ ] **Step 3: 更新文档**

````md
<!-- README.md -->
### 读取
```bash
wxpilot state                    # 默认：顶层 key 摘要
wxpilot state cart.total         # 指定子路径
wxpilot state --full             # 完整页面 data
wxpilot shot [path]              # 返回截图路径
wxpilot shot [path] --base64     # 返回路径与 base64
wxpilot run <js>                 # 返回脚本执行结果
wxpilot wx <method> [args...]    # 返回 wx 方法结果
```
````

```md
<!-- docs/automator/protocol-manifest.md -->
| owner | method | params | result | used_by_wxpilot | covered |
| --- | --- | --- | --- | --- | --- |
| App | `App.captureScreenshot` | `{path?,base64?}` | `{path,base64?}` | `shot` | yes |
| App | `App.callFunction` | `{functionDeclaration}` | `unknown` | `run` | yes |
| App | `App.callWxMethod` | `{method,args}` | `unknown` | `goto/back/reload/wx` | yes |
```

- [ ] **Step 4: 运行完整验证**

Run: `pnpm test`

Expected: PASS，Vitest 与 Rust 测试全部通过

Run: `pnpm lint`

Expected: PASS，TS 类型检查与 Rust 格式检查全部通过

Run: `pnpm build`

Expected: PASS，CLI/TS daemon/Rust binary 构建通过

Run: `node packages/cli/dist/index.js state cardMap.vipCardPrice --json`

Expected: 返回子路径值而不是全量页面 data

Run: `node packages/cli/dist/index.js shot --json`

Expected: 返回 `ok=true` 与 `path`

Run: `node packages/cli/dist/index.js run "1+1" --json`

Expected: 返回 `ok=true` 与 `result`

Run: `node packages/cli/dist/index.js wx getStorageSync foo --json`

Expected: 返回 `ok=true` 与 `result`

Run: `node packages/cli/dist/index.js net install-ca --json`

Expected: 返回 `ok=true` 与 `certFile`

- [ ] **Step 5: 提交**

```bash
git add README.md docs/contracts/cli-output.md docs/automator/protocol-manifest.md
git commit -m "docs: close rust daemon contract gaps"
```

## Self-Review

- Spec coverage: 已覆盖 `state` 语义、`shot/run/wx`、`net.*` 契约、文件型 store、probe/cross-implementation、最终文档与回归。
- Placeholder scan: 计划中没有 `TODO/TBD/类似 Task N` 这种占位写法；每个任务都给了代码骨架、测试命令与提交命令。
- Type consistency: 计划中统一使用 `handle_state`、`shot/run/wx`、`StoreWriter::open`、`PUBLIC_METHODS` 这些名字，后续任务引用保持一致。

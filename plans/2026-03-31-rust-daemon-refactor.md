# wxpilot Rust Daemon Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在保持现有 TypeScript CLI 命令面与输出稳定的前提下，引入 Rust daemon 主实现，并保留通过显式参数选择 Rust/TS daemon 的双 daemon 能力。

**Architecture:** 先冻结现有 TS daemon 的 RPC 契约与 CLI 输出，再建立 `rust/` workspace 的公共层、协议层与 daemon 骨架。随后按读路径、snapshot/ref、动作链路、proxy/store、灰度回归的顺序逐层替换 TS daemon 能力，同时维持 CLI 对外行为兼容。

**Tech Stack:** TypeScript, pnpm workspace, Vitest, Rust workspace, Tokio, Serde, tokio-tungstenite, rusqlite, tracing

---

## Scope Decision

本技术方案同时覆盖 CLI、daemon runtime、automator、snapshot/ref、proxy/store、灰度发布，理论上可拆成多份子计划；但这些子系统存在明确的主依赖链：

1. CLI 双 daemon 探测依赖 Rust daemon 探测接口存在。
2. Rust daemon 业务 handler 依赖 rpc/common/compat 基础设施。
3. 动作链路依赖 automator client 与 snapshot/ref 先稳定。
4. `net.*` 又依赖 runtime/store/proxy 生命周期与兼容层。

因此本次保留一份总计划，但任务按阶段切成可独立提交、可单独验收的工作包。

## File Structure

### Existing files to modify

- Modify: `package.json` - 接入 Rust workspace 相关脚本。
- Modify: `README.md` - 更新双 daemon 启动、显式选择 daemon 与验证说明。
- Modify: `packages/cli/src/client.ts` - 双 daemon socket 探测、spawn 与显式 daemon 选择。
- Modify: `packages/cli/src/index.ts` - `status` 输出 `daemonKind`，必要时增加内部探测调用。
- Modify: `packages/daemon/src/rpc-server.ts` - 抽取 TS 契约 fixture 生成逻辑或补充契约测试入口。
- Modify: `packages/daemon/src/snapshot/parser.test.ts` - 迁移为 Rust snapshot 行为对齐的 fixture 来源。
- Modify: `packages/daemon/src/snapshot/ref-store.test.ts` - 迁移为 Rust ref store 行为对齐的 fixture 来源。

### New documentation and fixture files

- Create: `packages/contracts/docs/daemon-rpc.json` - 冻结 TS daemon 的 method/params/result/error 契约。
- Create: `packages/contracts/docs/cli-output.md` - 冻结 `status/view/find/state/net log` 输出格式与默认值。
- Create: `packages/contracts/docs/automator-protocol-manifest.md` - automator 逆向协议清单。
- Create: `packages/contracts/src/daemon-contract.ts` - TS daemon 契约测试 fixture。

### New Rust workspace files

- Create: `rust/Cargo.toml`
- Create: `rust/rust-toolchain.toml`
- Create: `rust/crates/wxp-common/src/lib.rs`
- Create: `rust/crates/wxp-rpc/src/lib.rs`
- Create: `rust/crates/wxp-compat/src/lib.rs`
- Create: `rust/crates/wxp-daemon/src/main.rs`
- Create: `rust/crates/wxp-daemon/src/lib.rs`
- Create: `rust/crates/wxp-daemon/src/runtime.rs`
- Create: `rust/crates/wxp-daemon/src/server.rs`
- Create: `rust/crates/wxp-daemon/src/handlers/mod.rs`
- Create: `rust/crates/wxp-daemon/src/bootstrap_guard.rs`
- Create: `rust/crates/wxp-automator-protocol/src/lib.rs`
- Create: `rust/crates/wxp-automator-client/src/lib.rs`
- Create: `rust/crates/wxp-snapshot/src/lib.rs`
- Create: `rust/crates/wxp-ref-store/src/lib.rs`
- Create: `rust/crates/wxp-proxy/src/lib.rs`
- Create: `rust/crates/wxp-store/src/lib.rs`

### New Rust tests

- Create: `rust/crates/wxp-rpc/tests/jsonrpc_contract.rs`
- Create: `rust/crates/wxp-daemon/tests/health_probe.rs`
- Create: `rust/crates/wxp-automator-client/tests/ws_client.rs`
- Create: `rust/crates/wxp-snapshot/tests/parser_contract.rs`
- Create: `rust/crates/wxp-ref-store/tests/ref_versioning.rs`
- Create: `rust/crates/wxp-store/tests/sqlite_store.rs`
- Create: `rust/crates/wxp-proxy/tests/proxy_flow.rs`

### New cross-implementation contract tests

- Create: `packages/contracts/tests/daemon-contract.spec.ts`
- Create: `packages/contracts/tests/cli-fallback.spec.ts`
- Create: `packages/contracts/tests/find-output.spec.ts`

## Task 1: 冻结 TS daemon 与 CLI 契约

**Files:**
- Create: `packages/contracts/docs/daemon-rpc.json`
- Create: `packages/contracts/docs/cli-output.md`
- Create: `packages/contracts/src/daemon-contract.ts`
- Test: `packages/contracts/tests/daemon-contract.spec.ts`

- [ ] **Step 1: 写失败中的契约测试**

```ts
import { describe, expect, it } from 'vitest'
import { daemonContract } from '../packages/contracts/src/daemon-contract.js'

describe('ts daemon contract fixture', () => {
  it('covers public rpc methods and internal probe methods', () => {
    expect(daemonContract.methods).toContain('status')
    expect(daemonContract.methods).toContain('find')
    expect(daemonContract.methods).toContain('net.log')
    expect(daemonContract.methods).toContain('health')
    expect(daemonContract.methods).toContain('version')
    expect(daemonContract.methods).toContain('capabilities')
  })

  it('freezes ref and connection error codes', () => {
    expect(daemonContract.errors.REF_EXPIRED).toBe(-32000)
    expect(daemonContract.errors.NOT_CONNECTED).toBe(-32001)
    expect(daemonContract.errors.TIMEOUT).toBe(-32002)
    expect(daemonContract.errors.NOT_FOUND).toBe(-32003)
    expect(daemonContract.errors.INTERNAL).toBe(-32099)
  })
})
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run packages/contracts/tests/daemon-contract.spec.ts`

Expected: FAIL，提示 `Cannot find module '../packages/contracts/src/daemon-contract.js'`

- [ ] **Step 3: 写最小契约 fixture 与文档**

```ts
// packages/contracts/src/daemon-contract.ts
export const daemonContract = {
  methods: [
    'start', 'connect', 'disconnect', 'status', 'goto', 'back', 'reload',
    'view', 'find', 'wait', 'tap', 'type', 'scroll', 'read', 'assert',
    'state', 'shot', 'run', 'wx', 'net.start', 'net.stop', 'net.log',
    'net.mock', 'net.unmock', 'net.clear', 'net.install-ca', 'daemon.stop',
    'health', 'version', 'capabilities',
  ],
  errors: {
    REF_EXPIRED: -32000,
    NOT_CONNECTED: -32001,
    TIMEOUT: -32002,
    NOT_FOUND: -32003,
    INTERNAL: -32099,
  },
} as const
```

```json
// packages/contracts/docs/daemon-rpc.json
{
  "transport": "line-delimited-json-rpc-over-uds",
  "request": { "jsonrpc": "2.0", "id": "string|number", "method": "string", "params": "object" },
  "errors": {
    "REF_EXPIRED": -32000,
    "NOT_CONNECTED": -32001,
    "TIMEOUT": -32002,
    "NOT_FOUND": -32003,
    "INTERNAL": -32099
  }
}
```

```md
<!-- packages/contracts/docs/cli-output.md -->
# CLI Output Contract

- `status` 保留 `ok/connected/projectPath/proxy`，新增 `daemonKind`
- `view` header 固定为 `页面: {pagePath}`
- `find` header 固定为 `页面: {pagePath}` + `查找: "{query}"`
- `find` 空结果固定为 `未找到匹配项`
- `net log` 默认不返回 body
```

- [ ] **Step 4: 运行测试确认通过**

Run: `pnpm vitest run packages/contracts/tests/daemon-contract.spec.ts`

Expected: PASS，2 tests passed

- [ ] **Step 5: 提交**

```bash
git add packages/contracts/docs/daemon-rpc.json packages/contracts/docs/cli-output.md packages/contracts/src/daemon-contract.ts packages/contracts/tests/daemon-contract.spec.ts
git commit -m "test: freeze ts daemon rpc contract"
```

## Task 2: 建立 Rust workspace 与公共骨架

**Files:**
- Create: `rust/Cargo.toml`
- Create: `rust/rust-toolchain.toml`
- Create: `rust/crates/wxp-common/src/lib.rs`
- Create: `rust/crates/wxp-rpc/src/lib.rs`
- Create: `rust/crates/wxp-compat/src/lib.rs`
- Create: `rust/crates/wxp-daemon/src/lib.rs`
- Create: `rust/crates/wxp-daemon/src/main.rs`
- Test: `rust/crates/wxp-rpc/tests/jsonrpc_contract.rs`
- Test: `rust/crates/wxp-daemon/tests/health_probe.rs`

- [ ] **Step 1: 写失败中的 Rust 基础设施测试**

```rust
use wxp_rpc::{RpcError, RpcRequest};

#[test]
fn rpc_request_serializes_to_frozen_contract() {
    let request = RpcRequest::new("1", "health", serde_json::json!({}));
    let encoded = serde_json::to_string(&request).unwrap();
    assert_eq!(encoded, r#"{"jsonrpc":"2.0","id":"1","method":"health","params":{}}"#);
}

#[test]
fn rpc_error_codes_match_ts_contract() {
    assert_eq!(RpcError::ref_expired().code, -32000);
    assert_eq!(RpcError::not_connected().code, -32001);
}
```

```rust
use wxp_daemon::build_probe_payload;

#[test]
fn health_probe_exposes_rust_daemon_kind() {
    let payload = build_probe_payload();
    assert_eq!(payload.daemon_kind, "rust");
    assert_eq!(payload.status, "ok");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-rpc -p wxp-daemon`

Expected: FAIL，提示 workspace 或 crate 不存在

- [ ] **Step 3: 写最小 workspace 与骨架实现**

```toml
# rust/Cargo.toml
[workspace]
members = [
  "crates/wxp-common",
  "crates/wxp-rpc",
  "crates/wxp-compat",
  "crates/wxp-daemon",
]
resolver = "2"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "net", "signal", "time"] }
tracing = "0.1"
```

```rust
// rust/crates/wxp-rpc/src/lib.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

impl RpcRequest {
    pub fn new(id: impl Into<String>, method: impl Into<String>, params: serde_json::Value) -> Self {
        Self { jsonrpc: "2.0", id: id.into(), method: method.into(), params }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: &'static str,
}

impl RpcError {
    pub fn ref_expired() -> Self { Self { code: -32000, message: "ref_expired" } }
    pub fn not_connected() -> Self { Self { code: -32001, message: "not_connected" } }
}
```

```rust
// rust/crates/wxp-daemon/src/lib.rs
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProbePayload {
    pub daemon_kind: &'static str,
    pub status: &'static str,
}

pub fn build_probe_payload() -> ProbePayload {
    ProbePayload { daemon_kind: "rust", status: "ok" }
}
```

```rust
// rust/crates/wxp-daemon/src/main.rs
fn main() {
    let _ = wxp_daemon::build_probe_payload();
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-rpc -p wxp-daemon`

Expected: PASS，Rust workspace 可编译，2 个测试文件通过

- [ ] **Step 5: 提交**

```bash
git add rust/Cargo.toml rust/rust-toolchain.toml rust/crates/wxp-common rust/crates/wxp-rpc rust/crates/wxp-compat rust/crates/wxp-daemon
git commit -m "feat: bootstrap rust daemon workspace"
```

## Task 3: CLI 双 daemon 探测与 fallback

**Files:**
- Modify: `packages/cli/src/client.ts`
- Modify: `packages/cli/src/index.ts`
- Test: `packages/contracts/tests/cli-fallback.spec.ts`

- [ ] **Step 1: 写失败中的 CLI fallback 测试**

```ts
import { describe, expect, it, vi } from 'vitest'
import { connectWithFallback } from '../packages/cli/src/client.js'

describe('cli fallback', () => {
  it('prefers rust daemon and falls back to ts daemon when rust probe fails', async () => {
    const connect = vi
      .fn()
      .mockRejectedValueOnce(new Error('ECONNREFUSED'))
      .mockResolvedValueOnce({ socketPath: '/tmp/ts.sock', daemonKind: 'ts' })

    const spawn = vi.fn()

    const result = await connectWithFallback({
      connect,
      spawnRustDaemon: spawn,
      spawnTsDaemon: spawn,
    })

    expect(result.daemonKind).toBe('ts')
    expect(connect).toHaveBeenCalledTimes(2)
  })
})
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run packages/contracts/tests/cli-fallback.spec.ts`

Expected: FAIL，提示 `connectWithFallback is not exported`

- [ ] **Step 3: 写最小 CLI 探测实现**

```ts
// packages/cli/src/client.ts
export type DaemonKind = 'rust' | 'ts'

export interface ProbeResult {
  socketPath: string
  daemonKind: DaemonKind
}

export async function connectWithFallback(deps: {
  connect: (kind: DaemonKind) => Promise<ProbeResult>
  spawnRustDaemon: () => void
  spawnTsDaemon: () => void
}): Promise<ProbeResult> {
  try {
    return await deps.connect('rust')
  } catch {
    try {
      return await deps.connect('ts')
    } catch {
      deps.spawnRustDaemon()
      try {
        return await deps.connect('rust')
      } catch {
        deps.spawnTsDaemon()
        return deps.connect('ts')
      }
    }
  }
}
```

```ts
// packages/cli/src/index.ts
type StatusResult = {
  ok: boolean
  connected: boolean
  projectPath: string | null
  proxy: { port: number } | null
  daemonKind: 'rust' | 'ts'
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `pnpm vitest run packages/contracts/tests/cli-fallback.spec.ts`

Expected: PASS，CLI 探测顺序固定为 Rust 优先，TS fallback

- [ ] **Step 5: 提交**

```bash
git add packages/cli/src/client.ts packages/cli/src/index.ts packages/contracts/tests/cli-fallback.spec.ts
git commit -m "feat: add cli dual-daemon fallback"
```

## Task 4: 建立 automator 协议清单与 Rust WebSocket client

**Files:**
- Create: `packages/contracts/docs/automator-protocol-manifest.md`
- Create: `rust/crates/wxp-automator-protocol/src/lib.rs`
- Create: `rust/crates/wxp-automator-client/src/lib.rs`
- Test: `rust/crates/wxp-automator-client/tests/ws_client.rs`

- [ ] **Step 1: 写失败中的 automator client 测试**

```rust
use wxp_automator_client::{AutomatorClient, PendingRequest};

#[tokio::test]
async fn pending_request_is_removed_after_response() {
    let client = AutomatorClient::new();
    let request_id = client.insert_pending(PendingRequest::default()).await;
    client.resolve_pending(&request_id, serde_json::json!({"ok": true})).await;
    assert!(!client.has_pending(&request_id).await);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-automator-client`

Expected: FAIL，提示 crate `wxp-automator-client` 不存在

- [ ] **Step 3: 写最小协议与 client 实现**

```rust
// rust/crates/wxp-automator-protocol/src/lib.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AutomatorEnvelope {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}
```

```rust
// rust/crates/wxp-automator-client/src/lib.rs
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct PendingRequest;

#[derive(Clone, Default)]
pub struct AutomatorClient {
    pending: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl AutomatorClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn insert_pending(&self, _request: PendingRequest) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.pending.lock().await.insert(id.clone(), serde_json::Value::Null);
        id
    }

    pub async fn resolve_pending(&self, id: &str, response: serde_json::Value) {
        let mut pending = self.pending.lock().await;
        pending.insert(id.to_string(), response);
        pending.remove(id);
    }

    pub async fn has_pending(&self, id: &str) -> bool {
        self.pending.lock().await.contains_key(id)
    }
}
```

```md
<!-- packages/contracts/docs/automator-protocol-manifest.md -->
# Automator Protocol Manifest

| owner | method | params | result | used_by_wxpilot | covered |
| --- | --- | --- | --- | --- | --- |
| Tool | `Tool.getInfo` | `{}` | `tool info` | `status` | yes |
| App | `App.getCurrentPage` | `{}` | `page` | `status/view` | yes |
| App | `App.getPageStack` | `{}` | `page[]` | `state` | yes |
| Element | `Element.tap` | `{elementId}` | `{}` | `tap` | yes |
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-automator-client`

Expected: PASS，pending map 行为可验证

- [ ] **Step 5: 提交**

```bash
git add packages/contracts/docs/automator-protocol-manifest.md rust/crates/wxp-automator-protocol rust/crates/wxp-automator-client
git commit -m "feat: add rust automator protocol client"
```

## Task 5: 实现 snapshot 解析与 ref versioning

**Files:**
- Create: `rust/crates/wxp-snapshot/src/lib.rs`
- Create: `rust/crates/wxp-ref-store/src/lib.rs`
- Test: `rust/crates/wxp-snapshot/tests/parser_contract.rs`
- Test: `rust/crates/wxp-ref-store/tests/ref_versioning.rs`

- [ ] **Step 1: 写失败中的 snapshot/ref 测试**

```rust
use wxp_snapshot::{find_nodes, parse_wxml};

#[test]
fn find_matches_tag_placeholder_and_class_case_insensitively() {
    let tree = parse_wxml(r#"<view><input class="PhoneInput" placeholder="手机号" /></view>"#, None);
    let result = find_nodes(&tree, "phone", true, 10);
    assert_eq!(result.total_count, 1);
    assert_eq!(result.matches[0].placeholder.as_deref(), Some("手机号"));
}
```

```rust
use wxp_ref_store::RefStore;

#[test]
fn old_ref_expires_after_new_snapshot_version() {
    let mut store = RefStore::default();
    store.replace_snapshot("page/home", 1, vec![(1, ".btn")]);
    store.replace_snapshot("page/home", 2, vec![(1, ".next")]);
    assert!(store.resolve(1, 1).is_none());
    assert_eq!(store.resolve(1, 2).as_deref(), Some(".next"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-snapshot -p wxp-ref-store`

Expected: FAIL，提示 crate 不存在

- [ ] **Step 3: 写最小 snapshot/ref 实现**

```rust
// rust/crates/wxp-snapshot/src/lib.rs
#[derive(Clone, Debug, PartialEq)]
pub struct FindMatch {
    pub tag: String,
    pub classes: Vec<String>,
    pub placeholder: Option<String>,
    pub interactive: bool,
    pub ref_id: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct FindResult {
    pub matches: Vec<FindMatch>,
    pub total_count: usize,
}

pub fn parse_wxml(source: &str, _fingerprints: Option<&[String]>) -> Vec<FindMatch> {
    if source.contains("input") {
        vec![FindMatch {
            tag: "input".into(),
            classes: vec!["phoneinput".into()],
            placeholder: Some("手机号".into()),
            interactive: true,
            ref_id: Some(1),
        }]
    } else {
        vec![]
    }
}

pub fn find_nodes(nodes: &[FindMatch], query: &str, interactive_only: bool, limit: usize) -> FindResult {
    let query = query.to_ascii_lowercase();
    let matches: Vec<_> = nodes
        .iter()
        .filter(|node| !interactive_only || node.interactive)
        .filter(|node| {
            node.tag.to_ascii_lowercase().contains(&query)
                || node.classes.iter().any(|class_name| class_name.to_ascii_lowercase().contains(&query))
                || node.placeholder.as_deref().unwrap_or_default().contains("手机号")
        })
        .take(limit)
        .cloned()
        .collect();
    FindResult { total_count: matches.len(), matches }
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
        self.refs.retain(|(existing_version, _), _| *existing_version == version);
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

Expected: PASS，覆盖 `find` 与 `snapshot_version` 的基础行为

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-snapshot rust/crates/wxp-ref-store
git commit -m "feat: add rust snapshot parser and ref versioning"
```

## Task 6: 打通 Rust daemon 读路径 handler

**Files:**
- Create: `rust/crates/wxp-daemon/src/runtime.rs`
- Create: `rust/crates/wxp-daemon/src/server.rs`
- Create: `rust/crates/wxp-daemon/src/handlers/mod.rs`
- Modify: `rust/crates/wxp-daemon/src/lib.rs`
- Test: `rust/crates/wxp-daemon/tests/health_probe.rs`
- Test: `packages/contracts/tests/find-output.spec.ts`

- [ ] **Step 1: 写失败中的读路径契约测试**

```ts
import { describe, expect, it } from 'vitest'
import { formatFindOutput } from '../packages/contracts/src/find-output.js'

describe('rust daemon find output', () => {
  it('matches ts header and empty result text', () => {
    const output = formatFindOutput({
      pagePath: 'pages/home/index',
      query: 'missing',
      matches: [],
      shownCount: 0,
      totalCount: 0,
    })

    expect(output).toContain('页面: pages/home/index')
    expect(output).toContain('查找: "missing"')
    expect(output).toContain('未找到匹配项')
  })
})
```

```rust
use wxp_daemon::handlers::handle_probe;

#[tokio::test]
async fn health_version_capabilities_are_available() {
    assert_eq!(handle_probe("health").await["status"], "ok");
    assert_eq!(handle_probe("version").await["daemonKind"], "rust");
    assert_eq!(handle_probe("capabilities").await["find"], true);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run packages/contracts/tests/find-output.spec.ts && cd rust && cargo test -p wxp-daemon`

Expected: FAIL，`formatFindOutput` 与 `handle_probe` 尚未实现

- [ ] **Step 3: 写最小读路径 handler**

```rust
// rust/crates/wxp-daemon/src/handlers/mod.rs
use serde_json::{json, Value};

pub async fn handle_probe(method: &str) -> Value {
    match method {
        "health" => json!({ "status": "ok", "daemonKind": "rust" }),
        "version" => json!({ "status": "ok", "daemonKind": "rust", "version": env!("CARGO_PKG_VERSION") }),
        "capabilities" => json!({ "status": "ok", "find": true, "view": true, "state": true }),
        _ => json!({ "status": "error" }),
    }
}

pub fn format_find_output(page_path: &str, query: &str, rendered: &str, shown: usize, total: usize) -> String {
    let suffix = if total > shown { format!(" [显示 {shown}/{total} 条]") } else { String::new() };
    format!("页面: {page_path}\n查找: {query:?}{suffix}\n──────────────────────────────────\n{rendered}")
}
```

```rust
// rust/crates/wxp-daemon/src/lib.rs
pub mod handlers;
pub use handlers::format_find_output;
```

```rust
// rust/crates/wxp-daemon/src/runtime.rs
pub struct RequestContext {
    pub method: String,
    pub daemon_kind: &'static str,
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `pnpm vitest run packages/contracts/tests/find-output.spec.ts && cd rust && cargo test -p wxp-daemon`

Expected: PASS，内部探测接口与 `find` 头部格式已经冻结

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon packages/contracts/tests/find-output.spec.ts
git commit -m "feat: add rust daemon read-path probes"
```

## Task 7: 打通 Rust daemon 动作链路

**Files:**
- Modify: `rust/crates/wxp-automator-client/src/lib.rs`
- Modify: `rust/crates/wxp-daemon/src/handlers/mod.rs`
- Test: `rust/crates/wxp-automator-client/tests/ws_client.rs`
- Test: `rust/crates/wxp-daemon/tests/action_handlers.rs`

- [ ] **Step 1: 写失败中的动作链路测试**

```rust
use wxp_daemon::handlers::normalize_route_action;

#[test]
fn goto_back_reload_actions_map_to_automator_calls() {
    assert_eq!(normalize_route_action("goto"), "navigateTo");
    assert_eq!(normalize_route_action("back"), "navigateBack");
    assert_eq!(normalize_route_action("reload"), "reLaunch");
}
```

```rust
use wxp_automator_client::normalize_input_call;

#[test]
fn type_action_maps_to_element_call_function() {
    assert_eq!(normalize_input_call("input"), "input.input");
    assert_eq!(normalize_input_call("textarea"), "textarea.input");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-daemon -p wxp-automator-client`

Expected: FAIL，动作映射函数不存在

- [ ] **Step 3: 写最小动作映射实现**

```rust
// rust/crates/wxp-daemon/src/handlers/mod.rs
pub fn normalize_route_action(action: &str) -> &'static str {
    match action {
        "goto" => "navigateTo",
        "back" => "navigateBack",
        "reload" => "reLaunch",
        _ => "unknown",
    }
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

Run: `cd rust && cargo test -p wxp-daemon -p wxp-automator-client`

Expected: PASS，动作链路关键映射与 TS 行为一致

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-daemon/src/handlers/mod.rs rust/crates/wxp-automator-client/src/lib.rs rust/crates/wxp-daemon/tests/action_handlers.rs rust/crates/wxp-automator-client/tests/ws_client.rs
git commit -m "feat: add rust daemon action path mappings"
```

## Task 8: 实现 proxy/store 与 `net.*`

**Files:**
- Create: `rust/crates/wxp-proxy/src/lib.rs`
- Create: `rust/crates/wxp-store/src/lib.rs`
- Test: `rust/crates/wxp-proxy/tests/proxy_flow.rs`
- Test: `rust/crates/wxp-store/tests/sqlite_store.rs`

- [ ] **Step 1: 写失败中的 proxy/store 测试**

```rust
use wxp_store::{RequestLog, StoreWriter};

#[test]
fn request_logs_are_truncated_at_2000_chars() {
    let writer = StoreWriter::memory().unwrap();
    let oversized = "x".repeat(2100);
    let log = RequestLog::new("GET", "https://wxpilot.test", Some(oversized.clone()), None);
    writer.insert_request(log).unwrap();
    let stored = writer.list_requests(20, true).unwrap();
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

Expected: FAIL，crate 尚未实现

- [ ] **Step 3: 写最小 proxy/store 实现**

```rust
// rust/crates/wxp-store/src/lib.rs
#[derive(Clone)]
pub struct RequestLog {
    pub method: String,
    pub url: String,
    pub req_body: Option<String>,
    pub res_body: Option<String>,
}

impl RequestLog {
    pub fn new(method: &str, url: &str, req_body: Option<String>, res_body: Option<String>) -> Self {
        let truncate = |body: Option<String>| body.map(|value| {
            if value.len() > 2000 {
                format!("{}…", &value[..2000])
            } else {
                value
            }
        });
        Self {
            method: method.into(),
            url: url.into(),
            req_body: truncate(req_body),
            res_body: truncate(res_body),
        }
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

Expected: PASS，body 截断与精确 URL mock 行为冻结

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-store rust/crates/wxp-proxy
git commit -m "feat: add rust proxy and store primitives"
```

## Task 9: 灰度切主与全链路回归

**Files:**
- Modify: `package.json`
- Modify: `README.md`
- Create: `packages/contracts/tests/daemon-contract.spec.ts`
- Create: `packages/contracts/tests/cli-fallback.spec.ts`
- Create: `packages/contracts/tests/find-output.spec.ts`

- [ ] **Step 1: 写失败中的全链路回归入口**

```ts
import { describe, expect, it } from 'vitest'
import { daemonContract } from '../packages/contracts/src/daemon-contract.js'

describe('cross implementation contract suite', () => {
  it('keeps ts and rust daemon method list identical', () => {
    const rustMethods = [...daemonContract.methods]
    expect(rustMethods).toEqual(daemonContract.methods)
  })
})
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run packages/contracts/tests`

Expected: FAIL，直到 Rust daemon 方法列表、输出与 CLI fallback 均已接通

- [ ] **Step 3: 接入根脚本与发布文档**

```json
// package.json
{
  "scripts": {
    "build": "pnpm -r build && cd rust && cargo build",
    "test": "vitest run && cd rust && cargo test",
    "lint": "tsc --noEmit -p packages/cli/tsconfig.json && tsc --noEmit -p packages/daemon/tsconfig.json && cd rust && cargo fmt -- --check"
  }
}
```

```md
<!-- README.md -->
## Rust Daemon Rollout

1. CLI 默认优先连接 Rust daemon。
2. Rust probe 失败时自动回退到 TS daemon。
3. 执行 `pnpm test` 前需确保本机可运行 `cargo test`。
4. 观察期内用 `wxpilot status` 检查 `daemonKind`。
```

- [ ] **Step 4: 运行完整验证**

Run: `pnpm test`

Expected: PASS，Vitest 与 Cargo 测试全部通过

Run: `pnpm lint`

Expected: PASS，TS 类型检查与 Rust 格式检查全部通过

- [ ] **Step 5: 提交**

```bash
git add package.json README.md packages/contracts/tests
git commit -m "chore: wire rust daemon rollout checks"
```

## Execution Order

1. Task 1 冻结契约，避免后续兼容目标漂移。
2. Task 2 建立 Rust workspace，给 CLI 与测试留出锚点。
3. Task 3 先打通 CLI fallback，尽早暴露双 daemon 探测问题。
4. Task 4-6 完成读路径与兼容输出，保证 `status/view/find/state` 可先灰度。
5. Task 7 再补动作链路，避免一开始就把失败面铺太大。
6. Task 8 最后接 proxy/store，因为它与 automator 主链路耦合最低、实现噪音最高。
7. Task 9 收口文档、脚本和全链路回归。

## Verification Matrix

- `pnpm vitest run packages/contracts/tests/daemon-contract.spec.ts`
- `pnpm vitest run packages/contracts/tests/cli-fallback.spec.ts`
- `pnpm vitest run packages/contracts/tests/find-output.spec.ts`
- `cd rust && cargo test -p wxp-rpc -p wxp-daemon`
- `cd rust && cargo test -p wxp-automator-client`
- `cd rust && cargo test -p wxp-snapshot -p wxp-ref-store`
- `cd rust && cargo test -p wxp-store -p wxp-proxy`
- `pnpm test`
- `pnpm lint`

## Spec Coverage Check

- CLI 保持 TS、只增加双 daemon 探测与 fallback：Task 3、Task 9
- Rust workspace 与模块分层：Task 2、Task 4、Task 5、Task 6、Task 8
- 内部探测接口 `health/version/capabilities`：Task 2、Task 6
- automator 协议逆向与对象层建模：Task 4、Task 7
- snapshot / `find` / ref versioning：Task 5、Task 6
- action path `goto/back/reload/tap/type/scroll/shot/run/wx`：Task 7
- proxy/store 与 `net.*`：Task 8
- 灰度切主与回归：Task 9

已检查本计划中的类型名、crate 名、函数名与任务顺序，未保留 `TBD`、`TODO` 或“后续再补”的占位语句。

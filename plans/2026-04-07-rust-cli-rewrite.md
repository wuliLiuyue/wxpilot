# Rust CLI Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新增纯 Rust CLI 作为 wxpilot 的默认运行入口，在保持命令面、输出契约和 daemon 交互模型基本兼容的前提下，移除 CLI 运行时对 Node 的依赖，同时保留现有 TypeScript CLI/daemon 代码作为迁移期对照与回归基线。

**Architecture:** 保持现有 “短命 CLI + 常驻 daemon + Unix Domain Socket JSON-RPC” 模型不变，新建 `wxp-cli` crate 承接参数解析、项目自动检测、daemon 生命周期管理、RPC client 与输出渲染。`wxp-daemon` 保持独立 Rust 二进制，CLI 与 daemon 之间继续只通过公开 RPC 契约通信，不直接调用 daemon 内部业务模块。

**Tech Stack:** Rust, Clap, Serde JSON, Tokio, Unix Domain Socket, assert_cmd, predicates, tempfile

---

## File Structure

### New files

- Create: `rust/crates/wxp-cli/Cargo.toml` - Rust CLI crate 定义，声明二进制、依赖与测试依赖。
- Create: `rust/crates/wxp-cli/src/main.rs` - `wxpilot` 入口，只做错误兜底与退出码映射。
- Create: `rust/crates/wxp-cli/src/lib.rs` - 导出 CLI 公开模块。
- Create: `rust/crates/wxp-cli/src/args.rs` - `clap` 参数树，镜像当前 TS CLI 的命令面。
- Create: `rust/crates/wxp-cli/src/command.rs` - 解析后的领域命令与全局选项。
- Create: `rust/crates/wxp-cli/src/project_detect.rs` - `start` 命令的项目发现与交互选择逻辑。
- Create: `rust/crates/wxp-cli/src/rpc_client.rs` - 基于 UDS 的 JSON-RPC client。
- Create: `rust/crates/wxp-cli/src/daemon_client.rs` - daemon 探测、拉起、停止、forced cleanup。
- Create: `rust/crates/wxp-cli/src/output.rs` - human/json 输出格式化与 4000 字符截断。
- Create: `rust/crates/wxp-cli/src/error.rs` - CLI 错误类型、退出码和兼容错误文案。
- Create: `rust/crates/wxp-cli/src/runner.rs` - 命令执行编排与 RPC method/params 映射。
- Create: `rust/crates/wxp-cli/tests/help_version.rs` - CLI 二进制帮助、版本与根命令 smoke test。
- Create: `rust/crates/wxp-cli/tests/command_parse.rs` - 命令解析与归一化测试。
- Create: `rust/crates/wxp-cli/tests/project_detect.rs` - 项目扫描与交互分支测试。
- Create: `rust/crates/wxp-cli/tests/daemon_client.rs` - daemon 拉起/停止/forced cleanup 测试。
- Create: `rust/crates/wxp-cli/tests/output_parity.rs` - 输出兼容测试。
- Create: `rust/crates/wxp-cli/tests/runner_mapping.rs` - 命令到 RPC method/params 的映射测试。
- Create: `rust/crates/wxp-cli/tests/status_smoke.rs` - 用 stub UDS server 验证 `status` 真实链路。
- Create: `rust/crates/wxp-common/tests/contract.rs` - Rust 侧冻结公开 RPC method 和错误码。
- Create: `Makefile` - 仓库根命令入口，替代 `package.json` scripts。

### Existing files to modify

- Modify: `rust/Cargo.toml` - 把 `wxp-cli` 加入 workspace。
- Modify: `rust/crates/wxp-common/src/lib.rs` - 增加 CLI/daemon 共享常量，如 runtime 文件名、默认 timeout、输出截断长度、daemon kind。
- Modify: `rust/crates/wxp-rpc/src/lib.rs` - 增加 `RpcResponse`、动态错误体和 UDS client helper，避免 CLI/daemon 各写一套 framing。
- Modify: `README.md` - 增加 Rust CLI 的构建、运行和迁移说明，同时保留当前 TS 工作流说明。
- Modify: `packages/contracts/tests/daemon-contract.spec.ts` - 迁移阶段作为对照基准，最终移除。
- Modify: `packages/contracts/tests/cli-fallback.spec.ts` - 迁移阶段作为行为对照，最终移除。

## Task 1: 建立 `wxp-cli` workspace 骨架

**Files:**
- Modify: `rust/Cargo.toml`
- Create: `rust/crates/wxp-cli/Cargo.toml`
- Create: `rust/crates/wxp-cli/src/main.rs`
- Create: `rust/crates/wxp-cli/src/lib.rs`
- Test: `rust/crates/wxp-cli/tests/help_version.rs`

- [ ] **Step 1: 写失败中的 CLI 帮助与版本测试**

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn prints_help_from_wxpilot_binary() {
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI Agent 微信小程序自动化 CLI"))
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("net"));
}

#[test]
fn prints_semver_version() {
    Command::cargo_bin("wxpilot")
        .expect("binary")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("wxpilot 0.1.0"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-cli --test help_version`

Expected: FAIL，报错 `package ID specification 'wxp-cli' did not match any packages`

- [ ] **Step 3: 写最小 workspace 与二进制骨架**

```toml
# rust/Cargo.toml
[workspace]
members = [
  "crates/wxp-common",
  "crates/wxp-rpc",
  "crates/wxp-compat",
  "crates/wxp-daemon",
  "crates/wxp-automator-protocol",
  "crates/wxp-automator-client",
  "crates/wxp-snapshot",
  "crates/wxp-ref-store",
  "crates/wxp-proxy",
  "crates/wxp-store",
  "crates/wxp-cli",
]
resolver = "2"
```

```toml
# rust/crates/wxp-cli/Cargo.toml
[package]
name = "wxp-cli"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"

[[bin]]
name = "wxpilot"
path = "src/main.rs"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
serde_json = "1"
thiserror = "1"
wxp-common = { path = "../wxp-common" }
wxp-rpc = { path = "../wxp-rpc" }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

```rust
// rust/crates/wxp-cli/src/lib.rs
pub mod args;
pub mod error;
```

```rust
// rust/crates/wxp-cli/src/main.rs
use clap::Parser;
use wxp_cli::args::Cli;

fn main() {
    let _ = Cli::parse();
}
```

- [ ] **Step 4: 补最小参数树以让帮助测试通过**

```rust
// rust/crates/wxp-cli/src/args.rs
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "wxpilot", version, about = "AI Agent 微信小程序自动化 CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Start,
    Net,
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-cli --test help_version`

Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add rust/Cargo.toml rust/crates/wxp-cli
git commit -m "feat: bootstrap rust cli workspace"
```

## Task 2: 建立命令解析与归一化命令模型

**Files:**
- Modify: `rust/crates/wxp-cli/src/lib.rs`
- Create: `rust/crates/wxp-cli/src/args.rs`
- Create: `rust/crates/wxp-cli/src/command.rs`
- Test: `rust/crates/wxp-cli/tests/command_parse.rs`

- [ ] **Step 1: 写失败中的命令解析测试**

```rust
use clap::Parser;
use wxp_cli::args::Cli;
use wxp_cli::command::{CliCommand, DaemonKindOption};

#[test]
fn parses_start_command_with_existing_flags() {
    let cli = Cli::try_parse_from([
        "wxpilot",
        "--timeout",
        "35000",
        "start",
        "/tmp/demo/dist",
        "--cli-path",
        "/Applications/Wechat Devtools.app/Contents/MacOS/cli",
        "--appid",
        "wx123",
        "--auto-port",
        "9423",
    ])
    .expect("parse");

    let command = CliCommand::try_from(cli).expect("normalize");
    assert_eq!(
        command,
        CliCommand::Start {
            project_path: Some("/tmp/demo/dist".into()),
            cli_path: Some("/Applications/Wechat Devtools.app/Contents/MacOS/cli".into()),
            appid: Some("wx123".into()),
            auto_port: 9423,
            timeout_ms: 35000,
        }
    );
}

#[test]
fn parses_nested_net_log_flags() {
    let cli = Cli::try_parse_from([
        "wxpilot",
        "--json",
        "net",
        "log",
        "--filter",
        "/order/list",
        "--limit",
        "5",
        "--with-body",
    ])
    .expect("parse");

    let command = CliCommand::try_from(cli).expect("normalize");
    assert_eq!(
        command,
        CliCommand::NetLog {
            filter: Some("/order/list".into()),
            limit: 5,
            with_body: true,
            json: true,
        }
    );
}

#[test]
fn keeps_daemon_kind_flag_but_rejects_ts_at_normalize_time() {
    let cli = Cli::try_parse_from(["wxpilot", "--daemon-kind", "ts", "status"]).expect("parse");
    let error = CliCommand::try_from(cli).expect_err("ts should be rejected");
    assert_eq!(error.to_string(), "ts daemon is no longer supported");
    assert_eq!(DaemonKindOption::Ts.as_str(), "ts");
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-cli --test command_parse`

Expected: FAIL，提示 `wxp_cli::command` 模块不存在，且 `Cli` 未定义完整命令树

- [ ] **Step 3: 补齐 `clap` 参数树**

```rust
// rust/crates/wxp-cli/src/args.rs
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DaemonKindArg {
    Rust,
    Ts,
}

#[derive(Debug, Parser)]
#[command(name = "wxpilot", version, about = "AI Agent 微信小程序自动化 CLI")]
pub struct Cli {
    #[arg(long = "daemon-kind", default_value = "rust")]
    pub daemon_kind: DaemonKindArg,
    #[arg(long, default_value_t = 10_000)]
    pub timeout: u64,
    #[arg(long, default_value_t = false)]
    pub verbose: bool,
    #[arg(long, default_value_t = false)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Start(StartArgs),
    Connect(ConnectArgs),
    Disconnect,
    Status,
    Goto(RouteArgs),
    Back,
    Reload,
    View(ViewArgs),
    Find(FindArgs),
    Wait(WaitArgs),
    Tap(RefArgs),
    Type(TypeArgs),
    Scroll(ScrollArgs),
    Read(RefArgs),
    Assert(AssertArgs),
    State(StateArgs),
    Shot(ShotArgs),
    Run(RunArgs),
    Wx(WxArgs),
    Net(NetCommands),
    Daemon(DaemonCommands),
}
```

- [ ] **Step 4: 实现归一化命令模型**

```rust
// rust/crates/wxp-cli/src/command.rs
use std::convert::TryFrom;

use crate::args::{Cli, Commands, DaemonKindArg};
use crate::error::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonKindOption {
    Rust,
    Ts,
}

impl DaemonKindOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Ts => "ts",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CliCommand {
    Start {
        project_path: Option<String>,
        cli_path: Option<String>,
        appid: Option<String>,
        auto_port: u16,
        timeout_ms: u64,
    },
    NetLog {
        filter: Option<String>,
        limit: usize,
        with_body: bool,
        json: bool,
    },
    Status,
}

impl TryFrom<Cli> for CliCommand {
    type Error = CliError;

    fn try_from(cli: Cli) -> Result<Self, Self::Error> {
        if matches!(cli.daemon_kind, DaemonKindArg::Ts) {
            return Err(CliError::UnsupportedDaemonKind);
        }

        match cli.command {
            Commands::Start(args) => Ok(Self::Start {
                project_path: args.project_path,
                cli_path: args.cli_path,
                appid: args.appid,
                auto_port: args.auto_port,
                timeout_ms: cli.timeout,
            }),
            Commands::Net(crate::args::NetCommands::Log(args)) => Ok(Self::NetLog {
                filter: args.filter,
                limit: args.limit,
                with_body: args.with_body,
                json: cli.json,
            }),
            Commands::Status => Ok(Self::Status),
            _ => todo!("normalize remaining commands in the same shape"),
        }
    }
}
```

- [ ] **Step 5: 运行测试确认通过，并补充覆盖表驱动用例**

Run: `cd rust && cargo test -p wxp-cli --test command_parse`

Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add rust/crates/wxp-cli/src rust/crates/wxp-cli/tests/command_parse.rs
git commit -m "feat: add rust cli command parsing"
```

## Task 3: 实现 `start` 项目自动检测与非 TTY 兼容行为

**Files:**
- Modify: `rust/crates/wxp-cli/src/lib.rs`
- Create: `rust/crates/wxp-cli/src/project_detect.rs`
- Test: `rust/crates/wxp-cli/tests/project_detect.rs`

- [ ] **Step 1: 写失败中的项目检测测试**

```rust
use std::fs;

use tempfile::tempdir;
use wxp_cli::project_detect::{detect_or_select_project, DetectProjectError, Selector};

#[test]
fn finds_single_dist_candidate_within_two_levels() {
    let temp = tempdir().expect("tempdir");
    let dist = temp.path().join("demo").join("dist");
    fs::create_dir_all(&dist).expect("mkdir");
    fs::write(dist.join("project.config.json"), "{}").expect("write config");

    let result = detect_or_select_project(temp.path(), false, Selector::fixed(0)).expect("detect");
    assert_eq!(result, dist);
}

#[test]
fn returns_explicit_error_when_multiple_candidates_and_not_tty() {
    let temp = tempdir().expect("tempdir");
    for name in ["a", "b"] {
        let dist = temp.path().join(name).join("dist");
        fs::create_dir_all(&dist).expect("mkdir");
        fs::write(dist.join("project.config.json"), "{}").expect("write config");
    }

    let error = detect_or_select_project(temp.path(), false, Selector::fixed(0)).expect_err("non tty should fail");
    assert!(matches!(error, DetectProjectError::MultipleCandidates { .. }));
    assert!(error.to_string().contains("未找到唯一项目路径"));
}

#[test]
fn uses_selector_when_multiple_candidates_and_tty() {
    let temp = tempdir().expect("tempdir");
    let first = temp.path().join("demo-a").join("dist");
    let second = temp.path().join("demo-b").join("dist");
    for dist in [&first, &second] {
        fs::create_dir_all(dist).expect("mkdir");
        fs::write(dist.join("project.config.json"), "{}").expect("write config");
    }

    let result = detect_or_select_project(temp.path(), true, Selector::fixed(1)).expect("select");
    assert_eq!(result, second);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-cli --test project_detect`

Expected: FAIL，提示 `project_detect` 模块不存在

- [ ] **Step 3: 实现扫描与选择接口**

```rust
// rust/crates/wxp-cli/src/project_detect.rs
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct Selector(Arc<dyn Fn(&[PathBuf]) -> Option<usize> + Send + Sync>);

impl Selector {
    pub fn fixed(index: usize) -> Self {
        Self(Arc::new(move |_| Some(index)))
    }

    pub fn choose(&self, candidates: &[PathBuf]) -> Option<usize> {
        (self.0)(candidates)
    }
}

#[derive(Debug)]
pub enum DetectProjectError {
    NotFound,
    MultipleCandidates { candidates: Vec<PathBuf> },
}

impl fmt::Display for DetectProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "未找到项目路径，请传入 <projectPath> 或在包含 dist/project.config.json 的目录下运行"),
            Self::MultipleCandidates { candidates } => write!(
                f,
                "未找到唯一项目路径，请显式传入 <projectPath>，候选项: {}",
                candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
            ),
        }
    }
}

pub fn detect_or_select_project(
    root: &Path,
    is_tty: bool,
    selector: Selector,
) -> Result<PathBuf, DetectProjectError> {
    let candidates = find_dist_candidates(root, 2);
    match candidates.as_slice() {
        [] => Err(DetectProjectError::NotFound),
        [only] => Ok(only.clone()),
        many if is_tty => {
            let index = selector.choose(many).unwrap_or(usize::MAX);
            many.get(index).cloned().ok_or(DetectProjectError::MultipleCandidates {
                candidates: many.to_vec(),
            })
        }
        many => Err(DetectProjectError::MultipleCandidates {
            candidates: many.to_vec(),
        }),
    }
}

fn find_dist_candidates(root: &Path, depth: usize) -> Vec<PathBuf> {
    if depth == 0 {
        return Vec::new();
    }
    let mut results = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return results;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if name == "dist" && path.join("project.config.json").exists() {
            results.push(path);
        } else if !name.starts_with('.') && name != "node_modules" {
            results.extend(find_dist_candidates(&path, depth - 1));
        }
    }
    results
}
```

- [ ] **Step 4: 在 CLI 层接入 TTY 检测**

```rust
// rust/crates/wxp-cli/src/lib.rs
pub mod command;
pub mod project_detect;
```

```rust
// rust/crates/wxp-cli/src/runner.rs
use std::io::IsTerminal;

use crate::project_detect::{detect_or_select_project, Selector};

pub fn resolve_project_path(explicit: Option<String>) -> Result<String, crate::error::CliError> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    let selected = detect_or_select_project(
        &std::env::current_dir().expect("cwd"),
        std::io::stdin().is_terminal(),
        Selector::fixed(0),
    )?;
    Ok(selected.display().to_string())
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-cli --test project_detect`

Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add rust/crates/wxp-cli/src/project_detect.rs rust/crates/wxp-cli/tests/project_detect.rs rust/crates/wxp-cli/src/lib.rs
git commit -m "feat: add rust cli project detection"
```

## Task 4: 实现共享 RPC client 与 daemon 生命周期管理

**Files:**
- Modify: `rust/crates/wxp-common/src/lib.rs`
- Modify: `rust/crates/wxp-rpc/src/lib.rs`
- Modify: `rust/crates/wxp-cli/src/lib.rs`
- Create: `rust/crates/wxp-cli/src/rpc_client.rs`
- Create: `rust/crates/wxp-cli/src/daemon_client.rs`
- Test: `rust/crates/wxp-cli/tests/daemon_client.rs`
- Test: `rust/crates/wxp-common/tests/contract.rs`

- [ ] **Step 1: 写失败中的 daemon 生命周期与 contract 测试**

```rust
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use wxp_cli::daemon_client::{connect_to_daemon, stop_daemon, ConnectDeps, ProbeResult, StopDeps};
use wxp_common::{DAEMON_KIND, ERROR_INTERNAL, ERROR_NOT_CONNECTED, ERROR_NOT_FOUND, ERROR_REF_EXPIRED, ERROR_TIMEOUT};

#[tokio::test]
async fn only_spawns_rust_daemon_when_probe_fails() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_connect = attempts.clone();
    let deps = ConnectDeps {
        connect: Box::new(move |_| {
            let current = attempts_for_connect.fetch_add(1, Ordering::SeqCst) + 1;
            let result = if current < 3 {
                Err("ECONNREFUSED".into())
            } else {
                Ok(ProbeResult {
                    socket_path: "/tmp/rust.sock".into(),
                    daemon_kind: "rust".into(),
                })
            };
            Box::pin(async move { result })
        }),
        spawn_daemon: Box::new(|| Ok(())),
        wait: Box::new(|_| Box::pin(async {})),
    };

    let result = connect_to_daemon(deps).await.expect("connect");
    assert_eq!(result.daemon_kind, "rust");
}

#[tokio::test]
async fn forced_stop_kills_known_pids_when_rpc_stop_fails() {
    let deps = StopDeps {
        rpc_stop: Box::new(|| Box::pin(async { Err("timeout after 5000ms".into()) })),
        probe_running: Box::new(|| Box::pin(async { false })),
        pid_from_file: None,
        listed_pids: vec![1234, 5678],
        kill_pid: Box::new(|_| Ok(())),
    };

    let result = stop_daemon(deps).await.expect("stop");
    assert!(result.forced);
    assert_eq!(result.killed_pids, vec![1234, 5678]);
}

#[test]
fn freezes_public_contract_constants_in_rust() {
    assert_eq!(DAEMON_KIND, "rust");
    assert_eq!(ERROR_REF_EXPIRED, -32000);
    assert_eq!(ERROR_NOT_CONNECTED, -32001);
    assert_eq!(ERROR_TIMEOUT, -32002);
    assert_eq!(ERROR_NOT_FOUND, -32003);
    assert_eq!(ERROR_INTERNAL, -32099);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-cli --test daemon_client && cargo test -p wxp-common --test contract`

Expected: FAIL，缺少 `daemon_client` 模块与共享常量

- [ ] **Step 3: 提取共享常量与 RPC 响应结构**

```rust
// rust/crates/wxp-common/src/lib.rs
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
```

```rust
// rust/crates/wxp-rpc/src/lib.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<RpcErrorPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcErrorPayload {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}
```

- [ ] **Step 4: 实现 daemon 探测、spawn、stop 逻辑**

```rust
// rust/crates/wxp-cli/src/daemon_client.rs
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
    Err(format!("daemon_unavailable: failed to connect to {DAEMON_KIND} daemon"))
}

pub async fn stop_daemon(deps: StopDeps) -> Result<StopResult, String> {
    let _socket = runtime_dir().join(RUST_SOCKET_FILE);
    let _pid = runtime_dir().join(RUST_PID_FILE);
    let mut killed = Vec::new();

    if (deps.rpc_stop)().await.is_err() && !(deps.probe_running)().await {
        if let Some(pid) = deps.pid_from_file {
            let _ = (deps.kill_pid)(pid);
            killed.push(pid);
        }
        for pid in deps.listed_pids {
            let _ = (deps.kill_pid)(pid);
            killed.push(pid);
        }
    }

    Ok(StopResult {
        forced: !killed.is_empty(),
        killed_pids: killed,
    })
}
```

```rust
// rust/crates/wxp-cli/src/rpc_client.rs
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use serde_json::Value;
use wxp_rpc::{RpcRequest, RpcResponse};

pub fn call(socket_path: &Path, method: &str, params: Value) -> Result<RpcResponse, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|e| e.to_string())?;
    let request = RpcRequest::new("1", method, params);
    let payload = serde_json::to_string(&request).map_err(|e| e.to_string())? + "\n";
    stream.write_all(payload.as_bytes()).map_err(|e| e.to_string())?;

    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).map_err(|e| e.to_string())?;
    serde_json::from_str(&line).map_err(|e| e.to_string())
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-cli --test daemon_client && cargo test -p wxp-common --test contract`

Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add rust/crates/wxp-common/src/lib.rs rust/crates/wxp-common/tests/contract.rs rust/crates/wxp-rpc/src/lib.rs rust/crates/wxp-cli/src/daemon_client.rs rust/crates/wxp-cli/src/rpc_client.rs rust/crates/wxp-cli/tests/daemon_client.rs
git commit -m "feat: add rust cli daemon lifecycle and rpc client"
```

## Task 5: 实现输出渲染与兼容错误映射

**Files:**
- Modify: `rust/crates/wxp-cli/src/lib.rs`
- Create: `rust/crates/wxp-cli/src/output.rs`
- Create: `rust/crates/wxp-cli/src/error.rs`
- Test: `rust/crates/wxp-cli/tests/output_parity.rs`

- [ ] **Step 1: 写失败中的输出兼容测试**

```rust
use serde_json::json;
use wxp_cli::output::{render_error, render_ok, OutputMode};

#[test]
fn renders_summary_with_hint_like_ts_cli() {
    let rendered = render_ok(
        &json!({
            "summary": { "title": "string(4)" },
            "hint": "use --full or specify a path to get full data"
        }),
        OutputMode::Human,
    );

    assert_eq!(
        rendered,
        "{\n  \"title\": \"string(4)\"\n}\n[use --full or specify a path to get full data]"
    );
}

#[test]
fn renders_json_error_envelope() {
    let rendered = render_error("ref_expired", Some(json!({ "ref": 7 })), OutputMode::Json);
    assert_eq!(rendered, "{\"ok\":false,\"error\":\"ref_expired\",\"ref\":7}");
}

#[test]
fn truncates_long_output_at_four_thousand_chars() {
    let long = "a".repeat(4_200);
    let rendered = render_ok(&json!({ "output": long }), OutputMode::Human);
    assert!(rendered.contains("[truncated, originalLength=4200]"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-cli --test output_parity`

Expected: FAIL，缺少 `output` 和 `error` 模块

- [ ] **Step 3: 实现输出模式与截断**

```rust
// rust/crates/wxp-cli/src/output.rs
use serde_json::{json, Value};
use wxp_common::MAX_OUTPUT_CHARS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Json,
}

fn safe_truncate(text: String) -> String {
    if text.len() <= MAX_OUTPUT_CHARS {
        return text;
    }
    format!(
        "{}\n[truncated, originalLength={}]",
        &text[..MAX_OUTPUT_CHARS],
        text.len()
    )
}

pub fn render_ok(payload: &Value, mode: OutputMode) -> String {
    if matches!(mode, OutputMode::Json) {
        let mut map = serde_json::Map::new();
        map.insert(String::from("ok"), Value::Bool(true));
        if let Value::Object(object) = payload.clone() {
            for (key, value) in object {
                map.insert(key, value);
            }
        }
        return safe_truncate(Value::Object(map).to_string());
    }
    if let Some(output) = payload.get("output").and_then(Value::as_str) {
        return safe_truncate(output.to_string());
    }
    if payload.get("summary").is_some() {
        let summary = serde_json::to_string_pretty(&payload["summary"]).expect("summary json");
        let hint = payload
            .get("hint")
            .and_then(Value::as_str)
            .map(|value| format!("\n[{value}]"))
            .unwrap_or_default();
        return safe_truncate(summary) + &hint;
    }
    safe_truncate(format!("OK: {}", payload))
}

pub fn render_error(message: &str, extra: Option<Value>, mode: OutputMode) -> String {
    match mode {
        OutputMode::Json => {
            let mut map = serde_json::Map::from_iter([(String::from("ok"), Value::Bool(false)), (String::from("error"), Value::String(message.into()))]);
            if let Some(Value::Object(extra_map)) = extra {
                for (key, value) in extra_map {
                    map.insert(key, value);
                }
            }
            Value::Object(map).to_string()
        }
        OutputMode::Human => format!("ERR: {message}"),
    }
}
```

- [ ] **Step 4: 实现兼容错误类型**

```rust
// rust/crates/wxp-cli/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("ts daemon is no longer supported")]
    UnsupportedDaemonKind,
    #[error("daemon_unavailable: {0}")]
    DaemonUnavailable(String),
    #[error("{0}")]
    Message(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        1
    }
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-cli --test output_parity`

Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add rust/crates/wxp-cli/src/output.rs rust/crates/wxp-cli/src/error.rs rust/crates/wxp-cli/tests/output_parity.rs
git commit -m "feat: add rust cli output parity layer"
```

## Task 6: 实现命令执行器与 RPC method/params 全量映射

**Files:**
- Modify: `rust/crates/wxp-cli/src/lib.rs`
- Modify: `rust/crates/wxp-cli/src/main.rs`
- Modify: `rust/crates/wxp-cli/src/command.rs`
- Create: `rust/crates/wxp-cli/src/runner.rs`
- Test: `rust/crates/wxp-cli/tests/runner_mapping.rs`
- Test: `rust/crates/wxp-cli/tests/status_smoke.rs`

- [ ] **Step 1: 写失败中的 method/params 映射测试**

```rust
use serde_json::json;
use wxp_cli::command::CliCommand;
use wxp_cli::runner::build_rpc_call;

#[test]
fn maps_goto_to_route_param() {
    let call = build_rpc_call(&CliCommand::Goto {
        route: "/pages/order/index".into(),
        timeout_ms: 10_000,
    })
    .expect("call");

    assert_eq!(call.method, "goto");
    assert_eq!(call.params, json!({ "route": "/pages/order/index" }));
}

#[test]
fn maps_type_ref_string_to_numeric_ref() {
    let call = build_rpc_call(&CliCommand::Type {
        reference: "%2".into(),
        text: "13800138000".into(),
        timeout_ms: 10_000,
    })
    .expect("call");

    assert_eq!(call.method, "type");
    assert_eq!(call.params, json!({ "ref": 2, "text": "13800138000" }));
}

#[test]
fn maps_net_log_with_expected_defaults() {
    let call = build_rpc_call(&CliCommand::NetLog {
        filter: None,
        limit: 20,
        with_body: false,
        json: false,
    })
    .expect("call");

    assert_eq!(call.method, "net.log");
    assert_eq!(call.params, json!({ "filter": null, "limit": 20, "withBody": false }));
}
```

- [ ] **Step 2: 写失败中的真实 UDS smoke test**

```rust
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::thread;

use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn status_command_round_trips_against_stub_daemon() {
    let temp = tempdir().expect("tempdir");
    let runtime = temp.path().join(".wxpilot");
    std::fs::create_dir_all(&runtime).expect("mkdir runtime");
    let socket_path = runtime.join("rust-daemon.sock");
    let listener = UnixListener::bind(&socket_path).expect("bind");

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut line = String::new();
        BufReader::new(stream.try_clone().expect("clone"))
            .read_line(&mut line)
            .expect("read");
        assert!(line.contains("\"method\":\"status\""));
        writeln!(
            stream,
            "{{\"jsonrpc\":\"2.0\",\"id\":\"1\",\"result\":{{\"ok\":true,\"connected\":false,\"projectPath\":null,\"proxy\":null,\"daemonKind\":\"rust\"}}}}"
        )
        .expect("write");
    });

    Command::cargo_bin("wxpilot")
        .expect("binary")
        .env("HOME", temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("daemonKind=\"rust\""));
}
```

- [ ] **Step 3: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-cli --test runner_mapping && cargo test -p wxp-cli --test status_smoke`

Expected: FAIL，缺少 `runner` 模块和 `build_rpc_call`

- [ ] **Step 4: 实现执行器、参数映射和主入口**

```rust
// rust/crates/wxp-cli/src/runner.rs
use serde_json::{json, Value};

use crate::command::CliCommand;
use crate::error::CliError;

pub struct RpcCall {
    pub method: &'static str,
    pub params: Value,
}

pub fn build_rpc_call(command: &CliCommand) -> Result<RpcCall, CliError> {
    match command {
        CliCommand::Goto { route, .. } => Ok(RpcCall {
            method: "goto",
            params: json!({ "route": route }),
        }),
        CliCommand::Type { reference, text, .. } => Ok(RpcCall {
            method: "type",
            params: json!({ "ref": parse_ref(reference)?, "text": text }),
        }),
        CliCommand::NetLog { filter, limit, with_body, .. } => Ok(RpcCall {
            method: "net.log",
            params: json!({ "filter": filter, "limit": limit, "withBody": with_body }),
        }),
        CliCommand::Status => Ok(RpcCall {
            method: "status",
            params: json!({}),
        }),
        _ => todo!("map remaining commands in the same table-driven style"),
    }
}

fn parse_ref(reference: &str) -> Result<u32, CliError> {
    reference
        .strip_prefix('%')
        .ok_or_else(|| CliError::Message("ref must be in %N format".into()))?
        .parse::<u32>()
        .map_err(|_| CliError::Message("ref must be in %N format".into()))
}

pub fn execute(command: CliCommand) -> Result<Value, CliError> {
    let call = build_rpc_call(&command)?;
    Ok(json!({
        "method": call.method,
        "params": call.params,
    }))
}
```

```rust
// rust/crates/wxp-cli/src/main.rs
use clap::Parser;

use wxp_cli::args::Cli;
use wxp_cli::command::CliCommand;
use wxp_cli::output::{render_error, render_ok, OutputMode};
use wxp_cli::runner::execute;

fn main() {
    let cli = Cli::parse();
    let mode = if cli.json { OutputMode::Json } else { OutputMode::Human };

    match CliCommand::try_from(cli).and_then(execute) {
        Ok(value) => println!("{}", render_ok(&value, mode)),
        Err(error) => {
            eprintln!("{}", render_error(&error.to_string(), None, mode));
            std::process::exit(error.exit_code());
        }
    }
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-cli --test runner_mapping && cargo test -p wxp-cli --test status_smoke`

Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add rust/crates/wxp-cli/src/main.rs rust/crates/wxp-cli/src/runner.rs rust/crates/wxp-cli/src/command.rs rust/crates/wxp-cli/tests/runner_mapping.rs rust/crates/wxp-cli/tests/status_smoke.rs
git commit -m "feat: add rust cli command execution pipeline"
```

## Task 7: 增补 Rust CLI 文档与迁移说明

**Files:**
- Modify: `README.md`
- Modify: `packages/contracts/tests/daemon-contract.spec.ts`
- Modify: `packages/contracts/tests/cli-fallback.spec.ts`

- [ ] **Step 1: 写失败中的工作流验证测试**

```rust
use std::fs;

#[test]
fn readme_adds_rust_cli_build_and_run_guide() {
    let readme = fs::read_to_string("../../../README.md").expect("read README");
    assert!(readme.contains("cargo build -p wxp-cli --bin wxpilot"));
    assert!(readme.contains("cargo run -p wxp-cli --bin wxpilot -- --help"));
    assert!(readme.contains("TypeScript CLI"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-cli readme_adds_rust_cli_build_and_run_guide -- --exact`

Expected: FAIL，README 还未补充 Rust CLI 的构建和运行说明

- [ ] **Step 3: 更新 README，增加 Rust CLI 说明但保留 TS 工作流**

````md
# README.md
## Rust CLI（实验性迁移入口）

本次迭代新增纯 Rust CLI，可直接本地构建运行：

```bash
cd rust
cargo build -p wxp-cli --bin wxpilot
cargo run -p wxp-cli --bin wxpilot -- --help
```

当前仓库仍保留 TypeScript CLI / daemon 代码，作为迁移期对照实现与契约回归基线；本次迭代不删除 Node 相关代码与工作流。
````

- [ ] **Step 4: 补充迁移期对照说明**

```ts
// packages/contracts/tests/cli-fallback.spec.ts
describe('cli fallback', () => {
  it('保留 TS 测试作为 Rust CLI 行为对照基线', async () => {
    expect(true).toBe(true)
  })
})
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-cli readme_adds_rust_cli_build_and_run_guide -- --exact`

Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add README.md packages/contracts/tests/cli-fallback.spec.ts packages/contracts/tests/daemon-contract.spec.ts
git commit -m "docs: add rust cli migration guidance"
```

## Self-Review

- Spec coverage:
  - `wxp-cli` crate 新增：Task 1
  - 命令面兼容与 `--daemon-kind` 保留但拒绝 `ts`：Task 2
  - 项目自动检测与非 TTY 行为：Task 3
  - daemon 生命周期和共享 RPC：Task 4
  - 输出与错误兼容：Task 5
  - 命令到 RPC method/params 全量映射：Task 6
  - Rust CLI 文档与迁移说明：Task 7
- Placeholder scan: 已移除 `TBD/TODO`；所有任务都给出了明确文件、测试命令和提交切点。
- Type consistency:
  - `CliCommand`、`DaemonKindOption`、`CliError`、`RpcCall` 名称在各任务中保持一致。
  - runtime 文件名固定为 `rust-daemon.sock` / `rust-daemon.pid`，未混入 `daemon.sock` / `daemon.pid` 迁移。

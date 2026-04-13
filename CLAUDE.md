# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

wxpilot 是面向 AI Agent 的微信小程序自动化 CLI 工具。通过 Unix Domain Socket 与后台 Daemon 通信，Daemon 控制微信开发者工具完成页面操作、元素交互和网络代理。

## 常用命令

### 构建与开发

```bash
# Rust 主链路构建
cd rust && cargo build -p wxp-cli --bin wxpilot
cd rust && cargo build -p wxp-daemon --bin wxp-daemon

# Rust 主链路测试
make rust-test

# 查看 Rust CLI 帮助
make rust-cli-help
```

### 迁移期对照命令

```bash
# 仅在迁移期契约回归或对照排查时使用
pnpm test
pnpm exec vitest run packages/contracts/tests/cli-fallback.spec.ts packages/contracts/tests/daemon-contract.spec.ts
```

### 运行 CLI（需先构建）

```bash
# Rust CLI 为默认入口
cd rust && cargo run -p wxp-cli --bin wxpilot -- --help

# 或直接运行构建产物
rust/target/debug/wxpilot --help

# 典型使用流程
rust/target/debug/wxpilot start              # 自动检测 CWD 下 dist/project.config.json
rust/target/debug/wxpilot start /path/to/mp  # 明确指定路径
rust/target/debug/wxpilot connect            # 连接（使用 start 记录的端点）
rust/target/debug/wxpilot start /path/to/mp --proxy  # 抓包会话
```

## 架构设计

项目采用 CLI + Daemon 分离架构：

### CLI（主实现：`rust/crates/wxp-cli/`）

命令行工具，负责：
- 解析命令参数
- 自动启动 Daemon
- 通过 Unix Domain Socket 发送 JSON-RPC 请求
- 格式化输出结果

**关键文件**：
- `src/main.rs` - CLI 入口
- `src/command.rs` - 命令归一化模型
- `src/runner.rs` - daemon 探测 / 拉起 / RPC 编排
- `src/output.rs` - 输出格式化（支持 JSON 模式）

### Daemon（主实现：`rust/crates/wxp-daemon/`）

后台服务（PID 锁保证单实例），负责：
- JSON-RPC over Unix Domain Socket
- 30 分钟无操作自动退出
- 管理微信开发者工具连接
- 网络代理和 mock

**模块结构**：
- `src/main.rs` - Daemon 入口
- `src/server.rs` - JSON-RPC server / runtime path / 连接状态
- `src/read_path.rs` - `view/find/state/read/assert`
- `src/action_path.rs` - `goto/back/reload/tap/type/scroll/shot/run/wx`
- `src/net_path.rs` - `net.start/stop/log/mock/unmock/clear/install-ca`
- 共享 crate：`wxp-rpc`、`wxp-common`、`wxp-snapshot`、`wxp-ref-store`、`wxp-proxy`、`wxp-store`

### TypeScript 对照实现（`packages/`）

- `packages/cli/`、`packages/daemon/`、`packages/contracts/` 仍保留
- 仅用于迁移期契约回归、行为对照和历史实现参考
- 修改主行为时，应优先检查和更新 Rust 实现，不要默认在 TS 实现上继续扩展新能力

### 项目路径自动检测

`wxpilot start` 的 `projectPath` 参数为可选：
- 省略时扫描 CWD 下最多 2 层子目录，收集含 `project.config.json` 的 `dist` 目录
- 唯一候选：自动使用并打印路径
- 多个候选：打印列表并通过 readline 交互让用户选择
- 无候选：报错提示手动传入

### Ref 机制（核心概念）

执行 `wxpilot view` 后，页面可交互元素被分配临时编号 `%N`（从 %1 开始）。`goto`/`back`/`reload` 后编号失效，需重新执行 `view`。

实现细节：
- `rust/crates/wxp-snapshot/` - WXML → 元素树，包含交互节点检测
- `rust/crates/wxp-ref-store/` - Ref 存储和验证
- `rust/crates/wxp-daemon/src/read_path.rs` / `action_path.rs` - 方法调用时检查 ref 是否过期

**交互节点检测（双重信号）**：
微信开发者工具的 `outerWxml()` 快照**不保留** `bindtap` 等事件属性，因此无法直接通过事件属性判断交互性。当前采用双重信号：
1. `data-*` 属性：outerWxml 快照中保留，作为 bindtap 的代理信号（约 80% 覆盖）
2. 源文件指纹：读取 `{projectPath}/{pagePath}.wxml` 源文件，提取有 bindtap 节点的 `tag:sorted-classes` 指纹，补全无 data-* 的漏判节点（如仅有 `bindtap` 无 `data-*` 的按钮）

## 重要约束

1. **Rust 为主实现**：默认修改 `rust/crates/wxp-cli` 和 `rust/crates/wxp-daemon`，TS 代码仅作迁移期对照
2. **Unix Domain Socket**：Rust CLI 与 Rust daemon 通信依赖 `~/.wxpilot/rust-daemon.sock`
3. **Daemon 生命周期**：首次 CLI 调用时启动，30 分钟空闲后自动退出
4. **代理抓包主流程**：优先使用 `wxpilot start <projectPath> --proxy [--https]`，不要再把手工 `net start` 当默认主流程
5. **HTTPS 代理**：首次使用 `--https` 模式需运行 `wxpilot net install-ca` 安装 CA 证书并重启开发者工具
6. **pnpm 仍存在**：仅用于迁移期对照测试与历史 TS 工作流，不再是主运行链路

## 低 token 输出设计

各命令的默认输出均已针对 AI 上下文消耗优化：

| 命令 | 默认行为 | 完整输出 |
|------|----------|----------|
| `view` | 紧凑交互摘要（compact=true, depth=5, limit=50） | `--full` |
| `find` | 返回匹配元素最小摘要（默认仅交互节点，limit=10） | `--all` |
| `state` | 顶层 key 摘要（类型+大小） | `--full` 或指定 `path` |
| `shot` | 保存文件，返回路径 | `--base64` |
| `net log` | 摘要字段，limit=20，无 body | `--with-body` |

所有命令输出统一截断至 4000 字符，超限时附加 `[truncated, originalLength=X]` 提示。

## 测试

Rust 主链路测试：

```bash
make rust-test
cd rust && cargo test -p wxp-daemon --test read_path
cd rust && cargo test -p wxp-cli --test command_parse --test daemon_client --test status_smoke
```

迁移期对照测试：

```bash
pnpm exec vitest run packages/contracts/tests/cli-fallback.spec.ts packages/contracts/tests/daemon-contract.spec.ts
```

## 技术栈

| 模块 | 技术栈 |
|------|--------|
| CLI | Rust + clap |
| Daemon - automator | Rust + tokio-tungstenite |
| Daemon - proxy | Rust `wxp-proxy` |
| Daemon - store | Rust `wxp-store` |
| Daemon - snapshot | Rust `wxp-snapshot` |
| 构建 | Cargo |
| 主测试 | cargo test |
| 迁移期对照测试 | vitest |

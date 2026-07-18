<p align="right">
  <a href="README.md">English</a> | <strong>简体中文</strong>
</p>

<p align="center">
  <img src="docs/wxpilot.png" alt="wxpilot logo" width="250" />
</p>

<h1 align="center">wxpilot</h1>

<p align="center">
  面向 AI Agent 的微信小程序自动化 CLI<br/>
  让 Agent 像操作浏览器一样操作微信开发者工具——页面导航、元素交互、状态读取、网络抓包与 mock。
</p>

<p align="center">
  <img alt="License" src="https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg" />
  <img alt="Platform" src="https://img.shields.io/badge/platform-macOS-lightgrey.svg" />
  <img alt="Rust" src="https://img.shields.io/badge/rust-stable-orange.svg" />
  <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-green.svg" />
  <img alt="PRs Welcome" src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" />
</p>

---

## 目录

- [特性](#特性)
- [工作原理](#工作原理)
- [前置要求](#前置要求)
- [安装](#安装)
- [快速开始](#快速开始)
- [核心概念](#核心概念)
- [命令参考](#命令参考)
- [网络代理与 mock](#网络代理与-mock)
- [低 token 输出设计](#低-token-输出设计)
- [架构](#架构)
- [开发](#开发)
- [AI Agent 集成](#ai-agent-集成)
- [贡献](#贡献)
- [许可证](#许可证)

## 特性

- **为 Agent 而生**：所有命令输出默认紧凑化，统一截断至 4000 字符，最大程度降低上下文消耗
- **Ref 机制**：`view` 后页面可交互元素自动编号 `%N`，Agent 用编号即可点击/输入，无需维护选择器
- **低 token 查找**：`find <query>` 按 text/class/placeholder/tag 直接定位元素并返回 `%N`，无需通读整页
- **自动项目检测**：省略路径时自动扫描当前目录下的 `dist/project.config.json`，多候选时交互选择
- **内置代理抓包**：`--proxy` 一键启动 HTTP/HTTPS MITM 代理，支持请求 mock 与 body 查看
- **JSON 模式**：`--json` 输出结构化结果，便于程序/Agent 解析
- **Daemon 托管**：CLI 首次调用自动拉起后台 daemon，Unix Socket 通信，30 分钟空闲自动退出

## 工作原理

```
┌─────────┐   JSON-RPC    ┌──────────┐   WebSocket   ┌─────────────────────┐
│ wxpilot │ ────────────► │  daemon  │ ────────────► │  微信开发者工具      │
│  (CLI)  │ ◄──────────── │ (后台)   │ ◄──────────── │  (automator + 页面) │
└─────────┘   Unix Socket └────┬─────┘               └─────────────────────┘
                                │
                                ├── proxy      HTTP/HTTPS MITM 抓包 + mock
                                ├── snapshot   WXML → 元素树 + 交互节点检测
                                └── ref-store  %N 临时引用与过期校验
```

CLI 与 daemon 分离：CLI 仅负责参数解析与输出格式化，daemon 负责实际的自动化控制、代理与状态管理。两者通过 `~/.wxpilot/rust-daemon.sock` 通信。

## 前置要求

- **macOS**（当前仅提供 macOS 二进制；Linux/Windows 暂不支持）
- **微信开发者工具**已安装并启动
- 源码构建需 **Rust stable** 工具链（`rustup show` 查看）

## 安装

### 一键安装（macOS）

```bash
curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash
```

指定版本：

```bash
curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash -s -- --version v0.1.0
```

自定义安装目录：

```bash
curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash -s -- \
  --install-dir ~/.local/bin --daemon-dir ~/.wxpilot/bin
```

安装后：

- `wxpilot` → `~/.local/bin/wxpilot`
- `wxp-daemon` → `~/.wxpilot/bin/wxp-daemon`

若 `~/.local/bin` 不在 PATH 中，按提示追加：

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

### 源码构建

```bash
cd rust
cargo build -p wxp-cli --bin wxpilot
cargo build -p wxp-daemon --bin wxp-daemon
./target/debug/wxpilot --version
```

> `packages/web` 为官网（feai.work）源码，与 CLI 无关，源码构建无需关注。

## 快速开始

```bash
# 1. 启动自动化（自动检测 dist/project.config.json，或明确指定路径）
wxpilot start
wxpilot start /path/to/miniprogram
wxpilot start /path/to/miniprogram --proxy          # 抓包：自动拉起 8899 代理
wxpilot start /path/to/miniprogram --appid wx你的AppID

# 2. 连接
wxpilot connect

# 3. 查看页面，获取可交互元素引用 %N
wxpilot view
wxpilot find 提交            # 低 token 查找，直接返回匹配到的 %N

# 4. 交互
wxpilot tap %1
wxpilot type %2 "13800138000"
wxpilot goto /pages/order/index

# 5. 导航后引用失效，重新获取
wxpilot view

# 6. 读取状态 / 断言 / 截图
wxpilot state cart.total     # 只读需要的子字段
wxpilot assert %3 "提交成功"
wxpilot shot /tmp/result.png
```

## 核心概念

### Ref 机制

执行 `wxpilot view` 或 `wxpilot find` 后，页面可交互元素被分配临时编号 `%N`（从 %1 开始）。

- 编号在 `view` / `find` 时重新生成
- `goto` / `back` / `reload` 后编号**失效**，需重新执行 `view`
- 使用失效编号会报错：`ref_expired: %N`

**交互节点识别**（双重信号）：微信开发者工具的 `outerWxml()` 快照不保留 `bindtap` 等事件属性，因此采用：

1. `data-*` 属性——快照中保留，作为 bindtap 的代理信号（约 80% 覆盖）
2. 源文件指纹——读取 `.wxml` 源文件，按 `tag:sorted-classes` 提取有 bindtap 节点的指纹，补全无 `data-*` 的漏判节点

### 项目路径自动检测

`wxpilot start` 的 `projectPath` 参数可选：

- 省略时扫描 CWD 下最多 2 层子目录，收集含 `project.config.json` 的 `dist` 目录
- 唯一候选：自动使用
- 多个候选：交互选择
- 无候选：报错提示手动传入

## 命令参考

### 连接管理

```bash
wxpilot start                                      # 自动检测 dist/project.config.json
wxpilot start <projectPath>                        # 启动自动化（cli auto，等待就绪）
wxpilot start <projectPath> --appid <id>           # appid 为占位符时指定
wxpilot start <projectPath> --cli-path <path>      # 指定开发者工具 CLI 路径
wxpilot start <projectPath> --auto-port 9421       # 自动化端口（默认 9420）
wxpilot start <projectPath> --proxy                # 自动启动 127.0.0.1:8899 代理
wxpilot start <projectPath> --proxy --https        # HTTPS MITM 抓包（需先安装 CA）
wxpilot connect                                    # 连接（使用 start 记录的端点）
wxpilot connect --ws <wsEndpoint>                  # 直接指定 ws 端点
wxpilot disconnect
wxpilot status
```

### 导航

```bash
wxpilot goto <route>          # 如 /pages/order/index
wxpilot back
wxpilot reload
```

### 视图

```bash
wxpilot view                  # 紧凑交互摘要（compact, depth=5, limit=50）
wxpilot view --full           # 完整树（含非交互节点，depth=20）
wxpilot view --depth <n>
wxpilot view --limit <n>
wxpilot find <query>          # 查 text/class/placeholder/tag，默认只查交互节点
wxpilot find <query> --all    # 包含非交互节点
wxpilot find <query> --limit <n>
wxpilot wait %N [--timeout 5000]
```

### 交互

```bash
wxpilot tap %N
wxpilot type %N <text>
wxpilot scroll %N <up|down>
```

### 读取

```bash
wxpilot read %N               # 读取元素文本
wxpilot assert %N <expected>  # 断言文本（不匹配则退出码 1）
wxpilot state                 # 顶层 key 摘要（类型+大小）
wxpilot state [path]          # 指定子路径，如 cart.items
wxpilot state --full          # 完整页面 data
wxpilot shot [path]           # 截图，保存文件并返回路径
wxpilot shot [path] --base64  # 截图并附带 base64
```

### 网络代理

```bash
wxpilot net start [--port 8899] [--https]     # 独立启动代理（--https 可解密 HTTPS）
wxpilot net stop
wxpilot net log [--filter <url>] [--limit 20] # 摘要，不含 body
wxpilot net log [--filter <url>] --with-body  # 含请求/响应体（截断 2000 字符）
wxpilot net mock <url> <jsonFile>
wxpilot net unmock <url>
wxpilot net clear
wxpilot net install-ca                        # 安装 CA 到钥匙串（HTTPS 首次使用）
```

### 执行

```bash
wxpilot run <js>              # 在页面 VM 中执行 JS（不能访问 Node.js API）
wxpilot wx <method> [args...] # 调用 wx API
```

### Daemon

```bash
wxpilot daemon stop
wxpilot daemon restart
```

### 全局选项

```
--timeout <ms>    默认 10000ms
--verbose         详细日志
--json            JSON 格式输出
```

## 网络代理与 mock

抓包优先使用 `wxpilot start <projectPath> --proxy [--https]`，由 `start` 在同一会话内确保代理已启动。

### HTTP 模式

```bash
wxpilot start <projectPath> --proxy
wxpilot connect
wxpilot net clear
wxpilot goto /pages/xxx/index
sleep 3
wxpilot net log --filter api.example.com
```

### HTTPS MITM 模式（首次需安装证书）

```bash
# 首次配置（仅需一次）
wxpilot net start --https       # 生成 CA 证书
wxpilot net install-ca          # 安装到系统钥匙串
# 完全退出并重启微信开发者工具（必须重启）
# 开发者工具：设置 → 代理 → 手动 → 127.0.0.1:8899

# 每次抓包
wxpilot start <projectPath> --proxy --https
wxpilot connect
wxpilot net log --filter api.example.com --with-body
```

### mock

```bash
wxpilot net mock https://api.example.com/order ./mock-order.json
```

`mock-order.json` 格式：

```json
{
  "status": 200,
  "body": { "code": 0, "data": { "items": [] } }
}
```

### 注意事项

- 开发者工具设代理后，**工具自身的内部请求也走代理**；代理未运行时页面可能报 `TypeError: Failed to fetch`
- `net install-ca` 安装后必须**完全重启**开发者工具才生效
- HTTP 模式下 HTTPS 请求透明隧道放行（不记录）；要记录 HTTPS 流量必须用 `--https`
- 端口回退：默认 `9420`，若 `connect` 成功但 `status` 异常，执行 `wxpilot daemon stop` 后用 `--auto-port 9421` 重启，依次尝试 9422/9423

## 低 token 输出设计

各命令默认输出均针对 AI 上下文消耗优化：

| 命令 | 默认行为 | 完整输出 |
|------|----------|----------|
| `view` | 紧凑交互摘要（compact, depth=5, limit=50） | `--full` |
| `find` | 匹配元素最小摘要（仅交互节点，limit=10） | `--all` |
| `state` | 顶层 key 摘要（类型+大小） | `--full` 或指定 `path` |
| `shot` | 保存文件，返回路径 | `--base64` |
| `net log` | 摘要字段，limit=20，无 body | `--with-body` |

所有命令输出统一截断至 4000 字符，超限时附加 `[truncated, originalLength=X]`。

## 架构

| 模块 | 职责 |
|------|------|
| `wxp-cli` | 参数解析、daemon 探测/拉起、RPC 调用、输出格式化 |
| `wxp-daemon` | JSON-RPC server、运行时状态、automator、代理与存储 |
| `wxp-rpc` + `wxp-common` | RPC 协议与共享常量 |
| `wxp-snapshot` + `wxp-ref-store` | WXML → 元素树、交互节点检测、%N 引用存储 |
| `wxp-proxy` + `wxp-store` | HTTP/HTTPS 代理、网络日志存储 |

默认 runtime 文件：

- `~/.wxpilot/rust-daemon.sock` — daemon 通信 socket
- `~/.wxpilot/rust-daemon.pid` — daemon PID 锁（保证单实例）

## 开发

```bash
# 构建
cd rust && cargo build --workspace

# 测试
make rust-test                     # 等价于 cd rust && cargo test --workspace

# 本地打包安装闭环（macOS arm64 / x86_64）
make local-build                   # 产物在 dist/local/<target>/
make local-install                 # 安装到 ~/.local/bin 与 ~/.wxpilot/bin
make local-install-all             # 构建 + 安装一条龙

# 发布打包（生成 GitHub Releases 归档）
make public-release-package VERSION=v0.1.0
# → dist/public-release/v0.1.0/wxpilot-darwin-{arm64,x64}.tar.gz + checksums
```

将 `dist/public-release/<version>/` 下的三个文件（两个 tar.gz + `wxpilot-checksums.txt`）上传到 GitHub Releases，用户即可通过一键安装脚本获取。

## AI Agent 集成

仓库内置 [`skill/SKILL.md`](skill/SKILL.md)，是一份面向 AI Agent 的完整使用指南，包含：

- 项目路径检测的 Agent 决策逻辑
- 典型 Agent 工作流（启动 → 查找 → 交互 → 断言 → 截图）
- JSON 模式输出格式
- 端口回退排障策略
- 低 token 使用准则

接入 Agent 时可直接引用该文件作为技能说明。

## 贡献

欢迎提交 Issue 和 Pull Request。

- 主实现语言为 **Rust**，默认修改 `rust/crates/wxp-cli` 与 `rust/crates/wxp-daemon`
- 提交前请确保 `make rust-test` 通过
- 仓库未配置 CI，请本地运行测试后再提交

## 许可证

本项目基于 [GNU Affero General Public License v3.0 或更高版本](LICENSE)（AGPL-3.0-or-later）© wuliLiuyue 开源。

- ✅ 个人学习、研究、内部使用、修改、再分发均可（须保留版权声明并同样以 AGPL-3.0-or-later 开源）。
- ✅ 商业使用亦被允许，但**衍生作品与通过网络提供的服务必须以 AGPL-3.0-or-later 公开源码**（即「传染式开源」）。
- ❌ 不得将本项目或其衍生作品以闭源/专有形式分发或作为闭源服务对外提供。
- 💼 如需将本项目嵌入闭源/专有商业产品或服务，请另行联系作者获取商业授权。

相关第三方依赖（如 tokio、clap 等）仍遵循其各自的 MIT/Apache-2.0 等许可证。

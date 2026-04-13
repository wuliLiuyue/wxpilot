# wxpilot

面向 AI Agent 的微信小程序自动化 CLI 工具。通过 Unix Domain Socket 与后台 Daemon 通信，Daemon 控制微信开发者工具完成页面操作、元素交互和网络代理。

## 安装

```bash
cd rust
cargo build -p wxp-cli --bin wxpilot
cargo build -p wxp-daemon --bin wxp-daemon
./target/debug/wxpilot --help
```

当前仓库以 Rust CLI / Rust daemon 为默认实现。TypeScript CLI / daemon 代码仍保留在 `packages/`，仅作为迁移期对照实现与契约回归基线，不再作为主使用路径。

## 开发者本地打包与安装

面向仓库开发者的本地闭环，不是正式发布安装方案。

```bash
# 先构建 macOS arm64 / x86_64 本地二进制
make local-build

# 再按当前平台安装到用户目录
make local-install

# 或者一条命令串行执行构建 + 安装
make local-install-all

# 安装相关验证
make local-install-test
```

默认本地产物目录：

- `dist/local/aarch64-apple-darwin/`
- `dist/local/x86_64-apple-darwin/`

默认安装目录：

- `~/.local/bin/wxpilot`
- `~/.wxpilot/bin/wxp-daemon`

如果 `~/.local/bin` 不在 PATH 中，安装脚本会提示手动追加：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## 前置步骤

微信开发者工具需已安装并启动。

## 快速开始

```bash
# 第一步：启动自动化（自动检测 dist/project.config.json，或明确指定路径）
wxpilot start
wxpilot start /path/to/miniprogram
wxpilot start /path/to/miniprogram --proxy        # 抓包场景：启动项目前先自动拉起 8899 代理

# appid 为占位符时
wxpilot start /path/to/miniprogram --appid wx你的AppID

# 第二步：连接
wxpilot connect

# 查看页面，获取可交互元素引用 %N（默认已是紧凑摘要）
wxpilot view

# 低 token 查找元素，直接返回匹配到的 %N
wxpilot find 提交

# 点击、输入、导航
wxpilot tap %1
wxpilot type %2 "13800138000"
wxpilot goto /pages/order/index

# 导航后重新获取引用
wxpilot view

# 读取页面状态（默认只返回顶层 key 摘要）
wxpilot state
wxpilot state cart.total          # 只读需要的子字段

# 断言、截图（默认返回路径，不输出 base64）
wxpilot assert %3 "提交成功"
wxpilot shot /tmp/result.png
```

## Ref 机制

执行 `wxpilot view` 后，页面可交互元素被分配临时编号 `%N`（从 %1 开始）。`goto` / `back` / `reload` 后编号失效，需重新执行 `view`。

交互节点通过双重信号识别：`data-*` 属性（outerWxml 快照代理信号）+ 源文件 WXML 事件指纹（补全仅有 bindtap 无 data-* 的节点）。

## 命令参考

### 连接管理
```bash
wxpilot start                                      # 自动检测 dist/project.config.json
wxpilot start <projectPath>                        # 启动自动化（运行 cli auto，等待就绪）
wxpilot start <projectPath> --appid <id>           # appid 为占位符时指定
wxpilot start <projectPath> --cli-path <path>      # 指定开发者工具 CLI 路径
wxpilot start <projectPath> --auto-port 9421       # 指定自动化端口（默认 9420）
wxpilot start <projectPath> --proxy                # 抓包会话：自动先启动 127.0.0.1:8899 代理
wxpilot start <projectPath> --proxy --https        # HTTPS MITM 抓包（需先安装 CA）
wxpilot connect                                    # 连接（使用 start 记录的端点）
wxpilot connect --ws <wsEndpoint>                  # 直接指定 ws 端点（跳过 start）
wxpilot disconnect
wxpilot status
```

### 导航
```bash
wxpilot goto <route>
wxpilot back
wxpilot reload
```

### 视图
```bash
wxpilot view                       # 默认：紧凑交互摘要（compact=true, depth=5, limit=50）
wxpilot view --full                # 完整树（含非交互节点，depth=20）
wxpilot view --depth <n>           # 覆盖展开深度
wxpilot view --limit <n>           # 覆盖交互节点数量上限（默认 50）
wxpilot find <query>               # 查找 text/class/placeholder/tag，默认只查交互节点
wxpilot find <query> --all         # 包含非交互节点
wxpilot find <query> --limit <n>   # 覆盖匹配返回上限（默认 10）
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
wxpilot read %N
wxpilot assert %N <expected>
wxpilot state                    # 默认：顶层 key 摘要（类型+大小）
wxpilot state [path]             # 指定子路径，如 "cart.items"
wxpilot state --full             # 完整页面 data
wxpilot shot [path]              # 截图，保存文件并返回路径
wxpilot shot [path] --base64     # 截图并附带 base64
wxpilot run <js>                 # 返回脚本执行结果
wxpilot wx <method> [args...]    # 返回 wx 方法结果
```

### 网络代理
```bash
# 推荐：抓包时直接用 wxpilot start <projectPath> --proxy [--https]
# 若只想单独管理代理，再使用 net.start / net.stop
wxpilot net start [--port 8899] [--https]            # 独立启动代理；--https 模式可解密 HTTPS 流量
wxpilot net stop
wxpilot net log [--filter <url>] [--limit 20]         # 默认：摘要，无 body
wxpilot net log [--filter <url>] --with-body          # 含请求/响应体（截断至 2000 字符）
wxpilot net mock <url> <jsonFile>
wxpilot net unmock <url>
wxpilot net clear
wxpilot net install-ca                                # 安装 CA 证书到系统钥匙串（HTTPS 模式首次使用）
```

### 执行
```bash
wxpilot run <js>                 # 在小程序页面 VM 中执行 JS
wxpilot wx <method> [args...]    # 调用 wx API
```

### Daemon
```bash
wxpilot daemon stop
wxpilot daemon restart
```

## 全局选项

```
--timeout <ms>   默认 10000ms
--verbose        详细日志
--json           JSON 格式输出
```

## 架构说明

- **CLI**：`rust/crates/wxp-cli`，负责参数解析、daemon 探测/拉起、RPC 调用、输出格式化
- **Daemon**：`rust/crates/wxp-daemon`，负责 JSON-RPC over Unix Domain Socket、运行时状态、automator、代理与存储
- **RPC**：`rust/crates/wxp-rpc` + `rust/crates/wxp-common`
- **snapshot / ref**：`rust/crates/wxp-snapshot` + `rust/crates/wxp-ref-store`
- **proxy / store**：`rust/crates/wxp-proxy` + `rust/crates/wxp-store`

默认 runtime 文件位于：

- `~/.wxpilot/rust-daemon.sock`
- `~/.wxpilot/rust-daemon.pid`

## 代理使用注意事项

1. 开发者工具设置代理后，**工具自身的内部请求也走代理**；若代理未运行，页面可能直接报 `TypeError: Failed to fetch`
2. 抓包场景优先使用 `wxpilot start <projectPath> --proxy`，让 `start` 在同一会话里自动确保 `127.0.0.1:8899` 代理已启动
3. `start` 里仍保留了“代理错误时自动兜底重试”的兼容逻辑，但它不是主流程，不应替代显式 `--proxy`
4. `net install-ca` 安装后必须**完全重启**开发者工具才生效
5. HTTP 模式下 HTTPS 请求透明隧道放行（不记录）；要记录 HTTPS 流量必须用 `--https` 模式

## 迁移期说明

- 当前主力运行链路是 Rust CLI / Rust daemon
- `packages/cli`、`packages/daemon`、`packages/contracts` 仍保留，用于迁移期对照与契约回归
- 若只做 Rust 主链路验证，优先使用 `cargo` / `make rust-test`

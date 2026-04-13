---
name: wxpilot
description: 微信小程序自动化操作技能。当需要通过 wxpilot CLI 操作微信小程序时使用，包括连接开发者工具、页面导航、元素交互（点击/输入/滚动）、读取页面状态、网络代理与 mock。默认使用 Rust CLI / Rust daemon；运行 wxpilot start 前，优先检查当前工作目录下是否存在含 project.config.json 的 dist 子目录，有则可省略路径参数直接运行；多个候选时 CLI 会交互提示选择，避免在对话中询问用户。
---

# wxpilot — AI Agent 使用指南

wxpilot 是面向 AI Agent 的微信小程序自动化 CLI 工具，通过 Unix Domain Socket 与后台 Daemon 通信，Daemon 控制微信开发者工具（需已安装并运行）。

---

## 前置步骤

微信开发者工具需已安装并启动。

**项目路径检测（AI Agent 执行前先判断）：**
1. 用户明确提供了项目路径 → 直接使用
2. 当前工作目录下存在 `dist/project.config.json` → 省略路径参数，CLI 自动检测
3. 存在多个候选 `dist` 目录 → CLI 交互提示选择，无需在对话中询问用户
4. 均无 → 询问用户提供路径

---

## 快速开始

```bash
# 第一步：启动自动化（daemon 运行 cli auto，等待就绪）
wxpilot start                                            # 自动检测 dist/project.config.json
wxpilot start /path/to/miniprogram                      # 明确指定路径
wxpilot start /path/to/miniprogram --appid wx你的AppID   # appid 为占位符时
wxpilot start /path/to/miniprogram --proxy              # 抓包会话：先自动启动 127.0.0.1:8899 代理

# 第二步：连接（可单独重试，无需重新 start）
wxpilot connect

# 查看当前页面，获取 %N 引用
wxpilot view

# 低 token 查找元素，直接返回匹配到的 %N
wxpilot find 提交

# 交互
wxpilot tap %1
wxpilot type %2 "13800138000"

# 截图
wxpilot shot /tmp/screen.png
```

---

## Ref 机制（核心概念）

每次执行 `wxpilot view` 或 `wxpilot find` 后，页面中所有可交互元素都会被分配一个临时编号 `%N`。

- 编号在 `view` / `find` 时重新生成（从 %1 开始）
- `goto` / `back` / `reload` 后编号失效
- 使用失效的编号会报错：`ERR: %N expired, run 'wxpilot view' to refresh`
- 导航后需重新执行 `wxpilot view` 或 `wxpilot find` 以获取新编号

**交互节点识别**：基于双重信号检测——`data-*` 属性（outerWxml 快照保留，作为 bindtap 的代理）+ 源文件 WXML 的 tag+class 指纹（补全仅有 bindtap 无 data-* 的节点）。

---

## 命令参考

### 连接管理
```bash
wxpilot start                                      # 自动检测 dist/project.config.json
wxpilot start <projectPath>                        # 启动自动化（运行 cli auto，等待就绪）
wxpilot start <projectPath> --appid wx你的AppID    # appid 为占位符时指定
wxpilot start <projectPath> --cli-path <path>      # 指定开发者工具 CLI 路径
wxpilot start <projectPath> --auto-port 9421       # 指定自动化端口（默认 9420）
wxpilot start <projectPath> --proxy                # 抓包会话：自动先启动 127.0.0.1:8899 代理
wxpilot start <projectPath> --proxy --https        # HTTPS MITM 抓包（需先安装 CA）
wxpilot connect                                    # 连接（使用 start 记录的端点）
wxpilot connect --ws ws://127.0.0.1:9420           # 直接指定 ws 端点（跳过 start）
wxpilot disconnect                                 # 断开连接
wxpilot status                                     # 查看连接状态及代理状态
```

### 导航
```bash
wxpilot goto <route>             # 跳转页面，如 /pages/order/index
wxpilot back                     # 返回上一页
wxpilot reload                   # 重载当前页面
```

### 视图（必须先执行才能获取 %N）
```bash
wxpilot view                       # 默认：紧凑交互摘要（compact=true, depth=5, limit=50）
wxpilot view --full                # 完整树，含非交互节点（depth=20）
wxpilot view --depth <n>           # 覆盖展开深度
wxpilot view --limit <n>           # 覆盖交互节点数量上限（默认 50）
wxpilot find <query>               # 查找 text/class/placeholder/tag，默认只查交互节点
wxpilot find <query> --all         # 包含非交互节点
wxpilot find <query> --limit <n>   # 覆盖匹配返回上限（默认 10）
wxpilot wait %N [--timeout 5000]   # 等待元素出现
```

### 交互
```bash
wxpilot tap %N                   # 点击
wxpilot type %N <text>           # 输入文本
wxpilot scroll %N <up|down>      # 滚动
```

### 读取
```bash
wxpilot read %N                  # 读取元素文本
wxpilot assert %N <expected>     # 断言文本（不匹配则退出码 1）
wxpilot state                    # 默认：返回顶层 key 摘要（类型+大小）
wxpilot state [path]             # 返回指定子路径数据，如 "cart.items"
wxpilot state --full             # 返回完整页面 data（数据量大，按需使用）
wxpilot shot [path]              # 截图，保存到文件并返回路径
wxpilot shot [path] --base64     # 截图并附带 base64（token 消耗大，按需使用）
```

### 网络代理
```bash
# 推荐：抓包时优先使用 wxpilot start <projectPath> --proxy [--https]
# 仅在需要单独管理代理生命周期时，再直接使用 net.start / net.stop
wxpilot net start [--port 8899] [--https]            # 单独启动代理（HTTPS 模式可解密 HTTPS 流量）
wxpilot net stop                                      # 停止代理
wxpilot net log [--filter <url>] [--limit 20]         # 查看请求摘要（默认不含 body）
wxpilot net log [--filter <url>] --with-body          # 含请求/响应体（截断至 2000 字符）
wxpilot net mock <url> <jsonFile>                     # 添加 mock（jsonFile 包含响应 JSON）
wxpilot net unmock <url>                              # 移除 mock
wxpilot net clear                                     # 清空日志
wxpilot net install-ca                                # 安装 CA 证书到系统钥匙串（HTTPS 模式首次使用）
```

**mock 文件格式（jsonFile）：**
```json
{
  "status": 200,
  "body": { "code": 0, "data": { "items": [] } }
}
```

### 执行
```bash
wxpilot run <js>                 # 在页面上下文执行 JS，返回结果
wxpilot wx <method> [args...]    # 调用 wx API，args 为 JSON 字符串
```

### Daemon 管理
```bash
wxpilot daemon stop              # 停止 daemon
wxpilot daemon restart           # 重启 daemon
```

---

## 全局选项

```bash
--timeout <ms>   默认 10000ms
--verbose        打印调试信息
--json           所有输出改为 JSON 格式（适合程序解析）
```

---

## JSON 模式输出格式

成功：
```json
{"ok": true, "action": "tap", "ref": 1}
```

失败：
```json
{"ok": false, "error": "ref_expired", "ref": 3}
{"ok": false, "error": "not_connected"}
{"ok": false, "error": "timeout after 10000ms"}
```

---

## 典型 Agent 工作流

```bash
# 1. 启动并连接
wxpilot start /path/to/project
wxpilot connect

# 2. 检查当前状态（顶层 key 摘要，低 token）
wxpilot status
wxpilot state

# 3. 跳转到目标页面，优先用低 token 查找获取目标 %N
wxpilot goto /pages/order/index
wxpilot find 提交          # 优先按 text/class/placeholder/tag 查找
# 若需要完整上下文，再退回 wxpilot view

# 4. 执行操作
wxpilot type %2 "13800138000"
wxpilot tap %1

# 5. 等待页面变化后重新获取引用
wxpilot find 提交成功

# 6. 断言结果 / 读取特定字段
wxpilot assert %5 "提交成功"
wxpilot state cart.total   # 只读取需要的子字段

# 7. 截图留档（默认返回文件路径，不输出 base64）
wxpilot shot /tmp/result.png
```

**低 token 原则**：
- 优先用 `find <query>` 替代通读 `view`
- 优先用 `state <path>` 替代 `state --full`
- `shot` 默认只返回路径；仅在需要视觉分析时加 `--base64`
- `net log` 默认只返回摘要；仅排障时加 `--with-body`
- `view` 默认已紧凑；仅结构排查时加 `--full`

---

## 代理抓包工作流

### HTTP 模式（仅抓 HTTP，HTTPS 透明放行不记录）

```bash
# → 开发者工具：设置 → 代理 → 手动设置代理 → 127.0.0.1:8899
wxpilot start <projectPath> --proxy    # 推荐：由 start 统一确保代理已启动
wxpilot connect
wxpilot net clear
wxpilot goto /pages/xxx/index
sleep 3                          # 等待页面接口请求完成
wxpilot net log --filter api.example.com
```

### HTTPS MITM 模式（解密 HTTPS 流量，首次需安装证书）

```bash
# ── 首次配置（仅需一次）──
wxpilot net start --https       # 生成 CA 证书并启动 MITM 代理
wxpilot net install-ca          # 一键安装 CA 证书到系统钥匙串
# → 完全退出并重启微信开发者工具（必须重启，进程级证书缓存）
# → 开发者工具：设置 → 代理 → 手动设置代理 → 127.0.0.1:8899

# ── 每次抓包 ──
wxpilot start <projectPath> --proxy --https
wxpilot connect
wxpilot net clear
wxpilot goto /pages/xxx/index
sleep 3
wxpilot net log --filter api.example.com
```

### 关键注意事项

- 开发者工具设置代理后，**工具自身的内部请求也走代理**；若代理未运行，页面可能直接报 `TypeError: Failed to fetch`
- 抓包场景优先使用 `wxpilot start <projectPath> --proxy`，不要把“自动兜底重试”当主流程
- `net install-ca` 安装后必须**完全重启**开发者工具才生效
- HTTP 模式下 HTTPS 请求透明隧道放行（不记录）；要记录 HTTPS 流量必须用 `--https` 模式

---

## 注意事项

1. 微信开发者工具需已安装并启动，`wxpilot start [projectPath]` 会执行 `cli auto` 并等待就绪，之后再执行 `wxpilot connect`；省略路径时自动扫描 CWD 下含 `project.config.json` 的 `dist` 目录
2. Daemon 会在首次 CLI 调用时自动后台启动，30 分钟无操作自动退出
3. HTTPS 代理首次使用需运行 `wxpilot net install-ca` 安装 CA 证书并重启开发者工具
4. `wxpilot run` 中的 JS 在小程序页面 VM 中执行，不能访问 Node.js API
5. 所有命令输出统一截断至 4000 字符，超限时附加 `[truncated, originalLength=X]` 提示；需完整数据时使用对应的 `--full` 参数

**默认端口回退策略**：
- 默认先使用 `9420`
- 如果出现 `connect` 成功但 `status` 为 `connected=false`、`projectPath=null`、`view` 超时、或 `run` 报 `Connection closed`，不要继续重试当前会话
- 立即执行 `wxpilot daemon stop` 清理当前 daemon，再用新的端口重启，例如 `--auto-port 9421`
- 若 `9421` 仍异常，按序尝试 `9422`、`9423`
- 切换端口后必须显式执行 `wxpilot connect --ws ws://127.0.0.1:<新端口>`，再用 `status` 或 `run '1+1'` 做最小连通性确认
- 只有在最小连通性确认通过后，再继续 `view`、`find`、`tap` 等操作

**禁止行为**（避免 token 浪费）：
- 明明能用 `find` 定位时，先跑整页 `view`
- 不默认用 `shot` 输出 base64
- 不默认用 `state` 全量输出
- 不默认用 `net log` 附带 body
- 不连续多次执行 `view --full`

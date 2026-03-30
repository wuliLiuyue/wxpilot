# wxpilot

面向 AI Agent 的微信小程序自动化 CLI 工具。通过 Unix Domain Socket 与后台 Daemon 通信，Daemon 控制微信开发者工具完成页面操作、元素交互和网络代理。

## 安装

```bash
npm install -g wxpilot
```

## 前置步骤

微信开发者工具需已安装并启动。

## 快速开始

```bash
# 第一步：启动自动化（自动检测 dist/project.config.json，或明确指定路径）
wxpilot start
wxpilot start /path/to/miniprogram

# appid 为占位符时
wxpilot start /path/to/miniprogram --appid wx你的AppID

# 第二步：连接
wxpilot connect

# 查看页面，获取可交互元素引用 %N（默认已是紧凑摘要）
wxpilot view

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
```

### 网络代理
```bash
# 需在开发者工具手动设置代理：设置 → 代理设置 → 手动 → 127.0.0.1:8899
wxpilot net start [--port 8899] [--https]            # --https 模式可解密 HTTPS 流量
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

- **CLI**：Commander 命令行，自动后台启动 Daemon
- **Daemon**：PID 锁保证单实例，JSON-RPC over Unix Domain Socket，30 分钟无操作自动退出
- **automator**：miniprogram-automator WebSocket 连接
- **snapshot**：WXML 解析 → 元素树 → %N 引用分配
- **proxy**：mockttp HTTP/HTTPS 代理（支持 MITM 解密 HTTPS），请求日志存 SQLite，支持 mock 规则

## 代理使用注意事项

1. 开发者工具设置代理后，**工具自身的内部请求也走代理**；必须保证代理在运行，否则 `wxpilot start` 会报 `cli auto 失败 Failed to fetch`
2. 若忘记启动代理就执行了 `wxpilot start`，daemon 会自动检测并启动 HTTP 模式代理后重试
3. `net install-ca` 安装后必须**完全重启**开发者工具才生效
4. HTTP 模式下 HTTPS 请求透明隧道放行（不记录）；要记录 HTTPS 流量必须用 `--https` 模式

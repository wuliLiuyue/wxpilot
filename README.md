# wxpilot

面向 AI Agent 的微信小程序自动化 CLI 工具。通过 Unix Domain Socket 与后台 Daemon 通信，Daemon 控制微信开发者工具完成页面操作、元素交互和网络代理。

## 安装

```bash
npm install -g wxpilot
```

## 前置步骤

微信开发者工具需已安装并启动，项目已在工具中打开。

## 快速开始

```bash
# 连接（自动执行 cli auto 并连接）
wxpilot connect /path/to/miniprogram

# appid 为占位符时
wxpilot connect /path/to/miniprogram --appid wx你的AppID

# 查看页面，获取可交互元素引用 %N
wxpilot view --compact

# 点击、输入、导航
wxpilot tap %1
wxpilot type %2 "13800138000"
wxpilot goto /pages/order/index

# 导航后需重新获取引用
wxpilot view --compact

# 断言、截图
wxpilot assert %3 "提交成功"
wxpilot shot /tmp/result.png
```

## Ref 机制

执行 `wxpilot view` 后，页面可交互元素被分配临时编号 `%N`（从 %1 开始）。`goto` / `back` / `reload` 后编号失效，需重新执行 `view`。

## 命令参考

### 连接
```bash
wxpilot connect <projectPath>                     # 自动 cli auto + ws 连接
wxpilot connect <projectPath> --appid <id>        # appid 为占位符时指定
wxpilot connect <projectPath> --cli-path <path>   # 指定开发者工具 CLI 路径
wxpilot connect --ws <wsEndpoint>                 # 直接连接已知 ws 端口
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
wxpilot view [--compact] [--depth <n>]
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
wxpilot state [path]
wxpilot shot [path]
```

### 网络代理
```bash
# 需在开发者工具手动设置代理：设置 → 代理 → 手动 → 127.0.0.1:8899
wxpilot net start [--port 8899] [--https]
wxpilot net stop
wxpilot net log [--filter <url>]
wxpilot net mock <url> <jsonFile>
wxpilot net unmock <url>
wxpilot net clear
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
- **proxy**：mockttp HTTP/HTTPS 代理，请求日志存 SQLite，支持 mock 规则

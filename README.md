# wxpilot

面向 AI Agent 的微信小程序自动化 CLI 工具。通过 Unix Domain Socket 与后台 Daemon 通信，Daemon 控制微信开发者工具完成页面操作、元素交互和网络代理。

## 安装

```bash
npm install -g wxpilot
```

## 前置步骤：开启开发者工具自动化

每次启动微信开发者工具后，需先用工具自带 CLI 开启自动化端口：

```bash
# Mac - 标准版
/Applications/wechatwebdevtools.app/Contents/MacOS/cli auto \
  --project /path/to/miniprogram --port 9420

# Mac - Stable 版
"/Applications/Wechat Devtools Stable.app/Contents/MacOS/cli" auto \
  --project /path/to/miniprogram --port 9420

# 若 appid 是占位符，加 --appid
"/Applications/Wechat Devtools Stable.app/Contents/MacOS/cli" auto \
  --project /path/to/miniprogram --appid wx你的AppID --port 9420
```

看到 `✔ auto 自动化已开启` 后，再运行 wxpilot。

## 快速开始

```bash
# 连接（默认 ws://localhost:9420）
wxpilot connect

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
wxpilot connect [--ws <addr>]    # 连接，默认 ws://localhost:9420
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

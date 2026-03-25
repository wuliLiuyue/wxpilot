---
name: wxpilot
description: 微信小程序自动化操作技能。当需要通过 wxpilot CLI 操作微信小程序时使用，包括连接开发者工具、页面导航、元素交互（点击/输入/滚动）、读取页面状态、网络代理与 mock。运行 wxpilot start 前，优先检查当前工作目录下是否存在含 project.config.json 的 dist 子目录，有则可省略路径参数直接运行；多个候选时 CLI 会交互提示选择，避免在对话中询问用户。
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

# 第二步：连接（可单独重试，无需重新 start）
wxpilot connect

# 查看当前页面，获取 %N 引用
wxpilot view

# 交互
wxpilot tap %1
wxpilot type %2 "13800138000"

# 截图
wxpilot shot /tmp/screen.png
```

---

## Ref 机制（核心概念）

每次执行 `wxpilot view` 后，页面中所有可交互元素都会被分配一个临时编号 `%N`。

- 编号在 `view` 时重新生成（从 %1 开始）
- `goto` / `back` / `reload` 后编号失效
- 使用失效的编号会报错：`ERR: %3 expired, run 'wxpilot view' to refresh`
- **每次导航后必须重新执行 `wxpilot view` 以获取新编号**

---

## 命令参考

### 连接管理
```bash
wxpilot start                                      # 自动检测 dist/project.config.json
wxpilot start <projectPath>                        # 启动自动化（运行 cli auto，等待就绪）
wxpilot start <projectPath> --appid wx你的AppID    # appid 为占位符时指定
wxpilot start <projectPath> --cli-path <path>      # 指定开发者工具 CLI 路径
wxpilot start <projectPath> --auto-port 9421       # 指定自动化端口（默认 9420）
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
wxpilot view                     # 输出完整元素树
wxpilot view --compact           # 只显示可交互元素
wxpilot view --depth 3           # 限制展开深度
wxpilot wait %N [--timeout 5000] # 等待元素出现
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
wxpilot state [path]             # 读取页面 data（可选子路径如 "cart.items"）
wxpilot shot [path]              # 截图，不传路径则输出 base64
```

### 网络代理
```bash
# 用户需在微信开发者工具手动设置代理：设置 → 代理设置 → 手动 → 127.0.0.1:8899
wxpilot net start [--port 8899] [--https]   # 启动代理
wxpilot net stop                             # 停止代理
wxpilot net log [--filter <url>]             # 查看请求日志
wxpilot net mock <url> <jsonFile>            # 添加 mock（jsonFile 包含响应 JSON）
wxpilot net unmock <url>                     # 移除 mock
wxpilot net clear                            # 清空日志
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
# 1. 启动自动化（等待开发者工具就绪）
wxpilot start /path/to/project
# 2. 连接
wxpilot connect

# 2. 跳转到目标页面
wxpilot goto /pages/order/index

# 3. 查看页面，获取 ref
wxpilot view --compact

# 4. 执行操作
wxpilot type %2 "13800138000"
wxpilot tap %1

# 5. 等待页面变化后重新 view
wxpilot wait %5 --timeout 3000
wxpilot view --compact

# 6. 断言结果
wxpilot assert %5 "提交成功"

# 7. 截图留档
wxpilot shot /tmp/result.png
```

---

## 注意事项

1. 微信开发者工具需已安装并启动，`wxpilot start [projectPath]` 会执行 `cli auto` 并等待就绪，之后再执行 `wxpilot connect`；省略路径时自动扫描 CWD 下含 `project.config.json` 的 `dist` 目录
2. Daemon 会在首次 CLI 调用时自动后台启动，30 分钟无操作自动退出
3. HTTPS 代理需在开发者工具中忽略证书错误，或手动信任 CA 证书（`~/.wxpilot/ca/cert.pem`）
4. `wxpilot run` 中的 JS 在小程序页面 VM 中执行，不能访问 Node.js API

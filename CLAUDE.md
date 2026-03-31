# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

wxpilot 是面向 AI Agent 的微信小程序自动化 CLI 工具。通过 Unix Domain Socket 与后台 Daemon 通信，Daemon 控制微信开发者工具完成页面操作、元素交互和网络代理。

## 常用命令

### 构建与开发

```bash
# 安装依赖（必须使用 pnpm）
pnpm install

# 构建所有包
pnpm build

# 开发模式（所有包并行监听）
pnpm dev

# 运行测试
pnpm test

# 类型检查
pnpm lint
```

### 单包开发

```bash
# 构建 CLI
cd packages/cli && pnpm build

# 构建 Daemon
cd packages/daemon && pnpm build

# 开发模式（带监听）
cd packages/cli && pnpm dev
cd packages/daemon && pnpm dev
```

### 运行 CLI（需先构建）

```bash
# 全局安装后使用
npm install -g .
wxpilot --help

# 或直接运行构建产物
node packages/cli/dist/index.js --help

# 典型使用流程
wxpilot start              # 自动检测 CWD 下 dist/project.config.json
wxpilot start /path/to/mp  # 明确指定路径
wxpilot connect            # 连接（使用 start 记录的端点）
```

## 架构设计

项目采用 CLI + Daemon 分离架构：

### CLI (`packages/cli/`)

命令行工具，负责：
- 解析命令参数
- 自动启动 Daemon
- 通过 Unix Domain Socket 发送 JSON-RPC 请求
- 格式化输出结果

**关键文件**：
- `src/index.ts` - Commander 命令定义
- `src/client.ts` - Daemon 通信客户端（自动 spawn、重试）
- `src/output.ts` - 输出格式化（支持 JSON 模式）

### Daemon (`packages/daemon/`)

后台服务（PID 锁保证单实例），负责：
- JSON-RPC over Unix Domain Socket
- 30 分钟无操作自动退出
- 管理微信开发者工具连接
- 网络代理和 mock

**模块结构**：
- `src/index.ts` - Daemon 入口（PID 锁、信号处理）
- `src/rpc-server.ts` - JSON-RPC 服务器（方法路由、错误处理）
- `src/automator/` - 微信开发者工具 WebSocket 连接
- `src/snapshot/` - WXML 解析、元素树、Ref 分配
- `src/proxy/` - HTTP/HTTPS 代理（mockttp，支持 MITM 解密 HTTPS 流量）
- `src/store/` - SQLite 请求日志存储

### 项目路径自动检测

`wxpilot start` 的 `projectPath` 参数为可选：
- 省略时扫描 CWD 下最多 2 层子目录，收集含 `project.config.json` 的 `dist` 目录
- 唯一候选：自动使用并打印路径
- 多个候选：打印列表并通过 readline 交互让用户选择
- 无候选：报错提示手动传入

### Ref 机制（核心概念）

执行 `wxpilot view` 后，页面可交互元素被分配临时编号 `%N`（从 %1 开始）。`goto`/`back`/`reload` 后编号失效，需重新执行 `view`。

实现细节：
- `snapshot/parser.ts` - WXML → 元素树，包含交互节点检测与 `extractEventFingerprints`
- `snapshot/ref-store.ts` - Ref 存储和验证
- `rpc-server.ts` - 方法调用时检查 ref 是否过期

**交互节点检测（双重信号）**：
微信开发者工具的 `outerWxml()` 快照**不保留** `bindtap` 等事件属性，因此无法直接通过事件属性判断交互性。当前采用双重信号：
1. `data-*` 属性：outerWxml 快照中保留，作为 bindtap 的代理信号（约 80% 覆盖）
2. 源文件指纹：读取 `{projectPath}/{pagePath}.wxml` 源文件，提取有 bindtap 节点的 `tag:sorted-classes` 指纹，补全无 data-* 的漏判节点（如仅有 `bindtap` 无 `data-*` 的按钮）

## 重要约束

1. **必须使用 pnpm**：项目使用 pnpm workspace，不支持 npm/yarn
2. **better-sqlite3 预构建**：pnpm 仅构建特定依赖（见 `onlyBuiltDependencies`）
3. **Unix Domain Socket**：CLI 与 Daemon 通信依赖 `~/.wxpilot/daemon.sock`
4. **Daemon 生命周期**：首次 CLI 调用时启动，30 分钟空闲后自动退出
5. **HTTPS 代理**：首次使用 `--https` 模式需运行 `wxpilot net install-ca` 安装 CA 证书并重启开发者工具

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

测试使用 Vitest，配置见 `vitest.config.ts`：

```bash
# 运行所有测试
pnpm test

# 测试文件位置：packages/*/src/**/*.test.ts
```

已覆盖模块：
- `snapshot/parser.test.ts` - WXML 解析
- `snapshot/ref-store.test.ts` - Ref 管理
- `store/index.test.ts` - SQLite 存储

## TypeScript 配置

- `tsconfig.base.json` - 基础配置（ES2022、Node16）
- 目标环境：Node.js 18+
- 构建工具：tsup（CJS、source maps）

## 技术栈

| 模块 | 技术栈 |
|------|--------|
| CLI | Commander |
| Daemon - automator | miniprogram-automator |
| Daemon - proxy | mockttp |
| Daemon - store | better-sqlite3 |
| Daemon - snapshot | htmlparser2 |
| 构建 | tsup |
| 测试 | vitest |

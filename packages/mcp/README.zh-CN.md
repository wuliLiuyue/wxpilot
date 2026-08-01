<p align="right"><a href="README.md">English</a> | <strong>简体中文</strong></p>

# wxpilot MCP

wxpilot MCP 是一个可选的 MCP stdio 适配器，用于通过 MCP 客户端使用 wxpilot。

## 集成定位

推荐优先使用 **Skill + CLI**：

- Agent 直接引用 [`skills/wxpilot/SKILL.md`](../../skills/wxpilot/SKILL.md)
- 通过 `wxpilot` CLI 执行启动、导航、查找、交互、状态读取和网络操作
- 保持现有 CLI 工作流和低 token 输出设计

当宿主 Agent 已经通过 MCP 管理工具时，可以使用本适配器。

## 前置要求

- Node.js 18 或更高版本
- 已构建的 `wxpilot` CLI，并且位于 `PATH`，或通过 `WXPILOT_BIN` 指定路径
- 微信开发者工具已安装并运行

## MCP 客户端配置

先在仓库根目录构建 MCP 包：

```bash
pnpm install
pnpm --filter @wxpilot/mcp build
```

配置 MCP 客户端：

```json
{
  "mcpServers": {
    "wxpilot": {
      "command": "node",
      "args": [
        "/absolute/path/to/wxpilot/packages/mcp/dist/index.js"
      ],
      "env": {
        "WXPILOT_BIN": "/absolute/path/to/wxpilot/rust/target/debug/wxpilot",
        "WXPILOT_CWD": "/absolute/path/to/miniprogram"
      }
    }
  }
}
```

未设置 `WXPILOT_BIN` 时，适配器会从 `PATH` 查找 `wxpilot`。`WXPILOT_CWD` 用于设置 CLI 子进程的工作目录。

MCP 调用 `start` 时必须显式提供 `params.projectPath`，不会触发 CLI 的交互式项目选择。例如：

```json
{
  "operation": "start",
  "params": {
    "projectPath": "/absolute/path/to/miniprogram/dist"
  }
}
```

## 工具

服务只暴露一个工具：`wxpilot_execute`。

```json
{
  "operation": "find",
  "params": {
    "query": "提交",
    "limit": 10
  }
}
```

支持连接管理、页面导航、页面查看与元素交互、状态读取、截图、`run`、`wx` 以及网络代理操作。`daemon stop` 和 `daemon restart` 不通过 MCP 暴露。

结果保持为 CLI 文本返回，因此 CLI 的 JSON 输出、截断提示和错误信息都能直接传递给 MCP 客户端。调用会串行执行，以保持 `%N` 引用和页面状态的操作顺序。

## 开发

```bash
pnpm install
pnpm --filter @wxpilot/mcp typecheck
pnpm --filter @wxpilot/mcp build
pnpm --filter @wxpilot/mcp test
pnpm --filter @wxpilot/mcp dev
```

包使用 TypeScript 开发，发布时包含编译后的 `dist` 入口。`start` 直接运行编译产物，`dev` 会先编译再启动。测试会先编译到被忽略的 `.test-dist` 目录，再使用 Node 内置测试运行器执行。

stdio 模式下 stdout 只用于 MCP 协议消息，诊断信息输出到 stderr。

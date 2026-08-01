<p align="right"><strong>English</strong> | <a href="README.zh-CN.md">简体中文</a></p>

# wxpilot MCP

wxpilot MCP is an optional MCP stdio adapter for using wxpilot through MCP clients.

## Local configuration

Example MCP client configuration:

```json
{
  "mcpServers": {
    "wxpilot": {
      "command": "node",
      "args": [
        "/absolute/path/to/wxpilot/packages/mcp/dist/index.js"
      ],
      "env": {
        "WXPILOT_BIN": "/absolute/path/to/wxpilot",
        "WXPILOT_CWD": "/absolute/path/to/miniprogram"
      }
    }
  }
}
```

When `WXPILOT_BIN` is not set, the adapter looks for `wxpilot` on `PATH`. `WXPILOT_CWD` sets the CLI working directory. `start` still requires an explicit `projectPath` so MCP stdio never needs interactive project selection.

## Tool

The server exposes one tool: `wxpilot_execute`.

```json
{
  "operation": "find",
  "params": {
    "query": "submit",
    "limit": 10
  }
}
```

The supported operations cover connection management, navigation, view and element interaction, state, screenshots, `run`, `wx`, and network operations. Daemon stop and restart commands are intentionally not exposed through MCP.

Results are returned as CLI text. Calls are serialized so `%N` references and page state retain the same ordering as the CLI workflow.

## Development

```bash
pnpm install
pnpm --filter @wxpilot/mcp typecheck
pnpm --filter @wxpilot/mcp build
pnpm --filter @wxpilot/mcp test
pnpm --filter @wxpilot/mcp dev
```

The package is developed in TypeScript and publishes the compiled `dist` entrypoint. `start` runs the compiled entrypoint; `dev` compiles the TypeScript source before starting it. Tests compile into the ignored `.test-dist` directory before running with Node's built-in test runner.

The server uses stdout exclusively for MCP protocol messages. Diagnostics must go to stderr.

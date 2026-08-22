<p align="right"><strong>English</strong> | <a href="README.zh-CN.md">简体中文</a></p>

# wxpilot dsh Plugin

Native [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) (`dsh`) bundle plugin exposing wxpilot as one model-facing `wxpilot` tool. The tool spawns the wxpilot Rust CLI (`wxpilot --json …`) through the `ctx.subprocess` seam — no MCP hop, no shell layer.

## Prerequisites

- macOS with WeChat DevTools installed (the patch row disables itself on other platforms)
- The `wxpilot` binary on `PATH` (or configured via `binary` / `WXPILOT_BIN`), installed with the repository `install.sh`
- A dsh host with `ctx.tools`, `ctx.systemPrompt`, and `ctx.subprocess` services (`@deepseek-ai/dsh-base` provides them)

## Loading

Install the built package into a profile:

```sh
dsh plugin --profile demo add ./packages/dsh
```

For local development, load it as a patch overlay after building:

```sh
pnpm dsh:build
pnpm dsh web --patch ./packages/dsh/cordis.patch.yml   # from a dsh checkout
```

The `cordis.patch.yml` layer inserts one row:

```yaml
- insert:
    - id: wxpilot
      name: '@wxpilot/dsh'
      disabled: !!js process.platform !== 'darwin'
      config:
        binary: wxpilot
        timeoutMs: 120000
        maxResultChars: 16000
        enableJsEval: false
```

## Configuration

| Key | Default | Description |
| --- | --- | --- |
| `binary` | `wxpilot` | Binary name or absolute path, resolved through `ctx.subprocess.resolveExecutable`; `WXPILOT_BIN` takes precedence. |
| `timeoutMs` | `120000` | Cooperative tool-call timeout budget, enforced by `dsh-tool-call-timeout-policy`. |
| `maxResultChars` | `16000` | Maximum retained characters of one rendered result, with the CLI `[truncated, originalLength=N]` marker. |
| `enableJsEval` | `false` | Expose the `run` (JS evaluation) and `wx` (wx API call) operations. |

## Tool

`wxpilot(operation, params?)` runs one whitelisted CLI operation. The cwd defaults to the session workspace, and `start` requires `params.projectPath` (the tool layer never falls back to interactive project selection). The canonical result is `{ ok, exitCode, data, text }`; `data` carries the parsed CLI `--json` payload and `text` the model-facing rendering.

Operations are serialized through one queue: `view` assigns the `%N` refs that later operations consume, and `goto`/`back`/`reload` invalidate them. See the repository `skills/wxpilot/SKILL.md` for the full workflow guidance the system-prompt section condenses.

## Development

```sh
pnpm shared:test      # argv construction (shared with @wxpilot/mcp)
pnpm dsh:typecheck
pnpm dsh:test         # spawn/parse/queue units, no daemon or dsh runtime needed
pnpm dsh:build
```

The plugin shares its argv whitelist (`buildCliArgv`) with `@wxpilot/mcp` through `@wxpilot/shared`, so both adapters stay in lockstep with the CLI surface.

<p align="right"><a href="README.md">English</a> | <strong>简体中文</strong></p>

# wxpilot dsh 插件

面向 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（`dsh`）的原生组合包（bundle）插件，把 wxpilot 暴露为一个面向模型的 `wxpilot` 工具。工具通过 `ctx.subprocess` seam 直接拉起 wxpilot Rust CLI（`wxpilot --json …`）——不经过 MCP，也不经过 shell 层。

## 前置条件

- macOS 且已安装微信开发者工具（patch 行在其他平台自动禁用）
- `wxpilot` 二进制位于 `PATH`（或通过 `binary` / `WXPILOT_BIN` 配置），可用仓库 `install.sh` 安装
- 提供 `ctx.tools`、`ctx.systemPrompt`、`ctx.subprocess` 服务的 dsh 宿主（`@deepseek-ai/dsh-base` 即满足）

## 加载

将构建好的包安装进 profile：

```sh
dsh plugin --profile demo add ./packages/dsh
```

本地开发时，构建后以 patch overlay 加载：

```sh
pnpm dsh:build
pnpm dsh web --patch ./packages/dsh/cordis.patch.yml   # 在 dsh 源码 checkout 中执行
```

`cordis.patch.yml` 层只插入一行：

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

## 配置

| 键 | 默认值 | 说明 |
| --- | --- | --- |
| `binary` | `wxpilot` | 二进制名或绝对路径，经 `ctx.subprocess.resolveExecutable` 解析；`WXPILOT_BIN` 优先。 |
| `timeoutMs` | `120000` | 协作式工具调用超时预算，由 `dsh-tool-call-timeout-policy` 执行。 |
| `maxResultChars` | `16000` | 单次渲染结果的最大保留字符数，超限附 CLI 同款 `[truncated, originalLength=N]` 标记。 |
| `enableJsEval` | `false` | 暴露 `run`（执行 JS）与 `wx`（调用 wx API）操作。 |

## 工具

`wxpilot(operation, params?)` 执行一个白名单内的 CLI 操作。cwd 默认取会话工作区，`start` 必须传入 `params.projectPath`（工具层永不回退到交互式项目选择）。规范结果为 `{ ok, exitCode, data, text }`；`data` 是解析后的 CLI `--json` 载荷，`text` 是面向模型的渲染文本。

操作经单一队列串行执行：`view` 分配的 `%N` ref 会被后续操作消费，而 `goto`/`back`/`reload` 使其失效。完整工作流参见仓库 `skills/wxpilot/SKILL.md`（系统提示词 section 是它的浓缩）。

## 开发

```sh
pnpm shared:test      # argv 构造（与 @wxpilot/mcp 共享）
pnpm dsh:typecheck
pnpm dsh:test         # spawn/解析/队列单元测试，不需要 daemon 或 dsh 运行时
pnpm dsh:build
```

插件与 `@wxpilot/mcp` 通过 `@wxpilot/shared` 共享 argv 白名单（`buildCliArgv`），两个适配器与 CLI 命令面保持同步演进。

<p align="right">
  <strong>English</strong> | <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img src="docs/wxpilot.png" alt="wxpilot logo" width="250" />
</p>

<h1 align="center">wxpilot</h1>

<p align="center">
  A CLI for automating WeChat Mini Programs, built for AI Agents.<br/>
  Lets an Agent drive the WeChat DevTools like a browser — page navigation, element interaction, state reading, network capture & mocking.
</p>

<p align="center">
  <img alt="License" src="https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg" />
  <img alt="Platform" src="https://img.shields.io/badge/platform-macOS-lightgrey.svg" />
  <img alt="Rust" src="https://img.shields.io/badge/rust-stable-orange.svg" />
  <img alt="Version" src="https://img.shields.io/badge/version-0.1.0-green.svg" />
  <img alt="PRs Welcome" src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" />
</p>

---

## Table of Contents

- [Features](#features)
- [How It Works](#how-it-works)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Command Reference](#command-reference)
- [Network Proxy & Mocking](#network-proxy--mocking)
- [Low-Token Output Design](#low-token-output-design)
- [Architecture](#architecture)
- [Development](#development)
- [AI Agent Integration](#ai-agent-integration)
- [Contributing](#contributing)
- [License](#license)

## Features

- **Built for Agents**: All command output is compact by default and uniformly truncated to 4000 characters, minimizing context consumption.
- **Ref mechanism**: After `view`, interactive elements are auto-numbered `%N`; the Agent taps/types by number, with no need to maintain selectors.
- **Low-token lookup**: `find <query>` locates elements by text/class/placeholder/tag and returns `%N` directly — no need to read the whole page.
- **Auto project detection**: Omit the path and it auto-scans for `dist/project.config.json` under the current directory; prompts interactively when multiple candidates exist.
- **Built-in proxy capture**: `--proxy` spins up an HTTP/HTTPS MITM proxy in one command, supporting request mocking and body inspection.
- **JSON mode**: `--json` emits structured results for easy parsing by programs/Agents.
- **Managed daemon**: The CLI auto-spawns a background daemon on first call, communicating over a Unix socket, and auto-exits after 30 min idle.

## How It Works

```
┌─────────┐   JSON-RPC    ┌──────────┐   WebSocket   ┌─────────────────────┐
│ wxpilot │ ────────────► │  daemon  │ ────────────► │   WeChat DevTools   │
│  (CLI)  │ ◄──────────── │   (bg)   │ ◄──────────── │ (automator + page)  │
└─────────┘   Unix Socket └────┬─────┘               └─────────────────────┘
                                │
                                ├── proxy      HTTP/HTTPS MITM capture + mock
                                ├── snapshot   WXML → element tree + interaction detection
                                └── ref-store  %N temporary refs + expiry validation
```

The CLI and daemon are decoupled: the CLI only parses arguments and formats output, while the daemon handles the actual automation, proxy, and state. They communicate over `~/.wxpilot/rust-daemon.sock`.

## Prerequisites

- **macOS** (currently only macOS binaries are provided; Linux/Windows are not yet supported)
- **WeChat DevTools** installed and running
- Building from source requires the **Rust stable** toolchain (`rustup show`)

## Installation

### One-line install (macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash
```

Install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash -s -- --version v0.1.0
```

Custom install directories:

```bash
curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash -s -- \
  --install-dir ~/.local/bin --daemon-dir ~/.wxpilot/bin
```

After install:

- `wxpilot` → `~/.local/bin/wxpilot`
- `wxp-daemon` → `~/.wxpilot/bin/wxp-daemon`

If `~/.local/bin` is not in PATH, append it as prompted:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

### Build from source

```bash
cd rust
cargo build -p wxp-cli --bin wxpilot
cargo build -p wxp-daemon --bin wxp-daemon
./target/debug/wxpilot --version
```

> `packages/web` is the website (your-domain.com) source, unrelated to the CLI — you can ignore it when building from source.

## Quick Start

```bash
# 1. Start automation (auto-detect dist/project.config.json, or pass a path explicitly)
wxpilot start
wxpilot start /path/to/miniprogram
wxpilot start /path/to/miniprogram --proxy          # capture: auto-spawns a proxy on 8899
wxpilot start /path/to/miniprogram --appid wxYourAppID

# 2. Connect
wxpilot connect

# 3. View the page to get interactive element refs %N
wxpilot view
wxpilot find submit          # low-token lookup, returns matching %N directly

# 4. Interact
wxpilot tap %1
wxpilot type %2 "13800138000"
wxpilot goto /pages/order/index

# 5. Refs become invalid after navigation — re-fetch
wxpilot view

# 6. Read state / assert / screenshot
wxpilot state cart.total     # read only the sub-field you need
wxpilot assert %3 "Submitted successfully"
wxpilot shot /tmp/result.png
```

## Core Concepts

### Ref mechanism

After running `wxpilot view` or `wxpilot find`, interactive elements on the page are assigned temporary numbers `%N` (starting from %1).

- Numbers are regenerated on each `view` / `find`.
- After `goto` / `back` / `reload`, numbers are **invalidated** — re-run `view`.
- Using an expired ref errors with: `ref_expired: %N`.

**Interactive node detection** (dual signal): the `outerWxml()` snapshot from DevTools does not preserve event attributes like `bindtap`, so two signals are used:

1. `data-*` attributes — preserved in the snapshot, used as a proxy signal for bindtap (~80% coverage).
2. Source-file fingerprints — read the `.wxml` sources, extract `tag:sorted-classes` fingerprints of nodes that have bindtap, and recover nodes missed by the `data-*` heuristic.

### Project path auto-detection

The `projectPath` argument of `wxpilot start` is optional:

- When omitted, scans up to 2 levels of subdirectories under CWD for `dist` directories containing `project.config.json`.
- Single candidate: used automatically.
- Multiple candidates: interactive selection.
- No candidate: error, prompting you to pass it manually.

## Command Reference

### Connection management

```bash
wxpilot start                                      # auto-detect dist/project.config.json
wxpilot start <projectPath>                        # start automation (cli auto, wait for ready)
wxpilot start <projectPath> --appid <id>           # specify when appid is a placeholder
wxpilot start <projectPath> --cli-path <path>      # path to the DevTools CLI
wxpilot start <projectPath> --auto-port 9421       # automation port (default 9420)
wxpilot start <projectPath> --proxy                # auto-start a 127.0.0.1:8899 proxy
wxpilot start <projectPath> --proxy --https        # HTTPS MITM capture (install CA first)
wxpilot connect                                    # connect (using the endpoint recorded by start)
wxpilot connect --ws <wsEndpoint>                  # specify a ws endpoint directly
wxpilot disconnect
wxpilot status
```

### Navigation

```bash
wxpilot goto <route>          # e.g. /pages/order/index
wxpilot back
wxpilot reload
```

### View

```bash
wxpilot view                  # compact interactive summary (compact, depth=5, limit=50)
wxpilot view --full           # full tree (incl. non-interactive nodes, depth=20)
wxpilot view --depth <n>
wxpilot view --limit <n>
wxpilot find <query>          # match text/class/placeholder/tag, interactive nodes only by default
wxpilot find <query> --all    # include non-interactive nodes
wxpilot find <query> --limit <n>
wxpilot wait %N [--timeout 5000]
```

### Interaction

```bash
wxpilot tap %N
wxpilot type %N <text>
wxpilot scroll %N <up|down>
```

### Read

```bash
wxpilot read %N               # read element text
wxpilot assert %N <expected>  # assert text (exit code 1 on mismatch)
wxpilot state                 # top-level key summary (type + size)
wxpilot state [path]          # a specific sub-path, e.g. cart.items
wxpilot state --full          # full page data
wxpilot shot [path]           # screenshot, save file and return the path
wxpilot shot [path] --base64  # screenshot with a base64 payload
```

### Network proxy

```bash
wxpilot net start [--port 8899] [--https]     # start the proxy standalone (--https decrypts HTTPS)
wxpilot net stop
wxpilot net log [--filter <url>] [--limit 20] # summary, without bodies
wxpilot net log [--filter <url>] --with-body  # incl. request/response bodies (truncated to 2000 chars)
wxpilot net mock <url> <jsonFile>
wxpilot net unmock <url>
wxpilot net clear
wxpilot net install-ca                        # install the CA into the keychain (first HTTPS use)
```

### Execution

```bash
wxpilot run <js>              # run JS in the page VM (no Node.js API access)
wxpilot wx <method> [args...] # invoke a wx API
```

### Daemon

```bash
wxpilot daemon stop
wxpilot daemon restart
```

### Global options

```
--timeout <ms>    default 10000ms
--verbose         verbose logging
--json            JSON-formatted output
```

## Network Proxy & Mocking

For capture, prefer `wxpilot start <projectPath> --proxy [--https]`, which ensures the proxy is up within the same session.

### HTTP mode

```bash
wxpilot start <projectPath> --proxy
wxpilot connect
wxpilot net clear
wxpilot goto /pages/xxx/index
sleep 3
wxpilot net log --filter api.example.com
```

### HTTPS MITM mode (install the certificate the first time)

```bash
# One-time setup
wxpilot net start --https       # generate the CA certificate
wxpilot net install-ca          # install it into the system keychain
# Fully quit and restart WeChat DevTools (a restart is required)
# DevTools: Settings → Proxy → Manual → 127.0.0.1:8899

# Each capture session
wxpilot start <projectPath> --proxy --https
wxpilot connect
wxpilot net log --filter api.example.com --with-body
```

### Mock

```bash
wxpilot net mock https://api.example.com/order ./mock-order.json
```

`mock-order.json` format:

```json
{
  "status": 200,
  "body": { "code": 0, "data": { "items": [] } }
}
```

### Notes

- Once you set a proxy in DevTools, **the tool's own internal requests also go through the proxy**; when the proxy isn't running, the page may report `TypeError: Failed to fetch`.
- After `net install-ca`, you must **fully restart** DevTools for it to take effect.
- In HTTP mode, HTTPS requests are transparently tunneled (not recorded); to record HTTPS traffic you must use `--https`.
- Port fallback: default is `9420`. If `connect` succeeds but `status` is abnormal, run `wxpilot daemon stop`, then restart with `--auto-port 9421`, trying 9422/9423 next.

## Low-Token Output Design

The default output of each command is optimized for AI context consumption:

| Command | Default behavior | Full output |
|---------|------------------|-------------|
| `view` | Compact interactive summary (compact, depth=5, limit=50) | `--full` |
| `find` | Minimal summary of matched elements (interactive only, limit=10) | `--all` |
| `state` | Top-level key summary (type + size) | `--full` or a specific `path` |
| `shot` | Save a file, return the path | `--base64` |
| `net log` | Summary fields, limit=20, no body | `--with-body` |

All command output is uniformly truncated to 4000 characters; when truncated, a `[truncated, originalLength=X]` hint is appended.

## Architecture

| Module | Responsibility |
|--------|----------------|
| `wxp-cli` | Argument parsing, daemon detection/spawn, RPC calls, output formatting |
| `wxp-daemon` | JSON-RPC server, runtime state, automator, proxy and storage |
| `wxp-rpc` + `wxp-common` | RPC protocol and shared constants |
| `wxp-snapshot` + `wxp-ref-store` | WXML → element tree, interaction detection, %N ref storage |
| `wxp-proxy` + `wxp-store` | HTTP/HTTPS proxy, network log storage |

Default runtime files:

- `~/.wxpilot/rust-daemon.sock` — daemon communication socket
- `~/.wxpilot/rust-daemon.pid` — daemon PID lock (ensures a single instance)

## Development

```bash
# Build
cd rust && cargo build --workspace

# Test
make rust-test                     # equivalent to cd rust && cargo test --workspace

# Local build & install loop (macOS arm64 / x86_64)
make local-build                   # artifacts in dist/local/<target>/
make local-install                 # install to ~/.local/bin and ~/.wxpilot/bin
make local-install-all             # build + install in one go

# Release packaging (generates GitHub Releases archives)
make public-release-package VERSION=v0.1.0
# → dist/public-release/v0.1.0/wxpilot-darwin-{arm64,x64}.tar.gz + checksums
```

Upload the three files under `dist/public-release/<version>/` (two tar.gz archives + `wxpilot-checksums.txt`) to GitHub Releases, and users can install via the one-line script.

## AI Agent Integration

The repo ships [`skill/SKILL.md`](skill/SKILL.md), a complete usage guide for AI Agents, covering:

- Agent decision logic for project path detection
- A typical Agent workflow (start → find → interact → assert → screenshot)
- JSON-mode output format
- Port-fallback troubleshooting
- Low-token usage guidelines

Reference this file directly as a skill description when integrating with an Agent.

## Contributing

Issues and Pull Requests are welcome.

- The main implementation language is **Rust**; by default, modify `rust/crates/wxp-cli` and `rust/crates/wxp-daemon`.
- Make sure `make rust-test` passes before submitting.
- There is no CI configured — please run tests locally before committing.

## License

This project is open-sourced under the [GNU Affero General Public License v3.0 or later](LICENSE) (AGPL-3.0-or-later) © wuliLiuyue.

- ✅ Personal study, research, internal use, modification, and redistribution are all allowed (you must keep the copyright notice and likewise open-source under AGPL-3.0-or-later).
- ✅ Commercial use is also allowed, but **derivative works and services provided over a network must be open-sourced under AGPL-3.0-or-later** (i.e. "copyleft").
- ❌ You may not distribute this project or its derivatives in closed-source/proprietary form, or provide them as closed-source services.
- 💼 To embed this project into a closed-source/proprietary commercial product or service, please contact the author for a commercial license.

Third-party dependencies (such as tokio, clap, etc.) remain under their respective MIT/Apache-2.0 licenses.

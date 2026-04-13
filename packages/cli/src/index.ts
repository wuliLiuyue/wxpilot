import fs from 'fs'
import path from 'path'
import readline from 'readline'
import { Command } from 'commander'
import { call, setPreferredDaemonKind, stopDaemon, type DaemonKind } from './client.js'
import { buildNetMockParams } from './net-mock.js'
import { printOk, printErr, printRefExpired, setJsonMode, setVerboseMode } from './output.js'

const program = new Command()

type StatusResult = {
  ok: boolean
  connected: boolean
  projectPath: string | null
  proxy: { port: number } | null
  daemonKind: 'rust' | 'ts'
}

program
  .name('wxpilot')
  .description('AI Agent 微信小程序自动化 CLI')
  .version('0.1.0')
  .option('--daemon-kind <kind>', '选择 daemon 类型（rust|ts）', 'rust')
  .option('--timeout <ms>', '操作超时（毫秒）', '10000')
  .option('--verbose', '详细日志')
  .option('--json', '输出 JSON 格式')
  .hook('preAction', (thisCommand) => {
    const opts = thisCommand.opts()
    const daemonKind = opts['daemonKind']
    if (daemonKind !== 'rust' && daemonKind !== 'ts') {
      printErr(`invalid daemon kind: ${daemonKind}`)
      process.exit(1)
    }
    setPreferredDaemonKind(daemonKind as DaemonKind)
    setJsonMode(!!opts['json'])
    setVerboseMode(!!opts['verbose'])
  })

async function run(method: string, params: Record<string, unknown>, opts: { timeout?: string } = {}) {
  const timeout = parseInt(opts['timeout'] ?? program.opts()['timeout'] ?? '10000', 10)
  try {
    const result = await call(method, params, timeout)
    printOk(result as Record<string, unknown>)
  } catch (e: unknown) {
    const err = e as Error & { code?: number; data?: Record<string, unknown> }
    if (err.code === -32000 && err.data?.['ref'] != null) {
      printRefExpired(err.data['ref'] as number)
    } else {
      printErr(err.message)
    }
    process.exit(1)
  }
}

// ─── Auto-detect project path ──────────────────────────────────────────────

function findDistCandidates(dir: string, depth: number): string[] {
  if (depth === 0) return []
  const results: string[] = []
  let entries: fs.Dirent[]
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true })
  } catch {
    return []
  }
  for (const entry of entries) {
    if (!entry.isDirectory()) continue
    const full = path.join(dir, entry.name)
    if (entry.name === 'dist') {
      if (fs.existsSync(path.join(full, 'project.config.json'))) {
        results.push(full)
      }
    } else if (!entry.name.startsWith('.') && entry.name !== 'node_modules') {
      results.push(...findDistCandidates(full, depth - 1))
    }
  }
  return results
}

async function detectOrSelectProject(): Promise<string | undefined> {
  const candidates = findDistCandidates(process.cwd(), 2)
  if (candidates.length === 0) return undefined
  if (candidates.length === 1) {
    console.log(`自动检测到项目: ${candidates[0]}`)
    return candidates[0]
  }
  console.log('找到多个候选项目：')
  candidates.forEach((p, i) => console.log(`  ${i + 1}. ${p}`))
  return new Promise((resolve) => {
    const rl = readline.createInterface({ input: process.stdin, output: process.stdout })
    rl.question('请选择 (输入序号): ', (answer) => {
      rl.close()
      const idx = parseInt(answer, 10) - 1
      resolve(candidates[idx] ?? undefined)
    })
  })
}

// ─── Start ─────────────────────────────────────────────────────────────────

program
  .command('start [projectPath]')
  .description('启动微信开发者工具自动化（运行 cli auto，等待就绪）')
  .option('--cli-path <path>', '开发者工具 CLI 路径（默认自动查找）')
  .option('--appid <id>', 'AppID（project.config.json 中为占位符时必填）')
  .option('--auto-port <port>', '自动化 WebSocket 端口', '9420')
  .action(async (projectPath: string | undefined, opts: { cliPath?: string; appid?: string; autoPort?: string }) => {
    const resolved = projectPath ?? (await detectOrSelectProject())
    if (!resolved) {
      printErr('未找到项目路径，请传入 <projectPath> 或在包含 dist/project.config.json 的目录下运行')
      process.exit(1)
    }
    const port = parseInt(opts.autoPort ?? '9420', 10)
    await run('start', {
      projectPath: resolved,
      cliPath: opts.cliPath,
      appid: opts.appid,
      port,
    }, { timeout: '35000' })
  })

// ─── Connect ───────────────────────────────────────────────────────────────

program
  .command('connect')
  .description('连接微信开发者工具（使用 start 记录的端点，或 --ws 指定）')
  .option('--ws <wsEndpoint>', '直接指定 WebSocket 地址')
  .action(async (opts: { ws?: string }) => {
    // connect 含 5 次重试 × 2s，给足 10s+
    const connectTimeout = 30000
    try {
      const result = await call('connect', opts.ws ? { wsEndpoint: opts.ws } : {}, connectTimeout)
      printOk(result as Record<string, unknown>)
    } catch (e: unknown) {
      const err = e as Error & { code?: number }
      printErr(err.message)
      process.exit(1)
    }
  })

program
  .command('disconnect')
  .description('断开连接')
  .action(async () => {
    await run('disconnect', {})
  })

program
  .command('status')
  .description('查看连接状态')
  .action(async () => {
    const result = await callAndReturn('status', {})
    if (!result) return
    printOk(result as StatusResult)
  })

// ─── Navigation ────────────────────────────────────────────────────────────

program
  .command('goto <route>')
  .description('跳转页面')
  .action(async (route: string) => {
    await run('goto', { route })
  })

program
  .command('back')
  .description('返回上一页')
  .action(async () => {
    await run('back', {})
  })

program
  .command('reload')
  .description('重载当前页面')
  .action(async () => {
    await run('reload', {})
  })

// ─── View ──────────────────────────────────────────────────────────────────

program
  .command('view')
  .description('查看当前页面元素树（默认紧凑交互摘要，--full 输出完整树）')
  .option('--full', '输出完整树（含非交互节点）')
  .option('--compact', '只显示可交互元素（默认已启用）')
  .option('--depth <n>', '最大展开深度（默认 5，--full 时为 20）')
  .option('--limit <n>', '最多显示交互节点数（默认 50）')
  .action(async (opts: { full?: boolean; compact?: boolean; depth?: string; limit?: string }) => {
    await run('view', {
      full: opts.full ?? false,
      compact: opts.compact,
      depth: opts.depth ? parseInt(opts.depth, 10) : undefined,
      limit: opts.limit ? parseInt(opts.limit, 10) : undefined,
    })
  })

program
  .command('find <query>')
  .description('按 text/class/placeholder/tag 查找元素（默认只查交互节点）')
  .option('--all', '包含非交互节点')
  .option('--limit <n>', '最多返回匹配条数', '10')
  .action(async (query: string, opts: { all?: boolean; limit?: string }) => {
    await run('find', {
      query,
      all: opts.all ?? false,
      limit: opts.limit ? parseInt(opts.limit, 10) : 10,
    })
  })

program
  .command('wait <ref>')
  .description('等待元素出现（%N 或 selector）')
  .option('--timeout <ms>', '超时毫秒数', '5000')
  .action(async (ref: string, opts: { timeout?: string }) => {
    const refNum = ref.startsWith('%') ? parseInt(ref.slice(1), 10) : ref
    await run('wait', { ref: refNum, timeout: parseInt(opts.timeout ?? '5000', 10) })
  })

// ─── Interaction ───────────────────────────────────────────────────────────

program
  .command('tap <ref>')
  .description('点击元素（%N）')
  .action(async (ref: string) => {
    if (!ref.startsWith('%')) { printErr('ref must be in %N format'); process.exit(1) }
    await run('tap', { ref: parseInt(ref.slice(1), 10) })
  })

program
  .command('type <ref> <text>')
  .description('输入文本（%N）')
  .action(async (ref: string, text: string) => {
    if (!ref.startsWith('%')) { printErr('ref must be in %N format'); process.exit(1) }
    await run('type', { ref: parseInt(ref.slice(1), 10), text })
  })

program
  .command('scroll <ref> <direction>')
  .description('滚动元素（%N, up|down）')
  .action(async (ref: string, direction: string) => {
    if (!ref.startsWith('%')) { printErr('ref must be in %N format'); process.exit(1) }
    if (direction !== 'up' && direction !== 'down') { printErr('direction must be up or down'); process.exit(1) }
    await run('scroll', { ref: parseInt(ref.slice(1), 10), direction })
  })

// ─── Read ──────────────────────────────────────────────────────────────────

program
  .command('read <ref>')
  .description('读取元素文本（%N）')
  .action(async (ref: string) => {
    if (!ref.startsWith('%')) { printErr('ref must be in %N format'); process.exit(1) }
    await run('read', { ref: parseInt(ref.slice(1), 10) })
  })

program
  .command('assert <ref> <expected>')
  .description('断言元素文本（%N expected）')
  .action(async (ref: string, expected: string) => {
    if (!ref.startsWith('%')) { printErr('ref must be in %N format'); process.exit(1) }
    await run('assert', { ref: parseInt(ref.slice(1), 10), expected })
  })

program
  .command('state [path]')
  .description('读取页面 data（默认返回顶层 key 摘要，--full 返回完整数据）')
  .option('--full', '返回完整 data')
  .action(async (dataPath: string | undefined, opts: { full?: boolean }) => {
    await run('state', { path: dataPath, full: opts.full ?? false })
  })

program
  .command('shot [path]')
  .description('截图（默认保存到文件并返回路径，--base64 返回 base64）')
  .option('--base64', '返回 base64 编码')
  .action(async (savePath: string | undefined, opts: { base64?: boolean }) => {
    await run('shot', { path: savePath, base64: opts.base64 ?? false })
  })

// ─── Network ───────────────────────────────────────────────────────────────

const net = program.command('net').description('网络代理管理')

net
  .command('start')
  .description('启动代理')
  .option('--port <port>', '代理端口', '8899')
  .option('--https', '启用 HTTPS 解密')
  .action(async (opts: { port?: string; https?: boolean }) => {
    const result = await callAndReturn('net.start', {
      port: parseInt(opts.port ?? '8899', 10),
      https: opts.https ?? false,
    }, 30000)
    if (!result) return
    const r = result as Record<string, unknown>
    if (program.opts()['json']) {
      printOk(r)
    } else {
      console.log(`OK: proxy listening on 127.0.0.1:${r['port']}${r['https'] ? ' (HTTPS enabled)' : ''}`)
      console.log(`请在微信开发者工具设置代理 → 127.0.0.1:${r['port']}`)
      if (r['caPath']) {
        console.log(`CA 证书已生成: ${r['caPath']}`)
      }
    }
  })

net
  .command('stop')
  .description('停止代理')
  .action(async () => {
    await run('net.stop', {}, { timeout: '15000' })
  })

net
  .command('log')
  .description('查看请求日志（默认只返回摘要，--with-body 包含请求/响应体）')
  .option('--filter <url>', 'URL 过滤')
  .option('--limit <n>', '最多返回条数', '20')
  .option('--with-body', '包含请求/响应体（最多 2000 字符）')
  .action(async (opts: { filter?: string; limit?: string; withBody?: boolean }) => {
    const result = await callAndReturn('net.log', {
      filter: opts.filter,
      limit: opts.limit ? parseInt(opts.limit, 10) : 20,
      withBody: opts.withBody ?? false,
    })
    if (!result) return
    const r = result as {
      logs: Array<{ method: string; url: string; status: number | null; ts: number; mocked: number; duration: number | null; req_body?: string | null; res_body?: string | null }>
      truncated?: boolean; totalCount?: number; shownCount?: number
    }
    if (program.opts()['json']) {
      printOk(result as Record<string, unknown>)
    } else {
      if (r.truncated) {
        console.log(`[显示 ${r.shownCount}/${r.totalCount} 条，使用 --limit 调整]`)
      }
      for (const log of r.logs) {
        const mocked = log.mocked ? ' [MOCK]' : ''
        const dur = log.duration != null ? ` ${log.duration}ms` : ''
        console.log(`${new Date(log.ts).toISOString()} ${log.method} ${log.url} ${log.status ?? '-'}${mocked}${dur}`)
        if (log.req_body) {
          try { console.log(`  req: ${JSON.stringify(JSON.parse(log.req_body), null, 0)}`) } catch { console.log(`  req: ${log.req_body}`) }
        }
        if (log.res_body) {
          try { console.log(`  res: ${JSON.stringify(JSON.parse(log.res_body), null, 0)}`) } catch { console.log(`  res: ${log.res_body}`) }
        }
      }
    }
  })

net
  .command('mock <url> <jsonFile>')
  .description('添加 mock 规则')
  .option('--method <method>', 'HTTP 方法', 'GET')
  .action(async (url: string, jsonFile: string, opts: { method?: string }) => {
    await run('net.mock', buildNetMockParams(url, jsonFile, opts))
  })

net
  .command('unmock <url>')
  .description('移除 mock 规则')
  .action(async (url: string) => {
    await run('net.unmock', { url })
  })

net
  .command('clear')
  .description('清空请求日志')
  .action(async () => {
    await run('net.clear', {})
  })

net
  .command('install-ca')
  .description('将 HTTPS MITM CA 证书安装到系统钥匙串（需先运行 net start --https）')
  .action(async () => {
    const result = await callAndReturn('net.install-ca', {})
    if (!result) return
    const r = result as Record<string, unknown>
    if (program.opts()['json']) {
      printOk(r)
    } else {
      console.log(`OK: CA 证书已安装到系统钥匙串`)
      console.log(`cert: ${r['certFile']}`)
      console.log(`请完全重启微信开发者工具后生效`)
    }
  })

// ─── Daemon ────────────────────────────────────────────────────────────────

const daemon = program.command('daemon').description('Daemon 管理')

daemon
  .command('stop')
  .description('停止 daemon')
  .action(async () => {
    const daemonKind = program.opts()['daemonKind'] as DaemonKind
    try {
      const result = await stopDaemon(daemonKind)
      printOk({ ok: true, daemonKind, ...result })
    } catch (e: unknown) {
      const err = e as Error
      printErr(err.message)
      process.exit(1)
    }
  })

daemon
  .command('restart')
  .description('重启 daemon')
  .action(async () => {
    const daemonKind = program.opts()['daemonKind'] as DaemonKind
    try { await stopDaemon(daemonKind) } catch { /* ignore */ }
    // wait for daemon to stop, then reconnect (auto-spawn)
    await new Promise((r) => setTimeout(r, 1500))
    await run('status', {})
  })

// ─── Execute ───────────────────────────────────────────────────────────────

program
  .command('run <js>')
  .description('执行 JS 代码')
  .action(async (js: string) => {
    await run('run', { js })
  })

program
  .command('wx <method> [args...]')
  .description('调用 wx 方法')
  .action(async (method: string, args: string[]) => {
    const parsedArgs = args.map((a) => { try { return JSON.parse(a) } catch { return a } })
    await run('wx', { method, args: parsedArgs })
  })

// ─── Helpers ───────────────────────────────────────────────────────────────

async function callAndReturn(method: string, params: Record<string, unknown>, timeoutMs?: number): Promise<unknown | null> {
  const timeout = timeoutMs ?? parseInt(program.opts()['timeout'] ?? '10000', 10)
  try {
    return await call(method, params, timeout)
  } catch (e: unknown) {
    const err = e as Error & { code?: number; data?: Record<string, unknown> }
    if (err.code === -32000 && err.data?.['ref'] != null) {
      printRefExpired(err.data['ref'] as number)
    } else {
      printErr(err.message)
    }
    process.exit(1)
  }
}

program.parseAsync(process.argv).catch((e) => {
  printErr(String(e))
  process.exit(1)
})

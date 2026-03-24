import { Command } from 'commander'
import { call } from './client.js'
import { printOk, printErr, printRefExpired, setJsonMode, setVerboseMode } from './output.js'

const program = new Command()

program
  .name('wxpilot')
  .description('AI Agent 微信小程序自动化 CLI')
  .version('0.1.0')
  .option('--timeout <ms>', '操作超时（毫秒）', '10000')
  .option('--verbose', '详细日志')
  .option('--json', '输出 JSON 格式')
  .hook('preAction', (thisCommand) => {
    const opts = thisCommand.opts()
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

// ─── Connect ───────────────────────────────────────────────────────────────

program
  .command('connect')
  .description('连接微信开发者工具（需先通过 cli auto 开启自动化）')
  .option('--ws <wsEndpoint>', 'WebSocket 地址', 'ws://localhost:9420')
  .action(async (opts: { ws: string }) => {
    const timeout = parseInt(program.opts()['timeout'] ?? '10000', 10)
    try {
      const result = await call('connect', { wsEndpoint: opts.ws }, timeout)
      printOk(result as Record<string, unknown>)
    } catch (e: unknown) {
      const err = e as Error & { code?: number }
      printErr(err.message)
      if (!program.opts()['json']) {
        console.error('\n请先在微信开发者工具中执行：')
        console.error(`  cli auto --project <projectPath> --port 9420`)
        console.error('\n然后重新运行 wxpilot connect')
      }
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
    await run('status', {})
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
  .description('查看当前页面元素树')
  .option('--compact', '只显示可交互元素')
  .option('--depth <n>', '最大展开深度')
  .action(async (opts: { compact?: boolean; depth?: string }) => {
    await run('view', {
      compact: opts.compact ?? false,
      depth: opts.depth ? parseInt(opts.depth, 10) : undefined,
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
  .description('读取页面 data')
  .action(async (dataPath?: string) => {
    await run('state', { path: dataPath })
  })

program
  .command('shot [path]')
  .description('截图')
  .action(async (savePath?: string) => {
    await run('shot', { path: savePath })
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
    })
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
    await run('net.stop', {})
  })

net
  .command('log')
  .description('查看请求日志')
  .option('--filter <url>', 'URL 过滤')
  .action(async (opts: { filter?: string }) => {
    const result = await callAndReturn('net.log', { filter: opts.filter })
    if (!result) return
    const r = result as { logs: Array<{ method: string; url: string; status: number; ts: number; mocked: number }> }
    if (program.opts()['json']) {
      printOk(result as Record<string, unknown>)
    } else {
      for (const log of r.logs) {
        const mocked = log.mocked ? ' [MOCK]' : ''
        console.log(`${new Date(log.ts).toISOString()} ${log.method} ${log.url} ${log.status ?? '-'}${mocked}`)
      }
    }
  })

net
  .command('mock <url> <jsonFile>')
  .description('添加 mock 规则')
  .action(async (url: string, jsonFile: string) => {
    await run('net.mock', { url, jsonFile })
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

// ─── Daemon ────────────────────────────────────────────────────────────────

const daemon = program.command('daemon').description('Daemon 管理')

daemon
  .command('stop')
  .description('停止 daemon')
  .action(async () => {
    await run('daemon.stop', {})
  })

daemon
  .command('restart')
  .description('重启 daemon')
  .action(async () => {
    try {
      await call('daemon.stop', {}, 5000)
    } catch { /* ignore */ }
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

async function callAndReturn(method: string, params: Record<string, unknown>): Promise<unknown | null> {
  const timeout = parseInt(program.opts()['timeout'] ?? '10000', 10)
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

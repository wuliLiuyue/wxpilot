import net from 'net'

import fs from 'fs'
import path from 'path'
import { spawn, exec } from 'child_process'
import { promisify } from 'util'
import { SOCK_FILE, IDLE_TIMEOUT_MS } from './constants.js'
import * as auto from './automator/index.js'
import { parseWxml, assignRefs, renderTree } from './snapshot/parser.js'
import { RefStore } from './snapshot/ref-store.js'
import * as proxy from './proxy/index.js'
import { CA_DIR } from './constants.js'

const execAsync = promisify(exec)
const refStore = new RefStore()
let pendingWsEndpoint: string | null = null
let server: net.Server | null = null
let activeConnections = 0
let lastActivityTs = Date.now()
let idleTimer: NodeJS.Timeout | null = null

export function getLastActivity(): number {
  return lastActivityTs
}

function touch() {
  lastActivityTs = Date.now()
}

// JSON-RPC error codes
const ERR = {
  REF_EXPIRED: -32000,
  NOT_CONNECTED: -32001,
  TIMEOUT: -32002,
  NOT_FOUND: -32003,
  INTERNAL: -32099,
}

interface RpcRequest {
  jsonrpc: '2.0'
  id: number | string
  method: string
  params?: Record<string, unknown>
}

interface RpcResponse {
  jsonrpc: '2.0'
  id: number | string
  result?: unknown
  error?: { code: number; message: string; data?: unknown }
}

function ok(id: number | string, result: unknown): RpcResponse {
  return { jsonrpc: '2.0', id, result }
}

function err(id: number | string, code: number, message: string, data?: unknown): RpcResponse {
  return { jsonrpc: '2.0', id, error: { code, message, data } }
}

function resolveRef(ref: number): string {
  const sel = refStore.resolve(ref)
  if (!sel) throw Object.assign(new Error('ref_expired'), { code: ERR.REF_EXPIRED, ref })
  return sel
}

const CLI_CANDIDATES = [
  '/Applications/wechatwebdevtools.app/Contents/MacOS/cli',
  '/Applications/Wechat Devtools Stable.app/Contents/MacOS/cli',
]

function findDevToolsCli(cliPath?: string): string {
  if (cliPath) return cliPath
  for (const p of CLI_CANDIDATES) {
    if (fs.existsSync(p)) return p
  }
  throw new Error('未找到微信开发者工具 CLI，请用 --cli-path 指定路径')
}

function runCliAuto(projectPath: string, port: number, cliPath?: string, appid?: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const cli = findDevToolsCli(cliPath)
    const args = ['auto', '--project', projectPath, '--auto-port', String(port)]
    if (appid) args.push('--appid', appid)

    const child = spawn(cli, args, { stdio: ['ignore', 'pipe', 'pipe'] })
    let output = ''
    let resolved = false

    const timer = setTimeout(() => {
      child.kill()
      reject(new Error('cli auto 超时（30s），请确认开发者工具已启动并打开目标项目'))
    }, 30000)

    const onData = (data: Buffer) => {
      output += data.toString()
      if (!resolved && (output.includes('✔ auto') || output.includes('自动化已开启') || output.includes('auto success'))) {
        resolved = true
        clearTimeout(timer)
        resolve(`ws://127.0.0.1:${port}`)
      }
    }

    child.stdout.on('data', onData)
    child.stderr.on('data', onData)
    child.on('error', (e) => { clearTimeout(timer); reject(e) })
    child.on('exit', (code) => {
      if (resolved) return
      clearTimeout(timer)
      const success = output.includes('✔ auto') || output.includes('自动化已开启') || output.includes('auto success')
      if ((code === 0 || code === null) && success) {
        resolve(`ws://127.0.0.1:${port}`)
      } else {
        const isProxyError = output.includes('8899') || output.includes('tunneling socket') || output.includes('TLS connection') || output.includes('ECONNRESET') || output.includes('Failed to fetch')
        const msg = (output.match(/\[error\]\s*({[\s\S]*?}|\S+.*)/)?.[1] ?? output.trim()) || `cli auto 退出码 ${code}`
        reject(Object.assign(new Error(`cli auto 失败：${msg}`), { isProxyError }))
      }
    })
  })
}

async function handle(req: RpcRequest): Promise<RpcResponse> {
  const { id, method, params = {} } = req
  touch()

  try {
    switch (method) {
      case 'start': {
        const projectPath = params['projectPath'] as string
        const cliPath = params['cliPath'] as string | undefined
        const appid = params['appid'] as string | undefined
        const port = (params['port'] as number | undefined) ?? 9420
        let wsEndpoint: string
        try {
          wsEndpoint = await runCliAuto(projectPath, port, cliPath, appid)
        } catch (e: unknown) {
          const error = e as Error & { isProxyError?: boolean }
          if (error.isProxyError && !proxy.isRunning()) {
            await proxy.startProxy(8899, false)
            wsEndpoint = await runCliAuto(projectPath, port, cliPath, appid)
          } else {
            throw e
          }
        }
        pendingWsEndpoint = wsEndpoint
        auto.setProjectPath(projectPath)
        return ok(id, { ok: true, wsEndpoint, port })
      }

      case 'connect': {
        const wsEndpoint = (params['wsEndpoint'] as string | undefined) ?? pendingWsEndpoint ?? 'ws://127.0.0.1:9420'
        let lastErr: Error | undefined
        for (let i = 0; i < 5; i++) {
          if (i > 0) await sleep(2000)
          try {
            await auto.connectWs(wsEndpoint)
            refStore.invalidateAll()
            return ok(id, { ok: true, wsEndpoint })
          } catch (e) {
            lastErr = e as Error
          }
        }
        return err(id, ERR.INTERNAL, lastErr?.message ?? 'connect failed')
      }

      case 'disconnect': {
        await auto.disconnect()
        refStore.invalidateAll()
        return ok(id, { ok: true })
      }

      case 'status': {
        return ok(id, {
          ok: true,
          connected: auto.isConnected(),
          projectPath: auto.getConnectedPath(),
          proxy: proxy.isRunning() ? { port: proxy.getPort() } : null,
        })
      }

      case 'goto': {
        const route = params['route'] as string
        await auto.goto(route)
        refStore.invalidateAll()
        return ok(id, { ok: true, route })
      }

      case 'back': {
        await auto.back()
        refStore.invalidateAll()
        return ok(id, { ok: true })
      }

      case 'reload': {
        await auto.reload()
        refStore.invalidateAll()
        return ok(id, { ok: true })
      }

      case 'view': {
        const compact = params['compact'] as boolean | undefined
        const depth = params['depth'] as number | undefined
        const wxml = await auto.getWxml()
        const pagePath = await auto.getPagePath()
        const tree = parseWxml(wxml, depth ?? 20)
        const refs = assignRefs(tree)
        refStore.assign(refs)
        const rendered = renderTree(tree, { compact, depth })
        const header = `页面: ${pagePath}\n${'─'.repeat(34)}`
        return ok(id, { ok: true, pagePath, output: `${header}\n${rendered}` })
      }

      case 'tap': {
        const ref = params['ref'] as number
        const sel = resolveRef(ref)
        await auto.tapElement(sel)
        return ok(id, { ok: true, action: 'tap', ref })
      }

      case 'type': {
        const ref = params['ref'] as number
        const text = params['text'] as string
        const sel = resolveRef(ref)
        await auto.typeElement(sel, text)
        return ok(id, { ok: true, action: 'type', ref, text })
      }

      case 'scroll': {
        const ref = params['ref'] as number
        const direction = params['direction'] as 'up' | 'down'
        const sel = resolveRef(ref)
        await auto.scrollElement(sel, direction)
        return ok(id, { ok: true, action: 'scroll', ref, direction })
      }

      case 'read': {
        const ref = params['ref'] as number
        const sel = resolveRef(ref)
        const text = await auto.getElementText(sel)
        return ok(id, { ok: true, ref, text })
      }

      case 'assert': {
        const ref = params['ref'] as number
        const expected = params['expected'] as string
        const sel = resolveRef(ref)
        const actual = await auto.getElementText(sel)
        const pass = actual === expected
        if (!pass) {
          return err(id, ERR.INTERNAL, 'assertion_failed', { ref, expected, actual })
        }
        return ok(id, { ok: true, ref, expected, actual })
      }

      case 'wait': {
        const refOrSel = params['ref'] as number | string
        const timeout = (params['timeout'] as number | undefined) ?? 10000
        const interval = 200
        const deadline = Date.now() + timeout
        while (Date.now() < deadline) {
          if (typeof refOrSel === 'number') {
            if (refStore.has(refOrSel)) {
              return ok(id, { ok: true, ref: refOrSel })
            }
          } else {
            // try re-render snapshot and check
            try {
              const wxml = await auto.getWxml()
              const tree = parseWxml(wxml)
              const refs = assignRefs(tree)
              refStore.assign(refs)
              return ok(id, { ok: true })
            } catch {
              // continue waiting
            }
          }
          await sleep(interval)
        }
        return err(id, ERR.TIMEOUT, 'timeout', { timeout })
      }

      case 'state': {
        const dataPath = params['path'] as string | undefined
        const data = await auto.getPageData(dataPath)
        return ok(id, { ok: true, data })
      }

      case 'shot': {
        const savePath = params['path'] as string | undefined
        const result = await auto.screenshot(savePath)
        return ok(id, { ok: true, path: result.path, base64: result.base64 })
      }

      case 'run': {
        const js = params['js'] as string
        const result = await auto.runScript(js)
        return ok(id, { ok: true, result })
      }

      case 'wx': {
        const wxMethod = params['method'] as string
        const args = (params['args'] as unknown[]) ?? []
        const result = await auto.callWxMethod(wxMethod, args)
        return ok(id, { ok: true, result })
      }

      case 'net.start': {
        const port = (params['port'] as number | undefined) ?? 8899
        const httpsMode = (params['https'] as boolean | undefined) ?? false
        await proxy.startProxy(port, httpsMode)
        const caPath = httpsMode ? `${process.env['HOME']}/.wxpilot/ca/cert.pem` : undefined
        return ok(id, { ok: true, port, https: httpsMode, caPath })
      }

      case 'net.stop': {
        await proxy.stopProxy()
        return ok(id, { ok: true })
      }

      case 'net.log': {
        const filter = params['filter'] as string | undefined
        const logs = proxy.getNetLogs(filter).map((log) => ({
          ...log,
          req_body: log.req_body ? log.req_body.toString('utf8') : null,
          res_body: log.res_body ? log.res_body.toString('utf8') : null,
        }))
        return ok(id, { ok: true, logs })
      }

      case 'net.mock': {
        const url = params['url'] as string
        const jsonFile = params['jsonFile'] as string
        await proxy.addMock(url, jsonFile)
        return ok(id, { ok: true, url })
      }

      case 'net.unmock': {
        const url = params['url'] as string
        proxy.removeMock(url)
        return ok(id, { ok: true, url })
      }

      case 'net.clear': {
        proxy.clearNetLogs()
        return ok(id, { ok: true })
      }

      case 'net.install-ca': {
        const certFile = path.join(CA_DIR, 'cert.pem')
        if (!fs.existsSync(certFile)) {
          return err(id, ERR.NOT_FOUND, 'CA 证书不存在，请先运行 net start --https 生成证书')
        }
        const keychain = `${process.env['HOME']}/Library/Keychains/login.keychain-db`
        await execAsync(`security add-trusted-cert -d -r trustRoot -k "${keychain}" "${certFile}"`)
        return ok(id, { ok: true, certFile })
      }

      case 'daemon.stop': {
        setTimeout(() => process.exit(0), 100)
        return ok(id, { ok: true })
      }

      default:
        return err(id, -32601, `method_not_found: ${method}`)
    }
  } catch (e: unknown) {
    const error = e as Error & { code?: number; ref?: number }
    if (error.message === 'not_connected') {
      return err(id, ERR.NOT_CONNECTED, 'not_connected')
    }
    if (error.message === 'ref_expired') {
      return err(id, ERR.REF_EXPIRED, 'ref_expired', { ref: error.ref })
    }
    if (error.message === 'element_not_found') {
      return err(id, ERR.NOT_FOUND, 'element_not_found')
    }
    return err(id, ERR.INTERNAL, error.message ?? 'internal_error')
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

export function startServer(): void {
  fs.mkdirSync(path.dirname(SOCK_FILE), { recursive: true })

  server = net.createServer((socket) => {
    activeConnections++
    touch()

    let buf = ''

    socket.on('data', (chunk) => {
      buf += chunk.toString()
      const lines = buf.split('\n')
      buf = lines.pop() ?? ''
      for (const line of lines) {
        const trimmed = line.trim()
        if (!trimmed) continue
        let req: RpcRequest
        try {
          req = JSON.parse(trimmed)
        } catch {
          socket.write(JSON.stringify({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'parse_error' } }) + '\n')
          continue
        }
        handle(req).then((res) => {
          if (!socket.destroyed) socket.write(JSON.stringify(res) + '\n')
        }).catch((e) => {
          if (!socket.destroyed) socket.write(JSON.stringify(err(req.id, ERR.INTERNAL, String(e))) + '\n')
        })
      }
    })

    socket.on('close', () => {
      activeConnections--
      touch()
    })

    socket.on('error', () => {
      activeConnections--
    })
  })

  server.listen(SOCK_FILE)

  // idle timer
  idleTimer = setInterval(() => {
    if (activeConnections === 0 && Date.now() - lastActivityTs > IDLE_TIMEOUT_MS) {
      process.exit(0)
    }
  }, 60_000)

  if (idleTimer.unref) idleTimer.unref()
}

export function stopServer(): void {
  if (server) {
    server.close()
    server = null
  }
  if (idleTimer) {
    clearInterval(idleTimer)
    idleTimer = null
  }
}

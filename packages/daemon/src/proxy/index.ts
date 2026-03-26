import * as mockttp from 'mockttp'
import type { CompletedRequest } from 'mockttp'
import type { PassThroughResponse } from 'mockttp/dist/rules/requests/request-handler-definitions'
import http from 'http'
import net from 'net'
import fs from 'fs'
import path from 'path'
import { CA_DIR } from '../constants.js'
import { insertRequest, upsertMock, deleteMock, listMocks, clearRequests, queryRequests, type RequestLog } from '../store/index.js'

let proxyServer: mockttp.Mockttp | null = null
let outerServer: http.Server | null = null
let proxyPort = 8899

export function isRunning(): boolean {
  return proxyServer !== null
}

export function getPort(): number {
  return proxyPort
}

async function setupMocktpRules(): Promise<void> {
  if (!proxyServer) return

  const mocks = listMocks()
  for (const mock of mocks) {
    await applyMockRule(mock.url, mock.response)
  }

  await proxyServer.forAnyRequest().thenPassThrough({
    beforeRequest: async (req: CompletedRequest) => {
      ;(req as unknown as Record<string, unknown>)['_wxpilot_start'] = Date.now()
      return undefined
    },
    beforeResponse: async (res: PassThroughResponse, req: CompletedRequest) => {
      const start = (req as unknown as Record<string, unknown>)['_wxpilot_start'] as number ?? Date.now()
      const duration = Date.now() - start
      const reqBody = await req.body.getText()
      const resBody = await res.body.getText()
      insertRequest({
        ts: Date.now(),
        method: req.method,
        url: req.url,
        req_headers: JSON.stringify(req.headers),
        req_body: reqBody ? Buffer.from(reqBody) : null,
        status: res.statusCode,
        res_headers: JSON.stringify(res.headers),
        res_body: resBody ? Buffer.from(resBody) : null,
        mocked: 0,
        duration,
      })
      return undefined
    },
  })
}

export async function startProxy(port: number, https: boolean): Promise<void> {
  if (proxyServer) await stopProxy()

  proxyPort = port

  const certFile = path.join(CA_DIR, 'cert.pem')
  const keyFile = path.join(CA_DIR, 'key.pem')

  if (https) {
    // HTTPS MITM 模式：mockttp 直接监听外部端口，支持 TLS 解密和 CONNECT 拦截
    fs.mkdirSync(CA_DIR, { recursive: true })

    if (!fs.existsSync(certFile) || !fs.existsSync(keyFile)) {
      const ca = await mockttp.generateCACertificate({ commonName: 'wxpilot CA' })
      fs.writeFileSync(certFile, ca.cert)
      fs.writeFileSync(keyFile, ca.key)
    }

    proxyServer = mockttp.getLocal({
      https: { keyPath: keyFile, certPath: certFile },
    })
    await setupMocktpRules()
    await proxyServer.start(port)
  } else {
    // HTTP 模式：两层代理
    // 内层：mockttp 监听随机端口，负责 HTTP 流量的 passthrough、logging、mock rule
    proxyServer = mockttp.getLocal({})
    await setupMocktpRules()
    await proxyServer.start(0)
    const internalPort = proxyServer.port

    // 外层：纯 Node.js HTTP 服务器监听外部端口
    // - CONNECT 请求 → 原始 TCP 隧道（透明 HTTPS 转发）
    // - 普通 HTTP 请求 → 转发到内层 mockttp
    outerServer = http.createServer((req, res) => {
      const fwd = http.request(
        { host: '127.0.0.1', port: internalPort, path: req.url, method: req.method, headers: req.headers },
        (fwdRes) => {
          // 去除 transfer-encoding：Node.js 会将 chunked 响应解码后 pipe，
          // 若原样透传 transfer-encoding: chunked 头，客户端会按 chunked 格式解析
          // unchunked 数据，导致解析失败（Failed to fetch）
          const headers: http.IncomingHttpHeaders = {}
          for (const [k, v] of Object.entries(fwdRes.headers)) {
            if (k.toLowerCase() !== 'transfer-encoding') headers[k] = v
          }
          res.writeHead(fwdRes.statusCode!, headers)
          fwdRes.pipe(res, { end: true })
        },
      )
      req.pipe(fwd, { end: true })
      fwd.on('error', () => {
        if (!res.headersSent) res.writeHead(502)
        res.end()
      })
    })

    outerServer.on('connect', (req, clientSocket, head) => {
      const parts = (req.url ?? '').split(':')
      const hostname = parts[0]
      const targetPort = parseInt(parts[1] ?? '443') || 443

      const serverSocket = net.connect(targetPort, hostname, () => {
        clientSocket.write('HTTP/1.1 200 Connection Established\r\n\r\n')
        if (head.length > 0) serverSocket.write(head)
        serverSocket.pipe(clientSocket)
        clientSocket.pipe(serverSocket)
      })

      serverSocket.on('error', () => clientSocket.destroy())
      clientSocket.on('error', () => serverSocket.destroy())
    })

    await new Promise<void>((resolve) => outerServer!.listen(port, resolve))
  }
}

async function applyMockRule(url: string, responseJson: string): Promise<void> {
  if (!proxyServer) return
  const response = JSON.parse(responseJson) as { status?: number; body?: unknown }
  await proxyServer.forGet(url).thenJson(response.status ?? 200, (response.body ?? response) as object)
}

export async function stopProxy(): Promise<void> {
  if (outerServer) {
    // 先销毁所有已建立连接（含 keep-alive），再等待 close 回调
    outerServer.closeAllConnections?.()
    await new Promise<void>((resolve) => outerServer!.close(() => resolve()))
    outerServer = null
  }
  if (proxyServer) {
    await proxyServer.stop()
    proxyServer = null
  }
}

export async function addMock(url: string, jsonFile: string): Promise<void> {
  const content = fs.readFileSync(jsonFile, 'utf8')
  upsertMock(url, content)
  if (proxyServer) {
    await applyMockRule(url, content)
  }
}

export function removeMock(url: string): void {
  deleteMock(url)
}

export function getNetLogs(filter?: string): RequestLog[] {
  return queryRequests(filter)
}

export function clearNetLogs(): void {
  clearRequests()
}

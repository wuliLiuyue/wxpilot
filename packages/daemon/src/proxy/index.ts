import mockttp from 'mockttp'
import type { CompletedRequest } from 'mockttp'
import type { PassThroughResponse } from 'mockttp/dist/rules/requests/request-handler-definitions'
import fs from 'fs'
import path from 'path'
import { CA_DIR } from '../constants.js'
import { insertRequest, upsertMock, deleteMock, listMocks, clearRequests, queryRequests, type RequestLog } from '../store/index.js'

let proxyServer: mockttp.Mockttp | null = null
let proxyPort = 8899

export function isRunning(): boolean {
  return proxyServer !== null
}

export function getPort(): number {
  return proxyPort
}

export async function startProxy(port: number, https: boolean): Promise<void> {
  if (proxyServer) await stopProxy()

  proxyPort = port

  const certFile = path.join(CA_DIR, 'cert.pem')
  const keyFile = path.join(CA_DIR, 'key.pem')
  let serverOptions: mockttp.MockttpOptions = {}

  if (https) {
    fs.mkdirSync(CA_DIR, { recursive: true })

    if (!fs.existsSync(certFile) || !fs.existsSync(keyFile)) {
      const ca = await mockttp.generateCACertificate({ commonName: 'wxpilot CA' })
      fs.writeFileSync(certFile, ca.cert)
      fs.writeFileSync(keyFile, ca.key)
    }

    serverOptions = {
      https: { keyPath: keyFile, certPath: certFile },
    }
  }

  proxyServer = mockttp.getLocal(serverOptions)

  // apply saved mock rules first (higher priority)
  const mocks = listMocks()
  for (const mock of mocks) {
    await applyMockRule(mock.url, mock.response)
  }

  // passthrough everything else with logging
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

  await proxyServer.start(port)
}

async function applyMockRule(url: string, responseJson: string): Promise<void> {
  if (!proxyServer) return
  const response = JSON.parse(responseJson) as { status?: number; body?: unknown }
  await proxyServer.forGet(url).thenJson(response.status ?? 200, (response.body ?? response) as object)
}

export async function stopProxy(): Promise<void> {
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

import { describe, it, expect, afterEach, vi, beforeAll } from 'vitest'
import path from 'path'
import os from 'os'
import fs from 'fs'

const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'wxpilot-test-'))
const tmpDb = path.join(tmpDir, 'test.db')

vi.mock('../constants.js', () => ({
  WXPILOT_DIR: tmpDir,
  PID_FILE: path.join(tmpDir, 'daemon.pid'),
  SOCK_FILE: path.join(tmpDir, 'daemon.sock'),
  CA_DIR: path.join(tmpDir, 'ca'),
  DB_FILE: tmpDb,
  IDLE_TIMEOUT_MS: 30 * 60 * 1000,
}))

// Use lazy import inside describe to ensure mock is applied first
let store: typeof import('./index.js')

beforeAll(async () => {
  store = await import('./index.js')
})

afterEach(() => {
  store.closeDb()
  try { fs.unlinkSync(tmpDb) } catch { /* ignore */ }
})

describe('store', () => {
  it('insertRequest + queryRequests', () => {
    store.insertRequest({
      ts: Date.now(),
      method: 'GET',
      url: 'https://example.com/api/list',
      req_headers: '{}',
      req_body: null,
      status: 200,
      res_headers: '{}',
      res_body: null,
      mocked: 0,
      duration: 123,
    })
    const logs = store.queryRequests()
    expect(logs.length).toBe(1)
    expect(logs[0].url).toBe('https://example.com/api/list')
    expect(logs[0].method).toBe('GET')
  })

  it('queryRequests filter', () => {
    store.insertRequest({ ts: Date.now(), method: 'GET', url: 'https://a.com/foo', req_headers: null, req_body: null, status: 200, res_headers: null, res_body: null, mocked: 0, duration: null })
    store.insertRequest({ ts: Date.now(), method: 'POST', url: 'https://b.com/bar', req_headers: null, req_body: null, status: 200, res_headers: null, res_body: null, mocked: 0, duration: null })
    const logs = store.queryRequests('foo')
    expect(logs.length).toBe(1)
    expect(logs[0].url).toContain('foo')
  })

  it('clearRequests', () => {
    store.insertRequest({ ts: Date.now(), method: 'GET', url: 'https://a.com', req_headers: null, req_body: null, status: 200, res_headers: null, res_body: null, mocked: 0, duration: null })
    store.clearRequests()
    expect(store.queryRequests().length).toBe(0)
  })

  it('upsertMock + listMocks + deleteMock', () => {
    store.upsertMock('https://a.com/api', '{"status":200,"body":{}}')
    const mocks = store.listMocks()
    expect(mocks.length).toBe(1)
    expect(mocks[0].url).toBe('https://a.com/api')

    store.deleteMock('https://a.com/api')
    expect(store.listMocks().length).toBe(0)
  })

  it('upsertMock 覆盖已有规则', () => {
    store.upsertMock('https://a.com/api', '{"status":200}')
    store.upsertMock('https://a.com/api', '{"status":404}')
    const mocks = store.listMocks()
    expect(mocks.length).toBe(1)
    expect(mocks[0].response).toBe('{"status":404}')
  })
})

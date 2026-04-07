import fs from 'fs'
import net from 'net'
import path from 'path'
import os from 'os'
import { spawn, spawnSync } from 'child_process'

const TS_SOCK_FILE = path.join(os.homedir(), '.wxpilot', 'daemon.sock')
const RUST_SOCK_FILE = path.join(os.homedir(), '.wxpilot', 'rust-daemon.sock')
const DAEMON_ENTRY = path.join(__dirname, '..', '..', 'daemon', 'dist', 'index.js')
const RUST_DAEMON_ENTRY = path.join(__dirname, '..', '..', '..', 'rust', 'target', 'debug', 'wxp-daemon')
const TS_PID_FILE = path.join(os.homedir(), '.wxpilot', 'daemon.pid')
const RUST_PID_FILE = path.join(os.homedir(), '.wxpilot', 'rust-daemon.pid')

let idCounter = 0
let activeDaemonKind: DaemonKind = 'rust'

export type DaemonKind = 'rust' | 'ts'

export interface ProbeResult {
  socketPath: string
  daemonKind: DaemonKind
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function socketPathFor(kind: DaemonKind): string {
  return kind === 'rust' ? RUST_SOCK_FILE : TS_SOCK_FILE
}

function pidPathFor(kind: DaemonKind): string {
  return kind === 'rust' ? RUST_PID_FILE : TS_PID_FILE
}

function processPatternFor(kind: DaemonKind): string {
  return kind === 'rust' ? RUST_DAEMON_ENTRY : DAEMON_ENTRY
}

function spawnTsDaemon(): void {
  const child = spawn(process.execPath, [DAEMON_ENTRY], {
    detached: true,
    stdio: 'ignore',
  })
  child.on('error', () => undefined)
  child.unref()
}

function spawnRustDaemon(): void {
  const child = spawn(RUST_DAEMON_ENTRY, [], {
    detached: true,
    stdio: 'ignore',
  })
  child.on('error', () => undefined)
  child.unref()
}

async function connectSocketByPath(socketPath: string): Promise<net.Socket> {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection(socketPath)
    socket.once('connect', () => resolve(socket))
    socket.once('error', reject)
  })
}

async function probeDaemon(kind: DaemonKind): Promise<ProbeResult> {
  const socketPath = socketPathFor(kind)
  const socket = await connectSocketByPath(socketPath)
  socket.destroy()
  return { socketPath, daemonKind: kind }
}

export function setPreferredDaemonKind(kind: DaemonKind): void {
  activeDaemonKind = kind
}

function spawnDaemon(kind: DaemonKind, deps: Pick<ConnectDeps, 'spawnRustDaemon' | 'spawnTsDaemon'>): void {
  if (kind === 'rust') {
    deps.spawnRustDaemon()
  } else {
    deps.spawnTsDaemon()
  }
}

type ConnectDeps = {
  connect: (kind: DaemonKind) => Promise<ProbeResult>
  spawnRustDaemon: () => void
  spawnTsDaemon: () => void
  wait?: (ms: number) => Promise<void>
}

type StopDeps = {
  rpcStop: (kind: DaemonKind) => Promise<void>
  probe: (kind: DaemonKind) => Promise<boolean>
  readPidFile: (kind: DaemonKind) => number | null
  listProcessIds: (kind: DaemonKind) => number[]
  killPid: (pid: number) => void
  removeRuntimeFile: (path: string) => void
  wait?: (ms: number) => Promise<void>
}

export async function connectToDaemon(kind: DaemonKind, deps: ConnectDeps): Promise<ProbeResult> {
  const wait = deps.wait ?? sleep

  try {
    return await deps.connect(kind)
  } catch {
    spawnDaemon(kind, deps)
    for (let attempt = 0; attempt < 5; attempt++) {
      if (attempt > 0) {
        await wait(500)
      }
      try {
        return await deps.connect(kind)
      } catch {
        // keep probing the selected daemon kind after spawn
      }
    }
    throw new Error(`daemon_unavailable: failed to connect to ${kind} daemon`)
  }
}

function readPidFile(kind: DaemonKind): number | null {
  const pidPath = pidPathFor(kind)
  if (!fs.existsSync(pidPath)) {
    return null
  }
  const raw = fs.readFileSync(pidPath, 'utf8').trim()
  const pid = Number.parseInt(raw, 10)
  return Number.isNaN(pid) ? null : pid
}

function listProcessIds(kind: DaemonKind): number[] {
  const result = spawnSync('pgrep', ['-f', processPatternFor(kind)], {
    encoding: 'utf8',
  })
  if (result.status !== 0 || !result.stdout) {
    return []
  }
  return result.stdout
    .split('\n')
    .map((line) => Number.parseInt(line.trim(), 10))
    .filter((pid) => Number.isInteger(pid) && pid > 0 && pid !== process.pid)
}

function removeRuntimeFile(pathname: string): void {
  try {
    fs.unlinkSync(pathname)
  } catch {
    // ignore missing runtime files
  }
}

async function probeReachable(kind: DaemonKind): Promise<boolean> {
  try {
    await probeDaemon(kind)
    return true
  } catch {
    return false
  }
}

export async function stopDaemon(kind: DaemonKind, deps?: Partial<StopDeps>): Promise<{ forced: boolean }> {
  const wait = deps?.wait ?? sleep
  const rpcStop = deps?.rpcStop ?? (async (daemonKind: DaemonKind) => {
    await callWithDaemonKind(daemonKind, 'daemon.stop', {}, 5000)
  })
  const probe = deps?.probe ?? probeReachable
  const readPid = deps?.readPidFile ?? readPidFile
  const listPids = deps?.listProcessIds ?? listProcessIds
  const killPid = deps?.killPid ?? ((pid: number) => {
    process.kill(pid, 'SIGTERM')
  })
  const removeFile = deps?.removeRuntimeFile ?? removeRuntimeFile

  try {
    await rpcStop(kind)
    await wait(200)
  } catch {
    // ignore and continue with forced cleanup
  }

  const forcedPids = new Set<number>()
  const pidFromFile = readPid(kind)
  if (pidFromFile != null) {
    forcedPids.add(pidFromFile)
  }
  if (!(await probe(kind))) {
    for (const pid of listPids(kind)) {
      forcedPids.add(pid)
    }
  }

  let forced = false
  for (const pid of forcedPids) {
    try {
      killPid(pid)
      forced = true
    } catch {
      // ignore stale pids
    }
  }

  await wait(200)
  removeFile(socketPathFor(kind))
  removeFile(pidPathFor(kind))

  return { forced }
}

async function connectSocket(kind: DaemonKind = activeDaemonKind): Promise<{ socket: net.Socket; daemonKind: DaemonKind }> {
  const probe = await connectToDaemon(kind, {
    connect: probeDaemon,
    spawnRustDaemon,
    spawnTsDaemon,
  })
  const socket = await connectSocketByPath(probe.socketPath)
  activeDaemonKind = probe.daemonKind
  return { socket, daemonKind: probe.daemonKind }
}

async function callWithDaemonKind(
  kind: DaemonKind,
  method: string,
  params: Record<string, unknown> = {},
  timeout = 10000,
): Promise<unknown> {
  let socket: net.Socket | null = null
  let daemonKind: DaemonKind = kind

  for (let i = 0; i < 3; i++) {
    try {
      const connection = await connectSocket(kind)
      socket = connection.socket
      daemonKind = connection.daemonKind
      break
    } catch {
      await sleep(1000 * (i + 1))
    }
  }

  if (!socket) {
    throw new Error(`daemon_unavailable: failed to connect to ${kind} daemon after 3 attempts`)
  }

  return new Promise((resolve, reject) => {
    const id = ++idCounter
    const request = JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n'
    let buf = ''

    const timer = setTimeout(() => {
      socket!.destroy()
      reject(new Error(`timeout after ${timeout}ms`))
    }, timeout)

    socket!.on('data', (chunk) => {
      buf += chunk.toString()
      const lines = buf.split('\n')
      buf = lines.pop() ?? ''
      for (const line of lines) {
        const trimmed = line.trim()
        if (!trimmed) continue
        try {
          const res = JSON.parse(trimmed)
          if (res.id === id) {
            clearTimeout(timer)
            socket!.destroy()
            if (res.error) {
              reject(Object.assign(new Error(res.error.message), { code: res.error.code, data: res.error.data }))
            } else {
              if (method === 'status' && res.result && typeof res.result === 'object' && res.result.daemonKind == null) {
                res.result = { ...res.result, daemonKind }
              }
              resolve(res.result)
            }
          }
        } catch {
          // ignore parse errors for other lines
        }
      }
    })

    socket!.on('error', (e) => {
      clearTimeout(timer)
      reject(e)
    })

    socket!.write(request)
  })
}

export async function call(method: string, params: Record<string, unknown> = {}, timeout = 10000): Promise<unknown> {
  return callWithDaemonKind(activeDaemonKind, method, params, timeout)
}

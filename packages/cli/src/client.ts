import net from 'net'
import path from 'path'
import os from 'os'
import { spawn } from 'child_process'

const TS_SOCK_FILE = path.join(os.homedir(), '.wxpilot', 'daemon.sock')
const RUST_SOCK_FILE = path.join(os.homedir(), '.wxpilot', 'rust-daemon.sock')
const DAEMON_ENTRY = path.join(__dirname, '..', '..', 'daemon', 'dist', 'index.js')
const RUST_DAEMON_ENTRY = path.join(__dirname, '..', '..', '..', 'rust', 'target', 'debug', 'wxp-daemon')

let idCounter = 0
let activeDaemonKind: DaemonKind = 'ts'

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

export async function connectWithFallback(deps: {
  connect: (kind: DaemonKind) => Promise<ProbeResult>
  spawnRustDaemon: () => void
  spawnTsDaemon: () => void
}): Promise<ProbeResult> {
  try {
    return await deps.connect('rust')
  } catch {
    try {
      return await deps.connect('ts')
    } catch {
      deps.spawnRustDaemon()
      try {
        return await deps.connect('rust')
      } catch {
        deps.spawnTsDaemon()
        return deps.connect('ts')
      }
    }
  }
}

async function connectSocket(): Promise<{ socket: net.Socket; daemonKind: DaemonKind }> {
  const probe = await connectWithFallback({
    connect: probeDaemon,
    spawnRustDaemon,
    spawnTsDaemon,
  })
  const socket = await connectSocketByPath(probe.socketPath)
  activeDaemonKind = probe.daemonKind
  return { socket, daemonKind: probe.daemonKind }
}

export async function call(method: string, params: Record<string, unknown> = {}, timeout = 10000): Promise<unknown> {
  let socket: net.Socket | null = null
  let daemonKind: DaemonKind = activeDaemonKind

  for (let i = 0; i < 3; i++) {
    try {
      const connection = await connectSocket()
      socket = connection.socket
      daemonKind = connection.daemonKind
      break
    } catch {
      if (i === 0) spawnTsDaemon()
      await sleep(1000 * (i + 1))
    }
  }

  if (!socket) {
    throw new Error('daemon_unavailable: failed to connect after 3 attempts')
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

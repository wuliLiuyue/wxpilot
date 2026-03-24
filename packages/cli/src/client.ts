import net from 'net'
import path from 'path'
import os from 'os'
import { spawn } from 'child_process'

const SOCK_FILE = path.join(os.homedir(), '.wxpilot', 'daemon.sock')
const DAEMON_ENTRY = path.join(__dirname, '..', '..', 'daemon', 'dist', 'index.js')

let idCounter = 0

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function spawnDaemon(): void {
  const child = spawn(process.execPath, [DAEMON_ENTRY], {
    detached: true,
    stdio: 'ignore',
  })
  child.unref()
}

async function connectSocket(): Promise<net.Socket> {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection(SOCK_FILE)
    socket.once('connect', () => resolve(socket))
    socket.once('error', reject)
  })
}

export async function call(method: string, params: Record<string, unknown> = {}, timeout = 10000): Promise<unknown> {
  let socket: net.Socket | null = null

  for (let i = 0; i < 3; i++) {
    try {
      socket = await connectSocket()
      break
    } catch {
      if (i === 0) spawnDaemon()
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

import fs from 'fs'
import { PID_FILE, SOCK_FILE, WXPILOT_DIR } from './constants.js'
import { startServer } from './rpc-server.js'
import { closeDb } from './store/index.js'
import { disconnect } from './automator/index.js'
import { stopProxy } from './proxy/index.js'

function isProcessAlive(pid: number): boolean {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

function cleanup() {
  try { fs.unlinkSync(PID_FILE) } catch { /* ignore */ }
  try { fs.unlinkSync(SOCK_FILE) } catch { /* ignore */ }
}

async function main() {
  fs.mkdirSync(WXPILOT_DIR, { recursive: true })

  // check existing PID
  if (fs.existsSync(PID_FILE)) {
    const pidStr = fs.readFileSync(PID_FILE, 'utf8').trim()
    const existingPid = parseInt(pidStr, 10)
    if (!isNaN(existingPid) && isProcessAlive(existingPid)) {
      console.error(`ERR: daemon already running (PID ${existingPid})`)
      process.exit(1)
    }
    // stale PID — clean up
    cleanup()
  }

  // clean up stale socket
  if (fs.existsSync(SOCK_FILE)) {
    try { fs.unlinkSync(SOCK_FILE) } catch { /* ignore */ }
  }

  // write PID
  fs.writeFileSync(PID_FILE, String(process.pid))

  // graceful shutdown
  async function shutdown() {
    try { await stopProxy() } catch { /* ignore */ }
    try { await disconnect() } catch { /* ignore */ }
    closeDb()
    cleanup()
    process.exit(0)
  }

  process.on('SIGTERM', () => void shutdown())
  process.on('SIGINT', () => void shutdown())
  process.on('exit', cleanup)

  startServer()
  console.log(`wxpilot daemon started (PID ${process.pid}, socket: ${SOCK_FILE})`)
}

main().catch((e) => {
  console.error('daemon error:', e)
  process.exit(1)
})

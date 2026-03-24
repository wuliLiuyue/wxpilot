import os from 'os'
import path from 'path'

export const WXPILOT_DIR = path.join(os.homedir(), '.wxpilot')
export const PID_FILE = path.join(WXPILOT_DIR, 'daemon.pid')
export const SOCK_FILE = path.join(WXPILOT_DIR, 'daemon.sock')
export const CA_DIR = path.join(WXPILOT_DIR, 'ca')
export const DB_FILE = path.join(WXPILOT_DIR, 'wxpilot.db')

export const IDLE_TIMEOUT_MS = 30 * 60 * 1000

let jsonMode = false
let verboseMode = false

const MAX_OUTPUT_CHARS = 4000

export function setJsonMode(v: boolean) { jsonMode = v }
export function setVerboseMode(v: boolean) { verboseMode = v }
export function isJsonMode() { return jsonMode }

function safeTruncate(str: string): string {
  if (str.length <= MAX_OUTPUT_CHARS) return str
  return str.slice(0, MAX_OUTPUT_CHARS) + `\n[truncated, originalLength=${str.length}]`
}

export function printOk(data: Record<string, unknown>): void {
  if (jsonMode) {
    const raw = JSON.stringify({ ok: true, ...data })
    console.log(safeTruncate(raw))
    return
  }
  // human-readable output
  if (data['output']) {
    console.log(safeTruncate(String(data['output'])))
    return
  }
  if (data['action'] === 'tap') {
    console.log(`OK: tap %${data['ref']}`)
    return
  }
  if (data['action'] === 'type') {
    console.log(`OK: type %${data['ref']} "${data['text']}"`)
    return
  }
  if (data['action'] === 'scroll') {
    console.log(`OK: scroll %${data['ref']} ${data['direction']}`)
    return
  }
  if (data['text'] !== undefined && data['ref'] !== undefined) {
    console.log(`OK: read %${data['ref']} "${data['text']}"`)
    return
  }
  if (data['base64']) {
    const b64 = String(data['base64'])
    console.log(`OK: screenshot captured (base64)\n${safeTruncate(b64)}`)
    return
  }
  if (data['path'] !== undefined && data['action'] === undefined) {
    console.log(`OK: screenshot saved → ${data['path']}`)
    return
  }
  if (data['summary'] !== undefined) {
    const hint = data['hint'] ? `\n[${data['hint']}]` : ''
    console.log(safeTruncate(JSON.stringify(data['summary'], null, 2)) + hint)
    return
  }
  if (data['result'] !== undefined) {
    console.log(safeTruncate(`OK: ${JSON.stringify(data['result'])}`))
    return
  }
  if (data['data'] !== undefined) {
    const out = JSON.stringify(data['data'], null, 2)
    console.log(safeTruncate(out))
    if (data['truncated']) console.log(`[truncated, originalLength=${data['originalLength']}]`)
    return
  }
  // generic
  const parts = Object.entries(data)
    .filter(([k]) => k !== 'ok')
    .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
    .join(' ')
  console.log(safeTruncate(`OK${parts ? ': ' + parts : ''}`))
}

export function printErr(message: string, extra?: Record<string, unknown>): void {
  if (jsonMode) {
    console.error(JSON.stringify({ ok: false, error: message, ...extra }))
  } else {
    console.error(`ERR: ${message}`)
  }
}

export function printRefExpired(ref: number): void {
  if (jsonMode) {
    console.error(JSON.stringify({ ok: false, error: 'ref_expired', ref }))
  } else {
    console.error(`ERR: %${ref} expired, run 'wxpilot view' to refresh`)
  }
}

export function verbose(msg: string): void {
  if (verboseMode) console.error(`[verbose] ${msg}`)
}

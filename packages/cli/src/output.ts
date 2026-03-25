let jsonMode = false
let verboseMode = false

export function setJsonMode(v: boolean) { jsonMode = v }
export function setVerboseMode(v: boolean) { verboseMode = v }
export function isJsonMode() { return jsonMode }

export function printOk(data: Record<string, unknown>): void {
  if (jsonMode) {
    console.log(JSON.stringify({ ok: true, ...data }))
    return
  }
  // human-readable output
  if (data['output']) {
    console.log(data['output'])
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
    console.log(`OK: screenshot captured (base64)\n${data['base64']}`)
    return
  }
  if (data['path'] !== undefined && data['action'] === undefined) {
    console.log(`OK: screenshot saved → ${data['path']}`)
    return
  }
  if (data['result'] !== undefined) {
    console.log(`OK: ${JSON.stringify(data['result'])}`)
    return
  }
  if (data['data'] !== undefined) {
    console.log(JSON.stringify(data['data'], null, 2))
    return
  }
  // generic
  const parts = Object.entries(data)
    .filter(([k]) => k !== 'ok')
    .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
    .join(' ')
  console.log(`OK${parts ? ': ' + parts : ''}`)
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

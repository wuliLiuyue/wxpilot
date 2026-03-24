import type MiniProgram from 'miniprogram-automator/out/MiniProgram'
import type Element from 'miniprogram-automator/out/Element'
import type { InputElement, ScrollViewElement } from 'miniprogram-automator/out/Element'
import fs from 'fs'
import os from 'os'
import path from 'path'

// eslint-disable-next-line @typescript-eslint/no-require-imports
const automator = require('miniprogram-automator') as {
  connect: (opts: { wsEndpoint: string }) => Promise<MiniProgram>
}

let miniProgram: MiniProgram | null = null
let connectedPath: string | null = null

export function isConnected(): boolean {
  return miniProgram !== null
}

export function getConnectedPath(): string | null {
  return connectedPath
}

export async function connectWs(wsEndpoint: string): Promise<void> {
  if (miniProgram) {
    await disconnect()
  }
  miniProgram = await automator.connect({ wsEndpoint })
  connectedPath = wsEndpoint
}

export async function disconnect(): Promise<void> {
  if (miniProgram) {
    try { miniProgram.disconnect() } catch { /* ignore */ }
    miniProgram = null
    connectedPath = null
  }
}

function getMp(): MiniProgram {
  if (!miniProgram) throw new Error('not_connected')
  return miniProgram
}

async function getCurrentPage() {
  const page = await getMp().currentPage()
  if (!page) throw new Error('not_connected')
  return page
}

export async function goto(route: string): Promise<void> {
  await getMp().navigateTo(route)
}

export async function back(): Promise<void> {
  await getMp().navigateBack()
}

export async function reload(): Promise<void> {
  const page = await getCurrentPage()
  const path = page.path
  const query = page.query as Record<string, string>
  const qs = Object.keys(query).length
    ? '?' + Object.entries(query).map(([k, v]) => `${k}=${v}`).join('&')
    : ''
  await getMp().reLaunch(path + qs)
}

export async function screenshot(savePath?: string): Promise<{ path: string; base64?: string }> {
  const mp = getMp()
  const tmpPath = savePath ?? path.join(os.tmpdir(), `wxpilot-shot-${Date.now()}.png`)
  await mp.screenshot({ path: tmpPath })
  if (savePath) {
    return { path: savePath }
  }
  const data = fs.readFileSync(tmpPath)
  fs.unlinkSync(tmpPath)
  return { path: tmpPath, base64: data.toString('base64') }
}

export async function runScript(js: string): Promise<unknown> {
  return getMp().evaluate(js)
}

export async function getWxml(): Promise<string> {
  const page = await getCurrentPage()
  const el = await page.$('page')
  if (!el) throw new Error('element_not_found')
  return el.outerWxml()
}

export async function getPagePath(): Promise<string> {
  const page = await getCurrentPage()
  return page.path
}

export async function getPageData(dataPath?: string): Promise<unknown> {
  const page = await getCurrentPage()
  return page.data(dataPath)
}

async function getElement(selector: string): Promise<Element> {
  const page = await getCurrentPage()
  const el = await page.$(selector)
  if (!el) throw new Error('element_not_found')
  return el
}

export async function tapElement(selector: string): Promise<void> {
  const el = await getElement(selector)
  await el.tap()
}

export async function typeElement(selector: string, text: string): Promise<void> {
  const el = await getElement(selector)
  await (el as unknown as InputElement).input(text)
}

export async function scrollElement(selector: string, direction: 'up' | 'down'): Promise<void> {
  const el = await getElement(selector)
  const sv = el as unknown as ScrollViewElement
  // get current scrollTop, offset by ±300
  const currentTop = await el.property('scrollTop').catch(() => 0)
  const delta = direction === 'down' ? 300 : -300
  await sv.scrollTo(0, Number(currentTop) + delta)
}

export async function getElementText(selector: string): Promise<string> {
  const el = await getElement(selector)
  return el.text()
}

export async function callWxMethod(method: string, args: unknown[]): Promise<unknown> {
  return getMp().callWxMethod(method, ...args)
}

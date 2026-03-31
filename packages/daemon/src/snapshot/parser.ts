import { parseDocument } from 'htmlparser2'
import { Element, Text, Node } from 'domhandler'

export interface ElementNode {
  tag: string
  classes: string[]
  text?: string
  placeholder?: string
  attrs: Record<string, string>
  interactive: boolean
  selector: string
  ref?: number
  children: ElementNode[]
}

const INTERACTIVE_TAGS = new Set([
  'button', 'input', 'textarea', 'picker', 'switch', 'slider', 'checkbox', 'radio',
])

const INTERACTIVE_ATTRS = new Set([
  'bindtap', 'catchtap', 'bind:tap', 'catch:tap',
  'bindchange', 'bind:change',
  'bindinput', 'bind:input',
  'bindsubmit', 'bind:submit',
  'bindlongpress', 'bind:longpress',
])

// 框架内部 data-* 属性，不作为交互信号（如 Taro 的 data-sid、Vue 的 data-v-*）
const FRAMEWORK_DATA_ATTR_RE = /^data-(sid$|v-|__)/

function isInteractive(el: Element, eventFingerprints?: Set<string>): boolean {
  if (INTERACTIVE_TAGS.has(el.name)) return true
  for (const attr of INTERACTIVE_ATTRS) {
    if (el.attribs[attr] != null) return true
  }
  // Taro/NutUI: onClick 编译时附带 hover timing 属性，非交互节点不会显式声明
  if (el.attribs['hover-start-time'] != null || el.attribs['hover-stay-time'] != null) return true
  // data-* 作为代理信号：outerWxml 快照保留 data-* 属性，但会剥离 bindtap 等事件绑定
  // 排除框架内部属性（data-sid / data-v-* 等）避免误判
  for (const attr of Object.keys(el.attribs)) {
    if (attr.startsWith('data-') && !FRAMEWORK_DATA_ATTR_RE.test(attr)) return true
  }
  // 源文件补全：通过 tag+class 指纹匹配补充漏判的 bindtap 节点（无 data-* 但有 bindtap）
  if (eventFingerprints && el.attribs['class']) {
    const classes = el.attribs['class'].split(' ').filter(Boolean).sort().join(',')
    if (classes && eventFingerprints.has(`${el.name}:${classes}`)) return true
  }
  return false
}

/**
 * 从源 WXML 文件提取有事件绑定节点的 tag+class 指纹集合。
 * 用于补充 outerWxml 快照中被剥离的 bindtap 等属性。
 */
export function extractEventFingerprints(sourceWxml: string): Set<string> {
  const doc = parseDocument(sourceWxml, { xmlMode: false, lowerCaseTags: true })
  const fingerprints = new Set<string>()

  function walk(node: Node): void {
    if (!(node instanceof Element)) return
    const hasEvent = [...INTERACTIVE_ATTRS].some(a => node.attribs[a] != null)
    if (hasEvent && node.attribs['class']) {
      const classes = node.attribs['class'].split(' ').filter(Boolean).sort().join(',')
      if (classes) fingerprints.add(`${node.name}:${classes}`)
    }
    for (const child of node.children) walk(child)
  }

  for (const node of doc.children) walk(node)
  return fingerprints
}

function getTextContent(node: Node): string {
  if (node instanceof Text) return node.data.trim()
  if (node instanceof Element) {
    return node.children.map(getTextContent).join(' ').trim()
  }
  return ''
}

function buildSelector(el: Element, parentSelector: string, _index: number): string {
  const id = el.attribs['id']
  if (id) return `#${id}`
  const cls = el.attribs['class']?.split(' ').filter(Boolean)[0]
  const base = cls ? `${el.name}.${cls}` : el.name
  return parentSelector ? `${parentSelector} > ${base}` : base
}

function parseNode(
  node: Node,
  parentSelector: string,
  siblingIndex: number,
  depth: number,
  maxDepth: number,
  eventFingerprints?: Set<string>,
): ElementNode | null {
  if (!(node instanceof Element)) return null
  if (['wxs', 'script', 'style'].includes(node.name)) return null

  const selector = buildSelector(node, parentSelector, siblingIndex)
  const classes = (node.attribs['class'] ?? '').split(' ').filter(Boolean)
  const text = getTextContent(node)
  const placeholder = node.attribs['placeholder']
  const attrs: Record<string, string> = {}
  for (const [k, v] of Object.entries(node.attribs)) {
    if (!['class', 'style', 'id'].includes(k)) attrs[k] = v
  }
  const interactive = isInteractive(node, eventFingerprints)

  const children: ElementNode[] = []
  if (depth < maxDepth) {
    let idx = 0
    for (const child of node.children) {
      const parsed = parseNode(child, selector, idx, depth + 1, maxDepth, eventFingerprints)
      if (parsed) {
        children.push(parsed)
        idx++
      }
    }
  }

  return { tag: node.name, classes, text: text || undefined, placeholder, attrs, interactive, selector, children }
}

export function parseWxml(wxml: string, maxDepth = 20, eventFingerprints?: Set<string>): ElementNode[] {
  const doc = parseDocument(wxml, { xmlMode: false, lowerCaseTags: true })
  const roots: ElementNode[] = []
  let idx = 0
  for (const node of doc.children) {
    // outerWxml() includes the <page> root tag itself; pass empty parentSelector
    // so buildSelector produces 'page' (not 'page > page')
    const isPageRoot = node instanceof Element && node.name === 'page'
    const parsed = parseNode(node, isPageRoot ? '' : 'page', idx, 0, maxDepth, eventFingerprints)
    if (parsed) { roots.push(parsed); idx++ }
  }
  return roots
}

export function assignRefs(roots: ElementNode[]): Map<number, string> {
  const map = new Map<number, string>()
  let counter = 1

  function walk(node: ElementNode) {
    if (node.interactive) {
      node.ref = counter
      map.set(counter, node.selector)
      counter++
    }
    for (const child of node.children) walk(child)
  }

  for (const root of roots) walk(root)
  return map
}

export interface RenderResult {
  output: string
  shownCount: number
  totalCount: number
}

export interface FindMatch {
  ref?: number
  tag: string
  classes: string[]
  text?: string
  placeholder?: string
  selector: string
  interactive: boolean
}

export interface FindResult {
  matches: FindMatch[]
  shownCount: number
  totalCount: number
}

interface RenderCounter {
  shown: number
  total: number
}

function _renderNodes(
  nodes: ElementNode[],
  opts: { compact?: boolean; depth?: number; limit?: number; textLimit?: number },
  currentDepth: number,
  prefix: string,
  counter: RenderCounter,
): string {
  const lines: string[] = []
  const maxDepth = opts.depth ?? 999
  const textLimit = opts.textLimit ?? 40

  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i]
    const last = i === nodes.length - 1
    const connector = last ? '└─' : '├─'
    const childPrefix = prefix + (last ? '   ' : '│  ')

    if (opts.compact && !node.interactive && !hasInteractiveDescendant(node)) continue

    if (currentDepth <= maxDepth) {
      const limit = opts.limit
      if (node.interactive) {
        if (limit != null && counter.shown >= limit) continue
        counter.shown++
      }

      const tag = node.interactive ? `[${node.tag}]` : node.tag
      const cls = node.classes.length ? `.${node.classes[0]}` : ''
      const textPart = node.text ? ` "${node.text.slice(0, textLimit)}"` : ''
      const phPart = node.placeholder ? `  placeholder="${node.placeholder}"` : ''
      const refPart = node.ref != null ? `  → %${node.ref}` : ''
      lines.push(`${prefix}${connector} ${tag}${cls}${textPart}${phPart}${refPart}`)

      if (node.children.length && currentDepth < maxDepth) {
        const sub = _renderNodes(node.children, opts, currentDepth + 1, childPrefix, counter)
        if (sub) lines.push(sub)
      }
    }
  }

  return lines.join('\n')
}

export function renderTree(
  nodes: ElementNode[],
  opts: { compact?: boolean; depth?: number; limit?: number; textLimit?: number } = {},
): RenderResult {
  // count total interactive nodes first (needed even when limit is applied)
  const totalCounter: RenderCounter = { shown: 0, total: 0 }
  _countInteractive(nodes, totalCounter)

  const counter: RenderCounter = { shown: 0, total: 0 }
  const output = _renderNodes(nodes, opts, 0, '', counter)
  return { output, shownCount: counter.shown, totalCount: totalCounter.total }
}

function _countInteractive(nodes: ElementNode[], counter: RenderCounter): void {
  for (const node of nodes) {
    if (node.interactive) counter.total++
    _countInteractive(node.children, counter)
  }
}

function hasInteractiveDescendant(node: ElementNode): boolean {
  if (node.interactive) return true
  return node.children.some(hasInteractiveDescendant)
}

function matchesQuery(node: ElementNode, query: string): boolean {
  if (!query) return false
  const normalized = query.toLowerCase()
  if (node.text?.toLowerCase().includes(normalized)) return true
  if (node.placeholder?.toLowerCase().includes(normalized)) return true
  if (node.tag.toLowerCase().includes(normalized)) return true
  return node.classes.some(cls => cls.toLowerCase().includes(normalized))
}

export function findNodes(
  nodes: ElementNode[],
  query: string,
  opts: { interactiveOnly?: boolean; limit?: number } = {},
): FindResult {
  const matches: FindMatch[] = []
  let totalCount = 0
  const interactiveOnly = opts.interactiveOnly ?? true
  const limit = opts.limit

  function walk(node: ElementNode): void {
    const eligible = !interactiveOnly || node.interactive
    if (eligible && matchesQuery(node, query)) {
      totalCount++
      if (limit == null || matches.length < limit) {
        matches.push({
          ref: node.ref,
          tag: node.tag,
          classes: node.classes,
          text: node.text,
          placeholder: node.placeholder,
          selector: node.selector,
          interactive: node.interactive,
        })
      }
    }
    for (const child of node.children) walk(child)
  }

  for (const node of nodes) walk(node)

  return { matches, shownCount: matches.length, totalCount }
}

export function renderFindMatches(
  matches: FindMatch[],
  opts: { textLimit?: number } = {},
): string {
  const textLimit = opts.textLimit ?? 30
  return matches.map((match) => {
    const refPart = match.ref != null ? `%${match.ref}` : '-'
    const tagPart = match.interactive ? `[${match.tag}]` : match.tag
    const clsPart = match.classes.length ? `.${match.classes[0]}` : ''
    const textPart = match.text ? ` "${match.text.slice(0, textLimit)}"` : ''
    const placeholderPart = !match.text && match.placeholder
      ? ` placeholder="${match.placeholder.slice(0, textLimit)}"`
      : ''
    return `${refPart} ${tagPart}${clsPart}${textPart}${placeholderPart}`
  }).join('\n')
}

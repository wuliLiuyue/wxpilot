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

function isInteractive(el: Element): boolean {
  if (INTERACTIVE_TAGS.has(el.name)) return true
  for (const attr of INTERACTIVE_ATTRS) {
    if (el.attribs[attr] != null) return true
  }
  // elements with data-* attributes are typically interactive (event data params in WeChat)
  for (const attr of Object.keys(el.attribs)) {
    if (attr.startsWith('data-')) return true
  }
  return false
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
  const interactive = isInteractive(node)

  const children: ElementNode[] = []
  if (depth < maxDepth) {
    let idx = 0
    for (const child of node.children) {
      const parsed = parseNode(child, selector, idx, depth + 1, maxDepth)
      if (parsed) {
        children.push(parsed)
        idx++
      }
    }
  }

  return { tag: node.name, classes, text: text || undefined, placeholder, attrs, interactive, selector, children }
}

export function parseWxml(wxml: string, maxDepth = 20): ElementNode[] {
  const doc = parseDocument(wxml, { xmlMode: false, lowerCaseTags: true })
  const roots: ElementNode[] = []
  let idx = 0
  for (const node of doc.children) {
    // outerWxml() includes the <page> root tag itself; pass empty parentSelector
    // so buildSelector produces 'page' (not 'page > page')
    const isPageRoot = node instanceof Element && node.name === 'page'
    const parsed = parseNode(node, isPageRoot ? '' : 'page', idx, 0, maxDepth)
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

export function renderTree(
  nodes: ElementNode[],
  opts: { compact?: boolean; depth?: number } = {},
  currentDepth = 0,
  prefix = '',
  isLast = true,
): string {
  const lines: string[] = []
  const maxDepth = opts.depth ?? 999

  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i]
    const last = i === nodes.length - 1
    const connector = last ? '└─' : '├─'
    const childPrefix = prefix + (last ? '   ' : '│  ')

    if (opts.compact && !node.interactive && !hasInteractiveDescendant(node)) continue

    if (currentDepth <= maxDepth) {
      const tag = node.interactive ? `[${node.tag}]` : node.tag
      const cls = node.classes.length ? `.${node.classes[0]}` : ''
      const textPart = node.text ? ` "${node.text.slice(0, 40)}"` : ''
      const phPart = node.placeholder ? `  placeholder="${node.placeholder}"` : ''
      const refPart = node.ref != null ? `  → %${node.ref}` : ''
      lines.push(`${prefix}${connector} ${tag}${cls}${textPart}${phPart}${refPart}`)

      if (node.children.length && currentDepth < maxDepth) {
        const sub = renderTree(node.children, opts, currentDepth + 1, childPrefix, last)
        if (sub) lines.push(sub)
      }
    }
  }

  return lines.join('\n')
}

function hasInteractiveDescendant(node: ElementNode): boolean {
  if (node.interactive) return true
  return node.children.some(hasInteractiveDescendant)
}

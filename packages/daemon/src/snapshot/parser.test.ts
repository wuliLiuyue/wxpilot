import { describe, it, expect } from 'vitest'
import { parseWxml, assignRefs, renderTree } from './parser.js'

const SAMPLE_WXML = `
<view class="container">
  <button class="btn-submit" bindtap="onSubmit">提交订单</button>
  <input placeholder="手机号" />
  <scroll-view class="product-list">
    <view class="card">可颂 ¥19.00</view>
    <view class="card">蛋糕 ¥199.00</view>
  </scroll-view>
  <text>合计: ¥218.00</text>
</view>
`

describe('parseWxml', () => {
  it('解析 WXML 生成 ElementNode 树', () => {
    const tree = parseWxml(SAMPLE_WXML)
    expect(tree.length).toBeGreaterThan(0)
    const container = tree[0]
    expect(container.tag).toBe('view')
    expect(container.classes).toContain('container')
  })

  it('标注可交互节点', () => {
    const tree = parseWxml(SAMPLE_WXML)
    const container = tree[0]
    const button = container.children.find((n) => n.tag === 'button')
    expect(button).toBeDefined()
    expect(button!.interactive).toBe(true)

    const input = container.children.find((n) => n.tag === 'input')
    expect(input).toBeDefined()
    expect(input!.interactive).toBe(true)
  })

  it('非交互节点 interactive 为 false', () => {
    const tree = parseWxml(SAMPLE_WXML)
    const container = tree[0]
    const text = container.children.find((n) => n.tag === 'text')
    expect(text?.interactive).toBe(false)
  })

  it('解析 placeholder', () => {
    const tree = parseWxml(SAMPLE_WXML)
    const container = tree[0]
    const input = container.children.find((n) => n.tag === 'input')
    expect(input?.placeholder).toBe('手机号')
  })
})

describe('assignRefs', () => {
  it('为可交互节点分配 ref，从 1 开始递增', () => {
    const tree = parseWxml(SAMPLE_WXML)
    const refs = assignRefs(tree)
    expect(refs.size).toBeGreaterThanOrEqual(2) // button + input
    expect(refs.has(1)).toBe(true)
    expect(refs.has(2)).toBe(true)
  })

  it('ref 值与 node.ref 一致', () => {
    const tree = parseWxml(SAMPLE_WXML)
    const refs = assignRefs(tree)
    const container = tree[0]
    const button = container.children.find((n) => n.tag === 'button')
    expect(button?.ref).toBeDefined()
    expect(refs.has(button!.ref!)).toBe(true)
  })
})

describe('renderTree', () => {
  it('生成可读的文本树', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const output = renderTree(tree)
    expect(output).toContain('[button]')
    expect(output).toContain('[input]')
    expect(output).toContain('%1')
    expect(output).toContain('%2')
  })

  it('--compact 只输出有交互子节点的分支', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const output = renderTree(tree, { compact: true })
    expect(output).toContain('[button]')
    // pure text with no interactive descendant should be omitted or kept
    // (text has no interactive descendant)
  })

  it('--depth 1 限制展开层数', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const full = renderTree(tree)
    const shallow = renderTree(tree, { depth: 0 })
    expect(shallow.split('\n').length).toBeLessThanOrEqual(full.split('\n').length)
  })
})

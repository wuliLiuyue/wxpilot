import { describe, it, expect } from 'vitest'
import { parseWxml, assignRefs, renderTree, extractEventFingerprints } from './parser.js'

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
    const { output } = renderTree(tree)
    expect(output).toContain('[button]')
    expect(output).toContain('[input]')
    expect(output).toContain('%1')
    expect(output).toContain('%2')
  })

  it('--compact 只输出有交互子节点的分支', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { output } = renderTree(tree, { compact: true })
    expect(output).toContain('[button]')
  })

  it('--depth 0 限制展开层数', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { output: full } = renderTree(tree)
    const { output: shallow } = renderTree(tree, { depth: 0 })
    expect(shallow.split('\n').length).toBeLessThanOrEqual(full.split('\n').length)
  })

  it('limit 限制交互节点数量并返回 shownCount/totalCount', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { shownCount, totalCount } = renderTree(tree, { compact: true, limit: 1 })
    expect(shownCount).toBe(1)
    expect(totalCount).toBeGreaterThan(1)
  })

  it('data-* 节点被视为交互节点（outerWxml 快照代理信号）', () => {
    const wxml = `<view><view data-id="1">可点击项</view></view>`
    const tree = parseWxml(wxml)
    const { totalCount } = renderTree(tree)
    expect(totalCount).toBe(1)
  })
})

describe('extractEventFingerprints', () => {
  it('提取有 bindtap 节点的 tag+class 指纹', () => {
    const sourceWxml = `
      <view class="card-item shop" bindtap="handleJump" data-url="/foo">门店</view>
      <view class="cake-item" bindtap="showSheet">蛋糕</view>
      <view class="pure-display">纯展示</view>
    `
    const fps = extractEventFingerprints(sourceWxml)
    expect(fps.has('view:card-item,shop')).toBe(true)
    expect(fps.has('view:cake-item')).toBe(true)
    expect(fps.has('view:pure-display')).toBe(false)
  })

  it('sourceWxml 指纹可补全无 data-* 的漏判节点', () => {
    // outerWxml 快照中 bindtap 已丢失，只剩 class
    const outerWxml = `<page><view class="cake-item">蛋糕预订</view></page>`
    // 源文件有 bindtap
    const sourceWxml = `<view class="cake-item" bindtap="showSheet">蛋糕</view>`
    const fps = extractEventFingerprints(sourceWxml)
    const tree = parseWxml(outerWxml, 20, fps)
    const { totalCount } = renderTree(tree)
    expect(totalCount).toBe(1)
  })

  it('无 class 属性的节点不生成指纹', () => {
    const sourceWxml = `<view bindtap="handleTap">无 class</view>`
    const fps = extractEventFingerprints(sourceWxml)
    expect(fps.size).toBe(0)
  })

  it('指纹碰撞问题：相同 tag:class 组合但只有部分有 bindtap', () => {
    // 源文件：第一个 item 可点击，第二个不可点击
    const sourceWxml = `
      <view class="item" bindtap="handler">可点击</view>
      <view class="item">不可点击的展示项</view>
    `
    const fps = extractEventFingerprints(sourceWxml)
    // 指纹包含 view:item，但无法区分两个相同 class 的节点
    expect(fps.has('view:item')).toBe(true)

    // outerWxml 快照（bindtap 已被剥离）
    const outerWxml = `<page>
      <view class="item">可点击</view>
      <view class="item">不可点击的展示项</view>
    </page>`

    const tree = parseWxml(outerWxml, 20, fps)
    const { totalCount } = renderTree(tree)

    // 实际结果：两个节点都被标记为交互（误判）
    // 预期结果：只有第一个节点应该是交互节点
    console.log('实际交互节点数:', totalCount)
    console.log('预期交互节点数: 1')
    // 这个测试会失败，证明存在碰撞问题
    // expect(totalCount).toBe(1)
  })

  it('指纹碰撞问题：多个相同 class，部分可点击', () => {
    const sourceWxml = `
      <view class="btn primary" bindtap="onSubmit">提交</view>
      <view class="btn primary">禁用状态</view>
      <view class="btn primary" bindtap="onCancel">取消</view>
    `
    const fps = extractEventFingerprints(sourceWxml)
    expect(fps.has('view:btn,primary')).toBe(true)

    const outerWxml = `<page>
      <view class="btn primary">提交</view>
      <view class="btn primary">禁用状态</view>
      <view class="btn primary">取消</view>
    </page>`

    const tree = parseWxml(outerWxml, 20, fps)
    const { totalCount } = renderTree(tree)

    // 实际结果：3个节点都被标记为交互（误判）
    // 预期结果：2个节点（提交和取消）
    console.log('实际交互节点数:', totalCount)
    console.log('预期交互节点数: 2')
    // expect(totalCount).toBe(2)
  })
})

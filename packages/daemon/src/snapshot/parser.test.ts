import { describe, it, expect } from 'vitest'
import { parseWxml, assignRefs, renderTree, extractEventFingerprints, findNodes, renderFindMatches } from './parser.js'

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

  it('hover-start-time 节点被视为交互（Taro/NutUI onClick 编译特征）', () => {
    const wxml = `<page><view class="nut-sku-item" hover-class="none" hover-start-time="50" hover-stay-time="400">XL</view></page>`
    const tree = parseWxml(wxml)
    const { totalCount } = renderTree(tree)
    expect(totalCount).toBe(1)
  })

  it('仅有 hover-start-time 时也视为交互', () => {
    const wxml = `<page><view hover-start-time="50">按钮</view></page>`
    const tree = parseWxml(wxml)
    const { totalCount } = renderTree(tree)
    expect(totalCount).toBe(1)
  })

  it('无 hover timing 的普通 view 不误判为交互', () => {
    const wxml = `<page><view class="title">标题</view></page>`
    const tree = parseWxml(wxml)
    const { totalCount } = renderTree(tree)
    expect(totalCount).toBe(0)
  })

  it('data-sid 不触发交互判断（Taro 框架内部属性）', () => {
    const wxml = `<page><view data-sid="_XT" class="title">标题</view></page>`
    const tree = parseWxml(wxml)
    const { totalCount } = renderTree(tree)
    expect(totalCount).toBe(0)
  })

  it('data-v- 前缀不触发交互判断（Vue 框架内部属性）', () => {
    const wxml = `<page><view data-v-abc123="1">内容</view></page>`
    const tree = parseWxml(wxml)
    const { totalCount } = renderTree(tree)
    expect(totalCount).toBe(0)
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

  // 已知局限：tag:class 指纹无法区分相同 class 下可点击与不可点击的节点，会产生误判
  // 这是 tag+class 指纹机制的固有限制，可接受的误判率换取源文件补全覆盖率
  it('指纹碰撞：相同 tag:class 下所有节点均被视为交互（已知误判）', () => {
    const sourceWxml = `
      <view class="item" bindtap="handler">可点击</view>
      <view class="item">不可点击的展示项</view>
    `
    const fps = extractEventFingerprints(sourceWxml)
    expect(fps.has('view:item')).toBe(true)

    const outerWxml = `<page>
      <view class="item">可点击</view>
      <view class="item">不可点击的展示项</view>
    </page>`
    const tree = parseWxml(outerWxml, 20, fps)
    const { totalCount } = renderTree(tree)
    // 两个节点均被标记为交互（含 1 个误判），这是已知行为
    expect(totalCount).toBe(2)
  })

  it('指纹碰撞：多个相同 class 节点全部误判为交互（已知误判）', () => {
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
    // 3 个节点均被标记为交互（含 1 个误判），这是已知行为
    expect(totalCount).toBe(3)
  })
})

describe('findNodes', () => {
  it('按文本匹配交互节点并返回 ref', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { matches, totalCount } = findNodes(tree, '提交')
    expect(totalCount).toBe(1)
    expect(matches).toHaveLength(1)
    expect(matches[0]?.tag).toBe('button')
    expect(matches[0]?.ref).toBe(1)
  })

  it('按 placeholder 匹配输入框', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { matches } = findNodes(tree, '手机号')
    expect(matches).toHaveLength(1)
    expect(matches[0]?.tag).toBe('input')
    expect(matches[0]?.placeholder).toBe('手机号')
  })

  it('按 class 模糊匹配并忽略大小写', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { matches } = findNodes(tree, 'BTN-SUB')
    expect(matches).toHaveLength(1)
    expect(matches[0]?.classes).toContain('btn-submit')
  })

  it('按 tag 匹配交互节点', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { matches } = findNodes(tree, 'INPUT')
    expect(matches).toHaveLength(1)
    expect(matches[0]?.tag).toBe('input')
  })

  it('默认只返回交互节点', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { matches, totalCount } = findNodes(tree, '合计')
    expect(totalCount).toBe(0)
    expect(matches).toHaveLength(0)
  })

  it('interactiveOnly=false 时可返回非交互节点', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { matches, totalCount } = findNodes(tree, '合计', { interactiveOnly: false })
    expect(totalCount).toBe(2)
    expect(matches[0]?.interactive).toBe(false)
    expect(matches[0]?.ref).toBeUndefined()
  })

  it('limit 限制返回数量但保留总匹配数', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { matches, shownCount, totalCount } = findNodes(tree, '¥', { interactiveOnly: false, limit: 1 })
    expect(matches).toHaveLength(1)
    expect(shownCount).toBe(1)
    expect(totalCount).toBeGreaterThan(1)
  })
})

describe('renderFindMatches', () => {
  it('生成紧凑匹配摘要', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { matches } = findNodes(tree, '提交')
    const output = renderFindMatches(matches)
    expect(output).toContain('%1 [button].btn-submit "提交订单"')
  })

  it('无文本时回退显示 placeholder', () => {
    const tree = parseWxml(SAMPLE_WXML)
    assignRefs(tree)
    const { matches } = findNodes(tree, '手机号')
    const output = renderFindMatches(matches)
    expect(output).toContain('%2 [input] placeholder="手机号"')
  })
})

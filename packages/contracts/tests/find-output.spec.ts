import { describe, expect, it } from 'vitest'
import { formatFindOutput } from '../src/find-output.js'

describe('rust daemon find output', () => {
  it('matches ts header and empty result text', () => {
    const output = formatFindOutput({
      pagePath: 'pages/home/index',
      query: 'missing',
      matches: [],
      shownCount: 0,
      totalCount: 0,
    })

    expect(output).toContain('页面: pages/home/index')
    expect(output).toContain('查找: "missing"')
    expect(output).toContain('未找到匹配项')
  })

  it('adds truncation suffix when total exceeds shown count', () => {
    const output = formatFindOutput({
      pagePath: 'pages/home/index',
      query: '余额',
      matches: ['%1 [button].BalanceBtn "余额"'],
      shownCount: 1,
      totalCount: 3,
    })

    expect(output).toContain('查找: "余额" [显示 1/3 条]')
    expect(output).toContain('%1 [button].BalanceBtn "余额"')
  })
})

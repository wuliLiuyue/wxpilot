import { describe, expect, it } from 'vitest'
import { formatFindOutput } from '../../rust-bindings/find-output.js'

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
})

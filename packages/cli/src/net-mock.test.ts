import { describe, expect, it } from 'vitest'

import { buildNetMockParams } from './net-mock.js'

describe('buildNetMockParams', () => {
  it('includes explicit http method for net.mock requests', () => {
    expect(
      buildNetMockParams('https://testapi.casamiel.cn/api/v2/WaiMai/Product/GetProductList', '/tmp/mock.json', { method: 'post' }),
    ).toEqual({
      url: 'https://testapi.casamiel.cn/api/v2/WaiMai/Product/GetProductList',
      jsonFile: '/tmp/mock.json',
      method: 'POST',
    })
  })
})

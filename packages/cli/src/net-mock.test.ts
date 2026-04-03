import { describe, expect, it } from 'vitest'

import { buildNetMockParams } from './net-mock.js'

describe('buildNetMockParams', () => {
  it('includes explicit http method for net.mock requests', () => {
    expect(
      buildNetMockParams('https://api.example.com/v1/products', '/tmp/mock.json', { method: 'post' }),
    ).toEqual({
      url: 'https://api.example.com/v1/products',
      jsonFile: '/tmp/mock.json',
      method: 'POST',
    })
  })
})

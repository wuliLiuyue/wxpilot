import { describe, expect, it } from 'vitest'
import { daemonContract } from '../../packages/daemon/src/__fixtures__/rpc-contract.js'

describe('cross implementation contract suite', () => {
  it('keeps ts and rust daemon method list identical', () => {
    const rustMethods = [...daemonContract.methods]
    expect(rustMethods).toEqual(daemonContract.methods)
  })
})

import { describe, expect, it } from 'vitest'
import { daemonContract } from '../src/daemon-contract.js'
import { rustPublicMethods } from '../src/rust-capabilities.js'

describe('cross implementation contract suite', () => {
  it('keeps rust daemon public method list aligned with frozen contract', () => {
    for (const method of ['state', 'shot', 'run', 'wx', 'net.log']) {
      expect(daemonContract.methods).toContain(method)
      expect(rustPublicMethods).toContain(method)
    }
  })
})

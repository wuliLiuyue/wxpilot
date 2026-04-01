import { describe, expect, it } from 'vitest'
import { daemonContract } from '../../packages/daemon/src/__fixtures__/rpc-contract.js'

describe('cross implementation contract suite', () => {
  it('keeps rust daemon public method list aligned with frozen contract', () => {
    expect(daemonContract.methods).toContain('status')
    expect(daemonContract.methods).toContain('view')
    expect(daemonContract.methods).toContain('find')
    expect(daemonContract.methods).toContain('wait')
    expect(daemonContract.methods).toContain('read')
    expect(daemonContract.methods).toContain('assert')
    expect(daemonContract.methods).toContain('goto')
    expect(daemonContract.methods).toContain('tap')
    expect(daemonContract.methods).toContain('net.log')
    expect(daemonContract.methods).toContain('daemon.stop')
  })
})

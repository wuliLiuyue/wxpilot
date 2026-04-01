import { describe, expect, it } from 'vitest'
import { daemonContract } from '../../packages/daemon/src/__fixtures__/rpc-contract.js'

describe('ts daemon contract fixture', () => {
  it('covers public rpc methods and internal probe methods', () => {
    expect(daemonContract.methods).toContain('status')
    expect(daemonContract.methods).toContain('find')
    expect(daemonContract.methods).toContain('net.log')
    expect(daemonContract.methods).toContain('health')
    expect(daemonContract.methods).toContain('version')
    expect(daemonContract.methods).toContain('capabilities')
  })

  it('freezes ref and connection error codes', () => {
    expect(daemonContract.errors.REF_EXPIRED).toBe(-32000)
    expect(daemonContract.errors.NOT_CONNECTED).toBe(-32001)
    expect(daemonContract.errors.TIMEOUT).toBe(-32002)
    expect(daemonContract.errors.NOT_FOUND).toBe(-32003)
    expect(daemonContract.errors.INTERNAL).toBe(-32099)
  })
})

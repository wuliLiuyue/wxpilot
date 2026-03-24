import { describe, it, expect, beforeEach } from 'vitest'
import { RefStore } from './ref-store.js'

describe('RefStore', () => {
  let store: RefStore

  beforeEach(() => {
    store = new RefStore()
  })

  it('assign 后可 resolve', () => {
    store.assign(new Map([[1, '.btn'], [2, 'input']]))
    expect(store.resolve(1)).toBe('.btn')
    expect(store.resolve(2)).toBe('input')
  })

  it('resolve 不存在的 ref 返回 null', () => {
    store.assign(new Map([[1, '.btn']]))
    expect(store.resolve(99)).toBeNull()
  })

  it('has 正确判断', () => {
    store.assign(new Map([[1, '.btn']]))
    expect(store.has(1)).toBe(true)
    expect(store.has(2)).toBe(false)
  })

  it('invalidateAll 后 resolve 返回 null', () => {
    store.assign(new Map([[1, '.btn'], [2, 'input']]))
    store.invalidateAll()
    expect(store.resolve(1)).toBeNull()
    expect(store.resolve(2)).toBeNull()
    expect(store.size()).toBe(0)
  })

  it('重复 assign 覆盖旧映射', () => {
    store.assign(new Map([[1, '.old']]))
    store.assign(new Map([[1, '.new'], [2, 'input']]))
    expect(store.resolve(1)).toBe('.new')
    expect(store.size()).toBe(2)
  })
})

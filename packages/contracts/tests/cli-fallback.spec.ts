import { describe, expect, it, vi } from 'vitest'
import { connectToDaemon, stopDaemon } from '../../cli/src/client.js'

describe('cli fallback', () => {
  it('仅连接指定的 rust daemon，不自动回退到 ts daemon', async () => {
    const connect = vi
      .fn()
      .mockRejectedValueOnce(new Error('ECONNREFUSED'))
      .mockRejectedValueOnce(new Error('still booting'))
      .mockResolvedValueOnce({ socketPath: '/tmp/rust.sock', daemonKind: 'rust' })

    const spawnRustDaemon = vi.fn()
    const spawnTsDaemon = vi.fn()
    const wait = vi.fn().mockResolvedValue(undefined)

    const result = await connectToDaemon('rust', {
      connect,
      spawnRustDaemon,
      spawnTsDaemon,
      wait,
    })

    expect(result.daemonKind).toBe('rust')
    expect(connect).toHaveBeenCalledTimes(3)
    expect(connect).toHaveBeenNthCalledWith(1, 'rust')
    expect(connect).toHaveBeenNthCalledWith(2, 'rust')
    expect(connect).toHaveBeenNthCalledWith(3, 'rust')
    expect(spawnRustDaemon).toHaveBeenCalledTimes(1)
    expect(spawnTsDaemon).not.toHaveBeenCalled()
    expect(wait).toHaveBeenCalledTimes(1)
  })

  it('显式指定 ts 时只连接 ts daemon', async () => {
    const connect = vi
      .fn()
      .mockResolvedValueOnce({ socketPath: '/tmp/ts.sock', daemonKind: 'ts' })

    const spawnRustDaemon = vi.fn()
    const spawnTsDaemon = vi.fn()
    const wait = vi.fn().mockResolvedValue(undefined)

    const result = await connectToDaemon('ts', {
      connect,
      spawnRustDaemon,
      spawnTsDaemon,
      wait,
    })

    expect(result.daemonKind).toBe('ts')
    expect(connect).toHaveBeenCalledTimes(1)
    expect(connect).toHaveBeenNthCalledWith(1, 'ts')
    expect(spawnRustDaemon).not.toHaveBeenCalled()
    expect(spawnTsDaemon).not.toHaveBeenCalled()
    expect(wait).not.toHaveBeenCalled()
  })

  it('daemon stop 在 socket 丢失时会按进程名兜底清理', async () => {
    const rpcStop = vi.fn().mockRejectedValue(new Error('timeout after 10000ms'))
    const probe = vi.fn().mockResolvedValue(false)
    const readPidFile = vi.fn().mockReturnValue(null)
    const listProcessIds = vi.fn().mockReturnValue([1234, 5678])
    const killPid = vi.fn()
    const removeRuntimeFile = vi.fn()
    const wait = vi.fn().mockResolvedValue(undefined)

    const result = await stopDaemon('rust', {
      rpcStop,
      probe,
      readPidFile,
      listProcessIds,
      killPid,
      removeRuntimeFile,
      wait,
    })

    expect(result.forced).toBe(true)
    expect(rpcStop).toHaveBeenCalledTimes(1)
    expect(probe).toHaveBeenCalledWith('rust')
    expect(readPidFile).toHaveBeenCalledWith('rust')
    expect(listProcessIds).toHaveBeenCalledWith('rust')
    expect(killPid).toHaveBeenCalledTimes(2)
    expect(killPid).toHaveBeenNthCalledWith(1, 1234)
    expect(killPid).toHaveBeenNthCalledWith(2, 5678)
    expect(removeRuntimeFile).toHaveBeenCalledTimes(2)
    expect(wait).toHaveBeenCalled()
  })

  it('daemon stop 正常通过 RPC 停止时不应误报 forced', async () => {
    const rpcStop = vi.fn().mockResolvedValue(undefined)
    const probe = vi.fn().mockResolvedValue(false)
    const readPidFile = vi.fn().mockReturnValue(4321)
    const listProcessIds = vi.fn().mockReturnValue([])
    const killPid = vi.fn(() => {
      throw new Error('ESRCH')
    })
    const removeRuntimeFile = vi.fn()
    const wait = vi.fn().mockResolvedValue(undefined)

    const result = await stopDaemon('rust', {
      rpcStop,
      probe,
      readPidFile,
      listProcessIds,
      killPid,
      removeRuntimeFile,
      wait,
    })

    expect(result.forced).toBe(false)
    expect(rpcStop).toHaveBeenCalledTimes(1)
    expect(killPid).toHaveBeenCalledTimes(1)
    expect(killPid).toHaveBeenCalledWith(4321)
    expect(listProcessIds).toHaveBeenCalledWith('rust')
    expect(removeRuntimeFile).toHaveBeenCalledTimes(2)
  })
})

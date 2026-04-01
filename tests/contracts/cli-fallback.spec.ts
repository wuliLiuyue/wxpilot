import { describe, expect, it, vi } from 'vitest'
import { connectWithFallback } from '../../packages/cli/src/client.js'

describe('cli fallback', () => {
  it('prefers rust daemon and falls back to ts daemon when rust probe fails', async () => {
    const connect = vi
      .fn()
      .mockRejectedValueOnce(new Error('ECONNREFUSED'))
      .mockResolvedValueOnce({ socketPath: '/tmp/ts.sock', daemonKind: 'ts' })

    const spawn = vi.fn()

    const result = await connectWithFallback({
      connect,
      spawnRustDaemon: spawn,
      spawnTsDaemon: spawn,
    })

    expect(result.daemonKind).toBe('ts')
    expect(connect).toHaveBeenCalledTimes(2)
  })
})

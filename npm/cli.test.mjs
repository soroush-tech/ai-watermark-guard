// The wrapper's one observable behavior in a checkout with no binary packages installed: a clear
// message instead of a bare MODULE_NOT_FOUND, and this tool's "run failed" exit code.
import { spawnSync } from 'node:child_process'
import { join } from 'node:path'
import { expect, it } from 'vitest'

it('explains itself and exits 2 when no prebuilt binary matches', () => {
  const cli = join(import.meta.dirname, 'ai-watermark-guard', 'bin', 'cli.js')
  const result = spawnSync(process.execPath, [cli], { encoding: 'utf8' })
  expect(result.status).toBe(2)
  expect(result.stderr).toContain('no prebuilt binary')
  expect(result.stderr).toContain('cargo install ai-watermark-guard')
})

#!/usr/bin/env node
// Finds the prebuilt binary for this machine and runs it.
//
// The binaries ship as optionalDependencies, one package per platform, each declaring `os`, `cpu`
// and - on Linux - `libc`. The package manager installs the one that matches and skips the rest,
// so an install pulls a couple of megabytes rather than every target. Nothing is compiled and no
// install script runs, which is what makes `npx ai-watermark-guard` work from cold and keeps the
// tool usable in a CI that installs with --ignore-scripts.
'use strict'

const { spawnSync } = require('node:child_process')

const SCOPE = '@soroush.tech/ai-watermark-guard'
const { platform, arch } = process
const binary = platform === 'win32' ? 'aiwg.exe' : 'aiwg'

// glibc first, musl second. On Alpine the glibc package is not installed at all - the `libc` field
// tells the package manager to skip it - so the resolve below falls through to the musl build.
const candidates = [`${SCOPE}-${platform}-${arch}`]
if (platform === 'linux') candidates.push(`${SCOPE}-${platform}-${arch}-musl`)

let executable = null
for (const candidate of candidates) {
  try {
    executable = require.resolve(`${candidate}/bin/${binary}`)
    break
  } catch {
    // Try the next candidate. A miss here is ordinary: only one of them is ever installed.
  }
}

if (executable === null) {
  // The optionalDependencies skip is silent by design, so without this the user would get a bare
  // MODULE_NOT_FOUND naming a package they have never heard of.
  console.error(
    `ai-watermark-guard: no prebuilt binary for ${platform}-${arch}.\n` +
      `Supported: windows x64/arm64, macOS x64/arm64, Linux x64/arm64 (glibc) and x64 (musl).\n` +
      `Build from source with: cargo install ai-watermark-guard`
  )
  process.exit(2)
}

const result = spawnSync(executable, process.argv.slice(2), { stdio: 'inherit' })

// A binary killed by a signal reports no exit code. 2 is this tool's "the run itself failed".
process.exit(result.status === null ? 2 : result.status)

// Assembles the npm packages from binaries built by CI.
//
//   node npm/build.mjs <artifacts-dir> <version>
//
// <artifacts-dir> holds one directory per rust target, each containing the built binary. The
// output is npm/dist/: one package per platform, plus the wrapper, ready to publish in that order.
import { chmodSync, cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const root = join(here, '..')

/**
 * Every target published. `libc` is what lets a package manager tell an Alpine machine from a
 * Debian one; without it, a glibc binary installs on musl and dies at exec time.
 */
export const TARGETS = [
  { rust: 'x86_64-pc-windows-msvc', suffix: 'win32-x64', os: 'win32', cpu: 'x64' },
  { rust: 'aarch64-pc-windows-msvc', suffix: 'win32-arm64', os: 'win32', cpu: 'arm64' },
  { rust: 'x86_64-apple-darwin', suffix: 'darwin-x64', os: 'darwin', cpu: 'x64' },
  { rust: 'aarch64-apple-darwin', suffix: 'darwin-arm64', os: 'darwin', cpu: 'arm64' },
  { rust: 'x86_64-unknown-linux-gnu', suffix: 'linux-x64', os: 'linux', cpu: 'x64', libc: 'glibc' },
  {
    rust: 'aarch64-unknown-linux-gnu',
    suffix: 'linux-arm64',
    os: 'linux',
    cpu: 'arm64',
    libc: 'glibc',
  },
  {
    rust: 'x86_64-unknown-linux-musl',
    suffix: 'linux-x64-musl',
    os: 'linux',
    cpu: 'x64',
    libc: 'musl',
  },
]

const [artifacts, version] = process.argv.slice(2)
if (!artifacts || !version) {
  console.error('usage: node npm/build.mjs <artifacts-dir> <version>')
  process.exit(2)
}

const dist = join(here, 'dist')
rmSync(dist, { recursive: true, force: true })

for (const target of TARGETS) {
  const name = `@soroush.tech/ai-watermark-guard-${target.suffix}`
  const binary = target.os === 'win32' ? 'aiwg.exe' : 'aiwg'
  const out = join(dist, target.suffix)
  mkdirSync(join(out, 'bin'), { recursive: true })

  cpSync(join(artifacts, target.rust, binary), join(out, 'bin', binary))
  // npm preserves the mode from the tarball, and a binary without the executable bit is an EACCES
  // on every machine that installs it. Windows does not care; setting it anyway costs nothing.
  chmodSync(join(out, 'bin', binary), 0o755)

  writeFileSync(
    join(out, 'package.json'),
    `${JSON.stringify(
      {
        name,
        version,
        description: `The ${target.suffix} binary for ai-watermark-guard.`,
        repository: {
          type: 'git',
          url: 'git+https://github.com/soroush-tech/ai-watermark-guard.git',
        },
        license: 'MIT',
        author: 'Masoud Soroush <masoud@soroush.tech>',
        os: [target.os],
        cpu: [target.cpu],
        ...(target.libc ? { libc: [target.libc] } : {}),
        files: ['bin'],
      },
      null,
      2
    )}\n`
  )
  cpSync(join(root, 'LICENSE'), join(out, 'LICENSE'))
  console.log(`built ${name}`)
}

// The wrapper last, with every optional dependency pinned to this same version - a wrapper that
// floats would resolve a binary from a different build.
const wrapper = join(dist, 'ai-watermark-guard')
mkdirSync(wrapper, { recursive: true })
cpSync(join(here, 'ai-watermark-guard'), wrapper, { recursive: true })
cpSync(join(root, 'README.md'), join(wrapper, 'README.md'))
cpSync(join(root, 'LICENSE'), join(wrapper, 'LICENSE'))

const manifest = JSON.parse(readFileSync(join(wrapper, 'package.json'), 'utf8'))
manifest.version = version
manifest.optionalDependencies = Object.fromEntries(
  TARGETS.map((target) => [`@soroush.tech/ai-watermark-guard-${target.suffix}`, version])
)
writeFileSync(join(wrapper, 'package.json'), `${JSON.stringify(manifest, null, 2)}\n`)
console.log(`built ai-watermark-guard@${version}`)

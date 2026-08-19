---
description: How to write and cut a release of ai-watermark-guard, published by cd-publish.yml - notes live in release-notes/<version>.md, the version comes from Cargo.toml, the semver bump rule, a linked PR/issue reference as an absolute URL (waived for maintenance-only releases, which instead list every bump), and the breaking-change and packaging sections. Use when releasing or drafting release notes.
paths: release-notes/**
---

# Release notes

Publishing is **manual** (Actions dispatch, never a tag or a push), and notes live in a
**versioned file committed to the repo**: `release-notes/<version>.md`, where `<version>`
matches `version` in `Cargo.toml` - the single source of truth; the npm manifests get it
injected at build time by `npm/build.mjs`. The workflow refuses to publish without that file,
so a release can never ship with empty notes. One version covers the crate and all eight npm
packages (the `ai-watermark-guard` wrapper plus the seven `@soroush.tech/ai-watermark-guard-*`
platform packages) - they always ship together, pinned to each other.

The file is plain markdown with **ASCII punctuation only**: it is a tracked file, so CI's
self job (`aiwg --all`) scans it like everything else. Notes never ship to npm - the wrapper
package's `files` allowlist excludes them.

Every file **starts with a `## ai-watermark-guard@<version>` heading** - the only title the
file carries (the GitHub Release shows its `v<version>` tag separately). The directory is the
full per-version history: one file per released version.

**Repo links** are absolute:
`https://github.com/soroush-tech/ai-watermark-guard/blob/main/...` for a file,
`tree/main/...` for a directory.

## Before you write notes: bump the version (semver)

A release is bumping `version` in `Cargo.toml` on `main` **and** adding the matching
`release-notes/<version>.md`, in the same PR. The publish step skips a version already on
npm, so the **version number is the release**. Follow [semver](https://semver.org) - the
bump decides which sections the notes need.

| Bump      | `x.y.z` is | Use for                                                                  |
| --------- | ---------- | ------------------------------------------------------------------------ |
| **PATCH** | `x.y.Z+1`  | Backward-compatible bug fixes; dependency/maintenance-only changes       |
| **MINOR** | `x.Y+1.0`  | Backward-compatible **new features / new flags or config** (patch to 0)  |
| **MAJOR** | `X+1.0.0`  | **Any change that breaks backward compatibility** (reset minor+patch)    |

> Breaking a **platform or toolchain contract** is a breaking change even when no code
> changed: raising the MSRV, dropping a prebuilt platform package, or raising the wrapper's
> Node `engines` floor - bump **MAJOR**. A backward-compatible dependency bump is PATCH.

## Required contents

Every release body **must** have:

1. **A PR or issue reference, as a full link** -
   `[#<number>](https://github.com/soroush-tech/ai-watermark-guard/issues/<number>)`
   somewhere in the notes (lead line or a bullet). Ties a **feature or fix** release to its
   change history. Required for any release that changes behavior or CLI surface.

   Write the **absolute URL**, never a bare `#<number>`. A bare reference only autolinks in
   GitHub's issue/PR/Release UI - these files are also browsed in-repo at `blob/main/...`,
   where it renders as plain text. (Note this is the opposite of the
   [`github-issues`](../github-issues/SKILL.md) rule: inside an _issue or PR body_, reference
   issues bare so GitHub renders the title.)

   **Exception:** a maintenance-only release (a PATCH that only refreshes dependencies or
   toolchain) may have no owning issue - the reference is **not required** there; instead
   **list exactly what was bumped** (see rule 4).

2. **Breaking changes**, if any - a `### BREAKING CHANGES` section spelling out what broke
   and the migration. Its presence means the bump must be MAJOR.
3. **New CLI or config surface**, if any - name each new flag, mode, or
   `.ai-watermark-guard.toml` key **and link its README section** with an absolute URL:
   `https://github.com/soroush-tech/ai-watermark-guard/blob/main/README.md#<anchor>`
   (the README is the doc; there is no separate docs directory).
4. **A packaging side note** - a `### Packaging` section for packaging-level changes:
   platform packages added or dropped, MSRV, the wrapper's `engines.node`, dependency floors.
   For a maintenance-only release this section **is** the release notes - **name every bump
   with its `old -> new` version**, don't just say "dependency bumps". Omit the section only
   when there were genuinely no packaging changes.

## Template

`release-notes/<version>.md`:

```markdown
## ai-watermark-guard@<version>

<one-line summary of what changed and why>
([#<number>](https://github.com/soroush-tech/ai-watermark-guard/issues/<number>))

### Added

- **`--new-flag`** - one line on what it does.
  [docs](https://github.com/soroush-tech/ai-watermark-guard/blob/main/README.md#<anchor>)

### Changed

- <backward-compatible behavior change>.

### Fixed

- <bug fix> ([#<number>](https://github.com/soroush-tech/ai-watermark-guard/issues/<number>)).

### BREAKING CHANGES

- <what broke> - <how to migrate>.

### Packaging

- Raise MSRV `1.74` -> `<new>`.
- Add the `<target>` platform package.
```

Include only the sections that apply. Keep a linked issue reference for any feature/fix
release; keep the `### Packaging` note whenever packaging changed. Match the tone of the
previous `release-notes/*.md` - `0.1.0.md` is the reference.

## Cutting the release

1. In **one PR to `main`** (CI must pass): bump `version` in `Cargo.toml` **and** add
   `release-notes/<version>.md`. The pre-commit hook does not check the notes file - the
   workflow's Resolve step fails without it, and its smoke test fails if the built binary's
   `--version` disagrees with `Cargo.toml`. Keep the filename equal to the new version by hand.
2. Dispatch - Actions, **CD - Publish (npm)**, Run workflow on `main` (the publish job
   refuses any other ref). A plain dispatch publishes; check `dry_run` to rehearse first -
   it builds + packs everything without touching a registry.
3. The job publishes the seven platform packages first, then the wrapper, then cuts a GitHub
   Release tagged `v<version>` from the notes file. A version already on npm is skipped, and
   a rerun repairs a missing GitHub Release without republishing.

> This machine can't `git push` and has no `gh` CLI - commit the version bump + notes file
> locally, then ask the user to push and to run the dispatch from the Actions tab.

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Behavioral guidelines

### 1. Think before coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.

### 2. Propose before implementing

**Always present a plan and wait for confirmation before writing code.**

Cover: which files change, what changes in each, why. Then wait for explicit approval.

Exception: self-evident one-liners (typo fix, missing import).

### 3. Simplicity first

**Minimum code that solves the problem. Nothing speculative.**

- No features, abstractions, or error handling beyond what was asked.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

### 4. Surgical changes

**Touch only what you must. Clean up only your own mess.**

- Don't refactor, reformat, or "improve" adjacent code.
- Remove imports/variables/functions that YOUR changes made unused.
- Match existing style.
- If you notice unrelated dead code, mention it - don't delete it.

### 5. Goal-driven execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals: "Fix the bug" -> "Write a test that reproduces it, then make it pass."

- "Refactor X" -> "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:

```
1. [Step] -> verify: [check]
2. [Step] -> verify: [check]
3. [Step] -> verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

### 6. Test coverage after implementation

**After any implementation, add tests for the new logic, run `cargo test`, and check coverage of touched files with `cargo llvm-cov`.**

- Coverage gaps in touched files should be deliberate (e.g. `main`, trivial glue), not overlooked. Mention any gaps you leave.
- Requires a one-time install: `cargo install cargo-llvm-cov`.

## Skills

`.claude/skills/` holds this repo's coding-standard skills. Load them with the Skill tool when they apply: `project-structure` when creating files, extracting helpers, or placing tests; `rust-idioms` when writing or reviewing any Rust; `rust-testing` when writing tests; and `rust-error-handling`, `rust-linting`, `rust-docs-comments`, `rust-performance`, `rust-pointers`, `rust-dispatch`, `rust-type-state` per their descriptions.

## Project

ai-watermark-guard - a CLI (`aiwg`) that finds the characters marking text as machine-written (em/en dashes, curly quotes, ellipsis), plus invisible characters and mojibake, across a repo's tracked files and commit messages. `--fix` rewrites prose files (`.md`, `.txt`, `.rst`, ...) and commit-message files only; code is reported and deliberately left alone. Ships to npm as prebuilt per-platform binaries.

Single binary crate, no workspace. Edition 2021, MSRV 1.74 - both are deliberate so `cargo install` works on older stable toolchains; don't raise them casually. Runtime dependencies: git on PATH, nothing else. Never touches the network; git is invoked via `Command` argument lists, never through a shell.

## Commands

```
cargo test --all-targets                              # unit tests + end-to-end (tests/cli.rs)
cargo test <name>                                     # single test by name filter
cargo clippy --all-targets -- --deny warnings         # lint, exactly as CI runs it
cargo fmt --check                                     # format gate
cargo build --release && ./target/release/aiwg --all  # the CI "self" job locally
cargo llvm-cov                                        # coverage (rule 6)
node npm/build.mjs <artifacts-dir> <version>          # assemble npm packages (normally CI-only)
```

## The self-guard constraint (critical)

CI's `self` job runs the built binary against this repository (`aiwg --all`), with all tiers on. So no tracked file - source, tests, docs, this file, the skills - may contain a banned character: em/en dash, minus sign, curly quotes, ellipsis, no-break spaces, or any invisible from the table in `src/rules.rs`. Write ASCII punctuation everywhere. Test fixtures build banned characters from code points (`char::from_u32`), never as literals - `tests/cli.rs` shows the pattern.

## Architecture

One crate, five modules. `src/main.rs` parses the CLI (clap, bin name `aiwg`), resolves the mode in order (`--message`, `--messages`, then explicit paths / `--all` / `--staged` / `--since` / default merge-base diff against `--branch`), scans files in parallel with rayon, prints findings (capped at `MAX_SHOWN`), and exits 0 clean / 1 findings / 2 run failed.

- `src/rules.rs` - the core: the banned-character table with tiers (`invisible`, `punctuation`, `mojibake`) and per-character replacements, the scan, and the fix. `ALWAYS_ALLOWED` (ZWNJ, ZWJ, LRM, RLM) is never flagged whatever the tiers - Persian, emoji sequences, and bidi text need them. Mojibake is detected but never fixed: the original bytes cannot be recovered.
- `src/files.rs` - reading files, git-style NUL binary detection, strict UTF-8 (invalid files are reported and skipped, not lossily decoded).
- `src/git.rs` - all git plumbing: tracked/staged/changed file lists, merge-base, commit-message ranges, re-staging after `--staged --fix` (partially staged files are fixed but left unstaged, non-zero exit).
- `src/config.rs` - `.ai-watermark-guard.toml`: `rules` and `exclude` globs (repo-relative, forward slashes). CLI `--rules` wins over the file.
- `tests/cli.rs` - end-to-end tests of the built binary (`CARGO_BIN_EXE_aiwg`), one temp dir per test. Unit tests sit next to their module: sibling `_test.rs` files wired with `#[path]` (`rules_test.rs`, `config_test.rs`, `files_test.rs`), or an inline `#[cfg(test)] mod <name>_test` while the block still fits a screen (`git.rs`, `main.rs`). Either way the mod name ends in `_test`, so `cargo test _test` selects the whole unit tier.
- `npm/` - publishing glue, no logic: `npm/ai-watermark-guard` is the wrapper package (`bin/cli.js` execs the platform binary resolved from `optionalDependencies`, one prebuilt package per platform); `npm/build.mjs` assembles the per-target packages into gitignored `npm/dist`. The platform list must stay in sync across `build.mjs` `TARGETS`, the wrapper `package.json` `optionalDependencies`, and the matrix in `release.yml`.
- `.github/workflows/` - `ci.yml`: fmt, clippy `-D warnings`, tests on the three OSes, plus the `self` job above. `release.yml`: tag-driven (`v*`) - build every target, assemble the npm packages, publish; `workflow_dispatch` gives a dry run.

## Environment quirks

- Smart App Control is ON and sometimes blocks freshly compiled build scripts (`os error 4551`) on first execution. It's transient here: re-run the cargo command.
- Release binaries for other platforms are CI-built only; don't cross-compile locally.

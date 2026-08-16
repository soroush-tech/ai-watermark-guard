---
description: Folder structure and logic-split conventions for this single-crate Rust CLI - module layout, co-location doctrine, one-file-per-helper extraction, the test tiers (inline mod tests / sibling _test.rs and _spec.rs / tests/ end-to-end), and the npm publishing wrapper. Use when creating files, deciding where code lives, extracting helpers, or placing tests.
---

## Repo layout

```
repo/
├── Cargo.toml            # one binary crate, no workspace ([[bin]] aiwg)
├── src/
│   ├── main.rs           # CLI parsing, mode resolution, orchestration
│   ├── rules.rs          # banned-character tables + scan + fixes
│   ├── files.rs          # reading, binary/UTF-8 detection
│   ├── git.rs            # all git plumbing (Command, never a shell)
│   └── config.rs         # .ai-watermark-guard.toml
├── tests/                # black-box end-to-end runs of the built binary only
│   └── cli.rs
├── npm/                  # publishing wrapper - JS glue, no logic
│   ├── ai-watermark-guard/   # the wrapper package (bin/cli.js)
│   └── build.mjs             # assembles per-platform packages into npm/dist (gitignored)
└── .github/workflows/    # ci.yml (incl. the self-guard job), release.yml
```

- All logic is Rust. Nothing under `npm/` makes a decision the Rust side could
  make; `npm/dist` is generated and never committed.
- ASCII punctuation in every tracked file: CI runs the built guard against this
  repository, so source, docs, and skills must pass the tool's own rules.
  Banned characters in test fixtures are built from code points
  (`char::from_u32`), never written as themselves.

## Co-location doctrine

**Everything a unit of code needs sits next to it: tests, helpers, constants,
static data.** Distance is earned by reuse, not by category.

The promotion rule: code stays in the module that owns it until a **second**
consumer appears. Then it moves to its own module (`src/<name>.rs`), declared
in `main.rs`. Never create a `utils.rs` / `helpers.rs` dumping ground
speculatively.

## Splitting logic out of a module

When a module grows mixed concerns, extract by kind:

| Concern              | Where                                                    |
| -------------------- | -------------------------------------------------------- |
| Pure helpers (few)   | keep in the module, next to their callers                |
| Pure helpers (many)  | `src/<module>/<helper_name>.rs`, one public item per file |
| Constants            | top of the owning module; `src/<module>/consts.rs` once many |
| Static data / tables | in the owning module (the tables in `rules.rs` are the model) |

- **One file per helper** once there's more than a couple: each helper gets its
  own file named after it, with its tests co-located. No grab-bag modules that
  grow forever.
- Module folders use the modern layout: `name.rs` + `name/` subfolder, not
  `name/mod.rs`.

## Test tiers

| Tier        | Where                                                        | Access discipline              |
| ----------- | ------------------------------------------------------------ | ------------------------------ |
| Unit        | inline `#[cfg(test)] mod tests`; sibling `foo_test.rs` once past a screen | internals allowed (`super::*`) |
| Integration | sibling `foo_spec.rs`                                        | public paths only (`crate::...` style) |
| End-to-end  | `tests/cli.rs` - the built binary via `CARGO_BIN_EXE_aiwg`, one temp dir per test | the CLI surface only           |

Sibling test files don't match the default module layout, so declare them with
`#[path]` under `#[cfg(test)]`:

```rust
// src/rules.rs
#[cfg(test)]
#[path = "rules_test.rs"]
mod rules_test;
```

- `rules_test.rs`, `config_test.rs`, and `files_test.rs` are the sibling
  files today; the tiny blocks in `git.rs` and `main.rs` stay inline. Extract
  to a sibling `_test.rs` when a block outgrows a screen, and add the
  `#[path]` declaration at the same time - an undeclared file is silently
  ignored by Cargo.
- Name inline test mods `<name>_test` too (never plain `tests`), so the
  `cargo test _test` tier filter matches every unit test regardless of where
  it lives.
- `tests/` at the crate root is reserved for true end-to-end runs of the
  binary. Don't put there what an inline or sibling test can cover.
- Filter: `cargo test <name>`; end-to-end only: `cargo test --test cli`.

## Coverage

`cargo llvm-cov` after any implementation (CLAUDE.md rule 6). Gaps in touched
files must be deliberate (`main`'s glue, process-exit paths) and mentioned,
not overlooked.

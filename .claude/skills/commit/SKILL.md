---
description: How to write git commits in this repo that auto-close their GitHub task - a conventional-commits header with the closing keyword appended to the end of the subject line. Use when committing work that resolves a task or issue.
---

# Commit

## Conventional Commits format

Every commit follows the [Conventional Commits](https://www.conventionalcommits.org)
structure:

```text
<type>[optional scope]: <description>

[optional body]

[optional footer]
```

### Types - use only these

`fix`, `feat`, `chore`, `docs`, `style`, `refactor`, `perf`, `test`.

| Type       | Use for                                                 | SemVer |
| ---------- | ------------------------------------------------------- | ------ |
| `feat`     | A new feature                                           | MINOR  |
| `fix`      | A bug fix                                               | PATCH  |
| `docs`     | Documentation only                                      | -      |
| `style`    | Formatting, whitespace - no code-behavior change        | -      |
| `refactor` | Code change that neither fixes a bug nor adds a feature | -      |
| `perf`     | Performance improvement                                 | -      |
| `test`     | Adding or correcting tests                              | -      |
| `chore`    | Maintenance that fits no other type                     | -      |

No other types. Things like CI, build, or deps are expressed as a **scope** on one
of these types, never as the type itself.

### Scope

A scope in parentheses adds context: `feat(parser): add ability to parse arrays`.
Use a scope for the area touched - e.g. CI/CD changes are `fix(CI):` /
`chore(CI):`, **not** `ci:`.

### Breaking changes

Put `BREAKING CHANGE:` at the start of the body or footer (correlates with MAJOR).
Allowed on any type. May also be flagged with `!` before the colon:
`feat(api)!: drop v1 routes`.

### Examples

```text
feat: allow provided config object to extend other configs

BREAKING CHANGE: `extends` key in config file is now used for extending other config files
```

```text
docs: correct spelling of CHANGELOG
```

```text
chore(CI): gate lint, packages, and worker jobs behind the CI environment review
```

## One commit per task; closing keyword at the end of the header

A commit that completes a GitHub task ends its **subject line** (the header - the
first line) with the closing keyword and the task number:

```text
<type>(<scope>): <summary> - close #<tasknumber>
```

Example:

```text
feat(schema): extract contact validation into shared @soroush.tech/schema - close #155
```

- One task → one commit. Commit tasks separately, in dependency order.
- The keyword goes at the **end of the header**, never in a footer line.
- `close` / `closes` / `closed` all work - keep `#<number>` immediately after.
- Several tasks in one commit: `... - close #155, close #156`.

## Don't close issues by hand

The keyword auto-closes the task when the branch merges to `main`. Leave the
issue open; never close it manually.

## Mechanics

- Pass multi-line messages via a file (`git commit -F <file>`).
- The `.husky/pre-commit` hook (activated by `npm install`) runs clippy,
  `cargo test --all-targets`, oxlint, `oxfmt-quick --staged`, and the vitest
  suite before every commit. CI repeats those gates and adds
  `cargo fmt --check` and the self-guard (`aiwg --all`) on every PR.
- Write ASCII punctuation in the message itself. `aiwg --messages` scans commit
  messages, and this repo's own history should pass its own guard.
- Commits are GPG-signed and signing prompts the user; if a commit fails on the
  pinentry timeout, retry when the user says they are ready.

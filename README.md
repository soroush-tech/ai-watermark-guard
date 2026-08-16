# ai-watermark-guard

Finds the characters that mark text as machine-written, and the ones that should never have been
there at all.

```
$ npx ai-watermark-guard --all
docs/plan.md:12:34   U+2014 em dash - use -
src/Select.tsx:670:61  U+200B zero-width space - delete it
README.md:4:18       U+00E2 mojibake - UTF-8 read as Latin-1

aiwg: 3 finding(s) in 3 file(s); 0 fixed, 1604 scanned [invisible, punctuation, mojibake]
```

**What this is not:** there is no hidden watermark in machine-written text, and nothing here
defeats a detector. What it finds are ordinary characters - em dashes, curly quotes, ellipses -
that models reach for and people rarely type, alongside invisible characters and mojibake that no
one meant to commit. It is a text-hygiene guard. Judge it as one.

## Install

Nothing to set up. No toolchain, no compile step, no install script:

```sh
npx ai-watermark-guard --all          # one-off
npm install -D ai-watermark-guard     # pinned, for hooks and CI
cargo install ai-watermark-guard      # if you would rather build it
```

The npm package ships prebuilt binaries as optional dependencies, one per platform, so an install
fetches a couple of megabytes and runs. It works under npm, pnpm and yarn, and under
`--ignore-scripts`.

Supported: windows x64 and arm64, macOS x64 and arm64, Linux x64 and arm64 on glibc, Linux x64 on
musl. Anywhere else, `cargo install` builds it.

## What it looks for

Three tiers. All are on by default; `--rules` narrows them.

| tier          | characters                                                                  |
| ------------- | --------------------------------------------------------------------------- |
| `invisible`   | zero-width space, word joiner, byte order mark, soft hyphen, U+FFFD         |
| `punctuation` | en dash, em dash, minus sign, curly quotes, ellipsis, non-breaking spaces   |
| `mojibake`    | UTF-8 that was decoded as Latin-1 somewhere upstream                        |

`invisible` and `mojibake` are wrong anywhere. `punctuation` is a house rule - plain ASCII
punctuation survives every console code page, shell and editor - so turn it off if your prose
wants typographic quotes:

```sh
aiwg --all --rules invisible,mojibake
```

### Never flagged

`U+200C`, `U+200D`, `U+200E` and `U+200F` are invisible too, and this tool leaves them alone
whatever the rules say. Persian needs the first between the parts of a word, an emoji sequence
needs the second to hold itself together, and the last two set the direction of mixed
right-to-left text. Removing them corrupts text rather than cleaning it.

Letters are never touched, in any script. This is not an ASCII-only rule.

## Choosing what to scan

| invocation     | what it looks at                                            |
| -------------- | ----------------------------------------------------------- |
| `aiwg`         | files that differ from the merge-base with `--branch`       |
| `aiwg --all`   | every tracked text file                                     |
| `aiwg --staged`| what is staged, for a pre-commit hook                       |
| `aiwg ./docs`  | a path, with or without a repository                        |

The merge-base is used rather than the branch tip, so a branch that has fallen behind does not
drag in everything that landed on `main` meanwhile.

Binary files are skipped by the same NUL-byte test git uses. Files that are not valid UTF-8 are
reported and skipped rather than scanned through a lossy decode, which would blame the decoder.

## Fixing

`--fix` writes the plain equivalent: curly quotes become straight, dashes become hyphens, an
ellipsis becomes three periods, invisible characters are deleted.

**It only writes prose** - `.md`, `.markdown`, `.txt`, `.text`, `.rst`, `.adoc` - and commit
messages. Code is reported and left alone, deliberately. A curly apostrophe inside a
single-quoted string becomes a straight quote that ends the string, and the file stops parsing;
JSON, YAML and TOML values are quoted too. Fixing those safely needs a parser per language, which
this version does not have. Sweeping a codebase by hand, do it with tests running.

Mojibake is never fixed. What the original bytes were cannot be recovered from the damage, and
guessing would turn a visible problem into an invisible one.

## Commit messages

```sh
aiwg --message "$1"              # one message file, for a commit-msg hook
aiwg --messages main..HEAD       # a range, read-only, for CI
```

Git's own comment lines are ignored, since git strips them before the commit is made.

`--fix` works on a message file and refuses on a range. Rewriting a message makes a new commit
object: every sha from that point changes and every signature over them is void. Use
`git commit --amend`, or a rebase you drive yourself.

## Hooks

```sh
# .husky/pre-commit
aiwg --staged --fix

# .husky/commit-msg
aiwg --message "$1" --fix
```

Fixed files are re-staged, except a partially staged one: the rest of that file is deliberately
out of the commit, so it is fixed, left unstaged, and the run exits non-zero to tell you.

Hooks are local. A squash-merge typed in a web UI never runs them, which is what
`aiwg --messages` in CI is for.

## Flags

| flag              | effect                                                                |
| ----------------- | ---------------------------------------------------------------------- |
| `--all`           | every tracked text file                                                |
| `--staged`        | staged files only; re-stages what it fixes                             |
| `--since <rev>`   | compare against a revision instead of the merge-base                   |
| `--branch <name>` | merge-base target in the default mode (default `main`)                 |
| `--message <file>`| check one commit message file                                          |
| `--messages <range>` | check the messages in a range                                       |
| `--fix`           | write the plain equivalent (prose and messages only)                   |
| `--no-restage`    | with `--staged --fix`, leave staging to you                            |
| `--bail`          | exit non-zero even when everything found was fixed                     |
| `--rules <list>`  | `invisible`, `punctuation`, `mojibake`                                 |
| `--config <file>` | config path (default `.ai-watermark-guard.toml` at the repo root)      |
| `--verbose`       | name every file considered, and tag each finding with its tier         |

Exit codes: `0` clean, `1` findings, `2` the run itself failed.

## Config

```toml
# .ai-watermark-guard.toml
rules = ["invisible", "mojibake"]
exclude = ["**/fixtures/**", "vendor/**"]
```

`--rules` on the command line wins over the file. Exclude globs match repository-relative paths
with forward slashes, so one config reads the same on every platform.

## What it accesses

| what                | why                                                        |
| ------------------- | ----------------------------------------------------------- |
| `git`               | to list tracked, staged or changed files, and read messages |
| files you point it at | to read them, and to write them under `--fix`             |
| the network         | never                                                       |

No shell is invoked, so nothing user-supplied is ever interpreted as a command.

## Licence

MIT. See [LICENSE](LICENSE).

//! End-to-end runs of the built binary.
//!
//! Fixtures are built from code points rather than written as themselves: this crate is scanned by
//! its own guard, and a test file full of em dashes would fail that run.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXE: &str = env!("CARGO_BIN_EXE_aiwg");

fn character(point: u32) -> char {
    char::from_u32(point).expect("valid code point")
}

/// A directory of its own per test, so runs never see each other's fixtures.
fn workspace(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aiwg-test-{name}"));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).expect("create workspace");
    dir
}

fn run(args: &[&str]) -> Output {
    Command::new(EXE).args(args).output().expect("run aiwg")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(EXE)
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run aiwg")
}

/// Runs git in `dir` with a fixed identity and no user or system config, so a machine with
/// commit signing or hooks configured globally does not leak into the fixture.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", dir.join("no-global-config"))
        .env("GIT_CONFIG_SYSTEM", dir.join("no-system-config"))
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// A workspace that is also a repository on a branch named main.
fn repo(name: &str) -> PathBuf {
    let dir = workspace(name);
    git(&dir, &["init", "-q", "-b", "main"]);
    dir
}

#[test]
fn reports_findings_in_a_path_and_exits_one() {
    let dir = workspace("reports");
    let file = dir.join("notes.md");
    fs::write(&file, format!("a {} b\n", character(0x2014))).expect("write");

    let output = run(&[dir.to_str().expect("path")]);

    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("U+2014 em dash"), "{text}");
    assert!(text.contains("use -"), "{text}");
}

#[test]
fn is_silent_and_exits_zero_on_clean_text() {
    let dir = workspace("clean");
    fs::write(dir.join("notes.md"), "nothing but ascii here\n").expect("write");

    let output = run(&[dir.to_str().expect("path")]);

    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("clean"), "{}", stdout(&output));
}

#[test]
fn fixes_prose_in_place() {
    let dir = workspace("fix-prose");
    let file = dir.join("notes.md");
    let messy = format!(
        "{}quoted{} - it{}s {} and{}",
        character(0x201C),
        character(0x201D),
        character(0x2019),
        character(0x2014),
        character(0x2026)
    );
    fs::write(&file, &messy).expect("write");

    let output = run(&[dir.to_str().expect("path"), "--fix"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        "\"quoted\" - it's - and..."
    );
}

#[test]
fn reports_code_but_never_rewrites_it() {
    let dir = workspace("fix-code");
    let file = dir.join("thing.ts");
    // The exact hazard: a curly apostrophe inside a single-quoted string. Replacing it would end
    // the string early and leave a file that does not parse.
    let source = format!(
        "it('the run{}s own result', () => {{}})\n",
        character(0x2019)
    );
    fs::write(&file, &source).expect("write");

    let output = run(&[dir.to_str().expect("path"), "--fix"]);

    assert_eq!(output.status.code(), Some(1), "code findings survive --fix");
    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        source,
        "code was left untouched"
    );
}

#[test]
fn keeps_the_characters_that_carry_meaning() {
    let dir = workspace("allowed");
    let file = dir.join("notes.md");
    // Zero-width non-joiner, zero-width joiner, and both direction marks.
    let text: String = [0x200C, 0x200D, 0x200E, 0x200F]
        .iter()
        .map(|point| format!("a{}b", character(*point)))
        .collect();
    fs::write(&file, &text).expect("write");

    let output = run(&[dir.to_str().expect("path"), "--fix"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(fs::read_to_string(&file).expect("read"), text);
}

#[test]
fn honours_the_selected_tiers() {
    let dir = workspace("tiers");
    fs::write(dir.join("notes.md"), format!("a {} b", character(0x2014))).expect("write");

    let strict = run(&[dir.to_str().expect("path")]);
    assert_eq!(strict.status.code(), Some(1));

    let relaxed = run(&[dir.to_str().expect("path"), "--rules", "invisible,mojibake"]);
    assert_eq!(relaxed.status.code(), Some(0));
}

#[test]
fn checks_and_fixes_a_commit_message_file() {
    let dir = workspace("message");
    let file = dir.join("COMMIT_EDITMSG");
    let message = format!(
        "feat: a subject {}\n\n# Please enter the commit message {}\n",
        character(0x2014),
        character(0x2019)
    );
    fs::write(&file, &message).expect("write");

    let reported = run(&["--message", file.to_str().expect("path")]);
    assert_eq!(reported.status.code(), Some(1));

    let fixed = run(&["--message", file.to_str().expect("path"), "--fix"]);
    assert_eq!(fixed.status.code(), Some(0));
    assert!(fs::read_to_string(&file)
        .expect("read")
        .starts_with("feat: a subject -"));
}

#[test]
fn ignores_the_comment_lines_git_strips() {
    let dir = workspace("comments");
    let file = dir.join("COMMIT_EDITMSG");
    // The banned character is only in a comment, which never reaches the commit.
    fs::write(
        &file,
        format!("feat: plain subject\n\n# a note {}\n", character(0x2014)),
    )
    .expect("write");

    assert_eq!(
        run(&["--message", file.to_str().expect("path")])
            .status
            .code(),
        Some(0)
    );
}

#[test]
fn reports_mojibake_and_refuses_to_guess_at_it() {
    let dir = workspace("mojibake");
    let file = dir.join("notes.md");
    let damaged = format!("a {}{} b", character(0x00E2), character(0x20AC));
    fs::write(&file, &damaged).expect("write");

    let output = run(&[dir.to_str().expect("path"), "--fix"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("mojibake"), "{}", stdout(&output));
    assert_eq!(fs::read_to_string(&file).expect("read"), damaged);
}

#[test]
fn skips_binary_and_non_utf8_files() {
    let dir = workspace("binary");
    fs::write(dir.join("image.png"), [0x89, 0x50, 0x00, 0x4E]).expect("write");
    fs::write(dir.join("broken.txt"), [0xE2, 0x28, 0xA1]).expect("write");

    let output = run(&[dir.to_str().expect("path"), "--verbose"]);

    assert_eq!(output.status.code(), Some(0));
    let text = stdout(&output);
    assert!(text.contains("binary, skipped"), "{text}");
    assert!(text.contains("not UTF-8, skipped"), "{text}");
}

#[test]
fn never_walks_into_a_git_directory() {
    let dir = workspace("dotgit");
    // Dotfiles are scanned - .github and friends are text like any other - but git's own store
    // holds nothing a person wrote, and its loose objects are not UTF-8.
    fs::create_dir_all(dir.join(".git/objects")).expect("create .git");
    fs::write(
        dir.join(".git/objects/loose"),
        format!("a {} b", character(0x2014)),
    )
    .expect("write");
    fs::create_dir_all(dir.join(".github")).expect("create .github");
    fs::write(dir.join(".github/notes.md"), "plain\n").expect("write");

    let output = run(&[dir.to_str().expect("path"), "--verbose"]);

    assert_eq!(output.status.code(), Some(0));
    let text = stdout(&output);
    assert!(
        text.contains("notes.md"),
        "dotfiles are still scanned: {text}"
    );
    assert!(!text.contains("loose"), ".git was walked: {text}");
}

#[test]
fn bails_when_asked_even_though_it_fixed_everything() {
    let dir = workspace("bail");
    fs::write(dir.join("notes.md"), format!("a {} b", character(0x2014))).expect("write");

    let output = run(&[dir.to_str().expect("path"), "--fix", "--bail"]);

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn rejects_an_unknown_tier_and_a_missing_config() {
    let dir = workspace("bad-args");
    fs::write(dir.join("notes.md"), "plain").expect("write");
    let path = dir.to_str().expect("path");

    assert_eq!(run(&[path, "--rules", "typos"]).status.code(), Some(2));
    assert_eq!(
        run(&[path, "--config", "no-such-file.toml"]).status.code(),
        Some(2)
    );
}

#[test]
fn scans_every_tracked_file_with_all() {
    let dir = repo("all-mode");
    fs::write(
        dir.join("tracked.md"),
        format!("a {} b\n", character(0x2014)),
    )
    .expect("write");
    git(&dir, &["add", "tracked.md"]);
    git(&dir, &["commit", "-qm", "add tracked"]);
    fs::write(dir.join("stray.md"), format!("a {} b\n", character(0x2014))).expect("write");

    let output = run_in(&dir, &["--all"]);

    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("tracked.md"), "{text}");
    assert!(
        !text.contains("stray.md"),
        "untracked is not tracked: {text}"
    );
}

#[test]
fn scans_only_what_is_staged() {
    let dir = repo("staged-mode");
    fs::write(dir.join("committed.md"), "plain\n").expect("write");
    git(&dir, &["add", "committed.md"]);
    git(&dir, &["commit", "-qm", "start"]);
    fs::write(
        dir.join("staged.md"),
        format!("a {} b\n", character(0x2014)),
    )
    .expect("write");
    git(&dir, &["add", "staged.md"]);
    // A dirty file left unstaged is not part of the next commit and is not looked at.
    fs::write(
        dir.join("committed.md"),
        format!("a {} b\n", character(0x2014)),
    )
    .expect("write");

    let output = run_in(&dir, &["--staged"]);

    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("staged.md"), "{text}");
    assert!(!text.contains("committed.md"), "{text}");
}

#[test]
fn fixes_and_restages_a_staged_file() {
    let dir = repo("restage");
    fs::write(dir.join("notes.md"), format!("a {} b\n", character(0x2014))).expect("write");
    git(&dir, &["add", "notes.md"]);

    let output = run_in(&dir, &["--staged", "--fix"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(dir.join("notes.md")).expect("read"),
        "a - b\n"
    );
    // The fix went into the index too: index and working tree agree.
    assert_eq!(git(&dir, &["diff", "--name-only"]), "");
    assert!(git(&dir, &["diff", "--cached"]).contains("a - b"));
}

#[test]
fn leaves_a_fixed_partially_staged_file_unstaged() {
    let dir = repo("partial");
    fs::write(dir.join("notes.md"), "start\n").expect("write");
    git(&dir, &["add", "notes.md"]);
    git(&dir, &["commit", "-qm", "start"]);
    fs::write(
        dir.join("notes.md"),
        format!("start\na {} b\n", character(0x2014)),
    )
    .expect("write");
    git(&dir, &["add", "notes.md"]);
    fs::write(
        dir.join("notes.md"),
        format!(
            "start\na {} b\ntail kept out of this commit\n",
            character(0x2014)
        ),
    )
    .expect("write");

    let output = run_in(&dir, &["--staged", "--fix"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("partially staged"),
        "{}",
        stderr(&output)
    );
    // Fixed on disk, but the fix was not swept into the index.
    assert!(fs::read_to_string(dir.join("notes.md"))
        .expect("read")
        .contains("a - b"));
    assert!(git(&dir, &["diff", "--name-only"]).contains("notes.md"));
}

#[test]
fn leaves_restaging_to_the_caller_with_no_restage() {
    let dir = repo("no-restage");
    fs::write(dir.join("notes.md"), format!("a {} b\n", character(0x2014))).expect("write");
    git(&dir, &["add", "notes.md"]);

    let output = run_in(&dir, &["--staged", "--fix", "--no-restage"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(dir.join("notes.md")).expect("read"),
        "a - b\n"
    );
    assert!(
        git(&dir, &["diff", "--name-only"]).contains("notes.md"),
        "the fix stayed unstaged"
    );
}

#[test]
fn restages_nothing_when_nothing_was_fixed() {
    let dir = repo("restage-clean");
    fs::write(dir.join("notes.md"), "plain\n").expect("write");
    git(&dir, &["add", "notes.md"]);

    assert_eq!(run_in(&dir, &["--staged", "--fix"]).status.code(), Some(0));
}

#[test]
fn scans_what_changed_since_a_revision() {
    let dir = repo("since");
    fs::write(dir.join("old.md"), "plain\n").expect("write");
    git(&dir, &["add", "old.md"]);
    git(&dir, &["commit", "-qm", "one"]);
    fs::write(dir.join("new.md"), format!("a {} b\n", character(0x2014))).expect("write");
    git(&dir, &["add", "new.md"]);
    git(&dir, &["commit", "-qm", "two"]);

    let output = run_in(&dir, &["--since", "HEAD~1"]);
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("new.md"), "{text}");
    assert!(!text.contains("old.md"), "{text}");

    let bad = run_in(&dir, &["--since", "no-such-rev"]);
    assert_eq!(bad.status.code(), Some(2));
}

#[test]
fn diffs_against_the_merge_base_by_default() {
    let dir = repo("merge-base");
    fs::write(dir.join("base.md"), "plain\n").expect("write");
    git(&dir, &["add", "base.md"]);
    git(&dir, &["commit", "-qm", "base"]);
    git(&dir, &["switch", "-q", "-c", "feature"]);
    fs::write(
        dir.join("feature.md"),
        format!("a {} b\n", character(0x2014)),
    )
    .expect("write");
    git(&dir, &["add", "feature.md"]);
    git(&dir, &["commit", "-qm", "feature"]);

    let output = run_in(&dir, &[]);

    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("feature.md"), "{text}");
    assert!(!text.contains("base.md"), "{text}");
}

#[test]
fn errors_without_a_merge_base() {
    let dir = repo("no-base");
    fs::write(dir.join("notes.md"), "plain\n").expect("write");
    git(&dir, &["add", "notes.md"]);
    git(&dir, &["commit", "-qm", "start"]);

    let output = run_in(&dir, &["--branch", "missing"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("no merge-base with 'missing'"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn finds_no_merge_base_across_disjoint_histories() {
    let dir = repo("disjoint");
    fs::write(dir.join("a.md"), "plain\n").expect("write");
    git(&dir, &["add", "a.md"]);
    git(&dir, &["commit", "-qm", "on main"]);
    // An orphan branch shares no history with main, and git reports that silently: exit 1 with
    // nothing on stderr, the one git failure in this tool that comes without a message.
    git(&dir, &["switch", "-q", "--orphan", "other"]);
    fs::write(dir.join("b.md"), "plain\n").expect("write");
    git(&dir, &["add", "b.md"]);
    git(&dir, &["commit", "-qm", "on other"]);

    let output = run_in(&dir, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("no merge-base with 'main'"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn excludes_configured_globs_in_every_mode() {
    let dir = repo("excludes");
    fs::write(
        dir.join(".ai-watermark-guard.toml"),
        "exclude = [\"vendor/**\"]\n",
    )
    .expect("write");
    fs::create_dir_all(dir.join("vendor")).expect("create vendor");
    fs::write(
        dir.join("vendor/skip.md"),
        format!("a {} b\n", character(0x2014)),
    )
    .expect("write");
    fs::write(dir.join("notes.md"), format!("a {} b\n", character(0x2014))).expect("write");
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-qm", "start"]);

    let all = run_in(&dir, &["--all"]);
    assert_eq!(all.status.code(), Some(1));
    let text = stdout(&all);
    assert!(text.contains("notes.md"), "{text}");
    assert!(!text.contains("skip.md"), "{text}");

    // Explicit paths go through the same globs, relative to the repository root.
    let explicit = run_in(&dir, &[dir.to_str().expect("path")]);
    assert_eq!(explicit.status.code(), Some(1));
    let text = stdout(&explicit);
    assert!(text.contains("notes.md"), "{text}");
    assert!(!text.contains("skip.md"), "{text}");
}

#[test]
fn checks_a_range_of_commit_messages() {
    let dir = repo("messages");
    fs::write(dir.join("a.md"), "plain\n").expect("write");
    git(&dir, &["add", "a.md"]);
    git(&dir, &["commit", "-qm", "start"]);
    git(
        &dir,
        &["commit", "--allow-empty", "-qm", "chore: still clean"],
    );
    let messy = format!("feat: adds {} dash", character(0x2014));
    git(&dir, &["commit", "--allow-empty", "-qm", &messy]);

    // The range holds a clean commit and a dirty one; only the dirty one is printed.
    let dirty = run_in(&dir, &["--messages", "HEAD~2..HEAD"]);
    assert_eq!(dirty.status.code(), Some(1));
    let text = stdout(&dirty);
    assert!(text.contains("feat: adds"), "{text}");
    assert!(text.contains("U+2014"), "{text}");

    let clean = run_in(&dir, &["--messages", "HEAD~1..HEAD~1"]);
    assert_eq!(clean.status.code(), Some(0));
    assert!(stdout(&clean).contains("clean"), "{}", stdout(&clean));
}

#[test]
fn refuses_the_message_fix_even_when_unpushed_cannot_be_counted() {
    let dir = repo("messages-fix");
    fs::write(dir.join("a.md"), "plain\n").expect("write");
    git(&dir, &["add", "a.md"]);
    git(&dir, &["commit", "-qm", "start"]);

    // The range does not resolve, so the unpushed count is unavailable - the refusal stands, just
    // without the detail.
    let output = run_in(&dir, &["--messages", "no-such..range", "--fix"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("cannot rewrite commit messages."),
        "{}",
        stderr(&output)
    );
}

#[test]
fn labels_the_tier_in_verbose_mode() {
    let dir = workspace("verbose-tier");
    fs::write(dir.join("notes.md"), format!("a {} b\n", character(0x2014))).expect("write");

    let output = run(&[dir.to_str().expect("path"), "--verbose"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("[punctuation]"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn caps_the_listing_and_counts_the_rest() {
    let dir = workspace("cap");
    let many: String = (0..55)
        .map(|_| format!("{}\n", character(0x2014)))
        .collect();
    fs::write(dir.join("a.md"), &many).expect("write");
    fs::write(dir.join("b.md"), format!("{}\n", character(0x2014))).expect("write");

    let output = run(&[dir.to_str().expect("path")]);

    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("... and 6 more"), "{text}");
    assert!(text.contains("56 finding(s)"), "{text}");
}

#[test]
fn warns_about_invalid_utf8_without_verbose() {
    let dir = workspace("invalid-quiet");
    fs::write(dir.join("broken.txt"), [0xE2, 0x28, 0xA1]).expect("write");

    let output = run(&[dir.to_str().expect("path")]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("not valid UTF-8"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn needs_a_repository_or_paths() {
    let dir = workspace("no-repo");

    let output = run_in(&dir, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("not a git repository"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn refuses_to_rewrite_commit_messages() {
    // Runs inside this crate's own repository, which has a HEAD to name.
    let output = Command::new(EXE)
        .args(["--messages", "HEAD~1..HEAD", "--fix"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run aiwg");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot rewrite commit messages"),
        "{stderr}"
    );
}

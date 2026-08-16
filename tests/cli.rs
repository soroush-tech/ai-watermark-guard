//! End-to-end runs of the built binary.
//!
//! Fixtures are built from code points rather than written as themselves: this crate is scanned by
//! its own guard, and a test file full of em dashes would fail that run.

use std::fs;
use std::path::PathBuf;
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

//! `aiwg` - the command line.
//!
//! Modes, in the order they are resolved: `--message` (one commit message), `--messages` (a range
//! of them), then the file modes - explicit paths, `--all`, `--staged`, `--since`, and by default
//! whatever differs from the merge-base with `--branch`.

mod config;
mod files;
mod git;
mod rules;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use globset::GlobSet;
use rayon::prelude::*;

use crate::config::Config;
use crate::files::Content;
use crate::git::Git;
use crate::rules::{Finding, Tier, Tiers};

const OK: u8 = 0;
const FOUND: u8 = 1;
const FAILED: u8 = 2;

/// How many findings are printed before the rest are counted rather than listed.
const MAX_SHOWN: usize = 50;

#[derive(Parser, Debug)]
#[command(
    name = "ai-watermark-guard",
    bin_name = "aiwg",
    version,
    about = "Finds the characters that mark text as machine-written, plus invisible characters and mojibake.",
    after_help = "Exit codes: 0 clean, 1 findings, 2 the run itself failed."
)]
struct Cli {
    /// Files or directories to scan. Without any, a mode flag decides what is looked at.
    paths: Vec<PathBuf>,

    /// Every tracked text file in the repository.
    #[arg(long)]
    all: bool,

    /// Only what is staged, for a pre-commit hook. Fixed files are re-staged.
    #[arg(long)]
    staged: bool,

    /// Compare against a revision instead of the merge-base.
    #[arg(long, value_name = "REV")]
    since: Option<String>,

    /// The branch to take the merge-base from in the default mode.
    #[arg(long, value_name = "NAME", default_value = "main")]
    branch: String,

    /// Check one commit message file, for a commit-msg hook.
    #[arg(long, value_name = "FILE")]
    message: Option<PathBuf>,

    /// Check the commit messages in a range, e.g. main..HEAD.
    #[arg(long, value_name = "RANGE")]
    messages: Option<String>,

    /// Write the plain equivalent. Prose and commit messages only; code is reported, never edited.
    #[arg(long)]
    fix: bool,

    /// With --fix and --staged, leave re-staging to you.
    #[arg(long = "no-restage")]
    no_restage: bool,

    /// Exit non-zero even when everything found was fixed.
    #[arg(long)]
    bail: bool,

    /// Name every file considered, not only the ones with findings.
    #[arg(long)]
    verbose: bool,

    /// Path to a config file. Defaults to .ai-watermark-guard.toml at the repository root.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Tiers to enforce: invisible, punctuation, mojibake. Defaults to all three.
    #[arg(long, value_name = "LIST")]
    rules: Option<String>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("aiwg: {message}");
            ExitCode::from(FAILED)
        }
    }
}

fn run(cli: Cli) -> Result<u8, String> {
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let repo = Git::discover(&cwd);
    let root = repo.as_ref().map(|repo| repo.root().to_path_buf());

    let config = Config::load(cli.config.as_deref(), root.as_deref())?;
    let tiers = config.tiers(cli.rules.as_deref().map(Tiers::from_list).transpose()?)?;
    let excludes = config.excludes()?;

    if let Some(path) = &cli.message {
        return check_message_file(path, tiers, cli.fix);
    }
    if let Some(range) = &cli.messages {
        let repo = repo
            .as_ref()
            .ok_or("not a git repository, so there are no commit messages")?;
        return check_message_range(repo, range, tiers, cli.fix);
    }

    check_files(
        &cli,
        repo.as_ref(),
        root.as_deref(),
        tiers,
        excludes.as_ref(),
    )
}

/// Git strips its own comment lines before making the commit, so they are not part of the message
/// and are not scanned. Everything after the scissors line goes the same way.
fn message_body(text: &str) -> String {
    text.lines()
        .take_while(|line| !line.starts_with("# ------------------------ >8"))
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn check_message_file(path: &Path, tiers: Tiers, fix: bool) -> Result<u8, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;

    let mut findings = rules::scan(&message_body(&text), tiers);
    if fix && findings.iter().any(|finding| finding.fixable) {
        let fixed = rules::fix(&text, tiers);
        fs::write(path, &fixed)
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        findings = rules::scan(&message_body(&fixed), tiers);
    }

    if findings.is_empty() {
        return Ok(OK);
    }
    eprintln!(
        "aiwg: the commit message holds {} character(s) that do not belong:",
        findings.len()
    );
    for finding in findings.iter().take(MAX_SHOWN) {
        eprintln!(
            "  - line {}, column {}: {finding}",
            finding.line, finding.column
        );
    }
    if !fix {
        eprintln!("\nRun with --fix, or write the plain equivalent yourself.");
    }
    Ok(FOUND)
}

fn check_message_range(repo: &Git, range: &str, tiers: Tiers, fix: bool) -> Result<u8, String> {
    if fix {
        // Rewriting a message makes a new commit object: every sha from that point changes, and
        // every signature over the old ones is void. That is a history rewrite, not a fix, and it
        // is not something this tool does to a repository behind your back.
        //
        // The refusal comes first and asks git nothing, so it reads the same in a repository with
        // no commits yet as in one with ten thousand. The unpushed count is detail, not the reason.
        let detail = match repo.unpushed(range) {
            Ok(unpushed) => format!(" ({} of them are unpushed)", unpushed.len()),
            Err(_) => String::new(),
        };
        return Err(format!(
            "--fix cannot rewrite commit messages{detail}.\n       \
             Amending changes every sha from there on and voids the signatures over them.\n       \
             Use `git commit --amend` for the tip, or an interactive rebase you drive yourself."
        ));
    }

    let commits = repo.messages(range)?;
    let mut total = 0;
    for (sha, message) in &commits {
        let findings = rules::scan(&message_body(message), tiers);
        if findings.is_empty() {
            continue;
        }
        total += findings.len();
        let subject = message.lines().next().unwrap_or("").trim();
        println!("{}  {subject}", &sha[..sha.len().min(8)]);
        for finding in findings.iter().take(MAX_SHOWN) {
            println!(
                "  - line {}, column {}: {finding}",
                finding.line, finding.column
            );
        }
    }

    if total == 0 {
        println!("aiwg: {} commit message(s) are clean.", commits.len());
        return Ok(OK);
    }
    println!(
        "\naiwg: {total} character(s) across {} commit message(s).",
        commits.len()
    );
    Ok(FOUND)
}

/// The paths a run looks at, already filtered and de-duplicated.
fn select(
    cli: &Cli,
    repo: Option<&Git>,
    root: Option<&Path>,
    excludes: Option<&GlobSet>,
) -> Result<Vec<PathBuf>, String> {
    if cli.paths.is_empty() {
        let repo =
            repo.ok_or("not a git repository - pass paths to scan, or run this inside one")?;
        let root = root.ok_or("no repository root")?;
        return select_from_repo(cli, repo, root, excludes);
    }

    // Explicit paths are filtered against the same globs, relative to the root when there is one.
    let filtered = walk_explicit(&cli.paths)?
        .into_iter()
        .filter(|path| {
            let Some(globs) = excludes else { return true };
            let relative = relative_to(root, path).unwrap_or_default();
            !globs.is_match(&relative)
        })
        .collect();
    Ok(filtered)
}

/// Explicit paths work with no repository at all - `.gitignore` is still respected where there is
/// one, and the walk never descends into .git.
fn walk_explicit(paths: &[PathBuf]) -> Result<BTreeSet<PathBuf>, String> {
    let mut selected: BTreeSet<PathBuf> = BTreeSet::new();
    for path in paths {
        // `hidden(false)` so dotfiles are scanned - .github/, .gitignore and .husky/ are text
        // like any other. That also opens .git itself, which holds nothing anyone wrote and
        // plenty that is not UTF-8, so it is pruned by name.
        let walk = ignore::WalkBuilder::new(path)
            .hidden(false)
            .filter_entry(|entry| entry.file_name() != ".git")
            .build();
        for entry in walk {
            let entry = entry.map_err(|error| error.to_string())?;
            if entry.file_type().is_some_and(|kind| kind.is_file()) {
                selected.insert(entry.into_path());
            }
        }
    }
    Ok(selected)
}

/// The mode flag decides which repository-relative paths a run considers.
fn mode_paths(cli: &Cli, repo: &Git) -> Result<Vec<String>, String> {
    if cli.all {
        repo.tracked()
    } else if cli.staged {
        repo.staged()
    } else if let Some(rev) = &cli.since {
        repo.changed_since(rev)
    } else {
        let base = repo.merge_base(&cli.branch).ok_or_else(|| {
            format!(
                "no merge-base with '{}' - use --branch, --since or --all",
                cli.branch
            )
        })?;
        repo.changed_since(&base)
    }
}

fn select_from_repo(
    cli: &Cli,
    repo: &Git,
    root: &Path,
    excludes: Option<&GlobSet>,
) -> Result<Vec<PathBuf>, String> {
    let mut selected: BTreeSet<PathBuf> = BTreeSet::new();
    for path in mode_paths(cli, repo)? {
        if excludes.is_some_and(|globs| globs.is_match(&path)) {
            continue;
        }
        let full = root.join(&path);
        // A path can be staged or changed and no longer on disk - a deletion.
        if full.is_file() {
            selected.insert(full);
        }
    }
    Ok(selected.into_iter().collect())
}

/// What one file turned into.
struct Scanned {
    path: PathBuf,
    findings: Vec<Finding>,
    binary: bool,
    invalid: bool,
    fixed: bool,
}

fn check_files(
    cli: &Cli,
    repo: Option<&Git>,
    root: Option<&Path>,
    tiers: Tiers,
    excludes: Option<&GlobSet>,
) -> Result<u8, String> {
    let paths = select(cli, repo, root, excludes)?;

    // Fixed in a partially staged file must not be re-staged: the rest of that file is deliberately
    // out of this commit, and staging it would sweep those edits in.
    let partially_staged: BTreeSet<String> = if cli.staged && cli.fix {
        let unstaged: BTreeSet<String> = repo
            .map(|repo| repo.unstaged())
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .collect();
        repo.map(|repo| repo.staged())
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .filter(|path| unstaged.contains(path))
            .collect()
    } else {
        BTreeSet::new()
    };

    let results: Vec<Scanned> = paths
        .par_iter()
        .map(|path| scan_one(path, tiers, cli.fix))
        .collect::<Result<Vec<_>, String>>()?;

    report(cli, &results, root, tiers)?;

    let fixed: Vec<String> = results
        .iter()
        .filter(|result| result.fixed)
        .filter_map(|result| relative_to(root, &result.path))
        .filter(|path| !partially_staged.contains(path))
        .collect();

    // Staged mode cannot get this far without a repository, so the filter never drops one here.
    if let Some(repo) = repo.filter(|_| cli.staged && cli.fix && !cli.no_restage) {
        repo.stage(&fixed)?;
    }

    let remaining: usize = results.iter().map(|result| result.findings.len()).sum();
    let touched_partial = results
        .iter()
        .filter(|result| result.fixed)
        .filter_map(|result| relative_to(root, &result.path))
        .any(|path| partially_staged.contains(&path));

    if touched_partial {
        eprintln!("aiwg: a partially staged file was fixed but not re-staged - stage it yourself.");
        return Ok(FOUND);
    }
    if remaining > 0 {
        return Ok(FOUND);
    }
    if cli.bail && !fixed.is_empty() {
        return Ok(FOUND);
    }
    Ok(OK)
}

fn relative_to(root: Option<&Path>, path: &Path) -> Option<String> {
    let relative = root
        .and_then(|root| strip_root(root, path))
        .unwrap_or_else(|| path.to_path_buf());
    Some(relative.to_string_lossy().replace('\\', "/"))
}

/// `path` relative to `root`, surviving a spelling difference between the two - the path the user
/// typed reaches a symlinked temp directory on macOS as /var and git's root as /private/var, and
/// an 8.3 short path on Windows never textually matches the long form git prints.
fn strip_root(root: &Path, path: &Path) -> Option<PathBuf> {
    if let Ok(stripped) = path.strip_prefix(root) {
        return Some(stripped.to_path_buf());
    }
    let root = root.canonicalize().ok()?;
    let path = path.canonicalize().ok()?;
    path.strip_prefix(&root).ok().map(Path::to_path_buf)
}

fn scan_one(path: &Path, tiers: Tiers, fix: bool) -> Result<Scanned, String> {
    let mut scanned = Scanned {
        path: path.to_path_buf(),
        findings: Vec::new(),
        binary: false,
        invalid: false,
        fixed: false,
    };

    match files::read(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?
    {
        Content::Binary => scanned.binary = true,
        Content::InvalidUtf8 => scanned.invalid = true,
        Content::Text(text) => {
            scanned.findings = rules::scan(&text, tiers);
            let fixable = scanned.findings.iter().any(|finding| finding.fixable);
            if fix && fixable && files::is_fixable(path) {
                let fixed = rules::fix(&text, tiers);
                fs::write(path, &fixed)
                    .map_err(|error| format!("could not write {}: {error}", path.display()))?;
                scanned.findings = rules::scan(&fixed, tiers);
                scanned.fixed = true;
            }
        }
    }
    Ok(scanned)
}

fn report(cli: &Cli, results: &[Scanned], root: Option<&Path>, tiers: Tiers) -> Result<(), String> {
    let mut shown = 0;
    let total: usize = results.iter().map(|result| result.findings.len()).sum();

    for result in results {
        let name = relative_to(root, &result.path).unwrap_or_default();
        if cli.verbose && result.findings.is_empty() {
            println!("  {name}{}", file_note(result));
        }
        if result.invalid && !cli.verbose {
            eprintln!("aiwg: {name} is not valid UTF-8 and was not scanned.");
        }
        print_findings(result, &name, cli.verbose, total, &mut shown);
    }

    print_summary(results, total, tiers);
    Ok(())
}

fn file_note(result: &Scanned) -> &'static str {
    if result.binary {
        " (binary, skipped)"
    } else if result.invalid {
        " (not UTF-8, skipped)"
    } else {
        ""
    }
}

/// What to do about one finding, from the table's replacement.
fn hint(finding: &Finding) -> String {
    rules::banned(char::from_u32(finding.point).unwrap_or('?'))
        .map(|rule| {
            if rule.replacement.is_empty() {
                "delete it".to_string()
            } else {
                format!("use {}", rule.replacement)
            }
        })
        .unwrap_or_else(|| "cannot be repaired automatically".to_string())
}

/// Prints one file's findings, stopping at [`MAX_SHOWN`] across the whole run.
fn print_findings(result: &Scanned, name: &str, verbose: bool, total: usize, shown: &mut usize) {
    for finding in &result.findings {
        if *shown > MAX_SHOWN {
            return;
        }
        if *shown == MAX_SHOWN {
            println!("  ... and {} more", total - *shown);
            *shown += 1;
            return;
        }
        let tier = if verbose {
            format!(" [{}]", finding.tier.label())
        } else {
            String::new()
        };
        println!(
            "{name}:{}:{}  {finding} - {}{tier}",
            finding.line,
            finding.column,
            hint(finding)
        );
        *shown += 1;
    }
}

fn print_summary(results: &[Scanned], total: usize, tiers: Tiers) {
    let scanned = results
        .iter()
        .filter(|result| !result.binary && !result.invalid)
        .count();
    let skipped = results.len() - scanned;
    let fixed = results.iter().filter(|result| result.fixed).count();

    let tier_list: Vec<&str> = [Tier::Invisible, Tier::Punctuation, Tier::Mojibake]
        .into_iter()
        .filter(|tier| tiers.has(*tier))
        .map(Tier::label)
        .collect();

    if total == 0 {
        println!(
            "aiwg: clean - {scanned} file(s) scanned, {skipped} skipped [{}]",
            tier_list.join(", ")
        );
    } else {
        println!(
            "\naiwg: {total} finding(s) in {} file(s); {fixed} fixed, {scanned} scanned [{}]",
            results
                .iter()
                .filter(|result| !result.findings.is_empty())
                .count(),
            tier_list.join(", ")
        );
    }
}

#[cfg(test)]
mod main_test {
    use super::*;

    #[test]
    fn strips_the_comment_lines_git_would_strip() {
        let text = "subject\n\nbody\n# Please enter the commit message\n";
        assert_eq!(message_body(text), "subject\n\nbody");
    }

    #[test]
    fn stops_at_the_scissors_line() {
        let text = "subject\n# ------------------------ >8 ------------------------\ndiff --git";
        assert_eq!(message_body(text), "subject");
    }

    fn finding_at(point: u32, tier: Tier) -> Finding {
        Finding {
            line: 1,
            column: 1,
            point,
            name: "test",
            tier,
            fixable: tier != Tier::Mojibake,
        }
    }

    #[test]
    fn hints_the_replacement_from_the_table() {
        let em_dash = finding_at(0x2014, Tier::Punctuation);
        assert_eq!(hint(&em_dash), "use -");
    }

    #[test]
    fn hints_deletion_when_the_replacement_is_empty() {
        let zero_width_space = finding_at(0x200B, Tier::Invisible);
        assert_eq!(hint(&zero_width_space), "delete it");
    }

    #[test]
    fn hints_no_repair_for_a_point_outside_the_table() {
        let mojibake = finding_at(0x00C3, Tier::Mojibake);
        assert_eq!(hint(&mojibake), "cannot be repaired automatically");
    }

    #[test]
    fn notes_why_a_file_was_skipped() {
        let mut result = Scanned {
            path: PathBuf::from("a"),
            findings: Vec::new(),
            binary: false,
            invalid: false,
            fixed: false,
        };
        assert_eq!(file_note(&result), "");
        result.binary = true;
        assert_eq!(file_note(&result), " (binary, skipped)");
        result.binary = false;
        result.invalid = true;
        assert_eq!(file_note(&result), " (not UTF-8, skipped)");
    }

    #[test]
    fn strips_across_spelling_differences_or_not_at_all() {
        let temp = std::env::temp_dir();
        let outside = temp.join("aiwg-unit-strip-outside");
        fs::create_dir_all(&outside).expect("create dir");
        // A root that does not exist cannot be canonicalized.
        assert_eq!(strip_root(Path::new("aiwg-no-such-root"), &outside), None);
        // Both real, but neither contains the other.
        let elsewhere = temp.join("aiwg-unit-strip-elsewhere");
        fs::create_dir_all(&elsewhere).expect("create dir");
        assert_eq!(strip_root(&outside, &elsewhere), None);
    }

    #[test]
    fn makes_a_repository_relative_name() {
        let root = PathBuf::from("/repo");
        let path = PathBuf::from("/repo/src/main.rs");
        assert_eq!(
            relative_to(Some(&root), &path),
            Some("src/main.rs".to_string())
        );
        assert_eq!(
            relative_to(None, &path),
            Some("/repo/src/main.rs".to_string())
        );
    }
}

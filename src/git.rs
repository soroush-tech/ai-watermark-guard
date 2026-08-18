//! Everything this tool asks of git. Each call is one `git` process with fixed arguments; nothing
//! user-supplied ever reaches a shell, because no shell is involved.

use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Git {
    root: PathBuf,
}

/// Splits git's NUL-separated output, dropping the trailing empty entry.
fn split_nul(output: &str) -> Vec<String> {
    output
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

impl Git {
    /// The repository containing `from`, or `None` when there is no repository - the tool still
    /// works on plain paths, so this is a fact to report rather than an error.
    pub fn discover(from: &Path) -> Option<Git> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(from)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let root = String::from_utf8(output.stdout).ok()?.trim().to_string();
        // No git prints an empty toplevel and succeeds; the guard is for the shape, not a path any
        // real run takes.
        (!root.is_empty()).then(|| Git {
            root: PathBuf::from(root),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn run(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(|error| format!("could not run git: {error}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                format!("git {} failed", args.join(" "))
            } else {
                stderr
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Every tracked path. `-z` throughout, so a path with a space or a non-ASCII name arrives
    /// whole rather than quoted.
    pub fn tracked(&self) -> Result<Vec<String>, String> {
        Ok(split_nul(&self.run(&["ls-files", "-z"])?))
    }

    /// Paths staged for the next commit. Deletions are excluded - there is nothing left to read.
    pub fn staged(&self) -> Result<Vec<String>, String> {
        Ok(split_nul(&self.run(&[
            "diff",
            "--cached",
            "--name-only",
            "--diff-filter=ACMR",
            "-z",
        ])?))
    }

    /// Paths with changes that are not staged. Used to spot a partially staged file, whose fix
    /// must not be re-staged.
    pub fn unstaged(&self) -> Result<Vec<String>, String> {
        Ok(split_nul(&self.run(&[
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            "-z",
        ])?))
    }

    /// Paths that differ from `rev`.
    pub fn changed_since(&self, rev: &str) -> Result<Vec<String>, String> {
        Ok(split_nul(&self.run(&[
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            "-z",
            rev,
        ])?))
    }

    /// The merge-base with `branch`, so a feature branch that has fallen behind does not drag in
    /// everything that landed on the branch meanwhile. `None` when the branch is unknown, which is
    /// ordinary in a fresh clone or a repository whose default branch is named something else.
    pub fn merge_base(&self, branch: &str) -> Option<String> {
        let base = self.run(&["merge-base", branch, "HEAD"]).ok()?;
        let base = base.trim().to_string();
        // Real git never succeeds while printing nothing; the guard is for the shape.
        (!base.is_empty()).then_some(base)
    }

    pub fn stage(&self, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args = vec!["add", "--"];
        args.extend(paths.iter().map(String::as_str));
        self.run(&args).map(|_| ())
    }

    /// Every commit in `range`, as (sha, message). Records are separated by a record-separator
    /// byte and fields by a unit-separator byte, so a message containing blank lines - which every
    /// message with a body does - still parses.
    pub fn messages(&self, range: &str) -> Result<Vec<(String, String)>, String> {
        let output = self.run(&["log", "--format=%H%x1f%B%x1e", range])?;
        Ok(output
            .split('\u{1e}')
            .map(str::trim_start)
            .filter(|record| !record.trim().is_empty())
            .filter_map(|record| {
                let (sha, message) = record.split_once('\u{1f}')?;
                Some((sha.trim().to_string(), message.to_string()))
            })
            .collect())
    }

    /// The commits in `range` that no remote has yet. Rewriting a message is safe only for these:
    /// amending anything a remote already holds changes a published sha.
    pub fn unpushed(&self, range: &str) -> Result<Vec<String>, String> {
        Ok(self
            .run(&["rev-list", range, "--not", "--remotes"])?
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }
}

#[cfg(test)]
mod git_test {
    use super::*;

    #[test]
    fn splits_nul_separated_output_without_a_trailing_blank() {
        assert_eq!(split_nul("a\0b\0"), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(split_nul(""), Vec::<String>::new());
    }

    #[test]
    fn finds_no_repository_outside_one() {
        // A plain directory under temp, which is not inside a repository on any machine this
        // builds on.
        let outside = std::env::temp_dir().join("aiwg-unit-no-repo");
        std::fs::create_dir_all(&outside).expect("create dir");
        assert!(Git::discover(&outside).is_none());
    }

    #[test]
    fn discovers_the_repository_holding_this_crate() {
        let git = Git::discover(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("a repository");
        assert!(git.root().join("Cargo.toml").exists());
    }
}

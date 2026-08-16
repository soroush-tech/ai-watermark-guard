//! Reading files, and deciding which of them `--fix` is allowed to write.

use std::fs;
use std::io;
use std::path::Path;

/// What came back from disk.
pub enum Content {
    /// Holds a NUL byte - the same test git uses to call a file binary. Images and fonts are full
    /// of these code points and mean nothing by them.
    Binary,
    /// Not valid UTF-8. Scanning the lossy decode would report a replacement character on every
    /// invalid sequence, which describes the decoder rather than the file.
    InvalidUtf8,
    Text(String),
}

pub fn read(path: &Path) -> io::Result<Content> {
    let bytes = fs::read(path)?;
    if bytes.contains(&0) {
        return Ok(Content::Binary);
    }
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Content::Text(text)),
        Err(_) => Ok(Content::InvalidUtf8),
    }
}

/// The extensions `--fix` may write, and only these.
///
/// The rest are reported and left alone, because a blind replacement inside code is not safe: a
/// curly apostrophe sitting in a single-quoted string becomes a straight quote that ends the
/// string, and the file no longer parses. The same hazard exists in JSON, YAML and TOML, whose
/// values are quoted too. Fixing those needs a parser per language, which is a later version.
const FIXABLE: [&str; 6] = ["md", "markdown", "txt", "text", "rst", "adoc"];

pub fn is_fixable(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| FIXABLE.contains(&extension.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "files_test.rs"]
mod files_test;

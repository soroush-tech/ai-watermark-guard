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
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn fixes_prose_and_refuses_everything_else() {
        for name in [
            "notes.md",
            "NOTES.MD",
            "readme.markdown",
            "a.txt",
            "b.rst",
            "c.adoc",
        ] {
            assert!(is_fixable(&PathBuf::from(name)), "{name}");
        }
        for name in [
            "a.ts", "a.tsx", "a.js", "a.json", "a.yml", "a.toml", "a.rs", "Makefile",
        ] {
            assert!(!is_fixable(&PathBuf::from(name)), "{name}");
        }
    }

    #[test]
    fn classifies_what_it_reads() {
        let dir = std::env::temp_dir().join("aiwg-files-test");
        fs::create_dir_all(&dir).expect("temp dir");

        let text = dir.join("a.txt");
        fs::write(&text, "plain").expect("write");
        assert!(matches!(read(&text).expect("read"), Content::Text(body) if body == "plain"));

        let binary = dir.join("b.bin");
        fs::write(&binary, [0x41, 0x00, 0x42]).expect("write");
        assert!(matches!(read(&binary).expect("read"), Content::Binary));

        let invalid = dir.join("c.txt");
        fs::write(&invalid, [0xE2, 0x28, 0xA1]).expect("write");
        assert!(matches!(
            read(&invalid).expect("read"),
            Content::InvalidUtf8
        ));

        assert!(read(&dir.join("missing.txt")).is_err());
        fs::remove_dir_all(&dir).ok();
    }
}

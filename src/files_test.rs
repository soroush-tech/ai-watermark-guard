use super::*;
use std::path::PathBuf;

/// A directory of its own per test, so parallel runs never see each other's files.
fn workspace(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aiwg-files-test-{name}"));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn fixes_prose_extensions_whatever_their_case() {
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
}

#[test]
fn refuses_to_fix_code_and_config_files() {
    for name in [
        "a.ts", "a.tsx", "a.js", "a.json", "a.yml", "a.toml", "a.rs", "Makefile",
    ] {
        assert!(!is_fixable(&PathBuf::from(name)), "{name}");
    }
}

#[test]
fn reads_plain_text_as_text() {
    let dir = workspace("text");
    let path = dir.join("a.txt");
    fs::write(&path, "plain").expect("write");
    assert!(matches!(read(&path).expect("read"), Content::Text(body) if body == "plain"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn calls_a_file_with_a_nul_byte_binary() {
    let dir = workspace("binary");
    let path = dir.join("b.bin");
    fs::write(&path, [0x41, 0x00, 0x42]).expect("write");
    assert!(matches!(read(&path).expect("read"), Content::Binary));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn reports_invalid_utf8_rather_than_decoding_lossily() {
    let dir = workspace("invalid");
    let path = dir.join("c.txt");
    fs::write(&path, [0xE2, 0x28, 0xA1]).expect("write");
    assert!(matches!(read(&path).expect("read"), Content::InvalidUtf8));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn passes_through_the_error_for_a_missing_file() {
    let dir = workspace("missing");
    assert!(read(&dir.join("missing.txt")).is_err());
    fs::remove_dir_all(&dir).ok();
}

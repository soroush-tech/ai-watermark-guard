use super::*;

#[test]
fn an_empty_config_enforces_every_tier() {
    let config = Config::parse("").expect("valid");
    let tiers = config.tiers(None).expect("tiers");
    assert!(tiers.invisible && tiers.punctuation && tiers.mojibake);
    assert!(config.excludes().expect("globs").is_none());
}

#[test]
fn reads_the_rules_and_the_excludes() {
    let config =
        Config::parse("rules = [\"invisible\"]\nexclude = [\"vendor/**\"]").expect("valid");
    let tiers = config.tiers(None).expect("tiers");
    assert!(tiers.invisible && !tiers.punctuation && !tiers.mojibake);
    let globs = config.excludes().expect("globs").expect("some");
    assert!(globs.is_match("vendor/thing.md"));
    assert!(!globs.is_match("src/thing.md"));
}

#[test]
fn the_command_line_wins_over_the_file() {
    let config = Config::parse("rules = [\"invisible\"]").expect("valid");
    let flag = Tiers::from_list("punctuation").expect("valid");
    let tiers = config.tiers(Some(flag)).expect("tiers");
    assert!(tiers.punctuation && !tiers.invisible);
}

#[test]
fn rejects_an_unknown_key() {
    assert!(Config::parse("rule = [\"invisible\"]").is_err());
}

#[test]
fn rejects_an_unknown_tier_name() {
    assert!(Config::parse("rules = [\"typos\"]")
        .expect("parses")
        .tiers(None)
        .is_err());
}

#[test]
fn rejects_a_bad_exclude_glob() {
    let bad = Config {
        rules: None,
        exclude: Some(vec!["[".to_string()]),
    };
    assert!(bad.excludes().is_err());
}

#[test]
fn an_empty_exclude_list_is_no_filter() {
    let config = Config {
        rules: None,
        exclude: Some(vec![]),
    };
    assert!(config.excludes().expect("globs").is_none());
}

#[test]
fn a_missing_explicit_config_is_an_error() {
    // A typo in --config would otherwise silently enforce something other than what was asked.
    let missing = PathBuf::from("no-such-config.toml");
    assert!(Config::load(Some(&missing), None).is_err());
}

#[test]
fn a_missing_default_config_falls_back_to_the_defaults() {
    assert!(Config::load(None, None).is_ok());
}

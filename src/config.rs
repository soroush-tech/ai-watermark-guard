//! The optional config file: which tiers to enforce, and what to leave alone.
//!
//! ```toml
//! rules = ["invisible", "mojibake"]
//! exclude = ["**/fixtures/**", "vendor/**"]
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use crate::rules::Tiers;

/// What a config file may say. Both keys are optional, and an empty file is valid.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub rules: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

/// Read next to the repository root when `--config` is not given.
pub const DEFAULT_NAME: &str = ".ai-watermark-guard.toml";

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        toml::from_str(text).map_err(|error| error.message().to_string())
    }

    /// `--config <path>` when given, else the default file at `root` when it exists, else the
    /// defaults. A `--config` path that does not exist is an error: a typo there would otherwise
    /// silently enforce something other than what was asked for.
    pub fn load(explicit: Option<&Path>, root: Option<&Path>) -> Result<Config, String> {
        let path: Option<PathBuf> = match explicit {
            Some(path) if !path.exists() => {
                return Err(format!("config file not found: {}", path.display()))
            }
            Some(path) => Some(path.to_path_buf()),
            None => root
                .map(|root| root.join(DEFAULT_NAME))
                .filter(|path| path.exists()),
        };

        match path {
            None => Ok(Config::default()),
            Some(path) => {
                let text = fs::read_to_string(&path)
                    .map_err(|error| format!("could not read {}: {error}", path.display()))?;
                Config::parse(&text).map_err(|error| format!("{}: {error}", path.display()))
            }
        }
    }

    /// The tiers this config asks for, unless the command line already decided.
    pub fn tiers(&self, from_flag: Option<Tiers>) -> Result<Tiers, String> {
        if let Some(tiers) = from_flag {
            return Ok(tiers);
        }
        match &self.rules {
            None => Ok(Tiers::default()),
            Some(names) => Tiers::from_list(&names.join(",")),
        }
    }

    /// The exclude globs, compiled once. Matched against repository-relative paths with forward
    /// slashes, so one config reads the same on every platform.
    pub fn excludes(&self) -> Result<Option<GlobSet>, String> {
        let Some(patterns) = &self.exclude else {
            return Ok(None);
        };
        if patterns.is_empty() {
            return Ok(None);
        }
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            builder.add(Glob::new(pattern).map_err(|error| format!("bad exclude glob: {error}"))?);
        }
        builder.build().map(Some).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
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
    fn rejects_an_unknown_key_a_bad_tier_and_a_bad_glob() {
        assert!(Config::parse("rule = [\"invisible\"]").is_err());
        assert!(Config::parse("rules = [\"typos\"]")
            .expect("parses")
            .tiers(None)
            .is_err());
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
    fn a_missing_explicit_config_is_an_error_but_a_missing_default_is_not() {
        let missing = PathBuf::from("no-such-config.toml");
        assert!(Config::load(Some(&missing), None).is_err());
        assert!(Config::load(None, None).is_ok());
    }
}

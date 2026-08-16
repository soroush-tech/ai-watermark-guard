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
#[path = "config_test.rs"]
mod config_test;

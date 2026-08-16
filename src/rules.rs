//! The character tables and the scan over them.
//!
//! Every character is written as a code point rather than as itself. Six of them are invisible, so
//! source holding the characters would be source nobody can review - and this crate would report
//! its own tables.

use std::fmt;

/// Which group a character belongs to. Tiers are selected per run: the first two are wrong
/// anywhere, the third is a house rule and off unless asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    /// Invisible, and meaningless in text a person wrote.
    Invisible,
    /// Typographic punctuation, whose plain equivalent reads the same to everyone.
    Punctuation,
    /// UTF-8 that was decoded as Latin-1 somewhere upstream.
    Mojibake,
}

impl Tier {
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim() {
            "invisible" => Some(Tier::Invisible),
            "punctuation" => Some(Tier::Punctuation),
            "mojibake" => Some(Tier::Mojibake),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tier::Invisible => "invisible",
            Tier::Punctuation => "punctuation",
            Tier::Mojibake => "mojibake",
        }
    }
}

/// The tiers a run enforces.
#[derive(Clone, Copy, Debug)]
pub struct Tiers {
    pub invisible: bool,
    pub punctuation: bool,
    pub mojibake: bool,
}

impl Default for Tiers {
    /// All three. The tool exists to be strict; a caller who wants less says so.
    fn default() -> Self {
        Tiers {
            invisible: true,
            punctuation: true,
            mojibake: true,
        }
    }
}

impl Tiers {
    pub fn has(self, tier: Tier) -> bool {
        match tier {
            Tier::Invisible => self.invisible,
            Tier::Punctuation => self.punctuation,
            Tier::Mojibake => self.mojibake,
        }
    }

    /// Builds from a comma-separated list. An unknown name is returned as an error.
    pub fn from_list(list: &str) -> Result<Self, String> {
        let mut tiers = Tiers {
            invisible: false,
            punctuation: false,
            mojibake: false,
        };
        for name in list.split(',').filter(|name| !name.trim().is_empty()) {
            match Tier::parse(name) {
                Some(Tier::Invisible) => tiers.invisible = true,
                Some(Tier::Punctuation) => tiers.punctuation = true,
                Some(Tier::Mojibake) => tiers.mojibake = true,
                None => return Err(format!("unknown rule tier: {}", name.trim())),
            }
        }
        Ok(tiers)
    }
}

/// Deliberately never flagged, whatever the tiers. Each one carries meaning, and removing it
/// corrupts the text rather than cleaning it:
///
/// - `U+200C` zero-width non-joiner, which Persian needs between the parts of a word.
/// - `U+200D` zero-width joiner, which holds an emoji sequence together.
/// - `U+200E` / `U+200F` the direction marks, which order mixed right-to-left text.
///
/// They are absent from the table below, and this constant exists so that stays deliberate.
pub const ALWAYS_ALLOWED: [u32; 4] = [0x200C, 0x200D, 0x200E, 0x200F];

/// What a character is, and what to write instead. An empty replacement means "delete it".
pub struct Banned {
    pub name: &'static str,
    pub tier: Tier,
    pub replacement: &'static str,
}

/// The whole table. `None` for any character this tool has no opinion about, which is almost all
/// of them - letters in any script, emoji, and the four allowed above.
pub fn banned(character: char) -> Option<Banned> {
    // Checked first rather than left out of the table. Omission is how an exemption gets undone by
    // someone adding a plausible-looking row; a guard says it was meant.
    if ALWAYS_ALLOWED.contains(&(character as u32)) {
        return None;
    }
    let (name, tier, replacement) = match character as u32 {
        0x200B => ("zero-width space", Tier::Invisible, ""),
        0x2060 => ("word joiner", Tier::Invisible, ""),
        0xFEFF => ("byte order mark", Tier::Invisible, ""),
        0x00AD => ("soft hyphen", Tier::Invisible, ""),
        0xFFFD => ("replacement character", Tier::Invisible, ""),
        0x00A0 => ("no-break space", Tier::Punctuation, " "),
        0x202F => ("narrow no-break space", Tier::Punctuation, " "),
        0x2009 => ("thin space", Tier::Punctuation, " "),
        0x2018 => ("left single quote", Tier::Punctuation, "'"),
        0x2019 => ("right single quote", Tier::Punctuation, "'"),
        0x201A => ("low single quote", Tier::Punctuation, "'"),
        0x201B => ("high-reversed single quote", Tier::Punctuation, "'"),
        0x201C => ("left double quote", Tier::Punctuation, "\""),
        0x201D => ("right double quote", Tier::Punctuation, "\""),
        0x201E => ("low double quote", Tier::Punctuation, "\""),
        0x2013 => ("en dash", Tier::Punctuation, "-"),
        0x2014 => ("em dash", Tier::Punctuation, "-"),
        0x2212 => ("minus sign", Tier::Punctuation, "-"),
        0x2026 => ("ellipsis", Tier::Punctuation, "..."),
        _ => return None,
    };
    Some(Banned {
        name,
        tier,
        replacement,
    })
}

/// The opening of UTF-8 read as Latin-1: `E2 80 xx` (dashes, curly quotes, ellipsis) arrives as
/// a-circumflex followed by a euro sign, and `C2`/`C3 xx` (no-break space, accented letters) as
/// A-circumflex or A-tilde followed by a character from the Latin-1 supplement.
pub fn is_mojibake(first: char, second: char) -> bool {
    let (first, second) = (first as u32, second as u32);
    (first == 0x00E2 && second == 0x20AC)
        || ((first == 0x00C2 || first == 0x00C3) && (0x00A0..=0x00BF).contains(&second))
}

/// One offending character, located for a human.
#[derive(Debug, Clone)]
pub struct Finding {
    pub line: usize,
    pub column: usize,
    pub point: u32,
    pub name: &'static str,
    pub tier: Tier,
    /// False for mojibake: what the original bytes were cannot be recovered from the damage.
    pub fixable: bool,
}

impl fmt::Display for Finding {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "U+{:04X} {}", self.point, self.name)
    }
}

/// Every finding in one text, in reading order. Lines and columns are 1-based, and columns count
/// characters rather than bytes - a column that lands mid-character helps nobody.
pub fn scan(text: &str, tiers: Tiers) -> Vec<Finding> {
    let mut findings = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let characters: Vec<char> = line.chars().collect();
        let mut column = 0;
        while column < characters.len() {
            let character = characters[column];

            // Checked before the table, and it consumes both characters: with the punctuation tier
            // on, the second half of a mojibake pair is often a banned character in its own right,
            // and reporting it twice describes one problem as two.
            if tiers.mojibake
                && column + 1 < characters.len()
                && is_mojibake(character, characters[column + 1])
            {
                findings.push(Finding {
                    line: index + 1,
                    column: column + 1,
                    point: character as u32,
                    name: "mojibake - UTF-8 read as Latin-1",
                    tier: Tier::Mojibake,
                    fixable: false,
                });
                column += 2;
                continue;
            }

            if let Some(rule) = banned(character) {
                if tiers.has(rule.tier) {
                    findings.push(Finding {
                        line: index + 1,
                        column: column + 1,
                        point: character as u32,
                        name: rule.name,
                        tier: rule.tier,
                        fixable: true,
                    });
                }
            }
            column += 1;
        }
    }

    findings
}

/// The text with every fixable character replaced. Mojibake is left alone - guessing at the
/// original bytes would turn a visible problem into an invisible one.
pub fn fix(text: &str, tiers: Tiers) -> String {
    let mut fixed = String::with_capacity(text.len());
    for character in text.chars() {
        match banned(character) {
            Some(rule) if tiers.has(rule.tier) => fixed.push_str(rule.replacement),
            _ => fixed.push(character),
        }
    }
    fixed
}

#[cfg(test)]
#[path = "rules_test.rs"]
mod rules_test;

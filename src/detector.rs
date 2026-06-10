//! Swear-word detection.
//!
//! Matching is **case-insensitive** and **whole-word** (Unicode word
//! boundaries). `swear_count` returns the **total number of occurrences** in a
//! prompt, so a prompt containing two swears counts 2.

use anyhow::{Context, Result};
use regex::Regex;

/// Compiled matcher over a configurable word list.
#[derive(Debug, Clone)]
pub struct Detector {
    re: Option<Regex>,
}

impl Detector {
    /// Build a detector from a word list. Words are matched whole-word and
    /// case-insensitively. An empty list yields a detector that matches
    /// nothing.
    pub fn new<I, S>(words: I) -> Result<Detector>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let escaped: Vec<String> = words
            .into_iter()
            .map(|w| regex::escape(w.as_ref().trim()))
            .filter(|w| !w.is_empty())
            .collect();

        if escaped.is_empty() {
            return Ok(Detector { re: None });
        }

        // `\b(a|b|c)\b` with the case-insensitive flag. The regex crate's `\b`
        // is Unicode-aware, matching the PoC's `re.IGNORECASE` semantics.
        let pattern = format!(r"(?i)\b(?:{})\b", escaped.join("|"));
        let re = Regex::new(&pattern).context("building swear-word regex")?;
        Ok(Detector { re: Some(re) })
    }

    /// Total number of swear occurrences in `text`.
    pub fn swear_count(&self, text: &str) -> u32 {
        match &self.re {
            Some(re) => re.find_iter(text).count() as u32,
            None => 0,
        }
    }

    /// Whether `text` contains at least one swear.
    pub fn contains_swear(&self, text: &str) -> bool {
        match &self.re {
            Some(re) => re.is_match(text),
            None => false,
        }
    }
}

/// The default profanity list shipped in the binary. Users can override or
/// extend it via `config.toml`.
pub fn default_words() -> Vec<String> {
    const DEFAULTS: &[&str] = &[
        "fuck",
        "fucking",
        "fucked",
        "fucker",
        "motherfucker",
        "shit",
        "shitty",
        "bullshit",
        "ass",
        "asshole",
        "bastard",
        "bitch",
        "damn",
        "goddamn",
        "dammit",
        "crap",
        "piss",
        "pissed",
        "dick",
        "cock",
        "prick",
        "bollocks",
        "bugger",
        "wanker",
        "twat",
        "cunt",
        "hell",
        "screwed",
    ];
    DEFAULTS.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det() -> Detector {
        Detector::new(["fuck", "shit", "damn"]).unwrap()
    }

    #[test]
    fn counts_total_occurrences() {
        assert_eq!(det().swear_count("fuck this fucking shit"), 2); // fuck + shit
        assert_eq!(det().swear_count("damn damn damn"), 3);
        assert_eq!(det().swear_count("no swears here"), 0);
    }

    #[test]
    fn whole_word_only() {
        // "fucking" is not in this list, so it must not match "fuck".
        assert_eq!(det().swear_count("fucking"), 0);
        // substrings never match.
        assert_eq!(det().swear_count("scunthorpe shitake"), 0);
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(det().swear_count("FUCK Shit DaMn"), 3);
    }

    #[test]
    fn punctuation_boundaries() {
        assert_eq!(det().swear_count("fuck! shit, damn."), 3);
        assert_eq!(det().swear_count("(fuck)"), 1);
    }

    #[test]
    fn empty_list_matches_nothing() {
        let d = Detector::new(Vec::<String>::new()).unwrap();
        assert_eq!(d.swear_count("fuck shit damn"), 0);
        assert!(!d.contains_swear("fuck"));
    }

    #[test]
    fn default_list_is_nonempty() {
        let d = Detector::new(default_words()).unwrap();
        assert!(d.contains_swear("this is fucking broken"));
    }
}

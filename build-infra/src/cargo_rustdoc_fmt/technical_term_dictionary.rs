// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Dictionary of known technical terms and their canonical link targets.
//!
//! Builds from the seed file (`known_technical_term_link_dictionary.jsonc`) embedded at
//! compile time, or from a custom file provided via `--terms-file`.

use crate::cargo_rustdoc_fmt::types::FormatterResult;
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

/// Embedded seed file (JSON5 format with comments).
const EMBEDDED_SEED: &str = include_str!("known_technical_term_link_dictionary.jsonc");

/// Whether a term links to an internal Rust type or an external URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TechnicalTermTier {
    /// Links to a crate type via intra-doc link (e.g., `crate::CsiSequence`).
    Internal,
    /// Links to an external URL (e.g., `https://en.wikipedia.org/wiki/...`).
    External,
}

/// A known term entry with its canonical link target and tier.
#[derive(Debug, Clone)]
pub struct TechnicalTermEntry {
    /// The link target (crate path or URL).
    pub target: String,
    /// Whether this is an internal or external link.
    pub tier: TechnicalTermTier,
}

/// Seed file entry for deserialization.
#[derive(Deserialize)]
struct SeedEntry {
    target: String,
    tier: String,
}

/// Dictionary of known technical terms and their canonical link targets.
#[derive(Debug)]
pub struct TechnicalTermDictionary {
    terms: HashMap<String, TechnicalTermEntry>,
}

impl TechnicalTermDictionary {
    /// Builds the registry from the embedded seed file, or from a custom file
    /// provided via `--terms-file`.
    ///
    /// # Errors
    ///
    /// Returns an error if the terms file cannot be read or parsed.
    pub fn from_seed(terms_file: Option<&Path>) -> FormatterResult<Self> {
        let seed_content = match terms_file {
            Some(path) => std::fs::read_to_string(path)
                .map_err(|e| miette::miette!("Failed to read terms file: {e}"))?,
            None => EMBEDDED_SEED.to_string(),
        };

        let terms = parse_seed(&seed_content)?;
        Ok(Self { terms })
    }

    /// Looks up the entry for a term.
    #[must_use]
    pub fn get(&self, term: &str) -> Option<&TechnicalTermEntry> { self.terms.get(term) }

    /// Returns all known terms sorted longest-first.
    ///
    /// Longest-first ordering ensures overlapping terms like
    /// `` [`VT-100` spec] `` are matched before [`VT-100`].
    ///
    /// [`VT-100` spec]: https://vt100.net/docs/vt100-ug/chapter3.html
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    #[must_use]
    pub fn terms_longest_first(&self) -> Vec<(&str, &TechnicalTermEntry)> {
        let mut terms: Vec<_> = self.terms.iter().map(|(k, v)| (k.as_str(), v)).collect();
        terms.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(b.0)));
        terms
    }

    /// Returns the number of terms in the registry.
    #[must_use]
    pub fn len(&self) -> usize { self.terms.len() }

    /// Returns true if the registry contains no terms.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.terms.is_empty() }
}

/// Parses the seed file content (JSON5 format) into a term map.
fn parse_seed(content: &str) -> FormatterResult<HashMap<String, TechnicalTermEntry>> {
    let raw: HashMap<String, SeedEntry> = json5::from_str(content)
        .map_err(|e| miette::miette!("Failed to parse seed file: {e}"))?;

    let mut terms = HashMap::new();
    for (term, entry) in raw {
        let tier = match entry.tier.as_str() {
            "internal" => TechnicalTermTier::Internal,
            "external" => TechnicalTermTier::External,
            other => {
                return Err(miette::miette!("Unknown tier '{other}' for term '{term}'"));
            }
        };
        terms.insert(
            term,
            TechnicalTermEntry {
                target: entry.target,
                tier,
            },
        );
    }

    Ok(terms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_embedded_seed() {
        let registry = TechnicalTermDictionary::from_seed(None).unwrap();
        assert!(!registry.is_empty());

        // Check tier 1 (internal) terms.
        let csi = registry.get("CSI").unwrap();
        assert_eq!(csi.target, "crate::CsiSequence");
        assert_eq!(csi.tier, TechnicalTermTier::Internal);

        let sgr = registry.get("SGR").unwrap();
        assert_eq!(sgr.target, "crate::SgrCode");
        assert_eq!(sgr.tier, TechnicalTermTier::Internal);

        // Check internal crate dependency terms.
        let tokio = registry.get("tokio").unwrap();
        assert_eq!(tokio.target, "tokio");
        assert_eq!(tokio.tier, TechnicalTermTier::Internal);

        // Check tier 2 (external) terms.
        let ansi = registry.get("ANSI").unwrap();
        assert_eq!(
            ansi.target,
            "https://en.wikipedia.org/wiki/ANSI_escape_code"
        );
        assert_eq!(ansi.tier, TechnicalTermTier::External);

        let vt100 = registry.get("VT-100").unwrap();
        assert_eq!(
            vt100.target,
            "https://vt100.net/docs/vt100-ug/chapter3.html"
        );
        assert_eq!(vt100.tier, TechnicalTermTier::External);
    }

    #[test]
    fn test_terms_longest_first() {
        let registry = TechnicalTermDictionary::from_seed(None).unwrap();
        let terms = registry.terms_longest_first();

        // "`VT-100` spec" (14 chars) should come before "VT-100" (6 chars).
        let spec_idx = terms.iter().position(|(t, _)| *t == "`VT-100` spec");
        let vt100_idx = terms.iter().position(|(t, _)| *t == "VT-100");
        assert!(spec_idx.unwrap() < vt100_idx.unwrap());
    }

    #[test]
    fn test_all_seed_terms_present() {
        let registry = TechnicalTermDictionary::from_seed(None).unwrap();

        // Tier 1.
        for term in ["CSI", "SGR", "ESC", "DSR", "OSC"] {
            assert!(registry.get(term).is_some(), "Missing tier 1 term: {term}");
        }

        // Tier 2 (sample).
        for term in [
            "ANSI",
            "ASCII",
            "UTF-8",
            "VT-100",
            "xterm",
            "Alacritty",
            "Kitty",
            "tokio",
        ] {
            assert!(registry.get(term).is_some(), "Missing tier 2 term: {term}");
        }
    }
}

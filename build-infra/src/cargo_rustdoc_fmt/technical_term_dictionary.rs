// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Dictionary of known technical terms and their canonical link targets.
//!
//! Builds from two sources:
//! 1. A seed file (`known_technical_term_link_dictionary.jsonc`) embedded at compile time
//!    (or overridden via `--terms-file`)
//! 2. A workspace scan that discovers existing `` [`Term`]: target `` reference
//!    definitions in rustdoc comments

use crate::cargo_rustdoc_fmt::{extractor, types::FormatterResult};
use regex::Regex;
use serde::Deserialize;
use std::{collections::HashMap, path::Path, sync::LazyLock};
use walkdir::WalkDir;

/// Embedded seed file (JSON5 format with comments).
const EMBEDDED_SEED: &str = include_str!("known_technical_term_link_dictionary.jsonc");

/// Terms that are too generic to auto-linkify from workspace scanning.
///
/// These terms appear as common English words and cause false-positive linkification
/// (e.g., "send the signals directly" becomes "send the [`signals`] directly").
/// Only terms discovered via workspace-scanned ref defs (`` [`term`]: url ``)
/// need to be listed here; seed file terms are curated manually.
const WORKSPACE_SCAN_BLOCKLIST: &[&str] = &["pollable", "reset", "signals"];

/// Regex to find reference-style link definitions with backticked names.
///
/// Matches patterns like: `` [`CSI`]: https://example.com ``
/// Captures:
/// - Group 1: term text (e.g., [`CSI`])
/// - Group 2: link target (e.g., `https://example.com` or `crate::Foo`)
///
/// [`CSI`]: crate::CsiSequence
static REF_DEF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[`([^`]+)`\]:\s+(\S+)").expect("Invalid ref def regex")
});

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
    /// Builds the registry from the seed file only (no workspace scan).
    ///
    /// Use this when no workspace root is available, or for testing.
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

    /// Builds the registry from the seed file and workspace scan.
    ///
    /// # Errors
    ///
    /// Returns an error if the seed file cannot be read or parsed.
    pub fn build(
        workspace_root: &Path,
        terms_file: Option<&Path>,
    ) -> FormatterResult<Self> {
        let mut registry = Self::from_seed(terms_file)?;

        // Scan workspace for additional terms not in the seed.
        let discovered = scan_workspace(workspace_root);
        for (term, entry) in discovered {
            // Seed is authoritative - only add terms not already present.
            registry.terms.entry(term).or_insert(entry);
        }

        Ok(registry)
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

/// Scans workspace `.rs` files for reference-style link definitions in rustdoc
/// comments.
///
/// Only discovers external links (URLs starting with `http://` or `https://`).
/// Intra-doc links (containing `::`) are skipped since the seed file handles
/// those.
fn scan_workspace(workspace_root: &Path) -> HashMap<String, TechnicalTermEntry> {
    let mut discovered = HashMap::new();

    for entry in WalkDir::new(workspace_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.path().extension().is_some_and(|ext| ext == "rs")
                && !e.path().to_string_lossy().contains("/target/")
        })
    {
        let Ok(source) = std::fs::read_to_string(entry.path()) else {
            continue;
        };

        let blocks = extractor::extract_rustdoc_blocks(&source);
        for block in &blocks {
            for line in &block.lines {
                if let Some(caps) = REF_DEF_REGEX.captures(line) {
                    let term = caps[1].to_string();
                    let target = caps[2].to_string();

                    // Skip blocklisted generic terms.
                    if WORKSPACE_SCAN_BLOCKLIST.contains(&term.as_str()) {
                        continue;
                    }

                    // Skip intra-doc links (contain ::).
                    if target.contains("::") {
                        continue;
                    }

                    // Only accept URLs.
                    if !target.starts_with("http://") && !target.starts_with("https://") {
                        continue;
                    }

                    // First URL wins for duplicates.
                    discovered.entry(term).or_insert(TechnicalTermEntry {
                        target,
                        tier: TechnicalTermTier::External,
                    });
                }
            }
        }
    }

    discovered
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
        ] {
            assert!(registry.get(term).is_some(), "Missing tier 2 term: {term}");
        }
    }

    #[test]
    fn test_ref_def_regex() {
        let caps = REF_DEF_REGEX.captures("[`CSI`]: https://example.com");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(&caps[1], "CSI");
        assert_eq!(&caps[2], "https://example.com");
    }

    #[test]
    fn test_ref_def_regex_intra_doc() {
        let caps = REF_DEF_REGEX.captures("[`Parser`]: crate::core::Parser");
        assert!(caps.is_some());
        let caps = caps.unwrap();
        assert_eq!(&caps[1], "Parser");
        assert_eq!(&caps[2], "crate::core::Parser");
    }

    #[test]
    fn test_ref_def_regex_no_match() {
        // Not a ref def.
        assert!(
            REF_DEF_REGEX
                .captures("Some text with [`CSI`] link")
                .is_none()
        );
        // Missing backticks.
        assert!(
            REF_DEF_REGEX
                .captures("[CSI]: https://example.com")
                .is_none()
        );
    }

    #[test]
    fn test_blocklist_excludes_generic_terms() {
        // Verify that all blocklisted terms are present.
        let expected_blocklisted = ["pollable", "reset", "signals"];
        for term in expected_blocklisted {
            assert!(
                WORKSPACE_SCAN_BLOCKLIST.contains(&term),
                "Term '{term}' should be in the blocklist"
            );
        }

        // Verify the regex would match each blocklisted term - proving the
        // blocklist is the only thing preventing them from being added.
        for term in expected_blocklisted {
            let line = format!("[`{term}`]: https://example.com/{term}");
            let caps = REF_DEF_REGEX.captures(&line);
            assert!(
                caps.is_some(),
                "Regex should match blocklisted term '{term}'"
            );
            assert_eq!(&caps.unwrap()[1], term);
        }
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Dictionary of known technical terms and their canonical link targets.
//!
//! Builds from the seed file (`known_technical_term_link_dictionary.jsonc`) embedded at
//! compile time, or from a custom file provided via `--terms-file`.

use r3bl_tui::HashMap;
use std::path::Path;

/// Embedded seed file (JSON5 format with comments).
const EMBEDDED_SEED: &str = include_str!("known_technical_term_link_dictionary.jsonc");

/// Dictionary of known technical terms and their canonical link targets.
#[derive(Debug)]
pub struct TechnicalTermDictionary {
    terms: HashMap<String, String>,
}

impl TechnicalTermDictionary {
    /// Builds the registry from the embedded seed file, or from a custom file
    /// provided via `--terms-file`.
    ///
    /// # Errors
    ///
    /// Returns an error if the terms file cannot be read or parsed.
    pub fn from_seed(terms_file: Option<&Path>) -> miette::Result<Self> {
        let seed_content = match terms_file {
            Some(path) => std::fs::read_to_string(path)
                .map_err(|err| miette::miette!("Failed to read terms file: {err}"))?,
            None => EMBEDDED_SEED.to_string(),
        };

        let terms = parse_seed(&seed_content)?;
        Ok(Self { terms })
    }

    /// Looks up the canonical link target for a term.
    #[must_use]
    pub fn get(&self, term: &str) -> Option<&str> {
        self.terms.get(term).map(String::as_str)
    }

    /// Returns all known terms sorted longest-first.
    ///
    /// Longest-first ordering ensures overlapping terms like
    /// `` [`VT-100` spec] `` are matched before [`VT-100`].
    ///
    /// [`VT-100` spec]: https://vt100.net/docs/vt100-ug/chapter3.html
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    #[must_use]
    pub fn terms_longest_first(&self) -> Vec<&str> {
        let mut terms: Vec<&str> = self.terms.keys().map(String::as_str).collect();
        terms.sort_by(|left, right| {
            right.len().cmp(&left.len()).then_with(|| left.cmp(right))
        });
        terms
    }

    /// Returns true if the registry contains no terms.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.terms.is_empty() }
}

/// Parses the seed file content (JSON5 format) into a term-to-target map.
fn parse_seed(content: &str) -> miette::Result<HashMap<String, String>> {
    json5::from_str(content)
        .map_err(|err| miette::miette!("Failed to parse seed file: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_embedded_seed() {
        let registry = TechnicalTermDictionary::from_seed(None).unwrap();
        assert!(!registry.is_empty());

        // Check internal crate terms.
        assert_eq!(registry.get("CSI"), Some("crate::CsiSequence"));
        assert_eq!(registry.get("SGR"), Some("crate::SgrCode"));

        // Check internal crate dependency terms.
        assert_eq!(registry.get("tokio"), Some("tokio"));

        // Check external terms.
        assert_eq!(
            registry.get("ANSI"),
            Some("https://en.wikipedia.org/wiki/ANSI_escape_code")
        );
        assert_eq!(
            registry.get("VT-100"),
            Some("https://vt100.net/docs/vt100-ug/chapter3.html")
        );
    }

    #[test]
    fn test_terms_longest_first() {
        let registry = TechnicalTermDictionary::from_seed(None).unwrap();
        let terms = registry.terms_longest_first();

        // "`VT-100` spec" (14 chars) should come before "VT-100" (6 chars).
        let spec_idx = terms.iter().position(|term| *term == "`VT-100` spec");
        let vt100_idx = terms.iter().position(|term| *term == "VT-100");
        assert!(spec_idx.unwrap() < vt100_idx.unwrap());
    }

    #[test]
    fn test_all_seed_terms_present() {
        let registry = TechnicalTermDictionary::from_seed(None).unwrap();

        // Internal Rust types.
        for term in ["CSI", "SGR", "ESC", "DSR", "OSC", "pty"] {
            assert!(
                registry.get(term).is_some(),
                "Missing internal term: {term}"
            );
        }

        // Stdlib types.
        for term in ["stdout", "stderr", "stdin", "stdio"] {
            assert!(registry.get(term).is_some(), "Missing stdlib term: {term}");
        }

        // Crate dependencies.
        for term in ["crossterm", "mio", "portable_pty"] {
            assert!(registry.get(term).is_some(), "Missing crate term: {term}");
        }

        // External concepts (sample).
        for term in [
            "ANSI",
            "ASCII",
            "UTF-8",
            "VT-100",
            "xterm",
            "Alacritty",
            "Kitty",
            "PTY",
            "tokio",
            "EOF",
            "SIGWINCH",
            "SIGINT",
            "SIGTERM",
            "grapheme",
            "Unicode",
            "RGB",
            "epoll",
            "kqueue",
            "Bracketed paste",
            "truecolor",
            "termios",
        ] {
            assert!(
                registry.get(term).is_some(),
                "Missing external term: {term}"
            );
        }
    }

    #[test]
    fn test_from_seed_custom_file_and_error_branches() {
        let dir = tempfile::TempDir::new().unwrap();
        let valid_path = dir.path().join("terms.jsonc");
        std::fs::write(&valid_path, r#"{ "CSI": "crate::CsiSequence" }"#).unwrap();
        let dict = TechnicalTermDictionary::from_seed(Some(&valid_path)).unwrap();
        assert_eq!(dict.get("CSI"), Some("crate::CsiSequence"));

        // Non-existent file error branch.
        assert!(
            TechnicalTermDictionary::from_seed(Some(Path::new(
                "/nonexistent/terms.jsonc"
            )))
            .is_err()
        );

        // Malformed JSON5 error branch.
        let bad_json_path = dir.path().join("bad_json.jsonc");
        std::fs::write(&bad_json_path, "{ unclosed").unwrap();
        assert!(TechnicalTermDictionary::from_seed(Some(&bad_json_path)).is_err());
    }
}

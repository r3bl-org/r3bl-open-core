// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::core::script::env_source::diff::EnvDiff;

/// Escapes a string for safe inclusion in double-quoted `.env` entries.
#[must_use]
pub fn escape_dotenv_double_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

/// Formats an [`EnvDiff`] into key-value assignments suitable for `.env` files.
///
/// # Behavior
///
/// - Added and modified variables are emitted in alphabetical order as `KEY="VALUE"`.
/// - Values are double-quoted and special characters (`"`, `\`, `\n`, `\r`, `\t`) are
///   escaped.
/// - Removed variables are omitted from the output.
#[must_use]
pub fn format_dotenv(diff: &EnvDiff) -> String {
    let mut out = String::new();

    let mut combined_vars = diff.added.clone();
    for (k, v) in &diff.modified {
        combined_vars.insert(k.clone(), v.clone());
    }

    for (key, val) in &combined_vars {
        out.push_str(key);
        out.push_str("=\"");
        out.push_str(&escape_dotenv_double_quote(val));
        out.push_str("\"\n");
    }

    out
}

#[cfg(test)]
mod tests_dotenv {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_escape_dotenv_double_quote() {
        assert_eq!(escape_dotenv_double_quote("simple"), "simple");
        assert_eq!(
            escape_dotenv_double_quote("quote \"here\""),
            "quote \\\"here\\\""
        );
        assert_eq!(escape_dotenv_double_quote("multi\nline"), "multi\\nline");
    }

    #[test]
    fn test_format_dotenv_basic() {
        let mut added = BTreeMap::new();
        added.insert("KEY".to_string(), "VALUE".to_string());
        added.insert("MULTILINE".to_string(), "line 1\nline 2".to_string());

        let mut modified = BTreeMap::new();
        modified.insert("PATH".to_string(), "/new/path:/old/path".to_string());

        let diff = EnvDiff {
            added,
            modified,
            removed: vec!["OLD_VAR".to_string()],
        };

        let output = format_dotenv(&diff);
        let expected = "KEY=\"VALUE\"\n\
                        MULTILINE=\"line 1\\nline 2\"\n\
                        PATH=\"/new/path:/old/path\"\n";

        assert_eq!(output, expected);
    }
}

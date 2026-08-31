// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::core::script::env_source::diff::EnvDiffChunk;

/// Formats environment diff chunks into key-value assignments suitable for `.env` files.
///
/// # Behavior
///
/// - Added and modified variables are emitted in alphabetical order as `KEY="VALUE"`.
/// - Values are double-quoted and special characters (`"`, `\`, `\n`, `\r`, `\t`) are
///   escaped.
/// - Removed variables are omitted from the output.
#[must_use]
pub fn format_dotenv(chunks: &[EnvDiffChunk]) -> String {
    let mut out = String::new();

    for chunk in chunks {
        match chunk {
            EnvDiffChunk::Add { key, value } | EnvDiffChunk::Modify { key, value } => {
                out.push_str(key);
                out.push_str("=\"");
                out.push_str(&escape_dotenv_double_quote(value));
                out.push_str("\"\n");
            }
            EnvDiffChunk::Remove { .. } => {}
        }
    }

    out
}

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

#[cfg(test)]
mod tests_dotenv {
    use super::*;

    #[test]
    fn test_escape_dotenv_double_quote() {
        assert_eq!(escape_dotenv_double_quote("simple"), "simple");
        assert_eq!(
            escape_dotenv_double_quote("quote \"here\""),
            "quote \\\"here\\\""
        );
        assert_eq!(escape_dotenv_double_quote("multi\nline"), "multi\\nline");
        assert_eq!(
            escape_dotenv_double_quote("tab\tcr\rslash\\"),
            "tab\\tcr\\rslash\\\\"
        );
    }

    #[test]
    fn test_format_dotenv_basic() {
        let diff = vec![
            EnvDiffChunk::Add {
                key: "KEY".to_string(),
                value: "VALUE".to_string(),
            },
            EnvDiffChunk::Add {
                key: "MULTILINE".to_string(),
                value: "line 1\nline 2".to_string(),
            },
            EnvDiffChunk::Modify {
                key: "PATH".to_string(),
                value: "/new/path:/old/path".to_string(),
            },
            EnvDiffChunk::Remove {
                key: "OLD_VAR".to_string(),
            },
        ];

        let output = format_dotenv(&diff);
        let expected = "KEY=\"VALUE\"\n\
                        MULTILINE=\"line 1\\nline 2\"\n\
                        PATH=\"/new/path:/old/path\"\n";

        assert_eq!(output, expected);
    }
}

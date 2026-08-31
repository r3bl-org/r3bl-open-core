// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::core::script::env_source::diff::EnvDiffChunk;

/// Formats environment diff chunks into Fish shell commands.
///
/// # Behavior
///
/// - Standard variables are emitted as `set -gx KEY 'value';`.
/// - `PATH` entries are split by `:` and emitted as separate quoted list arguments: `set
///   -gx PATH '/dir1' '/dir2';`.
/// - Removed variables are emitted as `set -e KEY;`.
///
/// # Shell Integration Example
///
/// In [Fish shell], output can be piped directly into `source`:
///
/// ```fish
/// env-source -i ~/.profile -o fish | source
/// ```
///
/// [Fish shell]: https://fishshell.com
#[must_use]
pub fn format_fish(chunks: &[EnvDiffChunk]) -> String {
    let mut out = String::new();

    for chunk in chunks {
        match chunk {
            EnvDiffChunk::Add { key, value } | EnvDiffChunk::Modify { key, value } => {
                if key == "PATH" {
                    if value.is_empty() {
                        out.push_str("set -gx PATH '';\n");
                    } else {
                        out.push_str("set -gx PATH");
                        for part in value.split(':') {
                            out.push(' ');
                            out.push('\'');
                            out.push_str(&escape_fish_single_quote(part));
                            out.push('\'');
                        }
                        out.push_str(";\n");
                    }
                } else {
                    out.push_str("set -gx ");
                    out.push_str(key);
                    out.push_str(" '");
                    out.push_str(&escape_fish_single_quote(value));
                    out.push_str("';\n");
                }
            }
            EnvDiffChunk::Remove { key } => {
                out.push_str("set -e ");
                out.push_str(key);
                out.push_str(";\n");
            }
        }
    }

    out
}

/// Escapes a string for safe inclusion in single-quoted Fish shell strings.
///
/// In Fish, within single quotes `'...'`, backslashes `\` are escaped as `\\`
/// and single quotes `'` are escaped as `\'`.
#[must_use]
pub fn escape_fish_single_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests_fish {
    use super::*;

    #[test]
    fn test_escape_fish_single_quote() {
        assert_eq!(escape_fish_single_quote("simple"), "simple");
        assert_eq!(escape_fish_single_quote("don't"), "don\\'t");
        assert_eq!(
            escape_fish_single_quote("path\\to\\dir"),
            "path\\\\to\\\\dir"
        );
        assert_eq!(
            escape_fish_single_quote("quote'and\\slash"),
            "quote\\'and\\\\slash"
        );
    }

    #[test]
    fn test_format_fish_basic() {
        let diff = vec![
            EnvDiffChunk::Modify {
                key: "BAZ".to_string(),
                value: "new_val".to_string(),
            },
            EnvDiffChunk::Add {
                key: "FOO".to_string(),
                value: "bar".to_string(),
            },
            EnvDiffChunk::Add {
                key: "WITH_QUOTE".to_string(),
                value: "can't".to_string(),
            },
            EnvDiffChunk::Remove {
                key: "OLD_VAR".to_string(),
            },
        ];

        let output = format_fish(&diff);
        let expected = "set -gx BAZ 'new_val';\n\
                        set -gx FOO 'bar';\n\
                        set -gx WITH_QUOTE 'can\\'t';\n\
                        set -e OLD_VAR;\n";

        assert_eq!(output, expected);
    }

    #[test]
    fn test_format_fish_path() {
        // 1. Colon-separated paths with Add.
        let diff_add = vec![EnvDiffChunk::Add {
            key: "PATH".to_string(),
            value: "/usr/local/bin:/usr/bin:/bin".to_string(),
        }];
        assert_eq!(
            format_fish(&diff_add),
            "set -gx PATH '/usr/local/bin' '/usr/bin' '/bin';\n"
        );

        // 2. Colon-separated paths with Modify.
        let diff_modify = vec![EnvDiffChunk::Modify {
            key: "PATH".to_string(),
            value: "/home/user/bin:/usr/bin".to_string(),
        }];
        assert_eq!(
            format_fish(&diff_modify),
            "set -gx PATH '/home/user/bin' '/usr/bin';\n"
        );

        // 3. Path segments containing single quotes.
        let diff_quotes = vec![EnvDiffChunk::Modify {
            key: "PATH".to_string(),
            value: "/usr/local/bin:/opt/my'dir/bin".to_string(),
        }];
        assert_eq!(
            format_fish(&diff_quotes),
            "set -gx PATH '/usr/local/bin' '/opt/my\\'dir/bin';\n"
        );

        // 4. Empty PATH.
        let diff_empty = vec![EnvDiffChunk::Modify {
            key: "PATH".to_string(),
            value: String::new(),
        }];
        assert_eq!(format_fish(&diff_empty), "set -gx PATH '';\n");
    }
}

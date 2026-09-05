// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::core::script::env_source::diff::EnvDiff;

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

/// Formats an [`EnvDiff`] into Fish shell commands.
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
pub fn format_fish(diff: &EnvDiff) -> String {
    let mut out = String::new();

    // Added and modified variables (sorted by key via BTreeMap).
    let mut combined_vars = diff.added.clone();
    for (k, v) in &diff.modified {
        combined_vars.insert(k.clone(), v.clone());
    }

    for (key, val) in &combined_vars {
        if key == "PATH" {
            if val.is_empty() {
                out.push_str("set -gx PATH '';\n");
            } else {
                out.push_str("set -gx PATH");
                for part in val.split(':') {
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
            out.push_str(&escape_fish_single_quote(val));
            out.push_str("';\n");
        }
    }

    // Removed variables.
    for key in &diff.removed {
        out.push_str("set -e ");
        out.push_str(key);
        out.push_str(";\n");
    }

    out
}

#[cfg(test)]
mod tests_fish {
    use super::*;
    use std::collections::BTreeMap;

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
        let mut added = BTreeMap::new();
        added.insert("FOO".to_string(), "bar".to_string());
        added.insert("WITH_QUOTE".to_string(), "can't".to_string());

        let mut modified = BTreeMap::new();
        modified.insert("BAZ".to_string(), "new_val".to_string());

        let removed = vec!["OLD_VAR".to_string()];

        let diff = EnvDiff {
            added,
            modified,
            removed,
        };

        let output = format_fish(&diff);
        let expected = "set -gx BAZ 'new_val';\n\
                        set -gx FOO 'bar';\n\
                        set -gx WITH_QUOTE 'can\\'t';\n\
                        set -e OLD_VAR;\n";

        assert_eq!(output, expected);
    }

    #[test]
    fn test_format_fish_path() {
        let mut added = BTreeMap::new();
        added.insert(
            "PATH".to_string(),
            "/usr/local/bin:/usr/bin:/bin".to_string(),
        );

        let diff = EnvDiff {
            added,
            modified: BTreeMap::new(),
            removed: vec![],
        };

        let output = format_fish(&diff);
        let expected = "set -gx PATH '/usr/local/bin' '/usr/bin' '/bin';\n";

        assert_eq!(output, expected);
    }
}

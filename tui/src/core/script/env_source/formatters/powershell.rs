// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::core::script::env_source::diff::EnvDiff;

/// Escapes a string for safe inclusion in single-quoted PowerShell strings.
///
/// In PowerShell, within single quotes `'...'`, all characters are treated literally
/// except for the single quote `'`, which is escaped by doubling it (`''`).
#[must_use]
pub fn escape_powershell_single_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("''");
        } else {
            out.push(ch);
        }
    }
    out
}

/// Formats an [`EnvDiff`] into Windows PowerShell commands.
///
/// # Behavior
///
/// - Standard variables are emitted as `$env:KEY = 'value';`.
/// - `PATH` and other environment variables are emitted directly as single-quoted strings
///   (semicolon-delimited on Windows).
/// - Removed variables are emitted as `Remove-Item -Path 'env:KEY' -ErrorAction
///   SilentlyContinue;`.
///
/// # Shell Integration Example
///
/// In [PowerShell], output can be evaluated via `Invoke-Expression`:
///
/// ```powershell
/// env-source -i script.bat -o powershell | Invoke-Expression
/// ```
///
/// [PowerShell]: https://learn.microsoft.com/powershell/
#[must_use]
pub fn format_powershell(diff: &EnvDiff) -> String {
    let mut out = String::new();

    // Added and modified variables (sorted by key via BTreeMap).
    let mut combined_vars = diff.added.clone();
    for (k, v) in &diff.modified {
        combined_vars.insert(k.clone(), v.clone());
    }

    for (key, val) in &combined_vars {
        out.push_str("$env:");
        out.push_str(key);
        out.push_str(" = '");
        out.push_str(&escape_powershell_single_quote(val));
        out.push_str("';\n");
    }

    // Removed variables.
    for key in &diff.removed {
        out.push_str("Remove-Item -Path 'env:");
        out.push_str(&escape_powershell_single_quote(key));
        out.push_str("' -ErrorAction SilentlyContinue;\n");
    }

    out
}

#[cfg(test)]
mod tests_powershell {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_escape_powershell_single_quote() {
        assert_eq!(escape_powershell_single_quote("simple"), "simple");
        assert_eq!(escape_powershell_single_quote("don't"), "don''t");
        assert_eq!(
            escape_powershell_single_quote("C:\\path\\to\\dir"),
            "C:\\path\\to\\dir"
        );
        assert_eq!(
            escape_powershell_single_quote("$dollar and 'quote'"),
            "$dollar and ''quote''"
        );
    }

    #[test]
    fn test_format_powershell_basic() {
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

        let output = format_powershell(&diff);
        let expected = "$env:BAZ = 'new_val';\n\
                        $env:FOO = 'bar';\n\
                        $env:WITH_QUOTE = 'can''t';\n\
                        Remove-Item -Path 'env:OLD_VAR' -ErrorAction SilentlyContinue;\n";

        assert_eq!(output, expected);
    }

    #[test]
    fn test_format_powershell_path() {
        let mut added = BTreeMap::new();
        added.insert(
            "PATH".to_string(),
            "C:\\tools\\bin;C:\\Windows\\System32".to_string(),
        );

        let diff = EnvDiff {
            added,
            modified: BTreeMap::new(),
            removed: vec![],
        };

        let output = format_powershell(&diff);
        let expected = "$env:PATH = 'C:\\tools\\bin;C:\\Windows\\System32';\n";

        assert_eq!(output, expected);
    }
}

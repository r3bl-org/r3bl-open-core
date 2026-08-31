// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::core::script::env_source::diff::EnvDiffChunk;

/// Formats environment diff chunks into PowerShell commands.
///
/// # Behavior
///
/// - Standard variables are emitted as `$env:KEY = 'value';`.
/// - Single quotes within values are escaped by doubling them (`''`).
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
pub fn format_powershell(chunks: &[EnvDiffChunk]) -> String {
    let mut out = String::new();

    for chunk in chunks {
        match chunk {
            EnvDiffChunk::Add { key, value } | EnvDiffChunk::Modify { key, value } => {
                out.push_str("$env:");
                out.push_str(key);
                out.push_str(" = '");
                out.push_str(&escape_powershell_single_quote(value));
                out.push_str("';\n");
            }
            EnvDiffChunk::Remove { key } => {
                out.push_str("Remove-Item -Path 'env:");
                out.push_str(&escape_powershell_single_quote(key));
                out.push_str("' -ErrorAction SilentlyContinue;\n");
            }
        }
    }

    out
}

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

#[cfg(test)]
mod tests_powershell {
    use super::*;

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

        let output = format_powershell(&diff);
        let expected = "$env:BAZ = 'new_val';\n\
                        $env:FOO = 'bar';\n\
                        $env:WITH_QUOTE = 'can''t';\n\
                        Remove-Item -Path 'env:OLD_VAR' -ErrorAction SilentlyContinue;\n";

        assert_eq!(output, expected);
    }

    #[test]
    fn test_format_powershell_path() {
        let diff = vec![EnvDiffChunk::Add {
            key: "PATH".to_string(),
            value: "C:\\tools\\bin;C:\\Windows\\System32".to_string(),
        }];

        let output = format_powershell(&diff);
        let expected = "$env:PATH = 'C:\\tools\\bin;C:\\Windows\\System32';\n";

        assert_eq!(output, expected);
    }
}

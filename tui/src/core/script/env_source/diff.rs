// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{args::OutputFormat,
            filter::filter_env_map,
            formatters::{format_dotenv, format_fish, format_json, format_powershell}};
use crate::EnvMap;

/// Computes the difference between an initial environment and a mutated environment.
///
/// Pre-filters both `initial` and `mutated` environments using [`filter_env_map`] based
/// on the target [`OutputFormat`] before diffing.
///
/// [`filter_env_map`]: crate::filter_env_map
/// [`OutputFormat`]: crate::OutputFormat
#[must_use]
pub fn compute_env_diff(
    mut initial: EnvMap,
    mut mutated: EnvMap,
    format: OutputFormat,
) -> Vec<EnvDiffChunk> {
    filter_env_map(&mut initial, format);
    filter_env_map(&mut mutated, format);

    let mut chunks = Vec::new();

    // Added or Modified: inspect mutated variables against initial.
    for (key, new_val) in &mutated {
        match initial.get(key) {
            None => {
                chunks.push(EnvDiffChunk::Add {
                    key: key.clone(),
                    value: new_val.clone(),
                });
            }
            Some(old_val) if old_val != new_val => {
                chunks.push(EnvDiffChunk::Modify {
                    key: key.clone(),
                    value: new_val.clone(),
                });
            }
            Some(_) => { /* Unchanged */ }
        }
    }

    // Removed: in initial, but missing from mutated.
    for key in initial.keys() {
        if !mutated.contains_key(key) {
            chunks.push(EnvDiffChunk::Remove { key: key.clone() });
        }
    }

    chunks.sort_unstable_by(|left, right| left.key().cmp(right.key()));

    chunks
}

/// Serializes an environment diff into a formatted string using the specified
/// [`OutputFormat`].
#[must_use]
pub fn format_env_diff(chunks: &[EnvDiffChunk], format: OutputFormat) -> String {
    match format {
        OutputFormat::Fish => format_fish(chunks),
        OutputFormat::Powershell => format_powershell(chunks),
        OutputFormat::Json => format_json(chunks),
        OutputFormat::Dotenv => format_dotenv(chunks),
    }
}

/// Represents an individual environment variable mutation chunk.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum EnvDiffChunk {
    /// Environment variable was newly added.
    Add { key: String, value: String },
    /// Environment variable existed and its value was modified.
    Modify { key: String, value: String },
    /// Environment variable existed and was unset or removed.
    Remove { key: String },
}

impl EnvDiffChunk {
    /// Returns the environment variable name.
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            EnvDiffChunk::Add { key, .. }
            | EnvDiffChunk::Modify { key, .. }
            | EnvDiffChunk::Remove { key } => key,
        }
    }

    /// Returns the environment variable value if added or modified, or `None` if removed.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        match self {
            EnvDiffChunk::Add { value, .. } | EnvDiffChunk::Modify { value, .. } => {
                Some(value)
            }
            EnvDiffChunk::Remove { .. } => None,
        }
    }
}

/// Type alias representing an environment diff as a sequence of [`EnvDiffChunk`] items.
///
/// [`EnvDiffChunk`]: crate::EnvDiffChunk
pub type EnvDiff = Vec<EnvDiffChunk>;

#[cfg(test)]
mod tests_diff {
    use super::*;

    #[test]
    fn test_diff_empty() {
        let initial = EnvMap::default();
        let mutated = EnvMap::default();
        let diff = compute_env_diff(initial, mutated, OutputFormat::Fish);

        assert!(diff.is_empty());
    }

    #[test]
    fn test_diff_added_modified_removed() {
        let mut initial = EnvMap::default();
        initial.insert("KEEP_SAME".to_string(), "same_value".to_string());
        initial.insert("WILL_CHANGE".to_string(), "old_value".to_string());
        initial.insert("WILL_DELETE".to_string(), "delete_me".to_string());

        let mut mutated = EnvMap::default();
        mutated.insert("KEEP_SAME".to_string(), "same_value".to_string());
        mutated.insert("WILL_CHANGE".to_string(), "new_value".to_string());
        mutated.insert("NEW_VAR".to_string(), "hello".to_string());

        let diff = compute_env_diff(initial, mutated, OutputFormat::Fish);

        assert_eq!(diff.len(), 3);
        assert_eq!(
            diff,
            vec![
                EnvDiffChunk::Add {
                    key: "NEW_VAR".to_string(),
                    value: "hello".to_string(),
                },
                EnvDiffChunk::Modify {
                    key: "WILL_CHANGE".to_string(),
                    value: "new_value".to_string(),
                },
                EnvDiffChunk::Remove {
                    key: "WILL_DELETE".to_string(),
                },
            ]
        );

        // Verify key() and value() helper methods on chunks.
        assert_eq!(diff[0].key(), "NEW_VAR");
        assert_eq!(diff[0].value(), Some("hello"));

        assert_eq!(diff[1].key(), "WILL_CHANGE");
        assert_eq!(diff[1].value(), Some("new_value"));

        assert_eq!(diff[2].key(), "WILL_DELETE");
        assert_eq!(diff[2].value(), None);
    }

    #[test]
    fn test_diff_filters_out_internal_vars() {
        let mut initial = EnvMap::default();
        initial.insert("PWD".to_string(), "/home/old".to_string());
        initial.insert("SHLVL".to_string(), "1".to_string());
        initial.insert("VAR".to_string(), "val".to_string());

        let mut mutated = EnvMap::default();
        mutated.insert("PWD".to_string(), "/home/new".to_string());
        mutated.insert("SHLVL".to_string(), "2".to_string());
        mutated.insert("PS1".to_string(), "prompt> ".to_string());
        mutated.insert("BASH_FUNC_test%%".to_string(), "() { echo; }".to_string());
        mutated.insert("VAR".to_string(), "val2".to_string());

        let diff = compute_env_diff(initial, mutated, OutputFormat::Fish);

        assert_eq!(
            diff,
            vec![EnvDiffChunk::Modify {
                key: "VAR".to_string(),
                value: "val2".to_string(),
            }]
        );
    }

    #[test]
    fn test_diff_format_aware_filtering_fish_vs_other_formats() {
        let initial = EnvMap::default();
        let mut mutated = EnvMap::default();
        mutated.insert("version".to_string(), "1.2.3".to_string());
        mutated.insert("status".to_string(), "ready".to_string());
        mutated.insert("PWD".to_string(), "/home".to_string());

        // For Fish: status and version are dropped alongside PWD.
        let diff_fish =
            compute_env_diff(initial.clone(), mutated.clone(), OutputFormat::Fish);
        assert!(diff_fish.is_empty());

        // For PowerShell, Dotenv, and JSON: status and version are preserved, PWD
        // dropped.
        for format in [
            OutputFormat::Powershell,
            OutputFormat::Dotenv,
            OutputFormat::Json,
        ] {
            let diff = compute_env_diff(initial.clone(), mutated.clone(), format);
            assert_eq!(diff.len(), 2);
            assert_eq!(diff[0].key(), "status");
            assert_eq!(diff[0].value(), Some("ready"));
            assert_eq!(diff[1].key(), "version");
            assert_eq!(diff[1].value(), Some("1.2.3"));
        }
    }

    #[test]
    fn test_format_env_diff() {
        let diff = vec![
            EnvDiffChunk::Add {
                key: "FOO".to_string(),
                value: "bar".to_string(),
            },
            EnvDiffChunk::Remove {
                key: "OLD_VAR".to_string(),
            },
        ];

        let fish_out = format_env_diff(&diff, OutputFormat::Fish);
        assert!(fish_out.contains("set -gx FOO 'bar';"));
        assert!(fish_out.contains("set -e OLD_VAR;"));

        let ps_out = format_env_diff(&diff, OutputFormat::Powershell);
        assert!(ps_out.contains("$env:FOO = 'bar';"));
        assert!(ps_out.contains("Remove-Item -Path 'env:OLD_VAR'"));

        let dotenv_out = format_env_diff(&diff, OutputFormat::Dotenv);
        assert!(dotenv_out.contains("FOO=\"bar\""));
        assert!(!dotenv_out.contains("OLD_VAR"));

        let json_out = format_env_diff(&diff, OutputFormat::Json);
        assert!(json_out.contains("\"action\": \"add\""));
        assert!(json_out.contains("\"action\": \"remove\""));
    }
}

// cspell:words SHLVL

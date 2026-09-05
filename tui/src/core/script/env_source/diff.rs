// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{args::OutputFormat,
            filter::is_filtered_variable,
            formatters::{format_dotenv, format_fish, format_json, format_powershell}};
use crate::{EnvMap, HashMap};
use std::collections::BTreeMap;

/// Represents the delta between an initial environment and a mutated environment.
///
/// Uses [`BTreeMap`] for `added` and `modified` and a sorted [`Vec<String>`] for
/// `removed` to guarantee deterministic ordering in all formatters and test assertions.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct EnvDiff {
    /// Variables that exist in the mutated environment but did not exist in the
    /// initial environment.
    pub added: BTreeMap<String, String>,
    /// Variables that existed in both environments but whose value changed.
    pub modified: BTreeMap<String, String>,
    /// Variables that existed in the initial environment but were unset or removed.
    pub removed: Vec<String>,
}

impl EnvDiff {
    /// Returns `true` if there are no added, modified, or removed environment variables.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }

    /// Computes the difference between an initial environment and a mutated environment.
    ///
    /// Internal and read-only variables (as determined by [`is_filtered_variable`]) are
    /// ignored.
    ///
    /// [`is_filtered_variable`]: crate::is_filtered_variable
    #[must_use]
    pub fn compute(initial: EnvMap, mutated: EnvMap) -> Self {
        #[cfg(windows)]
        {
            Self::compute_windows(initial, mutated)
        }
        #[cfg(not(windows))]
        {
            Self::compute_unix(initial, mutated)
        }
    }

    /// Case-sensitive environment diffing for Unix platforms.
    ///
    /// [`is_filtered_variable`]: crate::is_filtered_variable
    #[must_use]
    pub fn compute_unix(mut initial: EnvMap, mutated: EnvMap) -> Self {
        let mut added = BTreeMap::new();
        let mut modified = BTreeMap::new();
        let mut removed = Vec::new();

        // Check for added or modified variables.
        for (key, val) in mutated {
            if is_filtered_variable(&key) {
                continue;
            }

            match initial.remove(&key) {
                Some(initial_val) => {
                    if initial_val != val {
                        modified.insert(key, val);
                    }
                }
                None => {
                    added.insert(key, val);
                }
            }
        }

        // Check for removed variables.
        for (key, _) in initial {
            if is_filtered_variable(&key) {
                continue;
            }
            removed.push(key);
        }

        removed.sort();

        Self {
            added,
            modified,
            removed,
        }
    }

    /// Case-insensitive environment diffing for Windows.
    ///
    /// [`is_filtered_variable`]: crate::is_filtered_variable
    #[must_use]
    pub fn compute_windows(initial: EnvMap, mutated: EnvMap) -> Self {
        let mut added = BTreeMap::new();
        let mut modified = BTreeMap::new();
        let mut removed = Vec::new();

        let mut initial_upper: HashMap<String, (String, String)> = initial
            .into_iter()
            .map(|(k, v)| (k.to_ascii_uppercase(), (k, v)))
            .collect();

        // Check for added or modified variables.
        for (key, val) in mutated {
            if is_filtered_variable(&key) {
                continue;
            }

            let key_upper = key.to_ascii_uppercase();
            match initial_upper.remove(&key_upper) {
                Some((_initial_key, initial_val)) => {
                    if initial_val != val {
                        modified.insert(key, val);
                    }
                }
                None => {
                    added.insert(key, val);
                }
            }
        }

        // Check for removed variables.
        for (_key_upper, (key, _val)) in initial_upper {
            if is_filtered_variable(&key) {
                continue;
            }
            removed.push(key);
        }

        removed.sort();

        Self {
            added,
            modified,
            removed,
        }
    }

    /// Serializes this environment diff into a formatted string using the specified
    /// [`OutputFormat`].
    #[must_use]
    pub fn serialize_to_string(&self, format: OutputFormat) -> String {
        match format {
            OutputFormat::Fish => format_fish(self),
            OutputFormat::Powershell => format_powershell(self),
            OutputFormat::Json => format_json(self),
            OutputFormat::Dotenv => format_dotenv(self),
        }
    }

    /// Applies this diff directly to the current process's environment variables.
    ///
    /// Adds new variables, updates modified variables, and unsets removed variables.
    pub fn apply_to_current_process(&self) {
        for (key, val) in &self.added {
            // Safety: Modifies current process environment variables.
            unsafe {
                std::env::set_var(key, val);
            }
        }
        for (key, val) in &self.modified {
            // Safety: Modifies current process environment variables.
            unsafe {
                std::env::set_var(key, val);
            }
        }
        for key in &self.removed {
            // Safety: Modifies current process environment variables.
            unsafe {
                std::env::remove_var(key);
            }
        }
    }
}

#[cfg(test)]
mod tests_diff {
    use super::*;

    #[test]
    fn test_diff_empty() {
        let initial = EnvMap::default();
        let mutated = EnvMap::default();
        let diff = EnvDiff::compute(initial, mutated);

        assert!(diff.is_empty());
        assert!(diff.added.is_empty());
        assert!(diff.modified.is_empty());
        assert!(diff.removed.is_empty());
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

        let diff = EnvDiff::compute(initial, mutated);

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added.get("NEW_VAR"), Some(&"hello".to_string()));

        assert_eq!(diff.modified.len(), 1);
        assert_eq!(
            diff.modified.get("WILL_CHANGE"),
            Some(&"new_value".to_string())
        );

        assert_eq!(diff.removed, vec!["WILL_DELETE".to_string()]);
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

        let diff = EnvDiff::compute(initial, mutated);

        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified.get("VAR"), Some(&"val2".to_string()));
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_diff_windows_case_insensitivity() {
        let mut initial = EnvMap::default();
        initial.insert("Path".to_string(), "C:\\Windows".to_string());
        initial.insert("USERPROFILE".to_string(), "C:\\Users\\test".to_string());

        let mut mutated = EnvMap::default();
        mutated.insert("PATH".to_string(), "C:\\Windows;C:\\tools".to_string());
        mutated.insert("userprofile".to_string(), "C:\\Users\\test".to_string());
        mutated.insert("NEW_VAR".to_string(), "val".to_string());

        let diff = EnvDiff::compute_windows(initial, mutated);

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added.get("NEW_VAR"), Some(&"val".to_string()));

        assert_eq!(diff.modified.len(), 1);
        assert_eq!(
            diff.modified.get("PATH"),
            Some(&"C:\\Windows;C:\\tools".to_string())
        );

        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_diff_serialize_to_string() {
        let mut diff = EnvDiff::default();
        diff.added.insert("FOO".to_string(), "bar".to_string());

        let fish_out = diff.serialize_to_string(OutputFormat::Fish);
        assert!(fish_out.contains("set -gx FOO 'bar';"));

        let ps_out = diff.serialize_to_string(OutputFormat::Powershell);
        assert!(ps_out.contains("$env:FOO = 'bar';"));

        let dotenv_out = diff.serialize_to_string(OutputFormat::Dotenv);
        assert!(dotenv_out.contains("FOO=\"bar\""));

        let json_out = diff.serialize_to_string(OutputFormat::Json);
        assert!(json_out.contains("\"added\":"));
        assert!(json_out.contains("\"FOO\": \"bar\""));
    }
}

// cspell:words SHLVL USERPROFILE userprofile

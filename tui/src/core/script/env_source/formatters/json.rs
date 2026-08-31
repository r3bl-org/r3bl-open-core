// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::core::script::env_source::diff::EnvDiff;

/// Formats an [`EnvDiff`] into a pretty-printed JSON object string.
///
/// # Output Schema
///
/// Serializes the diff into a JSON object containing three fields:
/// - `added`: Key-value map of newly introduced environment variables.
/// - `modified`: Key-value map of updated environment variables.
/// - `removed`: Array of deleted variable keys.
///
/// # Panics
///
/// Panics if serialization of [`EnvDiff`] to JSON fails (which cannot occur in practice
/// since [`EnvDiff`] contains only valid strings and collections).
#[must_use]
pub fn format_json(diff: &EnvDiff) -> String {
    let mut s = serde_json::to_string_pretty(diff)
        .expect("EnvDiff serialization to JSON cannot fail");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests_json {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_format_json() {
        let mut added = BTreeMap::new();
        added.insert("KEY".to_string(), "VALUE".to_string());

        let mut modified = BTreeMap::new();
        modified.insert("PATH".to_string(), "/new/path:/old/path".to_string());

        let removed = vec!["OLD_KEY".to_string()];

        let diff = EnvDiff {
            added,
            modified,
            removed,
        };

        let json_str = format_json(&diff);
        let expected = "{\n  \"added\": {\n    \"KEY\": \"VALUE\"\n  },\n  \"modified\": {\n    \"PATH\": \"/new/path:/old/path\"\n  },\n  \"removed\": [\n    \"OLD_KEY\"\n  ]\n}\n";

        assert_eq!(json_str, expected);
    }
}

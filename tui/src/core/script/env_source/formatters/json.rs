// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::core::script::env_source::diff::EnvDiffChunk;

/// Formats environment diff chunks into a pretty-printed JSON array string.
///
/// # Output Schema
///
/// Serializes the diff into a JSON array of [`EnvDiffChunk`] objects, each tagged
/// with its `action` (`add`, `modify`, or `remove`).
///
/// # Panics
///
/// Panics if serialization of [`EnvDiffChunk`] to JSON fails (which cannot occur in
/// practice since [`EnvDiffChunk`] contains only valid strings).
///
/// [`EnvDiffChunk`]: crate::EnvDiffChunk
#[must_use]
pub fn format_json(chunks: &[EnvDiffChunk]) -> String {
    let mut output = serde_json::to_string_pretty(chunks)
        .expect("EnvDiff serialization to JSON cannot fail");
    output.push('\n');
    output
}

#[cfg(test)]
mod tests_json {
    use super::*;

    #[test]
    fn test_format_json() {
        let diff = vec![
            EnvDiffChunk::Add {
                key: "KEY".to_string(),
                value: "VALUE".to_string(),
            },
            EnvDiffChunk::Modify {
                key: "PATH".to_string(),
                value: "/new/path:/old/path".to_string(),
            },
            EnvDiffChunk::Remove {
                key: "OLD_KEY".to_string(),
            },
        ];

        let json_str = format_json(&diff);
        let expected = "[\n  {\n    \"action\": \"add\",\n    \"key\": \"KEY\",\n    \"value\": \"VALUE\"\n  },\n  {\n    \"action\": \"modify\",\n    \"key\": \"PATH\",\n    \"value\": \"/new/path:/old/path\"\n  },\n  {\n    \"action\": \"remove\",\n    \"key\": \"OLD_KEY\"\n  }\n]\n";

        assert_eq!(json_str, expected);
    }
}

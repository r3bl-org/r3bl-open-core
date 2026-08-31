// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Set of variables that are internal to shells or read-only in Fish shell.
/// These should be excluded from environment diffs.
pub const FILTERED_EXACT_VARS: &[&str] = &[
    "PWD",
    "SHLVL",
    "history",
    "pipestatus",
    "status",
    "version",
    "FISH_VERSION",
    "fish_pid",
    "hostname",
    "_",
    "fish_private_mode",
    "PS1",
    "PROMPT",
    "XPC_SERVICE_NAME",
];

/// Prefix for Bash exported functions which should be excluded from diffs.
pub const BASH_FUNC_PREFIX: &str = "BASH_FUNC_";

/// Determines whether an environment variable key should be filtered out
/// (ignored) from environment diffing and output formatting.
#[must_use]
pub fn is_filtered_variable(key: &str) -> bool {
    if key.starts_with('=') {
        return true;
    }
    if FILTERED_EXACT_VARS.contains(&key) {
        return true;
    }
    if key.starts_with(BASH_FUNC_PREFIX) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests_filter {
    use super::*;

    #[test]
    fn test_is_filtered_variable() {
        assert!(is_filtered_variable("=C:"));
        assert!(is_filtered_variable("=::"));
        assert!(is_filtered_variable("=ExitCode"));
        assert!(is_filtered_variable("PWD"));
        assert!(is_filtered_variable("SHLVL"));
        assert!(is_filtered_variable("_"));
        assert!(is_filtered_variable("PS1"));
        assert!(is_filtered_variable("XPC_SERVICE_NAME"));
        assert!(is_filtered_variable("fish_pid"));
        assert!(is_filtered_variable("BASH_FUNC_my_func%%"));

        assert!(!is_filtered_variable("PATH"));
        assert!(!is_filtered_variable("HOME"));
        assert!(!is_filtered_variable("USER"));
        assert!(!is_filtered_variable("MY_CUSTOM_VAR"));
    }
}

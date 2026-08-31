// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{args::OutputFormat, parser::EQUAL_DELIM};
use crate::EnvMap;

/// Removes internal shell variables and format-specific read-only variables from an
/// [`EnvMap`].
///
/// Modifies the map in place using [`EnvMap::retain`] and [`FilterAction`].
///
/// # Filtering Strategy
///
/// Filtering is split into two distinct tiers:
///
/// 1. **Universal Subshell Noise (All Formats & Platforms)**: Spawning an isolated shell
///    (`/bin/sh` or `cmd.exe`) produces transient subshell artifacts that are not
///    intentional environment modifications. These variables are **always dropped**
///    across all [`OutputFormat`]s:
///    - `SHLVL` is incremented by each subshell invocation.
///    - `_` records the last command or binary executed.
///    - `PS1` and `PROMPT` define interactive prompt layouts.
///    - `CMDCMDLINE` records `cmd.exe`'s launch arguments.
///    - `BASH_FUNC_*` contains exported Bash function definitions.
///    - Windows dynamic variables starting with `'='` (such as `=C:` and `=ExitCode`)
///      record drive-specific working directories and process exit codes.
///
/// 2. **Target Shell Restrictions ([Fish] Only)**: In [Fish], variables such as `status`,
///    `pipestatus`, `fish_pid`, `history`, `version`, and `FISH_VERSION` are hardcoded
///    built-in read-only variables. Sourcing commands like `set -gx status 0` or `set -gx
///    fish_pid 123` causes Fish to fail with a runtime error (`set: Variable 'status' is
///    read-only`). These variables are **only dropped** when `format` is
///    [`OutputFormat::Fish`].
///
/// Other formats ([PowerShell], [dotenv], [JSON]) do not enforce read-only restrictions
/// on these variable names, allowing toolchains that set variables like `VERSION` or
/// `STATUS` to be preserved.
///
/// This filtering mirrors the logic used by [`bass` in its `__bass.py`
/// helper][bass-source] when generating [Fish] scripts.
///
/// [`bass`]: https://github.com/edc/bass
/// [`EnvMap::retain`]: std::collections::HashMap::retain
/// [`EnvMap`]: crate::EnvMap
/// [`FilterAction::classify`]: FilterAction::classify
/// [`FilterAction`]: FilterAction
/// [`OutputFormat::Fish`]: crate::OutputFormat::Fish
/// [`OutputFormat`]: crate::OutputFormat
/// [bass-source]: https://github.com/edc/bass/blob/master/functions/__bass.py
/// [dotenv]: https://github.com/motdotla/dotenv
/// [Fish]: https://fishshell.com/
/// [JSON]: https://www.json.org/
/// [PowerShell]: https://learn.microsoft.com/powershell/
pub fn filter_env_map(map: &mut EnvMap, format: OutputFormat) {
    map.retain(|key, _| FilterAction::classify(key, format) == FilterAction::Keep);
}

/// Action indicating whether an environment variable should be kept in or dropped from
/// the environment diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterAction {
    /// Keep this variable in the environment diff.
    Keep,
    /// Drop this variable from the environment diff (internal shell noise or read-only).
    Drop,
}

impl FilterAction {
    /// Classifies an environment variable key as either [`FilterAction::Keep`] or
    /// [`FilterAction::Drop`] based on the target [`OutputFormat`].
    ///
    /// # Classification Strategy
    ///
    /// 1. **Universal Subshell Noise**:
    ///    - Dynamic Windows drive or exit code variables starting with [`EQUAL_DELIM`].
    ///    - Exported Bash function variables starting with [`BASH_FUNC_PREFIX`].
    ///    - Transient subshell artifacts listed in [`SUBSHELL_NOISE_VARS`] (such as
    ///      `PWD`, `SHLVL`, `_`, `PS1`, `PROMPT`, `CMDCMDLINE`, `hostname`,
    ///      `XPC_SERVICE_NAME`).
    ///
    /// 2. **Target Shell Restrictions ([Fish] Only)**:
    ///    - In [Fish], hardcoded built-in read-only keywords listed in
    ///      [`FISH_READONLY_VARS`] (such as `status`, `pipestatus`, `fish_pid`,
    ///      `history`, `version`, `FISH_VERSION`, `fish_private_mode`) are dropped to
    ///      prevent fatal runtime errors when sourcing `set -gx`.
    ///    - For all other formats ([PowerShell], [dotenv], [JSON]), these variables are
    ///      preserved.
    ///
    /// [`BASH_FUNC_PREFIX`]: BASH_FUNC_PREFIX
    /// [`EQUAL_DELIM`]: EQUAL_DELIM
    /// [`FilterAction::Drop`]: FilterAction::Drop
    /// [`FilterAction::Keep`]: FilterAction::Keep
    /// [`FISH_READONLY_VARS`]: FISH_READONLY_VARS
    /// [`OutputFormat::Fish`]: crate::OutputFormat::Fish
    /// [`OutputFormat`]: crate::OutputFormat
    /// [`SUBSHELL_NOISE_VARS`]: SUBSHELL_NOISE_VARS
    /// [dotenv]: https://github.com/motdotla/dotenv
    /// [Fish]: https://fishshell.com/
    /// [JSON]: https://www.json.org/
    /// [PowerShell]: https://learn.microsoft.com/powershell/
    #[must_use]
    pub fn classify(key: &str, format: OutputFormat) -> Self {
        let is_universal_noise = key.starts_with(EQUAL_DELIM)
            || key.starts_with(BASH_FUNC_PREFIX)
            || SUBSHELL_NOISE_VARS.contains(&key);

        let is_fish_readonly =
            format == OutputFormat::Fish && FISH_READONLY_VARS.contains(&key);

        if is_universal_noise || is_fish_readonly {
            Self::Drop
        } else {
            Self::Keep
        }
    }
}

/// Transient subshell noise generated by `/bin/sh` or `cmd.exe` that should be excluded
/// from all environment diffs regardless of output format.
pub const SUBSHELL_NOISE_VARS: &[&str] = &[
    "PWD",
    "SHLVL",
    "_",
    "PS1",
    "PROMPT",
    "CMDCMDLINE",
    "hostname",
    "XPC_SERVICE_NAME",
];

/// Variables that are hardcoded built-in read-only keywords in [Fish] shell.
/// These must be excluded when formatting for [`OutputFormat::Fish`].
///
/// [`OutputFormat::Fish`]: crate::OutputFormat::Fish
/// [Fish]: https://fishshell.com/
pub const FISH_READONLY_VARS: &[&str] = &[
    "history",
    "pipestatus",
    "status",
    "version",
    "FISH_VERSION",
    "fish_pid",
    "fish_private_mode",
];

/// Prefix for Bash exported functions which should be excluded from diffs.
pub const BASH_FUNC_PREFIX: &str = "BASH_FUNC_";

#[cfg(test)]
mod tests_filter {
    use super::*;

    #[test]
    fn test_filter_action_classify_universal_noise() {
        for format in [
            OutputFormat::Fish,
            OutputFormat::Powershell,
            OutputFormat::Dotenv,
            OutputFormat::Json,
        ] {
            assert_eq!(FilterAction::classify("=C:", format), FilterAction::Drop);
            assert_eq!(FilterAction::classify("=::", format), FilterAction::Drop);
            assert_eq!(
                FilterAction::classify("=ExitCode", format),
                FilterAction::Drop
            );
            assert_eq!(
                FilterAction::classify("BASH_FUNC_my_func%%", format),
                FilterAction::Drop
            );

            // Universal subshell noise is dropped across all formats.
            for &noise_var in SUBSHELL_NOISE_VARS {
                assert_eq!(
                    FilterAction::classify(noise_var, format),
                    FilterAction::Drop
                );
            }

            // User variables are kept across all formats.
            assert_eq!(FilterAction::classify("PATH", format), FilterAction::Keep);
            assert_eq!(FilterAction::classify("HOME", format), FilterAction::Keep);
            assert_eq!(FilterAction::classify("USER", format), FilterAction::Keep);
            assert_eq!(
                FilterAction::classify("MY_CUSTOM_VAR", format),
                FilterAction::Keep
            );

            // Uppercase variables are kept across all formats (Fish read-only vars are
            // lowercase).
            assert_eq!(
                FilterAction::classify("VERSION", format),
                FilterAction::Keep
            );
            assert_eq!(FilterAction::classify("STATUS", format), FilterAction::Keep);
        }
    }

    #[test]
    fn test_filter_action_classify_fish_readonly_only_dropped_for_fish() {
        for &fish_var in FISH_READONLY_VARS {
            // Dropped for Fish.
            assert_eq!(
                FilterAction::classify(fish_var, OutputFormat::Fish),
                FilterAction::Drop
            );

            // Kept for PowerShell, Dotenv, JSON.
            assert_eq!(
                FilterAction::classify(fish_var, OutputFormat::Powershell),
                FilterAction::Keep
            );
            assert_eq!(
                FilterAction::classify(fish_var, OutputFormat::Dotenv),
                FilterAction::Keep
            );
            assert_eq!(
                FilterAction::classify(fish_var, OutputFormat::Json),
                FilterAction::Keep
            );
        }
    }

    #[test]
    fn test_filter_env_map() {
        let mut template = EnvMap::default();
        template.insert("PATH".to_string(), "/bin".to_string());
        template.insert("PWD".to_string(), "/home".to_string());
        template.insert("SHLVL".to_string(), "1".to_string());
        template.insert("=C:".to_string(), "C:\\".to_string());
        template.insert("status".to_string(), "0".to_string());
        template.insert("version".to_string(), "1.0".to_string());
        template.insert("BASH_FUNC_foo%%".to_string(), "() { echo; }".to_string());

        // For Fish: status and version are dropped alongside universal noise.
        let mut map_fish = template.clone();
        filter_env_map(&mut map_fish, OutputFormat::Fish);
        assert_eq!(map_fish.len(), 1);
        assert!(map_fish.contains_key("PATH"));
        assert!(!map_fish.contains_key("status"));
        assert!(!map_fish.contains_key("version"));

        // For PowerShell, Dotenv, and JSON: status and version are preserved.
        for format in [
            OutputFormat::Powershell,
            OutputFormat::Dotenv,
            OutputFormat::Json,
        ] {
            let mut map = template.clone();
            filter_env_map(&mut map, format);
            assert_eq!(map.len(), 3);
            assert!(map.contains_key("PATH"));
            assert!(map.contains_key("status"));
            assert!(map.contains_key("version"));
            assert!(!map.contains_key("PWD"));
            assert!(!map.contains_key("SHLVL"));
            assert!(!map.contains_key("=C:"));
            assert!(!map.contains_key("BASH_FUNC_foo%%"));
        }
    }
}

// cspell:words SHLVL pipestatus CMDCMDLINE

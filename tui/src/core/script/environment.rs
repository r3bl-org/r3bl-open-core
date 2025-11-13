// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{InlineString, SCRIPT_MOD_DEBUG, inline_string, ok};
use miette::IntoDiagnostic;
use std::{env, path::Path};
use strum_macros::{Display, EnumString};

#[cfg(target_os = "windows")]
const OS_SPECIFIC_ENV_PATH_SEPARATOR: &str = ";";
#[cfg(not(target_os = "windows"))]
const OS_SPECIFIC_ENV_PATH_SEPARATOR: &str = ":";

#[derive(Debug, Display, EnumString, Copy, Clone, PartialEq, Eq)]
pub enum EnvKeys {
    #[strum(serialize = "PATH")]
    Path,
}

pub type EnvVars = Vec<(String, String)>;
pub type EnvVarsSlice<'a> = &'a [(String, String)];

/// Returns the `PATH` and given value as a vector of tuple.
///
/// # Example
///
/// ```
/// use r3bl_tui::{gen_path_env_vars, EnvKeys};
///
/// let path_envs = gen_path_env_vars("/usr/bin");
/// let expected = vec![
///     ("PATH".to_string(), "/usr/bin".to_string())
/// ];
/// assert_eq!(path_envs, expected);
/// ```
///
/// # Example of using the returned value as a slice
///
/// The returned value can also be passed around as a `&[(String, String)]`.
///
/// ```
/// use r3bl_tui::{gen_path_env_vars, EnvVars, EnvVarsSlice, EnvKeys};
///
/// let path_envs: EnvVars = gen_path_env_vars("/usr/bin");
/// let path_envs_ref: EnvVarsSlice = &path_envs;
/// let path_envs_ref_2 = path_envs.as_slice();
/// let path_envs_ref_clone = path_envs_ref.to_owned();
/// assert_eq!(path_envs_ref, path_envs_ref_clone);
/// assert_eq!(path_envs_ref, path_envs_ref_2);
/// ```
#[must_use]
pub fn gen_path_env_vars(path_value: &str) -> EnvVars {
    vec![(EnvKeys::Path.to_string(), path_value.to_string())]
}

/// # Errors
///
/// Returns an error if the environment variable is not set.
pub fn try_get(key: EnvKeys) -> miette::Result<String> {
    env::var(key.to_string()).into_diagnostic()
}

/// # Errors
///
/// Returns an error if the PATH environment variable is not set.
pub fn try_get_path_prefixed(
    prefix_path: impl AsRef<Path>,
) -> miette::Result<InlineString> {
    let path = try_get(EnvKeys::Path)?;
    let add_to_path = inline_string!(
        "{}{}{}",
        prefix_path.as_ref().display(),
        OS_SPECIFIC_ENV_PATH_SEPARATOR,
        path
    );
    SCRIPT_MOD_DEBUG.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "try_get_path_prefixed",
            add_to_path = %add_to_path
        );
    });

    ok!(add_to_path)
}

#[cfg(test)]
mod tests_environment {
    use super::*;

    #[test]
    fn test_try_get_path_from_env() {
        let path = try_get(EnvKeys::Path).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_try_get() {
        let path = try_get(EnvKeys::Path).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_get_path_envs() {
        let path_envs = gen_path_env_vars("/usr/bin");
        let expected = vec![("PATH".to_string(), "/usr/bin".to_string())];
        assert_eq!(path_envs, expected);
    }

    #[test]
    fn test_get_path() {
        let path = try_get(EnvKeys::Path).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_get_path_prefixed() {
        let prefix_path = "/usr/bin";
        let path = try_get_path_prefixed(prefix_path).unwrap();
        assert!(!path.is_empty());
        assert!(path.starts_with(prefix_path));
    }
}

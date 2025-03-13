/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::{env, path::Path};

use miette::IntoDiagnostic;
use r3bl_core::{InlineString, inline_string, ok};
use strum_macros::{Display, EnumString};

use crate::SCRIPT_MOD_DEBUG;

#[cfg(target_os = "windows")]
const OS_SPECIFIC_ENV_PATH_SEPARATOR: &str = ";";
#[cfg(not(target_os = "windows"))]
const OS_SPECIFIC_ENV_PATH_SEPARATOR: &str = ":";

#[derive(Debug, Display, EnumString)]
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
/// use r3bl_script::environment::{gen_path_env_vars, EnvKeys};
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
/// use r3bl_script::environment::{gen_path_env_vars, EnvVars, EnvVarsSlice, EnvKeys};
///
/// let path_envs: EnvVars = gen_path_env_vars("/usr/bin");
/// let path_envs_ref: EnvVarsSlice = &path_envs;
/// let path_envs_ref_2 = path_envs.as_slice();
/// let path_envs_ref_clone = path_envs_ref.to_owned();
/// assert_eq!(path_envs_ref, path_envs_ref_clone);
/// assert_eq!(path_envs_ref, path_envs_ref_2);
/// ```
pub fn gen_path_env_vars(path_value: &str) -> EnvVars {
    vec![(EnvKeys::Path.to_string(), path_value.to_string())]
}

pub fn try_get(key: EnvKeys) -> miette::Result<String> {
    env::var(key.to_string()).into_diagnostic()
}

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
    use crate::environment;

    #[test]
    fn test_try_get_path_from_env() {
        let path = environment::try_get(EnvKeys::Path).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_try_get() {
        let path = environment::try_get(EnvKeys::Path).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_get_path_envs() {
        let path_envs = environment::gen_path_env_vars("/usr/bin");
        let expected = vec![("PATH".to_string(), "/usr/bin".to_string())];
        assert_eq!(path_envs, expected);
    }

    #[test]
    fn test_get_path() {
        let path = environment::try_get(EnvKeys::Path).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_get_path_prefixed() {
        let prefix_path = "/usr/bin";
        let path = environment::try_get_path_prefixed(prefix_path).unwrap();
        assert!(!path.is_empty());
        assert!(path.starts_with(prefix_path));
    }
}

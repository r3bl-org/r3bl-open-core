// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test harness scaffolding and mock environment builders for `env-source` conformance
//! tests.

use crate::{EnvMap,
            core::script::env_source::{BaseEnv, OutputFormat, try_env_source},
            try_create_temp_dir};
use std::io::Write;

/// Generates a deterministic hermetic mock initial environment for Unix.
#[must_use]
pub fn create_mock_initial_env_unix() -> EnvMap {
    let mut map = EnvMap::default();
    map.insert("HOME".to_string(), "/home/testuser".to_string());
    map.insert("USER".to_string(), "testuser".to_string());
    map.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
    map
}

/// Generates a deterministic mock initial environment for Unix with an existing variable
/// to be removed or unset.
#[must_use]
pub fn create_mock_initial_env_unix_with_existing_var() -> EnvMap {
    let mut map = create_mock_initial_env_unix();
    map.insert("EXISTING_VAR".to_string(), "to_be_deleted".to_string());
    map
}

/// Generates a deterministic hermetic mock initial environment for Windows.
#[must_use]
pub fn create_mock_initial_env_windows() -> EnvMap {
    let mut map = EnvMap::default();
    map.insert("USERPROFILE".to_string(), r"C:\Users\testuser".to_string());
    map.insert(
        "PATH".to_string(),
        r"C:\Windows\system32;C:\Windows".to_string(),
    );
    map.insert(
        "ComSpec".to_string(),
        r"C:\Windows\system32\cmd.exe".to_string(),
    );
    map.insert(
        "PATHEXT".to_string(),
        ".COM;.EXE;.BAT;.CMD;.VBS;.JS;.WS;.MSC".to_string(),
    );
    map
}

/// Generates a deterministic mock initial environment for Windows with an existing
/// variable to be removed or unset.
#[must_use]
pub fn create_mock_initial_env_windows_with_existing_var() -> EnvMap {
    let mut map = create_mock_initial_env_windows();
    map.insert("EXISTING_VAR".to_string(), "to_be_deleted".to_string());
    map
}

/// Helper to write a `.sh` script to a temporary directory and evaluate it via
/// [`try_env_source`].
///
/// # Errors
///
/// Returns an error if creating the temporary directory or evaluating the shell script
/// fails.
///
/// # Panics
///
/// Panics if creating the script file or writing content fails.
#[cfg(unix)]
pub fn run_fixture_sh(
    fixture_content: &str,
    output_format: OutputFormat,
    base_env: BaseEnv,
) -> miette::Result<String> {
    let temp_dir = try_create_temp_dir()?;
    let script_path = temp_dir.join("test_fixture.sh");
    let mut file = std::fs::File::create(&script_path).unwrap();
    file.write_all(fixture_content.as_bytes()).unwrap();
    file.flush().unwrap();
    try_env_source(&script_path, output_format, base_env)
}

/// Helper to write a `.bat` script to a temporary directory and evaluate it via
/// [`try_env_source`].
///
/// # Errors
///
/// Returns an error if creating the temporary directory or evaluating the shell script
/// fails.
///
/// # Panics
///
/// Panics if creating the script file or writing content fails.
#[cfg(windows)]
pub fn run_fixture_bat(
    fixture_content: &str,
    output_format: OutputFormat,
    base_env: BaseEnv,
) -> miette::Result<String> {
    let temp_dir = try_create_temp_dir()?;
    let script_path = temp_dir.join("test_fixture.bat");
    let mut file = std::fs::File::create(&script_path).unwrap();
    file.write_all(fixture_content.as_bytes()).unwrap();
    file.flush().unwrap();
    try_env_source(&script_path, output_format, base_env)
}

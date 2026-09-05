// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{assert_eq2,
            core::script::env_source::{BaseEnv, OutputFormat,
                                       conformance_tests::{conformance_data::*,
                                                           test_fixtures_env_source::*},
                                       try_env_source},
            try_create_temp_dir};
use std::io::Write;

#[test]
fn test_subshell_windows_cargo_env() -> miette::Result<()> {
    let mock_env = create_mock_initial_env_windows();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = run_fixture_bat(
        INPUT_CARGO_ENV_BAT,
        OutputFormat::Powershell,
        base_env,
    )?;

    assert_eq2!(formatted, EXPECTED_CARGO_ENV_POWERSHELL);
    Ok(())
}

#[test]
fn test_subshell_windows_noisy_script() -> miette::Result<()> {
    let mock_env = create_mock_initial_env_windows();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = run_fixture_bat(
        INPUT_NOISY_SCRIPT_BAT,
        OutputFormat::Powershell,
        base_env,
    )?;

    assert_eq2!(formatted, EXPECTED_NOISY_SCRIPT_POWERSHELL);
    Ok(())
}

#[test]
fn test_subshell_windows_edge_cases() -> miette::Result<()> {
    let mock_env = create_mock_initial_env_windows_with_existing_var();
    let base_env = BaseEnv::Hermetic(mock_env);

    let ps_out = run_fixture_bat(
        INPUT_EDGE_CASES_BAT,
        OutputFormat::Powershell,
        base_env.clone(),
    )?;
    assert_eq2!(ps_out, EXPECTED_EDGE_CASES_POWERSHELL);

    let json_out = run_fixture_bat(
        INPUT_EDGE_CASES_BAT,
        OutputFormat::Json,
        base_env.clone(),
    )?;
    assert_eq2!(json_out, EXPECTED_EDGE_CASES_JSON_WINDOWS);

    let env_out = run_fixture_bat(INPUT_EDGE_CASES_BAT, OutputFormat::Dotenv, base_env)?;
    assert_eq2!(env_out, EXPECTED_EDGE_CASES_ENV_WINDOWS);

    Ok(())
}

#[test]
fn test_subshell_windows_from_real_file() -> miette::Result<()> {
    let temp_dir = try_create_temp_dir()?;
    let script_path = temp_dir.join("test_script.bat");
    let mut file = std::fs::File::create(&script_path).unwrap();
    file.write_all(INPUT_CARGO_ENV_BAT.as_bytes()).unwrap();
    file.flush().unwrap();

    let mock_env = create_mock_initial_env_windows();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = try_env_source(&script_path, OutputFormat::Powershell, base_env)?;

    assert_eq2!(formatted, EXPECTED_CARGO_ENV_POWERSHELL);
    Ok(())
}

#[test]
fn test_subshell_windows_path_with_spaces() -> miette::Result<()> {
    let temp_dir = try_create_temp_dir()?;
    let folder_with_spaces = temp_dir.join("folder with spaces");
    std::fs::create_dir_all(&folder_with_spaces).unwrap();
    let script_path = folder_with_spaces.join("my script.bat");
    let mut file = std::fs::File::create(&script_path).unwrap();
    file.write_all(b"@echo off\r\nset SPACE_TEST_VAR=passed\r\n")
        .unwrap();
    file.flush().unwrap();

    let mock_env = create_mock_initial_env_windows();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = try_env_source(&script_path, OutputFormat::Powershell, base_env)?;

    assert!(formatted.contains("$env:SPACE_TEST_VAR = 'passed';"));
    Ok(())
}

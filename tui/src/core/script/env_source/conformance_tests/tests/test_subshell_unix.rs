// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{assert_eq2,
            core::script::env_source::{BaseEnv, OutputFormat,
                                       conformance_tests::{test_data::*,
                                                           test_fixtures::*},
                                       try_env_source},
            try_create_temp_dir};
use std::io::Write;

#[test]
fn test_subshell_sanitized_user_profile() -> miette::Result<()> {
    let mock_env = create_mock_initial_env_unix();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = run_fixture_sh(
        INPUT_SANITIZED_USER_PROFILE_SH,
        OutputFormat::Fish,
        base_env,
    )?;

    assert_eq2!(formatted, EXPECTED_SANITIZED_USER_PROFILE_FISH);
    Ok(())
}

#[test]
fn test_subshell_cargo_env() -> miette::Result<()> {
    let mock_env = create_mock_initial_env_unix();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = run_fixture_sh(INPUT_CARGO_ENV_SH, OutputFormat::Fish, base_env)?;

    assert_eq2!(formatted, EXPECTED_CARGO_ENV_FISH);
    Ok(())
}

#[test]
fn test_subshell_noisy_script() -> miette::Result<()> {
    let mock_env = create_mock_initial_env_unix();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = run_fixture_sh(INPUT_NOISY_SCRIPT_SH, OutputFormat::Fish, base_env)?;

    assert_eq2!(formatted, EXPECTED_NOISY_SCRIPT_FISH);
    Ok(())
}

#[test]
fn test_subshell_homebrew_env() -> miette::Result<()> {
    let mock_env = create_mock_initial_env_unix();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = run_fixture_sh(INPUT_HOMEBREW_ENV_SH, OutputFormat::Fish, base_env)?;
    assert!(formatted.contains("set -gx HOMEBREW_PREFIX '/opt/homebrew';"));
    assert!(formatted.contains("set -gx HOMEBREW_CELLAR '/opt/homebrew/Cellar';"));
    assert!(formatted.contains(
        "set -gx PATH '/opt/homebrew/bin' '/opt/homebrew/sbin' '/usr/bin' '/bin';"
    ));
    Ok(())
}

#[test]
fn test_subshell_edge_cases() -> miette::Result<()> {
    let mock_env = create_mock_initial_env_unix_with_existing_var();
    let base_env = BaseEnv::Hermetic(mock_env);

    let fish_out =
        run_fixture_sh(INPUT_EDGE_CASES_SH, OutputFormat::Fish, base_env.clone())?;
    assert_eq2!(fish_out, EXPECTED_EDGE_CASES_FISH);

    let json_out =
        run_fixture_sh(INPUT_EDGE_CASES_SH, OutputFormat::Json, base_env.clone())?;
    assert_eq2!(json_out, EXPECTED_EDGE_CASES_JSON_UNIX);

    let env_out = run_fixture_sh(INPUT_EDGE_CASES_SH, OutputFormat::Dotenv, base_env)?;
    assert_eq2!(env_out, EXPECTED_EDGE_CASES_ENV_UNIX);

    Ok(())
}

#[test]
fn test_subshell_from_real_file() -> miette::Result<()> {
    let temp_dir = try_create_temp_dir()?;
    let script_path = temp_dir.join("test_profile.sh");
    let mut file = std::fs::File::create(&script_path).unwrap();
    file.write_all(INPUT_SANITIZED_USER_PROFILE_SH.as_bytes())
        .unwrap();
    file.flush().unwrap();

    let mock_env = create_mock_initial_env_unix();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = try_env_source(&script_path, OutputFormat::Fish, base_env)?;

    assert_eq2!(formatted, EXPECTED_SANITIZED_USER_PROFILE_FISH);
    Ok(())
}

#[test]
fn test_subshell_path_with_spaces() -> miette::Result<()> {
    let temp_dir = try_create_temp_dir()?;
    let folder_with_spaces = temp_dir.join("folder with spaces");
    std::fs::create_dir_all(&folder_with_spaces).unwrap();
    let script_path = folder_with_spaces.join("my script.sh");
    let mut file = std::fs::File::create(&script_path).unwrap();
    file.write_all(b"export SPACE_TEST_VAR=passed\n").unwrap();
    file.flush().unwrap();

    let mock_env = create_mock_initial_env_unix();
    let base_env = BaseEnv::Hermetic(mock_env);
    let formatted = try_env_source(&script_path, OutputFormat::Fish, base_env)?;

    assert!(formatted.contains("set -gx SPACE_TEST_VAR 'passed';"));
    Ok(())
}

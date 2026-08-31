// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Conformance test data for the `env-source` cross-platform environment loader.
//!
//! Organizes input test scripts and expected outputs across supported shells.
//!
//! # Platform Organization
//!
//! - `unix/`: POSIX shell scripts (`.sh`) tested against Fish, JSON, and Dotenv golden outputs.
//! - `windows/`: Windows batch scripts (`.bat`) tested against PowerShell, JSON, and Dotenv golden outputs.

// Unix test data constants.
pub const INPUT_CARGO_ENV_SH: &str = include_str!("unix/input_cargo_env.sh");
pub const EXPECTED_CARGO_ENV_FISH: &str = include_str!("unix/expected_cargo_env.fish");

pub const INPUT_SANITIZED_USER_PROFILE_SH: &str =
    include_str!("unix/input_sanitized_user_profile.sh");
pub const EXPECTED_SANITIZED_USER_PROFILE_FISH: &str =
    include_str!("unix/expected_sanitized_user_profile.fish");

pub const INPUT_NOISY_SCRIPT_SH: &str = include_str!("unix/input_noisy_script.sh");
pub const EXPECTED_NOISY_SCRIPT_FISH: &str = include_str!("unix/expected_noisy_script.fish");

pub const INPUT_HOMEBREW_ENV_SH: &str = include_str!("unix/input_homebrew_env.sh");

pub const INPUT_EDGE_CASES_SH: &str = include_str!("unix/input_edge_cases.sh");
pub const EXPECTED_EDGE_CASES_FISH: &str = include_str!("unix/expected_edge_cases.fish");
pub const EXPECTED_EDGE_CASES_JSON_UNIX: &str = include_str!("unix/expected_edge_cases.json");
pub const EXPECTED_EDGE_CASES_ENV_UNIX: &str = include_str!("unix/expected_edge_cases.env");

// Windows test data constants.
pub const INPUT_CARGO_ENV_BAT: &str = include_str!("windows/input_cargo_env.bat");
pub const EXPECTED_CARGO_ENV_POWERSHELL: &str =
    include_str!("windows/expected_cargo_env.ps1");

pub const INPUT_NOISY_SCRIPT_BAT: &str = include_str!("windows/input_noisy_script.bat");
pub const EXPECTED_NOISY_SCRIPT_POWERSHELL: &str =
    include_str!("windows/expected_noisy_script.ps1");

pub const INPUT_EDGE_CASES_BAT: &str = include_str!("windows/input_edge_cases.bat");
pub const EXPECTED_EDGE_CASES_POWERSHELL: &str =
    include_str!("windows/expected_edge_cases.ps1");
pub const EXPECTED_EDGE_CASES_JSON_WINDOWS: &str =
    include_str!("windows/expected_edge_cases.json");
pub const EXPECTED_EDGE_CASES_ENV_WINDOWS: &str =
    include_str!("windows/expected_edge_cases.env");

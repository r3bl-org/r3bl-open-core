// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{assert_eq2,
            core::script::env_source::{OutputFormat,
                                       conformance_tests::conformance_data::*,
                                       diff::EnvDiff,
                                       formatters::{format_fish, format_powershell}}};
use std::collections::BTreeMap;

#[test]
fn test_golden_sanitized_user_profile_fish() {
    let mut added = BTreeMap::new();
    added.insert(
        "RUSTUP_DIST_SERVER".to_string(),
        "https://fastly-static.rust-lang.org".to_string(),
    );
    added.insert("GITHUB_TOKEN".to_string(), "mock_token_12345".to_string());
    added.insert("MOZ_AMO_KEY".to_string(), "user:12345:678".to_string());
    added.insert(
        "MOZ_AMO_SECRET".to_string(),
        "secret_abcdef123456".to_string(),
    );
    added.insert("GTK_IM_MODULE".to_string(), String::new());
    added.insert("QT_IM_MODULE".to_string(), String::new());
    added.insert("XMODIFIERS".to_string(), String::new());

    let mut modified = BTreeMap::new();
    modified.insert(
        "PATH".to_string(),
        "/home/testuser/bin:/home/testuser/.local/bin:/usr/bin:/bin".to_string(),
    );

    let diff = EnvDiff {
        added,
        modified,
        removed: vec![],
    };

    let formatted = format_fish(&diff);
    assert_eq2!(formatted, EXPECTED_SANITIZED_USER_PROFILE_FISH);
}

#[test]
fn test_golden_cargo_env_fish() {
    let mut modified = BTreeMap::new();
    modified.insert(
        "PATH".to_string(),
        "/home/testuser/.cargo/bin:/usr/bin:/bin".to_string(),
    );

    let diff = EnvDiff {
        added: BTreeMap::new(),
        modified,
        removed: vec![],
    };

    let formatted = format_fish(&diff);
    assert_eq2!(formatted, EXPECTED_CARGO_ENV_FISH);
}

#[test]
fn test_golden_noisy_script_fish() {
    let mut added = BTreeMap::new();
    added.insert("NOISY_VAR".to_string(), "success".to_string());
    added.insert("LOUD_SETTING".to_string(), "1".to_string());

    let diff = EnvDiff {
        added,
        modified: BTreeMap::new(),
        removed: vec![],
    };

    let formatted = format_fish(&diff);
    assert_eq2!(formatted, EXPECTED_NOISY_SCRIPT_FISH);
}

#[test]
fn test_golden_edge_cases_all_formats_unix() {
    let mut added = BTreeMap::new();
    added.insert(
        "MULTILINE".to_string(),
        "line 1\nline 2\nline 3".to_string(),
    );
    added.insert("QUOTE_SINGLE".to_string(), "don't fail".to_string());
    added.insert("QUOTE_DOUBLE".to_string(), "said \"hello\"".to_string());
    added.insert("WITH_BACKSLASH".to_string(), "path\\to\\dir".to_string());
    added.insert("WITH_SEMICOLON".to_string(), "foo; bar; baz".to_string());
    added.insert("EMPTY_VAR".to_string(), String::new());

    let removed = vec!["EXISTING_VAR".to_string()];

    let diff = EnvDiff {
        added,
        modified: BTreeMap::new(),
        removed,
    };

    let fish_out = diff.serialize_to_string(OutputFormat::Fish);
    assert_eq2!(fish_out, EXPECTED_EDGE_CASES_FISH);

    let json_out = diff.serialize_to_string(OutputFormat::Json);
    assert_eq2!(json_out, EXPECTED_EDGE_CASES_JSON_UNIX);

    let env_out = diff.serialize_to_string(OutputFormat::Dotenv);
    assert_eq2!(env_out, EXPECTED_EDGE_CASES_ENV_UNIX);
}

#[test]
fn test_golden_cargo_env_powershell() {
    let mut modified = BTreeMap::new();
    modified.insert(
        "PATH".to_string(),
        r"C:\Users\testuser\.cargo\bin;C:\Windows\system32;C:\Windows".to_string(),
    );

    let diff = EnvDiff {
        added: BTreeMap::new(),
        modified,
        removed: vec![],
    };

    let formatted = format_powershell(&diff);
    assert_eq2!(formatted, EXPECTED_CARGO_ENV_POWERSHELL);
}

#[test]
fn test_golden_noisy_script_powershell() {
    let mut added = BTreeMap::new();
    added.insert("NOISY_VAR".to_string(), "success".to_string());
    added.insert("LOUD_SETTING".to_string(), "1".to_string());

    let diff = EnvDiff {
        added,
        modified: BTreeMap::new(),
        removed: vec![],
    };

    let formatted = format_powershell(&diff);
    assert_eq2!(formatted, EXPECTED_NOISY_SCRIPT_POWERSHELL);
}

#[test]
fn test_golden_edge_cases_all_formats_windows() {
    let mut added = BTreeMap::new();
    added.insert("QUOTE_SINGLE".to_string(), "don't fail".to_string());
    added.insert("WITH_BACKSLASH".to_string(), "path\\to\\dir".to_string());
    added.insert("WITH_SEMICOLON".to_string(), "foo; bar; baz".to_string());

    let removed = vec!["EXISTING_VAR".to_string()];

    let diff = EnvDiff {
        added,
        modified: BTreeMap::new(),
        removed,
    };

    let ps_out = diff.serialize_to_string(OutputFormat::Powershell);
    assert_eq2!(ps_out, EXPECTED_EDGE_CASES_POWERSHELL);

    let json_out = diff.serialize_to_string(OutputFormat::Json);
    assert_eq2!(json_out, EXPECTED_EDGE_CASES_JSON_WINDOWS);

    let env_out = diff.serialize_to_string(OutputFormat::Dotenv);
    assert_eq2!(env_out, EXPECTED_EDGE_CASES_ENV_WINDOWS);
}

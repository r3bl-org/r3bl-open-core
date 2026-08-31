// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{assert_eq2,
            core::script::env_source::{OutputFormat,
                                       conformance_tests::test_data::*,
                                       diff::{self, EnvDiffChunk},
                                       formatters::{format_fish, format_powershell}}};

#[test]
fn test_golden_sanitized_user_profile_fish() {
    let diff = vec![
        EnvDiffChunk::Add {
            key: "GITHUB_TOKEN".to_string(),
            value: "mock_token_12345".to_string(),
        },
        EnvDiffChunk::Add {
            key: "GTK_IM_MODULE".to_string(),
            value: String::new(),
        },
        EnvDiffChunk::Add {
            key: "MOZ_AMO_KEY".to_string(),
            value: "user:12345:678".to_string(),
        },
        EnvDiffChunk::Add {
            key: "MOZ_AMO_SECRET".to_string(),
            value: "secret_abcdef123456".to_string(),
        },
        EnvDiffChunk::Modify {
            key: "PATH".to_string(),
            value: "/home/testuser/bin:/home/testuser/.local/bin:/usr/bin:/bin"
                .to_string(),
        },
        EnvDiffChunk::Add {
            key: "QT_IM_MODULE".to_string(),
            value: String::new(),
        },
        EnvDiffChunk::Add {
            key: "RUSTUP_DIST_SERVER".to_string(),
            value: "https://fastly-static.rust-lang.org".to_string(),
        },
        EnvDiffChunk::Add {
            key: "XMODIFIERS".to_string(),
            value: String::new(),
        },
    ];

    let formatted = format_fish(&diff);
    assert_eq2!(formatted, EXPECTED_SANITIZED_USER_PROFILE_FISH);
}

#[test]
fn test_golden_cargo_env_fish() {
    let diff = vec![EnvDiffChunk::Modify {
        key: "PATH".to_string(),
        value: "/home/testuser/.cargo/bin:/usr/bin:/bin".to_string(),
    }];

    let formatted = format_fish(&diff);
    assert_eq2!(formatted, EXPECTED_CARGO_ENV_FISH);
}

#[test]
fn test_golden_noisy_script_fish() {
    let diff = vec![
        EnvDiffChunk::Add {
            key: "LOUD_SETTING".to_string(),
            value: "1".to_string(),
        },
        EnvDiffChunk::Add {
            key: "NOISY_VAR".to_string(),
            value: "success".to_string(),
        },
    ];

    let formatted = format_fish(&diff);
    assert_eq2!(formatted, EXPECTED_NOISY_SCRIPT_FISH);
}

#[test]
fn test_golden_edge_cases_all_formats_unix() {
    let diff = vec![
        EnvDiffChunk::Add {
            key: "EMPTY_VAR".to_string(),
            value: String::new(),
        },
        EnvDiffChunk::Remove {
            key: "EXISTING_VAR".to_string(),
        },
        EnvDiffChunk::Add {
            key: "MULTILINE".to_string(),
            value: "line 1\nline 2\nline 3".to_string(),
        },
        EnvDiffChunk::Add {
            key: "QUOTE_DOUBLE".to_string(),
            value: "said \"hello\"".to_string(),
        },
        EnvDiffChunk::Add {
            key: "QUOTE_SINGLE".to_string(),
            value: "don't fail".to_string(),
        },
        EnvDiffChunk::Add {
            key: "WITH_BACKSLASH".to_string(),
            value: "path\\to\\dir".to_string(),
        },
        EnvDiffChunk::Add {
            key: "WITH_SEMICOLON".to_string(),
            value: "foo; bar; baz".to_string(),
        },
    ];

    let fish_out = diff::format_env_diff(&diff, OutputFormat::Fish);
    assert_eq2!(fish_out, EXPECTED_EDGE_CASES_FISH);

    let json_out = diff::format_env_diff(&diff, OutputFormat::Json);
    assert_eq2!(json_out, EXPECTED_EDGE_CASES_JSON_UNIX);

    let env_out = diff::format_env_diff(&diff, OutputFormat::Dotenv);
    assert_eq2!(env_out, EXPECTED_EDGE_CASES_ENV_UNIX);
}

#[test]
fn test_golden_cargo_env_powershell() {
    let diff = vec![EnvDiffChunk::Modify {
        key: "PATH".to_string(),
        value: r"C:\Users\testuser\.cargo\bin;C:\Windows\system32;C:\Windows".to_string(),
    }];

    let formatted = format_powershell(&diff);
    assert_eq2!(formatted, EXPECTED_CARGO_ENV_POWERSHELL);
}

#[test]
fn test_golden_noisy_script_powershell() {
    let diff = vec![
        EnvDiffChunk::Add {
            key: "LOUD_SETTING".to_string(),
            value: "1".to_string(),
        },
        EnvDiffChunk::Add {
            key: "NOISY_VAR".to_string(),
            value: "success".to_string(),
        },
    ];

    let formatted = format_powershell(&diff);
    assert_eq2!(formatted, EXPECTED_NOISY_SCRIPT_POWERSHELL);
}

#[test]
fn test_golden_edge_cases_all_formats_windows() {
    let diff = vec![
        EnvDiffChunk::Remove {
            key: "EXISTING_VAR".to_string(),
        },
        EnvDiffChunk::Add {
            key: "QUOTE_SINGLE".to_string(),
            value: "don't fail".to_string(),
        },
        EnvDiffChunk::Add {
            key: "WITH_BACKSLASH".to_string(),
            value: "path\\to\\dir".to_string(),
        },
        EnvDiffChunk::Add {
            key: "WITH_SEMICOLON".to_string(),
            value: "foo; bar; baz".to_string(),
        },
    ];

    let ps_out = diff::format_env_diff(&diff, OutputFormat::Powershell);
    assert_eq2!(ps_out, EXPECTED_EDGE_CASES_POWERSHELL);

    let json_out = diff::format_env_diff(&diff, OutputFormat::Json);
    assert_eq2!(json_out, EXPECTED_EDGE_CASES_JSON_WINDOWS);

    let env_out = diff::format_env_diff(&diff, OutputFormat::Dotenv);
    assert_eq2!(env_out, EXPECTED_EDGE_CASES_ENV_WINDOWS);
}

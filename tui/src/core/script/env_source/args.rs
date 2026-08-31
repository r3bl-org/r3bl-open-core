// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module contains all the args that are passed into the [`try_env_source`] public
//! API entry point. These can come from CLI arguments or programmatically.
//!
//! [`try_env_source`]: super::try_env_source

use crate::EnvMap;
use strum_macros::{Display, EnumString};

/// The initial environment specification when [`try_env_source()`] evaluates a script.
///
/// [`try_env_source()`]: super::try_env_source
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum BaseEnv {
    /// Inherits the ambient host process environment ([`std::env::vars_os()`]).
    ///
    /// This is the standard mode for shell startup hooks where user scripts expect
    /// existing environment variables (such as `PATH`, `HOME`, or `USER`) to be
    /// present.
    #[default]
    Inherit,
    /// Seeds the subshell with a hermetic environment mapping.
    ///
    /// The subshell starts with only the provided variables. This is useful for testing,
    /// sandboxed execution, and hermetic environment resolution.
    Hermetic(EnvMap),
}

/// The output format (shell syntax or structured data) for emitting environment
/// mutations.
///
/// This determines how [`diff::format_env_diff()`] serializes added, modified, and
/// removed environment variables.
///
/// [`diff::format_env_diff()`]: crate::diff::format_env_diff
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum OutputFormat {
    /// [Fish] shell syntax (`set -gx KEY 'VAL';`).
    ///
    /// [Fish]: https://fishshell.com/
    Fish,
    /// [PowerShell] syntax (`$env:KEY = 'VAL';`).
    ///
    /// [PowerShell]: https://learn.microsoft.com/en-us/powershell/
    Powershell,
    /// JSON format.
    Json,
    /// Standard `.env` file syntax (`KEY="VAL"`).
    Dotenv,
}

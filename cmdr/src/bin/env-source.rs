// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! `env-source`: Fast cross-platform environment loader CLI in native Rust.
//!
//! A high-performance CLI binary for evaluating shell scripts by running an isolated
//! shell, capturing environment mutations, and emitting formatted deltas to [`stdout`].
//! Works across Linux, macOS, and Windows.
//!
//! For architecture details, shell execution lifecycle, and the underlying library API,
//! see [`try_env_source`].
//!
//! # Usage Examples
//!
//! ## Unix (Linux and macOS)
//!
//! ```bash
//! # Source a script into Fish shell:
//! env-source -i ~/.profile -o fish | source
//!
//! # Emit JSON:
//! env-source -i ~/.profile -o json
//!
//! # Emit Dotenv:
//! env-source -i ~/.profile -o dotenv > .env
//! ```
//!
//! ## Windows
//!
//! ```powershell
//! # Source a batch file into PowerShell:
//! env-source -i setenv.bat -o powershell | Invoke-Expression
//!
//! # Emit JSON:
//! env-source -i setenv.bat -o json
//!
//! # Emit Dotenv:
//! env-source -i setenv.bat -o dotenv > .env
//! ```
//!
//! [`try_env_source`]: r3bl_tui::try_env_source

use clap::Parser;
use miette::IntoDiagnostic;
use r3bl_cmdr::env_source::CLIArg;
use r3bl_tui::{BaseEnv, CommonResult, ok, run_with_safe_stack, set_mimalloc_in_main,
               try_env_source};
use std::io::{Write, stdout};

fn main() -> CommonResult { run_with_safe_stack!(main_impl()) }

fn main_impl() -> CommonResult {
    set_mimalloc_in_main!();

    let cli_arg = CLIArg::parse();

    let output =
        try_env_source(cli_arg.input_file, cli_arg.output_format, BaseEnv::Inherit)?;

    stdout()
        .lock()
        .write_all(output.as_bytes())
        .into_diagnostic()?;

    ok!()
}

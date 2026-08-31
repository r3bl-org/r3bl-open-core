// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{args::{BaseEnv, OutputFormat},
            diff,
            run_shell::try_source_and_export_env};
pub use crate::EnvMap;
use crate::VarsOsExt;
use std::{env::vars_os, path::PathBuf};

/// Main public API entry point for cross-platform, cross-shell, [`source`].
///
/// Evaluates a shell script by running an isolated shell, captures environment variable
/// mutations, compares them against the initial environment (before the script is run),
/// and serializes the changes as a [`String`], formatted with the specified
/// [`OutputFormat`].
///
/// # Use Case
///
/// Shell startup files (e.g. `~/.profile` or `~/.cargo/env` on POSIX, or `.bat` and
/// `.cmd` scripts on Windows) serve as the canonical source of truth for toolchain paths,
/// API tokens, and user environment variables.
///
/// On modern desktop environments (Wayland, `systemd --user`) and macOS, terminal
/// emulators spawn interactive shells directly without evaluating `~/.profile` (which is
/// only read by POSIX login shells). Furthermore, alternative shells like [Fish] or
/// [PowerShell] use their own distinct syntax and cannot natively parse POSIX constructs
/// (`export`, `case`, parameter expansions) or Windows `.bat` syntax (`SET`).
///
/// On Linux and macOS, tools like [`bass`] bridge this gap for [Fish], but they rely on
/// an external Python runtime that spawns multiple subshells and writes temporary files
/// to `/tmp/`. In our testing, we saw ~220 ms of overhead to every shell startup! We
/// profiled how long it took a 3 KiB `.profile` on an Intel Core Ultra 9 285H running
/// Linux kernel 7.2 and [Fish] to be loaded with [`bass`] on low-power island cores aka
/// LP E-cores.
///
/// 1. While 220 ms may not seem like a lot, this delay happens at a critical moment when
///    a user launches the [Fish] shell, and it is perceived as a "hang" at the start.
/// 2. This runtime overhead is quite expensive considering the tiny size of scripts that
///    are usually [`sourced`][`source`].
/// 3. It is incurred every time a [Fish] shell is launched.
///
/// [`try_env_source`] is a pure Rust replacement for [`bass`] on Linux and macOS (for
/// [Fish]), and [`Invoke-Environment`] on Windows (for [PowerShell]). It works without
/// creating temporary files or needing external runtimes. It serializes the changes in
/// environment variables to a variety of [`OutputFormat`]s:
/// 1. [Fish] shell syntax via [`OutputFormat::Fish`].
/// 2. [PowerShell] syntax via [`OutputFormat::Powershell`].
/// 3. [JSON] via [`OutputFormat::Json`].
/// 4. [dotenv] via [`OutputFormat::Dotenv`].
///
/// This serialized output can then be piped directly into the calling shell's evaluation
/// command (e.g. `| source` in [Fish] or `| Invoke-Expression` in [PowerShell]) or parsed
/// by external tools (via [JSON] or [dotenv]). Because an OS child process (i.e., the
/// process running [`try_env_source`]) cannot directly mutate its parent process
/// environment, generating and sourcing these statements in the parent shell context is
/// what allows foreign scripts to update the active session.
///
/// # Architecture
///
/// [`try_env_source`] acts as the orchestrator of this 3-stage pipeline:
///
/// ```text
/// Shell Execution & Env Capture ──► Diff Computation ──► Serialization
/// ```
///
/// 1. Shell Execution & Env Capture: Runs an isolated shell (`sh` on POSIX, `cmd.exe` on
///    Windows) to evaluate the script file, and parses the captured environment output
///    into an [`EnvMap`] (see [`try_source_and_export_env`]).
/// 2. Diff Computation: Computes additions, modifications, and deletions against the
///    initial environment ([`BaseEnv`]), filtering out internal shell variables (see
///    [`diff::compute_env_diff`] and [`filter_env_map`]).
/// 3. Serialization: Formats the diff into [`source`]-able commands for the target shell
///    ([Fish], [PowerShell]) or structured data formats ([JSON], [dotenv]) (see
///    [`diff::format_env_diff`]).
///
/// # Examples
///
/// ## Sourcing a Script File
///
/// ```no_run
/// use r3bl_tui::{BaseEnv, OutputFormat, ok, try_env_source};
///
/// fn example() -> miette::Result<()> {
///     let fish_code = try_env_source("~/.profile", OutputFormat::Fish, BaseEnv::Inherit)?;
///     println!("{fish_code}");
///     ok!()
/// }
/// ```
///
/// ## Sourcing with a Hermetic Initial Environment
///
/// ```no_run
/// use r3bl_tui::{BaseEnv, EnvMap, OutputFormat, ok, try_env_source};
///
/// fn example() -> miette::Result<()> {
///     let mut mock_initial = EnvMap::default();
///     mock_initial.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
///     let output = try_env_source(
///         "~/.profile", OutputFormat::Fish, BaseEnv::Hermetic(mock_initial)
///     )?;
///     assert!(output.contains("set -gx PATH '/custom/bin:/usr/bin:/bin';"));
///     ok!()
/// }
/// ```
///
/// # Errors
///
/// Returns an error if spawning or executing the shell process fails.
///
/// [`BaseEnv`]: crate::BaseEnv
/// [`bass`]: https://github.com/edc/bass
/// [`diff::compute_env_diff`]: crate::diff::compute_env_diff
/// [`diff::format_env_diff`]: crate::diff::format_env_diff
/// [`EnvMap`]: crate::EnvMap
/// [`filter_env_map`]: crate::filter_env_map
/// [`Invoke-Environment`]: https://github.com/nightroman/PowerShelf
/// [`OutputFormat::Dotenv`]: crate::OutputFormat::Dotenv
/// [`OutputFormat::Fish`]: crate::OutputFormat::Fish
/// [`OutputFormat::Json`]: crate::OutputFormat::Json
/// [`OutputFormat::Powershell`]: crate::OutputFormat::Powershell
/// [`OutputFormat`]: crate::OutputFormat
/// [`source`]: https://en.wikipedia.org/wiki/Source_(command)
/// [`try_source_and_export_env`]: crate::try_source_and_export_env
/// [dotenv]: https://github.com/motdotla/dotenv
/// [Fish]: https://fishshell.com/
/// [JSON]: https://www.json.org/
/// [PowerShell]: https://learn.microsoft.com/en-us/powershell/
pub fn try_env_source(
    script_path: impl Into<PathBuf>,
    output_format: OutputFormat,
    base_env: BaseEnv,
) -> miette::Result<String> {
    let script_path = script_path.into();
    let mutated_env = try_source_and_export_env(&script_path, &base_env)?;

    // This is not the first statement in this function so as not to clone base_env.
    let initial_env = match base_env {
        BaseEnv::Hermetic(env_map) => env_map,
        BaseEnv::Inherit => vars_os().into_env_map(),
    };

    let diff = diff::compute_env_diff(initial_env, mutated_env, output_format);

    let formatted_output = diff::format_env_diff(&diff, output_format);

    Ok(formatted_output)
}

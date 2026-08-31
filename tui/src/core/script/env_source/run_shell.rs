// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{args::BaseEnv, parser};
use crate::{CommandOutputResult, DEBUG_ENV_SOURCE, EnvMap};
use miette::miette;
use std::{path::Path, process::Command};

/// Runs a shell to source the specified target script file and captures the resulting
/// environment.
///
/// Script output (such as `echo` commands or warnings) is redirected to `/dev/null` on
/// Unix (`nul` on Windows) so it doesn't mix into the captured environment variables or
/// error messages.
///
/// # Arguments
///
/// - `script_path`: Path to the script file to source:
///     - `. "<path>"` on Unix.
///     - `call <path>` on Windows.
/// - `base_env`: Initial environment variables for the child shell process:
///     - [`BaseEnv::Hermetic`]: Inherited host environment variables are cleared via
///       [`Command::env_clear`] and only the keys in the hermetic map are set.
///     - [`BaseEnv::Inherit`]: The shell process naturally inherits the host environment.
///
/// # Errors
///
/// Returns an error if spawning the shell process fails, capturing output fails, or the
/// command exits with an error status.
///
/// # Cross-platform
///
/// - For more details on the Unix implementation see [`try_source_and_export_env_unix`].
/// - For more details on the Windows implementation see
///   [`try_source_and_export_env_windows`].
///
/// [`BaseEnv::Hermetic`]: BaseEnv::Hermetic
/// [`BaseEnv::Inherit`]: BaseEnv::Inherit
/// [`Command::env_clear`]: Command::env_clear
pub fn try_source_and_export_env(
    script_path: &Path,
    base_env: &BaseEnv,
) -> miette::Result<EnvMap> {
    #[cfg(unix)]
    {
        try_source_and_export_env_unix(script_path, base_env)
    }
    #[cfg(windows)]
    {
        try_source_and_export_env_windows(script_path, base_env)
    }
}

/// Evaluates the target shell script using `/bin/sh` on Unix (Linux, macOS) and captures
/// the resulting environment.
///
/// # Why We Use Compound Grouping `{ ... }` Instead of a Subshell `( ... )`
///
/// Sourcing a script (in a `/bin/sh` process) requires mutating the active shell's
/// environment table in place. In POSIX shells, there is a fundamental difference between
/// parentheses `()` and curly braces `{}`.
///
/// Here are two commands we can execute using [`Command`]:
/// 1. `/bin/sh -c '{ . "$1"; }'`
/// 2. `/bin/sh -c '( . "$1"; )'`
///
/// So what's the difference?
///
/// - Parentheses `( . "$1"; )`: Inner Subshell:
///     - Process boundary: Invokes `fork()` to run the script inside an isolated child
///       process.
///     - Environment isolation: When the script exports variables (such as `export
///       FOO=bar`), they are written only to the child process memory.
///     - Mutations lost: As soon as the subshell exits, its memory is destroyed. The
///       parent `/bin/sh` environment remains untouched, so subsequent execution of `env
///       -0` observes zero mutations.
///
/// - Curly Braces `{ . "$1"; }`: Compound Group Command:
///     - Runs in-process: Executes directly within `/bin/sh`'s active environment table
///       without forking.
///     - Mutations persist: All exports, unsets, and variable modifications persist in
///       the shell's process memory, allowing `env -0` to capture them.
///     - Stream redirection scoping: Braces establish a clean lexical boundary for I/O
///       redirection (`{ . "$1"; } >/dev/null 2>&1`), discarding noisy script output (to
///       [`stdout`] or [`stderr`]) while allowing `env -0` to write clean, null-delimited
///       records to [`stdout`].
///
/// # Positional Parameter Passing via `$1`
///
/// POSIX `sh -c` executes an inline shell snippet while allowing the caller to set
/// positional arguments out-of-band:
///
/// ```text
///              ┌────────────────────── command_string: Inline snippet (sources the file in $1)
///              │             ┌───────── command_name: Name shown in error messages ($0)
///              │             │        ┌─ argument...: Positional parameters ($1, $2, ...)
///              │             │        │
///       ┌──────┴─────┐ ┌─────┴─────┐ ┌┴───────────┐
/// sh -c command_string [command_name [argument...]]
/// ```
///
/// When spawning `/bin/sh`, we pass our arguments as follows:
///
/// | Arg Index | Value passed by `env-source`              | How `/bin/sh` receives it    |
/// | :-------- | :---------------------------------------- | :--------------------------- |
/// | `0`       | `"/bin/sh"`                               | The shell executable         |
/// | `1`       | `"-c"`                                    | Option flag                  |
/// | `2`       | `"{ . \"$1\"; } >/dev/null 2>&1; env -0"` | `command_string` (uses `$1`) |
/// | `3`       | `"env-source"`                            | Assigned to `$0`             |
/// | `4`       | `script_path`                             | Assigned to `$1`             |
///
/// Here are some reasons why we don't just inline the script path into the
/// `command_string`:
/// - Passing the target path out-of-band via `$1` rather than interpolating it into the
///   `command_string` prevents command injection and syntax breakage when file paths
///   contain spaces or special characters.
/// - Quoting `"$1"` inside the `command_string` ensures that the shell's dot operator
///   resolves paths with spaces without secondary word splitting.
///
/// # Prior Art in [`bass`]
///
/// Passing command arguments out-of-band via `$1` is an established pattern seen in
/// [`bass`] ([`__bass.py`]):
///
/// ```python
/// args = [BASH, '-c', command, 'bass', ' '.join(sys.argv[1:])]
/// ```
///
/// Here is how those arguments map in `bass`:
///
/// | Argument Index | Value passed by `bass`         | How Bash receives it |
/// | :------------- | :----------------------------- | :------------------- |
/// | `0`            | `BASH` (`/bin/bash`)           | The shell executable |
/// | `1`            | `'-c'`                         | Flag                 |
/// | `2`            | `command` (`'eval $1 && ...'`) | Command string       |
/// | `3`            | `'bass'`                       | Assigned to `$0`     |
/// | `4`            | `' '.join(sys.argv[1:])`       | Assigned to `$1`     |
///
/// `bass` explicitly passes the string `'bass'` so that it occupies the `$0` slot!
///
/// Our implementation differs from `bass` in two key ways:
/// 1. Direct sourcing: Rather than evaluating arbitrary command strings with `eval $1`,
///    we directly source the target file with `. "$1"`.
/// 2. Native null-delimited serialization: Rather than injecting a Python JSON serializer
///    into the subshell, we use native [`env -0`], preserving multi-line variables with
///    zero external runtime dependencies.
///
/// The environment is parsed via [`parse_env_unix`].
///
/// # Errors
///
/// Returns an error if spawning `/bin/sh` fails, capturing output fails, or the shell
/// process exits with an error status.
///
/// [`__bass.py`]: https://github.com/edc/bass/blob/master/functions/__bass.py
/// [`bass`]: https://github.com/edc/bass
/// [`Command`]: std::process::Command
/// [`env -0`]: https://www.gnu.org/software/coreutils/manual/html_node/env.html
/// [`parse_env_unix`]: parser::parse_env_unix
/// [`stderr`]: std::io::stderr
/// [`stdout`]: std::io::stdout
#[cfg(any(unix, doc))]
pub fn try_source_and_export_env_unix(
    script_path: &Path,
    base_env: &BaseEnv,
) -> miette::Result<EnvMap> {
    let mut cmd = Command::new("/bin/sh");

    // Configure hermetic initial environment if provided.
    if let BaseEnv::Hermetic(init_env_map) = base_env {
        cmd.env_clear();
        cmd.envs(init_env_map);
    }

    cmd.arg("-c")
        .arg(concat!(
            /* $1 passed in out of band, silence output of $1 */
            "{ . \"$1\"; } >/dev/null 2>&1",
            /* command separator */ " ; ",
            /* prints env vars (null-delimited) to stdout */ "env -0"
        ))
        .arg("env-source")
        .arg(script_path);

    let cmd_output_result = cmd.output();
    let output = match CommandOutputResult::from(cmd_output_result) {
        CommandOutputResult::SpawnFailed(err) => {
            DEBUG_ENV_SOURCE.then(|| {
                // % is Display, ? is Debug.
                tracing::error!(
                    message = "Failed to spawn or execute /bin/sh for env_source",
                    script_path = %script_path.display(),
                    error = %err
                );
            });
            return Err(miette!(
                "Failed to spawn or execute /bin/sh for script {}: {err}",
                script_path.display()
            ));
        }
        CommandOutputResult::NonZeroExit(out) => {
            DEBUG_ENV_SOURCE.then(|| {
                // % is Display, ? is Debug.
                tracing::warn!(
                    message = "Shell process exited with non-zero status in env_source",
                    script_path = %script_path.display(),
                    status = %out.status,
                    stderr = %String::from_utf8_lossy(&out.stderr)
                );
            });
            out
        }
        CommandOutputResult::Success(out) => out,
    };

    let env_map = parser::parse_env_unix(&output.stdout);

    Ok(env_map)
}

/// Evaluates the input using `cmd.exe` on Windows and captures the environment:
///
/// ```cmd
/// cmd.exe /c "(call %1) >nul 2>&1 & set"
/// ```
///
/// # Command Anatomy
///
/// - `cmd.exe /c`: Runs the command string in the Windows Command Prompt and terminates.
/// - `(call %1)`: Evaluates the batch script or inline command in the current environment
///   context.
/// - `>nul 2>&1 &`: Redirects both stdout and stderr to the `nul` device during
///   evaluation.
/// - `set`: Emits all current environment variables in `KEY=VALUE\r\n` format.
///
/// The environment is parsed via [`parse_env_windows`].
///
/// # Errors
///
/// Returns an error if spawning `cmd.exe` fails, capturing output fails, or the shell
/// process exits with an error status.
///
/// [`parse_env_windows`]: parser::parse_env_windows
#[cfg(any(windows, doc))]
pub fn try_source_and_export_env_windows(
    script_path: &Path,
    base_env: &BaseEnv,
) -> miette::Result<EnvMap> {
    let mut cmd = Command::new("cmd.exe");

    // Configure hermetic initial environment if provided.
    if let BaseEnv::Hermetic(mock_env) = base_env {
        cmd.env_clear();
        cmd.envs(mock_env);
    }

    // This strange block is gated to Windows because the function is allowed to be
    // compiled for docs (which compile in Linux).
    #[cfg(windows)]
    {
        // Isolate script stdout/stderr and output set environment.
        use std::os::windows::process::CommandExt;
        cmd.raw_arg(format!(
            "/c \"call \"{}\" >nul 2>&1 & set\"",
            script_path.display()
        ));
    }

    let cmd_output_result = cmd.output();
    let output = match CommandOutputResult::from(cmd_output_result) {
        CommandOutputResult::SpawnFailed(err) => {
            DEBUG_ENV_SOURCE.then(|| {
                // % is Display, ? is Debug.
                tracing::error!(
                    message = "Failed to spawn or execute cmd.exe for env_source",
                    script_path = %script_path.display(),
                    error = %err
                );
            });
            return Err(miette!(
                "Failed to spawn or execute cmd.exe for script {}: {err}",
                script_path.display()
            ));
        }
        CommandOutputResult::NonZeroExit(out) => {
            DEBUG_ENV_SOURCE.then(|| {
                // % is Display, ? is Debug.
                tracing::warn!(
                    message = "Shell process exited with non-zero status in env_source",
                    script_path = %script_path.display(),
                    status = %out.status,
                    stderr = %String::from_utf8_lossy(&out.stderr)
                );
            });
            out
        }
        CommandOutputResult::Success(out) => out,
    };

    let env_map = parser::parse_env_windows(&output.stdout);

    Ok(env_map)
}

#[cfg(all(test, unix))]
mod tests_run_shell_unix {
    use super::*;
    use crate::try_create_temp_dir;
    use std::io::Write;

    #[test]
    fn test_run_shell_file_input() -> miette::Result<()> {
        let temp_dir = try_create_temp_dir()?;
        let script_path = temp_dir.join("test_run_shell.sh");
        let mut file = std::fs::File::create(&script_path).unwrap();
        file.write_all(b"export SCRIPT_FILE_VAR=sourced_successfully\n")
            .unwrap();
        file.flush().unwrap();

        let mut mock_env = EnvMap::default();
        mock_env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());

        let mutated =
            try_source_and_export_env(&script_path, &BaseEnv::Hermetic(mock_env))?;

        assert_eq!(
            mutated.get("SCRIPT_FILE_VAR"),
            Some(&"sourced_successfully".to_string())
        );
        Ok(())
    }

    #[test]
    fn test_run_shell_stdout_isolation() -> miette::Result<()> {
        let temp_dir = try_create_temp_dir()?;
        let script_path = temp_dir.join("test_noisy.sh");
        let mut file = std::fs::File::create(&script_path).unwrap();
        file.write_all(
            b"echo 'loud stdout message'\necho 'loud stderr' >&2\nexport IS_QUIET=true\n",
        )
        .unwrap();
        file.flush().unwrap();

        let mut mock_env = EnvMap::default();
        mock_env.insert("BASE_VAR".to_string(), "base_val".to_string());
        mock_env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());

        let mutated =
            try_source_and_export_env(&script_path, &BaseEnv::Hermetic(mock_env))?;

        assert_eq!(mutated.get("IS_QUIET"), Some(&"true".to_string()));
        assert_eq!(mutated.get("BASE_VAR"), Some(&"base_val".to_string()));
        Ok(())
    }

    #[test]
    fn test_run_shell_inherit_env() -> miette::Result<()> {
        let temp_dir = try_create_temp_dir()?;
        let script_path = temp_dir.join("test_inherit.sh");
        let mut file = std::fs::File::create(&script_path).unwrap();
        file.write_all(b"export INHERIT_TEST_VAR=passed\n").unwrap();
        file.flush().unwrap();

        let mutated = try_source_and_export_env(&script_path, &BaseEnv::Inherit)?;

        assert_eq!(mutated.get("INHERIT_TEST_VAR"), Some(&"passed".to_string()));
        assert!(mutated.contains_key("PATH"));
        Ok(())
    }

    #[test]
    fn test_run_shell_nonzero_exit_still_captures_env() -> miette::Result<()> {
        let temp_dir = try_create_temp_dir()?;
        let script_path = temp_dir.join("test_nonzero.sh");
        let mut file = std::fs::File::create(&script_path).unwrap();
        file.write_all(b"export BEFORE_FAIL=captured_value\nfalse\n")
            .unwrap();
        file.flush().unwrap();

        let mutated = try_source_and_export_env(&script_path, &BaseEnv::Inherit)?;

        assert_eq!(
            mutated.get("BEFORE_FAIL"),
            Some(&"captured_value".to_string())
        );
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests_run_shell_windows {
    use super::*;
    use crate::try_create_temp_dir;
    use std::io::Write;

    #[test]
    fn test_run_shell_windows_file_input() -> miette::Result<()> {
        let temp_dir = try_create_temp_dir()?;
        let script_path = temp_dir.join("test_run_shell.bat");
        let mut file = std::fs::File::create(&script_path).unwrap();
        file.write_all(b"@echo off\r\nset SCRIPT_FILE_VAR=sourced_successfully\r\n")
            .unwrap();
        file.flush().unwrap();

        let mut mock_env = EnvMap::default();
        mock_env.insert("PATH".to_string(), "C:\\Windows\\system32".to_string());

        let mutated =
            try_source_and_export_env(&script_path, &BaseEnv::Hermetic(mock_env))?;

        assert_eq!(
            mutated.get("SCRIPT_FILE_VAR"),
            Some(&"sourced_successfully".to_string())
        );
        Ok(())
    }

    #[test]
    fn test_run_shell_windows_inherit_env() -> miette::Result<()> {
        let temp_dir = try_create_temp_dir()?;
        let script_path = temp_dir.join("test_inherit.bat");
        let mut file = std::fs::File::create(&script_path).unwrap();
        file.write_all(b"@echo off\r\nset INHERIT_TEST_VAR=passed\r\n")
            .unwrap();
        file.flush().unwrap();

        let mutated = try_source_and_export_env(&script_path, &BaseEnv::Inherit)?;

        assert_eq!(mutated.get("INHERIT_TEST_VAR"), Some(&"passed".to_string()));
        assert!(mutated.contains_key("PATH") || mutated.contains_key("Path"));
        Ok(())
    }
}

// cspell:words Popen

// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::ProcessExecutionOutcome;
use std::process::{ExitStatus, Stdio};
use tokio::{io::AsyncWriteExt, process::Child};

/// Disambiguate the [`tokio::process::Command`] type from the [`std::process::Command`]
/// type. Here are the key differences between them:
///
/// 1. **Execution Model**: [`tokio`]'s `Command` is asynchronous and doesn't block the
///    thread, while std's `Command` is synchronous and blocks until completion.
/// 2. **Method Signatures**: Similar methods but [`tokio`]'s version returns futures that
///    must be awaited.
/// 3. **Runtime Integration**: [`tokio`]'s `Command` integrates with [`tokio`]'s runtime
///    and event loop, allowing it to work with other async features like select!.
/// 4. **Process Management**: [`tokio`] provides additional features like
///    `kill_on_drop()` and non-blocking `start_kill()` for better process management in
///    async contexts.
/// 5. **Use Case**: Use [`tokio`]'s `Command` when working in async contexts to maintain
///    non-blocking behavior, and std's `Command` for synchronous operations where
///    blocking is acceptable.
///
/// [`tokio`]: tokio
pub type TokioCommand = tokio::process::Command;

/// Extension trait on [`TokioCommand`] for running a command non-interactively and
/// returning its captured [`stdout`] bytes with typed [`CommandRunError`] diagnostics.
///
/// Use this trait (typically alongside [`command!`]) when you want ergonomic method
/// syntax (`cmd.run().await`) instead of calling [`try_run`] directly and manually checking
/// [`std::process::Output::status`].
///
/// [`command!`]: crate::command
/// [`stdout`]: std::io::stdout
pub trait Run {
    /// Executes the command non-interactively with piped [`stdin`], [`stdout`], and
    /// [`stderr`], returning the captured [`stdout`] bytes on zero exit status.
    ///
    /// Delegates to [`try_run`]. The returned future is [`Send`], so it can be awaited
    /// inside spawned Tokio tasks.
    ///
    /// # Errors
    ///
    /// Returns [`CommandRunError`] if:
    /// - The command program does not exist or cannot be executed
    ///   ([`CommandRunError::SpawnFailed`]).
    /// - The command exits with a non-zero status ([`CommandRunError::NonZeroExit`]).
    ///
    /// [`stderr`]: std::io::stderr
    /// [`stdin`]: std::io::stdin
    /// [`stdout`]: std::io::stdout
    fn run(
        &mut self,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, CommandRunError>> + Send;
}

impl Run for TokioCommand {
    async fn run(&mut self) -> Result<Vec<u8>, CommandRunError> { try_run(self).await }
}

/// Outcome of running a [`TokioCommand`] together with the [`TokioCommand`] that was
/// executed.
#[derive(Debug)]
pub struct CommandRunOutcome<T> {
    /// The result produced by the command that executed.
    pub result: Result<T, CommandRunError>,

    /// The command that was executed to produce the result. This can be used for
    /// display, logging, or auditing.
    pub command: TokioCommand,
}

/// Error returned by [`Run::run`], [`try_run`], and [`try_pipe`] when spawning, piping, or
/// waiting on a child process fails.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CommandRunError {
    /// Failed to start the command process.
    #[error("Command '{program}' failed to start")]
    SpawnFailed {
        /// Program name or path passed to [`TokioCommand`].
        program: String,
        /// Underlying OS spawn error.
        #[source]
        src_err: std::io::Error,
    },

    /// Command ran to completion and exited with a non-zero status.
    #[error("Command '{cmd_str}' exited with non-zero status {status}: {stderr}")]
    NonZeroExit {
        /// Debug representation of the executed command.
        cmd_str: String,
        /// Non-zero exit status returned by the child process.
        status: ExitStatus,
        /// Captured [`stderr`] decoded as lossy [`UTF-8`].
        ///
        /// [`stderr`]: std::io::stderr
        /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
        stderr: String,
    },

    /// Writing piped bytes to a child process [`stdin`] failed.
    ///
    /// [`stdin`]: std::io::stdin
    #[error("Pipe I/O failed while running '{program}'")]
    PipeIo {
        /// Program name of the receiving command in a pipeline.
        program: String,
        /// Underlying OS I/O error.
        #[source]
        src_err: std::io::Error,
    },
}

/// This macro to create a [`TokioCommand`] that receives a set of arguments and
/// returns it.
///
/// # Example of command and args
///
/// ```
/// # use r3bl_tui::command;
/// # use r3bl_tui::TokioCommand;
///
/// async fn run_command() {
///     let arg_2 = "world!";
///     let mut command = command!(
///         program => "echo",
///         args => "Hello,", arg_2,
///     );
///     let output = command.output().await.expect("Failed to execute command");
///     assert!(output.status.success());
///     assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, world!\n");
/// }
/// ```
///
/// # Example of command, env, and args
///
/// ```
/// # use r3bl_tui::{command, TokioCommand, gen_path_env_vars};
///
/// async fn run_command() {
///     let my_path = "/usr/bin";
///     let env_vars = gen_path_env_vars(my_path);
///     let mut command = command!(
///         program => "printenv",
///         envs => env_vars,
///         args => "PATH",
///     );
///     let output = command.output().await.expect("Failed to execute command");
///     assert!(output.status.success());
///     assert_eq!(String::from_utf8_lossy(&output.stdout), "/usr/bin\n");
/// }
/// ```
///
/// # Examples of using the [Run] trait and [`tokio::process::Command::output()`].
///
/// ```
/// # use r3bl_tui::command;
/// # use r3bl_tui::Run;
///
/// async fn run_command() {
///     // Example 1.
///     let output = command!(
///        program => "echo",
///        args => "Hello,", "world!",
///     )
///     .output()
///     .await
///     .expect("conversion error");
///     assert!(output.status.success());
///
///     // Example 2.
///     let arg_2 = "world!";
///     let run_bytes = command!(
///       program => "echo",
///       args => "Hello,", arg_2,
///     )
///     .run()
///     .await
///     .expect("conversion error");
///     assert_eq!(String::from_utf8_lossy(&run_bytes), "Hello, world!\n");
///
///     // Example 3.
///     let items = vec!["item1", "item2"];
///     let cmd = command!(
///         program => "echo",
///         args => "Hello, world!",
///         + items => items
///     );
///     assert_eq!(String::from_utf8_lossy(&run_bytes), "Hello, world! item1 item2\n");
/// }
/// ```
#[macro_export]
macro_rules! command {
    // Variant that receives a command and args & items.
    (program=> $cmd:expr, args => $($args:expr,)* + items => $items:expr)
    => {{
        let mut it = $crate::TokioCommand::new($cmd);
        $(
            it.arg($args);
        )*
        for item in $items {
            // The item must implement `AsRef<OsStr>` and `SmallString` does not, so convert it to a `String`.
            it.arg(item.to_string());
        }
        it
    }};

    // Variant that receives a command and args.
    (program=> $cmd:expr, args=> $($args:expr),* $(,)?) => {{
        let mut it = $crate::TokioCommand::new($cmd);
        $(
            it.arg($args);
        )*
        it
    }};

    // Variant that receives a command, env (vec), and args.
    (program=> $cmd:expr, envs=> $envs:expr, args=> $($args:expr),* $(,)?) => {{
        let mut it = $crate::TokioCommand::new($cmd);
        it.envs($envs.to_owned());
        // The following is equivalent to the line above:
        // it.envs($envs.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        $(
            it.arg($args);
        )*
        it
    }};
}

#[macro_export]
macro_rules! bail_command_ran_and_failed {
    ($command:expr, $status:expr, $stderr:expr) => {
        return Err($crate::CommandRunError::NonZeroExit {
            cmd_str: format!("{:?}", $command),
            status: $status,
            stderr: String::from_utf8_lossy(&$stderr).into_owned(),
        });
    };
}

/// This command is not allowed to have user interaction. It does not inherit the
/// [`stdin`], [`stdout`], [`stderr`] from the parent (aka current) process.
///
/// See the tests for examples of how to use this.
///
/// # Errors
///
/// Returns [`CommandRunError`] if:
/// - The command program does not exist or cannot be executed
///   ([`CommandRunError::SpawnFailed`])
/// - The command fails with a non-zero exit status ([`CommandRunError::NonZeroExit`])
///
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
pub async fn try_run(command: &mut TokioCommand) -> Result<Vec<u8>, CommandRunError> {
    // Try to run command (might be unable to run it if the program is invalid).
    let res_output = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match ProcessExecutionOutcome::from(res_output) {
        ProcessExecutionOutcome::Success(output) => ok!(output.stdout),
        ProcessExecutionOutcome::NonZeroExit(output) => {
            bail_command_ran_and_failed!(command, output.status, output.stderr);
        }
        ProcessExecutionOutcome::SpawnFailed(src_err) => {
            Err(CommandRunError::SpawnFailed {
                program: command
                    .as_std()
                    .get_program()
                    .to_string_lossy()
                    .into_owned(),
                src_err,
            })
        }
    }
}

/// Mimics the behavior of the Unix pipe operator `|`, ie: `command_one |
/// command_two`.
/// - The output of the first command is passed as input to the second command.
/// - The output of the second command is returned.
/// - If either command fails, an error is returned.
///
/// Only `command_one` is allowed to have any user interaction. It is set to inherit
/// the [`stdin`], [`stdout`], [`stderr`] from the parent (aka current) process. Here's an
/// example which will block on user input from an interactive terminal if executed:
///
/// ```
/// use r3bl_tui::command;
///
/// let mut command_one = command!(
///     program => "/usr/bin/bash",
///     args => "-c", "read -p 'Enter your input: ' input"
/// );
/// ```
///
/// # Errors
///
/// Returns [`CommandRunError`] if:
/// - Either command program does not exist or cannot be executed
///   ([`CommandRunError::SpawnFailed`])
/// - Either command fails with a non-zero exit status ([`CommandRunError::NonZeroExit`])
/// - Writing piped bytes to `command_two` fails ([`CommandRunError::PipeIo`])
///
/// [`stderr`]: std::io::stderr
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
pub async fn try_pipe(
    command_one: &mut TokioCommand,
    command_two: &mut TokioCommand,
) -> Result<String, CommandRunError> {
    // Run the first command & get the output.
    let command_one = command_one
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Try to run command_one (might be unable to run it if the program is invalid).
    let res_output_one = command_one.output().await;
    let command_one_stdout = match ProcessExecutionOutcome::from(res_output_one) {
        ProcessExecutionOutcome::Success(output) => output.stdout,
        ProcessExecutionOutcome::NonZeroExit(output) => {
            bail_command_ran_and_failed!(command_one, output.status, output.stderr);
        }
        ProcessExecutionOutcome::SpawnFailed(src_err) => {
            return Err(CommandRunError::SpawnFailed {
                program: command_one
                    .as_std()
                    .get_program()
                    .to_string_lossy()
                    .into_owned(),
                src_err,
            });
        }
    };

    // Spawn the second command, make it to accept piped input from the first command.
    let command_two = command_two
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let command_two_program = command_two
        .as_std()
        .get_program()
        .to_string_lossy()
        .into_owned();
    let mut child_handle: Child =
        command_two
            .spawn()
            .map_err(|src_err| CommandRunError::SpawnFailed {
                program: command_two_program.clone(),
                src_err,
            })?;
    if let Some(mut child_stdin) = child_handle.stdin.take() {
        child_stdin
            .write_all(&command_one_stdout)
            .await
            .map_err(|src_err| CommandRunError::PipeIo {
                program: command_two_program.clone(),
                src_err,
            })?;
    }
    let res_output_two = child_handle.wait_with_output().await;
    match ProcessExecutionOutcome::from(res_output_two) {
        ProcessExecutionOutcome::Success(output) => {
            ok!(String::from_utf8_lossy(&output.stdout).to_string())
        }
        ProcessExecutionOutcome::NonZeroExit(output) => {
            bail_command_ran_and_failed!(command_two, output.status, output.stderr);
        }
        ProcessExecutionOutcome::SpawnFailed(src_err) => {
            Err(CommandRunError::SpawnFailed {
                program: command_two_program,
                src_err,
            })
        }
    }
}

#[cfg(test)]
mod tests_command_run {
    use super::*;
    use crate::ItemsOwned;

    #[test]
    fn test_command_with_list_of_args() {
        let items: ItemsOwned = (&["item1", "item2"]).into();
        let cmd = command!(
            program => "echo",
            args => "Hello, world!",
            + items => items
        );
        println!("Command: {cmd:?}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_run() {
        let output = command!(
            program => "echo",
            args => "Hello, world!",
        )
        .run()
        .await
        .expect("conversion error");

        // This captures the output.
        assert_eq!(String::from_utf8_lossy(&output), "Hello, world!\n");
    }

    /// Windows variant: `echo` is a shell builtin (no `echo.exe`), so we
    /// invoke it via `cmd /c echo`. The output includes a trailing `\r\n`.
    ///
    /// **Important**: Each word must be a separate arg to avoid Rust's MSVC
    /// command-line quoting. A single `"Hello, world!"` arg gets quoted as
    /// `"Hello, world!"` on the command line, and cmd.exe's `echo` includes
    /// those quotes in its output. Splitting into `"Hello,"` and `"world!"`
    /// avoids quoting since neither contains spaces.
    #[cfg(windows)]
    #[tokio::test]
    async fn test_run() {
        let output = command!(
            program => "cmd",
            args => "/c", "echo", "Hello,", "world!",
        )
        .run()
        .await
        .expect("conversion error");

        assert_eq!(String::from_utf8_lossy(&output).trim(), "Hello, world!");
    }

    #[tokio::test]
    async fn test_run_invalid_command() {
        let result = command!(
            program => "does_not_exist",
            args => "Hello, world!",
        )
        .run()
        .await;
        if let Err(err) = result {
            assert!(err.to_string().contains("does_not_exist"));
        } else {
            panic!("Expected an error, but got success");
        }
    }

    /// This test pipes `echo` into `bash` — both are Unix executables (on
    /// Windows, `echo` is a shell builtin and `bash` is not in the default
    /// PATH).
    #[cfg(unix)]
    #[tokio::test]
    async fn test_pipe_command_two_not_interactive_terminal() {
        let mut command_one = command!(
            program => "echo",
            args => "hello world",
        );
        // Use "bash" without path to find it in PATH (works on both Linux and macOS).
        // Linux has bash at /usr/bin/bash, macOS has it at /bin/bash.
        let mut command_two = command!(
            program => "bash",
            args => "-c", "read -p 'Enter your input: ' input"
        );
        let result = try_pipe(&mut command_one, &mut command_two).await;
        // This is not an error when using tokio::process::Command. However, when using
        // std::process::Command, this will result in an error.
        assert_eq!("", result.expect("conversion error"));
    }

    #[tokio::test]
    async fn test_pipe_invalid_command() {
        let result = try_pipe(
            &mut command!(
                program => "does_not_exist",
                args => "Hello, world!",
            ),
            &mut command!(
                program => "wc",
                args => "-w",
            ),
        )
        .await;
        if let Err(err) = result {
            assert!(err.to_string().contains("does_not_exist"));
        } else {
            panic!("Expected an error, but got success");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_run_nonzero_exit() {
        let result = command!(
            program => "sh",
            args => "-c", "exit 42",
        )
        .run()
        .await;

        match result {
            Err(CommandRunError::NonZeroExit { status, .. }) => {
                assert_eq!(status.code(), Some(42));
            }
            other => panic!("Expected NonZeroExit(42), got: {other:?}"),
        }
    }
}

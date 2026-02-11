// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::ok;
use miette::{Context, IntoDiagnostic};
use std::process::Stdio;
use tokio::{io::AsyncWriteExt,
            process::{Child, Command}};

/// Disambiguate the [`tokio::process::Command`] type from the [`std::process::Command`]
/// type. Here are the key differences between them:
///
/// 1. **Execution Model**: tokio's `Command` is asynchronous and doesn't block the
///    thread, while std's `Command` is synchronous and blocks until completion.
/// 2. **Method Signatures**: Similar methods but tokio's version returns futures that
///    must be awaited.
/// 3. **Runtime Integration**: tokio's `Command` integrates with tokio's runtime and
///    event loop, allowing it to work with other async features like select!.
/// 4. **Process Management**: tokio provides additional features like `kill_on_drop()`
///    and non-blocking `start_kill()` for better process management in async contexts.
/// 5. **Use Case**: Use tokio's `Command` when working in async contexts to maintain
///    non-blocking behavior, and std's `Command` for synchronous operations where
///    blocking is acceptable.
pub type TokioCommand = tokio::process::Command;

/// This macro to create a [`TokioCommand`] that receives a set of arguments and
/// returns it.
///
/// # Example of command and args
///
/// ```
/// # use r3bl_tui::command;
/// # use r3bl_tui::command_runner::TokioCommand;
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
/// # use r3bl_tui::command_runner::Run;
///
/// async fn run_command() {
///     // Example 1.
///     let output = command!(
///        program => "echo",
///        args => "Hello,", "world!",
///     )
///     .output()
///     .await
///     .unwrap();
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
///     .unwrap();
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

pub trait Run {
    /// # Errors
    ///
    /// Returns an error if:
    /// - The command program does not exist or cannot be executed
    /// - The command fails with a non-zero exit status
    /// - I/O errors occur during command execution
    fn run(
        &mut self,
    ) -> impl std::future::Future<Output = miette::Result<Vec<u8>>> + Send;

    /// # Errors
    ///
    /// Returns an error if:
    /// - The command program does not exist or cannot be executed
    /// - The command fails with a non-zero exit status
    /// - I/O errors occur during command execution
    fn run_interactive(
        &mut self,
    ) -> impl std::future::Future<Output = miette::Result<Vec<u8>>> + Send;
}

impl Run for TokioCommand {
    #[allow(clippy::missing_errors_doc)]
    async fn run(&mut self) -> miette::Result<Vec<u8>> { run(self).await }

    #[allow(clippy::missing_errors_doc)]
    async fn run_interactive(&mut self) -> miette::Result<Vec<u8>> {
        run_interactive(self).await
    }
}

#[macro_export]
macro_rules! bail_command_ran_and_failed {
        ($command:expr, $status:expr, $stderr:expr) => {
            use $crate::{fg_lizard_green, fg_frozen_blue, fg_magenta};
            miette::bail!(
                "{name} failed\n{cmd_label}: '{cmd:?}'\n{status_label}: '{status}'\n{stderr_label}: '{stderr}'",
                cmd_label = fg_lizard_green("[command]"),
                status_label = fg_lizard_green("[status]"),
                stderr_label = fg_lizard_green("[stderr]"),
                name = fg_frozen_blue(stringify!($command)),
                cmd = $command,
                status = fg_magenta(&format!("{:?}", $status)),
                stderr = fg_magenta(&String::from_utf8_lossy(&$stderr)),
            );
        };
    }

/// This command is not allowed to have user interaction. It does not inherit the
/// `stdin`, `stdout`, `stderr` from the parent (aka current) process.
///
/// See the tests for examples of how to use this.
///
/// # Errors
///
/// Returns an error if:
/// - The command program does not exist or cannot be executed
/// - The command fails with a non-zero exit status
/// - I/O errors occur during command execution
pub async fn run(command: &mut Command) -> miette::Result<Vec<u8>> {
    // Try to run command (might be unable to run it if the program is invalid).
    let output = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .into_diagnostic()
        .wrap_err(miette::miette!("Unable to run command: {:?}", command))?;

    // At this point, command_one has run, but it might result in a success or failure.
    if output.status.success() {
        ok!(output.stdout)
    } else {
        bail_command_ran_and_failed!(command, output.status, output.stderr);
    }
}

/// This command is allowed to have full user interaction. It inherits the `stdin`,
/// `stdout`, `stderr` from the parent (aka current) process.
///
/// See the tests for examples of how to use this.
///
/// Here's an example which will block on user input from an interactive terminal if
/// executed:
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
/// Returns an error if:
/// - The command program does not exist or cannot be executed
/// - The command fails with a non-zero exit status
/// - I/O errors occur during command execution
pub async fn run_interactive(command: &mut Command) -> miette::Result<Vec<u8>> {
    // Try to run command (might be unable to run it if the program is invalid).
    let output = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .await
        .into_diagnostic()
        .wrap_err(miette::miette!("Unable to run command: {:?}", command))?;

    // At this point, command_one has run, but it might result in a success or failure.
    if output.status.success() {
        ok!(output.stdout)
    } else {
        bail_command_ran_and_failed!(command, output.status, output.stderr);
    }
}

/// Mimics the behavior of the Unix pipe operator `|`, ie: `command_one |
/// command_two`.
/// - The output of the first command is passed as input to the second command.
/// - The output of the second command is returned.
/// - If either command fails, an error is returned.
///
/// Only `command_one` is allowed to have any user interaction. It is set to inherit
/// the `stdin`, `stdout`, `stderr` from the parent (aka current) process. Here's an
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
/// Returns an error if:
/// - Either command program does not exist or cannot be executed
/// - Either command fails with a non-zero exit status
/// - I/O errors occur during command execution or piping
pub async fn pipe(
    command_one: &mut Command,
    command_two: &mut Command,
) -> miette::Result<String> {
    // Run the first command & get the output.
    let command_one = command_one
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Try to run command_one (might be unable to run it if the program is invalid).
    let command_one_output =
        command_one
            .output()
            .await
            .into_diagnostic()
            .wrap_err(miette::miette!(
                "Unable to run command_one: {:?}",
                command_one
            ))?;
    // At this point, command_one has run, but it might result in a success or failure.
    if !command_one_output.status.success() {
        bail_command_ran_and_failed!(
            command_one,
            command_one_output.status,
            command_one_output.stderr
        );
    }
    let command_one_stdout = command_one_output.stdout;

    // Spawn the second command, make it to accept piped input from the first command.
    let command_two = command_two
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Try to run command_one (might be unable to run it if the program is invalid).
    let mut child_handle: Child =
        command_two
            .spawn()
            .into_diagnostic()
            .wrap_err(miette::miette!(
                "Unable to run command_two: {:?}",
                command_two
            ))?;
    if let Some(mut child_stdin) = child_handle.stdin.take() {
        child_stdin
            .write_all(&command_one_stdout)
            .await
            .into_diagnostic()?;
    }
    // At this point, command_one has run, but it might result in a success or failure.
    let command_two_output = child_handle.wait_with_output().await.into_diagnostic()?;
    if command_two_output.status.success() {
        ok!(String::from_utf8_lossy(&command_two_output.stdout).to_string())
    } else {
        bail_command_ran_and_failed!(
            command_two,
            command_two_output.status,
            command_two_output.stderr
        );
    }
}

#[cfg(test)]
mod tests_command_runner {
    use super::*;
    use crate::ItemsOwned;

    #[tokio::test]
    async fn test_command_with_list_of_args() {
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
        .unwrap();

        // This captures the output.
        assert_eq!(String::from_utf8_lossy(&output), "Hello, world!\n");

        // This dumps the output to the parent process' stdout & is captured by
        // tokio::process::Command but not by std::process::Command.
        let output = command!(
            program => "echo",
            args => "Hello, world!",
        )
        .run_interactive()
        .await
        .unwrap();
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
        .unwrap();

        assert_eq!(String::from_utf8_lossy(&output).trim(), "Hello, world!");

        let output = command!(
            program => "cmd",
            args => "/c", "echo", "Hello,", "world!",
        )
        .run_interactive()
        .await
        .unwrap();
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
        let result = pipe(&mut command_one, &mut command_two).await;
        // This is not an error when using tokio::process::Command. However, when using
        // std::process::Command, this will result in an error.
        assert_eq!("", result.unwrap());
    }

    #[tokio::test]
    async fn test_pipe_invalid_command() {
        let result = pipe(
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
}

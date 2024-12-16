/*
 *   Copyright (c) 2024 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::process::Stdio;

use miette::{Context, IntoDiagnostic};
use r3bl_core::ok;
use tokio::{io::AsyncWriteExt as _,
            process::{Child, Command}};

/// This macro to create a [std::process::Command] that receives a set of arguments and
/// returns it.
///
/// # Example of command and args
///
/// ```
/// use r3bl_script::command;
/// use std::process::Command;
///
/// async fn run_command() {
///     let mut command = command!(
///         program => "echo",
///         args => "Hello, world!",
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
/// use r3bl_script::command;
/// use r3bl_script::environment::{self, EnvKeys};
/// use std::process::Command;
///
/// async fn run_command() {
///     let my_path = "/usr/bin";
///     let env_vars = environment::get_env_vars(EnvKeys::Path, my_path);
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
/// # Examples of using the [Run] trait, and [std::process::Output].
///
/// ```
/// use r3bl_script::command;
/// use r3bl_script::command_runner::Run;
///
/// async fn run_command() {
///     let output = command!(
///        program => "echo",
///        args => "Hello, world!",
///     )
///     .output()
///     .await
///     .unwrap();
///     assert!(output.status.success());
///
///     let run_bytes = command!(
///       program => "echo",
///       args => "Hello, world!",
///     )
///     .run()
///     .await
///     .unwrap();
///     assert_eq!(String::from_utf8_lossy(&run_bytes), "Hello, world!\n");
/// }
/// ```
#[macro_export]
macro_rules! command {
        // Variant that receives a command and args.
        (program=> $cmd:expr, args=> $($args:expr),* $(,)?) => {{
            let mut it = tokio::process::Command::new($cmd);
            $(
                it.arg($args);
            )*
            it
        }};

        // Variant that receives a command, env (vec), and args.
        (program=> $cmd:expr, envs=> $envs:expr, args=> $($args:expr),* $(,)?) => {{
            let mut it = tokio::process::Command::new($cmd);
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
    fn run(
        &mut self,
    ) -> impl std::future::Future<Output = miette::Result<Vec<u8>>> + Send;
    fn run_interactive(
        &mut self,
    ) -> impl std::future::Future<Output = miette::Result<Vec<u8>>> + Send;
}

impl Run for Command {
    async fn run(&mut self) -> miette::Result<Vec<u8>> { run(self).await }

    async fn run_interactive(&mut self) -> miette::Result<Vec<u8>> {
        run_interactive(self).await
    }
}

#[macro_export]
macro_rules! bail_command_ran_and_failed {
        ($command:expr, $status:expr, $stderr:expr) => {
            use crossterm::style::Stylize as _;
            miette::bail!(
                "{name} failed\n{cmd_label}: '{cmd:?}'\n{status_label}: '{status}'\n{stderr_label}: '{stderr}'",
                cmd_label = "[command]".to_string().yellow(),
                status_label = "[status]".to_string().yellow(),
                stderr_label = "[stderr]".to_string().yellow(),
                name = stringify!($command).blue(),
                cmd = $command,
                status = format!("{:?}", $status).magenta(),
                stderr = String::from_utf8_lossy(&$stderr).magenta(),
            );
        };
    }

/// This command is not allowed to have user interaction. It does not inherit the
/// `stdin`, `stdout`, `stderr` from the parent (aka current) process.
///
/// See the tests for examples of how to use this.
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
/// use r3bl_script::command;
///
/// let mut command_one = command!(
///     program => "/usr/bin/bash",
///     args => "-c", "read -p 'Enter your input: ' input"
/// );
/// ```
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
/// use r3bl_script::command;
///
/// let mut command_one = command!(
///     program => "/usr/bin/bash",
///     args => "-c", "read -p 'Enter your input: ' input"
/// );
/// ```
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

    #[tokio::test]
    async fn test_pipe_command_two_not_interactive_terminal() {
        let mut command_one = command!(
            program => "echo",
            args => "hello world",
        );
        let mut command_two = command!(
            program => "/usr/bin/bash",
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

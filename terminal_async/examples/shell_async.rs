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

// 00: 📌 fix prompt size in terminal_async, which size the prompt with ansi stripped length
// 00: add a colorized prompt in this example

use std::io::Write;

use child_process_constructor::*;
use crossterm::style::Stylize;
use miette::IntoDiagnostic;
use r3bl_rs_utils_core::ok;
use r3bl_terminal_async::{ReadlineEvent, SharedWriter, TerminalAsync};
use terminal_async_constructor::*;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    process::Child,
    sync::broadcast,
};

/// This program uses the `r3bl_terminal_async` crate to provide a prompt and get user
/// input, pass that to the `stdin` of a `bash` child process, and then display the output
/// from the child process in the terminal. The followings steps outline what the program
/// does:
///
/// # Create some shared global variables
///
/// - A broadcast channel to signal shutdown to the child process, and all the spawned
///   tasks.
/// - [r3bl_terminal_async::TerminalAsync] to write to the terminal. This provides the
///   mechanism to collect user input and display output.
/// - [tokio::process::Child] to spawn the child process (`bash`) and interact with it.
///   This child process lives as long as the `main` function and exits when the user
///   chooses to exit the program.
///   - The [tokio::process::Command] starts `bash`.
///   - Both `stdin` and `stdout` are piped using [std::process::Stdio::piped].
///
/// # 🧵 The main event loop simply waits for the following (on the current thread)
///
/// - Start a main event loop (on the current thread):
///   - The shutdown signal from the broadcast channel, and monitors the
///     [r3bl_terminal_async::TerminalAsync] for user input. It writes the user input to the
///     [tokio::process::ChildStdin].
///   - Any exit inputs (user types "exit" or "Ctrl+D") from the user are captured here and
///     sent to the shutdown broadcast channel. It also listens to the broadcast channel to
///     break out of the loop on shutdown.
///   - It [tokio::process::Child::kill]s the child process when it gets the exit signal.
///   - It does not monitor the terminal for user input or the child process for output.
///
/// # 🚀 Spawn a new task to loop and read the output from the child process and display it
///
/// - Spawn a task to loop:
///   - Read the [tokio::process::ChildStdout] and write it to the
///     [r3bl_terminal_async::SharedWriter].
///   - Also listen to the broadcast channel to break out of the loop on shutdown.
///
/// # Run the binary
///
/// ```text
/// ┌───────────────────────────────────┐
/// │ > cargo run --example shell_async │
/// └───────────────────────────────────┘
/// ```
///
/// Type the following commands to have a go at this.
///
/// ```text
/// msg="hello nadia!"
/// echo $msg
/// ```
///
/// You should see something like the following.
///
/// ```text
/// [1606192] > msg="hello nadia!"
/// [1606192] > echo $msg
/// hello nadia!
/// [1606192] >
/// ```
///
/// # Clean up any left over processes
/// ```text
/// ┌───────────────────────────────┐
/// │ > killall -9 bash shell_async │
/// └───────────────────────────────┘
/// ```
#[tokio::main]
pub async fn main() -> miette::Result<()> {
    let (shutdown_sender, _) = broadcast::channel::<()>(1);

    let ChildProcessHandle {
        pid,
        child,
        stdout,
        stdin,
        stderr,
    } = create_child_process("bash")?;

    let TerminalAsyncHandle {
        terminal_async,
        shared_writer,
    } = create_terminal_async(pid).await?;

    _ = tokio::join!(
        // New green thread.
        spawn_monitor_output_from_child(stdout, stderr, shared_writer, shutdown_sender.clone()),
        // Current thread.
        monitor_user_input_to_child::start_event_loop(
            stdin,
            terminal_async,
            child,
            shutdown_sender.clone(),
        )
    );

    ok!()
}

pub mod monitor_user_input_to_child {
    use super::*;

    pub async fn start_event_loop(
        mut stdin: tokio::process::ChildStdin,
        mut terminal_async: TerminalAsync,
        child: Child,
        shutdown_sender: broadcast::Sender<()>,
    ) -> miette::Result<()> {
        let mut shutdown_receiver = shutdown_sender.subscribe();

        loop {
            tokio::select! {
                // Branch: Monitor shutdown signal. This is cancel safe as `recv()` is
                // cancel safe.
                _ = shutdown_receiver.recv() => {
                    _ = perform_shutdown(child).await;
                    break;
                },
                // Branch: Monitor terminal_async for user input. This is cancel safe as
                // `get_readline_event()` is cancel safe.
                result_readline_event = terminal_async.get_readline_event() => {
                    handle_user_input_event(
                        result_readline_event,
                        shutdown_sender.clone(),
                        &mut stdin
                    ).await
                }
            }
        }

        ok!()
    }

    enum EarlyReturn {
        Yes,
        YesAndShutdown,
        No(String),
    }

    impl From<miette::Result<ReadlineEvent>> for EarlyReturn {
        fn from(result_readline_event: miette::Result<ReadlineEvent>) -> Self {
            match result_readline_event {
                Ok(readline_event) => match readline_event {
                    ReadlineEvent::Line(input) => EarlyReturn::No(input),
                    ReadlineEvent::Resized => EarlyReturn::Yes,
                    ReadlineEvent::Eof | ReadlineEvent::Interrupted => EarlyReturn::YesAndShutdown,
                },
                Err(_) => EarlyReturn::YesAndShutdown,
            }
        }
    }

    /// - If the user types a line, then write it to the child process.
    /// - If this function needs to exit, then send a shutdown signal.
    async fn handle_user_input_event(
        result_readline_event: miette::Result<ReadlineEvent>,
        shutdown_sender: broadcast::Sender<()>,
        stdin: &mut tokio::process::ChildStdin,
    ) {
        // Early return check.
        let input = match EarlyReturn::from(result_readline_event) {
            EarlyReturn::No(line) => line,
            // Just return.
            EarlyReturn::Yes => return,
            // Return and shutdown.
            EarlyReturn::YesAndShutdown => {
                let _ = shutdown_sender.send(());
                return;
            }
        };

        // Trim leading & trailing whitespace (including newlines).
        let input = input.trim();

        match input {
            // Send a shutdown signal. Don't write this input to the child process.
            "exit" => {
                let _ = shutdown_sender.send(());
            }
            // Write the user input to the child process.
            _ => {
                let input = format!("{}\n", input);
                _ = stdin.write_all(input.as_bytes()).await;
                _ = stdin.flush().await;
            }
        }
    }

    /// Perform graceful shutdown tasks.
    async fn perform_shutdown(mut child: Child) -> miette::Result<()> {
        child.kill().await.into_diagnostic()?;

        ok!()
    }
}

pub fn spawn_monitor_output_from_child(
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    mut shared_writer: r3bl_terminal_async::SharedWriter,
    shutdown_sender: tokio::sync::broadcast::Sender<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut stdout_buf_reader = tokio::io::BufReader::new(stdout).lines();
        let mut stderr_buf_reader = tokio::io::BufReader::new(stderr).lines();
        let mut shutdown_receiver = shutdown_sender.subscribe();

        loop {
            tokio::select! {
                // Branch: Monitor shutdown signal. This is cancel safe as `recv()` is
                // cancel safe.
                _ = shutdown_receiver.recv() => {
                    break;
                },
                // Branch: Monitor stderr for output from the child process. This is
                // cancel safe as `next_line()` is cancel safe.
                result_line = stderr_buf_reader.next_line() => {
                    match result_line {
                        Ok(Some(line)) => {
                            let line = line.to_string().red();
                            _ = writeln!(shared_writer, "{}", line).into_diagnostic();
                        },
                        _ => {
                            _ = shutdown_sender.send(()).into_diagnostic();
                            break;
                        },
                    }
                },
                // Branch: Monitor stdout for output from the child process. This is
                // cancel safe as `next_line()` is cancel safe.
                result_line = stdout_buf_reader.next_line() => {
                    match result_line {
                        Ok(Some(line)) => {
                            let line = line.to_string().green();
                            _ = writeln!(shared_writer, "{}", line).into_diagnostic();
                        },
                        _ => {
                            _ = shutdown_sender.send(()).into_diagnostic();
                            break;
                        },
                    }

                }
            }
        }
    })
}

pub mod terminal_async_constructor {
    use super::*;

    pub struct TerminalAsyncHandle {
        pub terminal_async: TerminalAsync,
        pub shared_writer: SharedWriter,
    }

    pub async fn create_terminal_async(pid: u32) -> miette::Result<TerminalAsyncHandle> {
        let prompt = format!("[{pid}] > ");
        let terminal_async = TerminalAsync::try_new(prompt.as_str())
            .await?
            .ok_or_else(|| miette::miette!("Failed to create terminal"))?;
        let shared_writer = terminal_async.clone_shared_writer();
        ok!(TerminalAsyncHandle {
            terminal_async,
            shared_writer,
        })
    }
}

pub mod child_process_constructor {
    use super::*;

    pub struct ChildProcessHandle {
        pub pid: u32,
        pub child: tokio::process::Child,
        pub stdout: tokio::process::ChildStdout,
        pub stdin: tokio::process::ChildStdin,
        pub stderr: tokio::process::ChildStderr,
    }

    pub fn create_child_process(program: &str) -> miette::Result<ChildProcessHandle> {
        let mut child = tokio::process::Command::new(program)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .into_diagnostic()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| miette::miette!("Failed to open stdout on child process"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| miette::miette!("Failed to open stdin on child process"))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| miette::miette!("Failed to open stderr on child process"))?;

        Ok(ChildProcessHandle {
            pid: child.id().unwrap_or(0),
            child,
            stdin,
            stdout,
            stderr,
        })
    }
}

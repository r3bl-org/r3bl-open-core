/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

//! This program uses the `r3bl_tui` crate to provide a prompt and get user
//! input, pass that to the `stdin` of a `bash` child process, and then display the output
//! from the child process in the terminal.
//!
//! # `YouTube` video of live coding this example
//!
//! Please watch the following video to see how this example was created.
//! - [Build with Naz : Create an async shell in Rust](https://youtu.be/jXzFCDIJQag)
//! - [YouTube channel](https://www.youtube.com/@developerlifecom?sub_confirmation=1)
//!
//! The followings steps outline what this example program does.
//!
//! # Create some shared global variables
//!
//! - A broadcast channel to signal shutdown to the child process, and all the spawned
//!   tasks.
//! - [`r3bl_tui::readline_async::ReadlineAsyncContext`] to write to the terminal. This
//!   provides the mechanism to collect user input and display output.
//! - [`tokio::process::Child`] to spawn the child process (`bash`) and interact with it.
//!   This child process lives as long as the `main` function and exits when the user
//!   chooses to `request_shutdown` the program.
//!   - The [`tokio::process::Command`] starts `bash`.
//!   - Both `stdin` and `stdout` are piped using [`std::process::Stdio::piped`].
//!
//! # 🧵 The main event loop simply waits for the following (on the current thread)
//!
//! - Start a main event loop (on the current thread) that does the following:
//!   - Monitor the shutdown signal from the broadcast channel
//!   - Monitor the [`r3bl_tui::readline_async::ReadlineAsyncContext`] for user input and
//!     write any input (the user provides interactively) to to the
//!     [`tokio::process::ChildStdin`].
//!   - Any `request_shutdown` inputs (when the user types "`request_shutdown`" or
//!     "Ctrl+D") from the user are captured here and sent to the shutdown broadcast
//!     channel. It also listens to the broadcast channel to break out of the loop on
//!     shutdown.
//!   - It [`tokio::process::Child::kill`]s the child process when it gets the
//!     `request_shutdown` signal.
//!   - It does not monitor the child process for output.
//!
//! # 🚀 Spawn a new task to loop and read the output from the child process and display it
//!
//! - Spawn a task to loop:
//!   - Read the [`tokio::process::ChildStdout`] and write it to the
//!     [`r3bl_tui::SharedWriter`].
//!   - Also listen to the broadcast channel to break out of the loop on shutdown.
//!
//! # Run the binary
//!
//! ```text
//! ┌───────────────────────────────────┐
//! │ > cargo run --example shell_async │
//! └───────────────────────────────────┘
//! ```
//!
//! Type the following commands to have a go at this.
//!
//! ```text
//! msg="hello nadia!"
//! echo $msg
//! ```
//!
//! You should see something like the following.
//!
//! ```text
//! [1606192] > msg="hello nadia!"
//! [1606192] > echo $msg
//! hello nadia!
//! [1606192] >
//! ```
//!
//! # Clean up any left over processes
//! ```text
//! ┌───────────────────────────────┐
//! │ > killall -9 bash shell_async │
//! └───────────────────────────────┘
//! ```

use std::io::Write;

use miette::IntoDiagnostic;
use r3bl_tui::{fg_guards_red, fg_lizard_green, fg_slate_gray, inline_string, ok,
               readline_async::{ReadlineAsyncContext, ReadlineEvent,
                                ReadlineEvent::{Eof, Interrupted, Line, Resized}},
               set_mimalloc_in_main, SharedWriter};
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt},
            join,
            sync::broadcast,
            task::JoinHandle};

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> miette::Result<()> {
    set_mimalloc_in_main!();

    // Create a broadcast channel for shutdown.
    let (shutdown_sender, _) = broadcast::channel::<()>(1);

    // Create a long-running `bash` child process using tokio::process::Command.
    let child_process_constructor::ChildProcessHandle {
        pid,
        child,
        stdin,
        stdout,
        stderr,
    } = child_process_constructor::new("bash")?;

    // Create a `r3bl_terminal_async` instance.
    let terminal_async_constructor::TerminalAsyncHandle {
        rl_ctx: readline_async,
        shared_writer,
    } = terminal_async_constructor::new(pid).await?;

    // Create 2 tasks, join on them:
    // 1. monitor the output from the child process.
    // 2. monitor the input from the user (and relay it to the child process).
    let _unused: (JoinHandle<_>, _) = join!(
        // New green thread.
        monitor_child_output::spawn(
            stdout,
            stderr,
            shared_writer.clone(),
            shutdown_sender.clone()
        ),
        // Current thread.
        monitor_user_input_and_send_to_child::start_event_loop(
            stdin,
            readline_async,
            child,
            shutdown_sender.clone()
        )
    );

    ok!()
}

pub mod monitor_user_input_and_send_to_child {
    use super::{broadcast, AsyncWriteExt, Eof, Interrupted, Line, ReadlineAsyncContext,
                ReadlineEvent, Resized};

    /// Determine the control flow of the program based on the [`ReadlineEvent`] received
    /// from user input.
    enum ControlFlow {
        ShutdownKillChild,
        ProcessLine(String),
        Resized,
    }

    /// Convert a [`miette::Result`]`<ReadlineEvent>` to a [`ControlFlow`]. This leverages
    /// the type system to make it simpler to reason about what to do with the user
    /// input.
    impl From<miette::Result<ReadlineEvent>> for ControlFlow {
        fn from(result_readline_event: miette::Result<ReadlineEvent>) -> Self {
            match result_readline_event {
                Ok(readline_event) => match readline_event {
                    Line(input) => {
                        let input = input.trim().to_string();
                        if input == "request_shutdown" {
                            ControlFlow::ShutdownKillChild
                        } else {
                            ControlFlow::ProcessLine(input)
                        }
                    }
                    Eof | Interrupted => ControlFlow::ShutdownKillChild,
                    Resized => ControlFlow::Resized,
                },
                _ => ControlFlow::ShutdownKillChild,
            }
        }
    }

    pub async fn start_event_loop(
        mut stdin: tokio::process::ChildStdin,
        mut rl_async: ReadlineAsyncContext,
        mut child: tokio::process::Child,
        shutdown_sender: broadcast::Sender<()>,
    ) {
        let mut shutdown_receiver = shutdown_sender.subscribe();

        loop {
            tokio::select! {
                // Branch: Monitor shutdown signal. This is cancel safe as `recv()` is
                // cancel safe.
                _ = shutdown_receiver.recv() => {
                    break;
                }

                // Branch: Monitor readline_async for user input. This is cancel safe as
                // `get_readline_event()` is cancel safe.
                result_readline_event = rl_async.read_line() => {
                    match ControlFlow::from(result_readline_event) {
                        ControlFlow::ShutdownKillChild => {
                            // We don't care about the result of the following operations.
                            child.kill().await.ok();
                            rl_async.request_shutdown(Some("❪◕‿◕❫ Goodbye")).await.ok();
                            shutdown_sender.send(()).ok();
                            break;
                        }
                        ControlFlow::ProcessLine(input) => {
                            let input = format!("{input}\n");
                            // We don't care about the result of the following operations.
                            stdin.write_all(input.as_bytes()).await.ok();
                            stdin.flush().await.ok();
                        }
                        ControlFlow::Resized => {}
                    }
                }
            }
        }
    }
}

pub mod monitor_child_output {
    use super::{broadcast, fg_guards_red, fg_lizard_green, AsyncBufReadExt,
                SharedWriter, Write};

    pub async fn spawn(
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
        mut shared_writer: SharedWriter,
        shutdown_sender: broadcast::Sender<()>,
    ) -> tokio::task::JoinHandle<()> {
        let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
        let mut stderr_lines = tokio::io::BufReader::new(stderr).lines();
        let mut shutdown_receiver = shutdown_sender.subscribe();

        tokio::spawn(async move {
            loop {
                // Branch: Monitor shutdown signal. This is cancel safe as `recv()` is
                // cancel safe.
                tokio::select! {
                    _ = shutdown_receiver.recv() => {
                        break;
                    }

                    // Branch: Monitor stdout for output from the child process. This is
                    // cancel safe as `next_line()` is cancel safe.
                    result_line = stdout_lines.next_line() => {
                        match result_line {
                            Ok(Some(line)) => {
                                // We don't care about the result of this operation.
                                writeln!(shared_writer, "{}", fg_lizard_green(&line)).ok();
                            },
                            _ => {
                                // We don't care about the result of this operation.
                                shutdown_sender.send(()).ok();
                                break;
                            }
                        }
                    }

                    // Branch: Monitor stderr for output from the child process. This is
                    // cancel safe as `next_line()` is cancel safe.
                    result_line = stderr_lines.next_line() => {
                        match result_line {
                            Ok(Some(line)) => {
                                // We don't care about the result of this operation.
                                writeln!(shared_writer, "{}", fg_guards_red(&line)).ok();
                            }
                            _ => {
                                // We don't care about the result of this operation.
                                shutdown_sender.send(()).ok();
                                break;
                            }
                        }
                    },
                }
            }
        })
    }
}

pub mod terminal_async_constructor {
    use super::{fg_slate_gray, inline_string, ok, ReadlineAsyncContext, SharedWriter};

    #[allow(missing_debug_implementations)]
    pub struct TerminalAsyncHandle {
        pub rl_ctx: ReadlineAsyncContext,
        pub shared_writer: SharedWriter,
    }

    pub async fn new(pid: u32) -> miette::Result<TerminalAsyncHandle> {
        let prompt = {
            let prompt_str = inline_string!("┤{pid}├");
            let prompt_seg_1 = fg_slate_gray(&prompt_str).bg_moonlight_blue();
            let prompt_seg_2 = " ";
            format!("{prompt_seg_1}{prompt_seg_2}")
        };

        let Ok(Some(rl_ctx)) = ReadlineAsyncContext::try_new(Some(prompt)).await else {
            miette::bail!("Failed to create ReadlineAsyncContext instance");
        };

        let shared_writer = rl_ctx.clone_shared_writer();

        ok!(TerminalAsyncHandle {
            rl_ctx,
            shared_writer
        })
    }
}

pub mod child_process_constructor {
    use super::{ok, IntoDiagnostic};

    #[derive(Debug)]
    pub struct ChildProcessHandle {
        pub stdin: tokio::process::ChildStdin,
        pub stdout: tokio::process::ChildStdout,
        pub stderr: tokio::process::ChildStderr,
        pub pid: u32,
        pub child: tokio::process::Child,
    }

    pub fn new(program: &str) -> miette::Result<ChildProcessHandle> {
        let mut child: tokio::process::Child = tokio::process::Command::new(program)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .into_diagnostic()?;

        let stdout: tokio::process::ChildStdout = child
            .stdout
            .take()
            .ok_or_else(|| miette::miette!("Failed to open stdout of child process"))?;

        let stdin: tokio::process::ChildStdin = child
            .stdin
            .take()
            .ok_or_else(|| miette::miette!("Failed to open stdin of child process"))?;

        let stderr: tokio::process::ChildStderr = child
            .stderr
            .take()
            .ok_or_else(|| miette::miette!("Failed to open stderr of child process"))?;

        let pid = child
            .id()
            .ok_or_else(|| miette::miette!("Failed to get PID of child process"))?;

        ok!(ChildProcessHandle {
            pid,
            child,
            stdin,
            stdout,
            stderr,
        })
    }
}

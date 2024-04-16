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

use crossterm::style::Stylize;
use miette::IntoDiagnostic;
use r3bl_terminal_async::{LineControlSignal, Spinner, SpinnerStyle};
use r3bl_terminal_async::{Readline, ReadlineEvent, SharedWriter, TerminalAsync};
use std::{io::Write, ops::ControlFlow, time::Duration};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};
use tokio::select;
use tokio::time::interval;
use tracing::info;

/// Load dependencies for this examples file.
mod helpers;
use helpers::tracing_setup::{self};

/// More info:
/// - <https://docs.rs/strum_macros/latest/strum_macros/derive.EnumString.html>
/// - <https://docs.rs/strum_macros/latest/strum_macros/derive.Display.html>
/// - <https://docs.rs/strum_macros/latest/strum_macros/derive.EnumIter.html>
#[derive(Debug, PartialEq, EnumString, EnumIter, Display)]
enum Command {
    #[strum(ascii_case_insensitive)]
    Spinner,

    #[strum(ascii_case_insensitive)]
    Tree,

    #[strum(ascii_case_insensitive)]
    StartTask1,

    #[strum(ascii_case_insensitive)]
    StopTask1,

    #[strum(ascii_case_insensitive)]
    StartTask2,

    #[strum(ascii_case_insensitive)]
    StopTask2,

    #[strum(ascii_case_insensitive)]
    StartPrintouts,

    #[strum(ascii_case_insensitive)]
    StopPrintouts,

    #[strum(ascii_case_insensitive)]
    Info,

    #[strum(ascii_case_insensitive)]
    Exit,
}

fn get_info_message() -> String {
    let available_commands = {
        let commands = Command::iter()
            .map(|it| it.to_string())
            .collect::<Vec<String>>();
        format!("{:?}", commands).blue()
    };
    let info_message = format!(
        "try Ctrl+D, Up, Down, Left, Right, Ctrl+left, Ctrl+right, `{}`, `{}`, `{}`, `{}`, and `{}`",
        Command::StartTask1,
        Command::Tree,
        Command::Spinner,
        Command::StartTask2,
        Command::StopPrintouts
    );
    format!(
        "{}: \n{}\n{}",
        format!("{}", "Available commands".bold())
            .magenta()
            .bold()
            .underlined(),
        available_commands,
        info_message.to_string().white().bold().on_dark_grey()
    )
}

#[derive(Debug, Clone, Copy)]
struct State {
    pub task_1_state: TaskState,
    pub task_2_state: TaskState,
}

#[derive(Debug, Clone, Copy)]
struct TaskState {
    pub interval_delay: Duration,
    pub counter: u64,
    pub is_running: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            task_1_state: TaskState {
                interval_delay: Duration::from_secs(1),
                counter: 0,
                is_running: false,
            },
            task_2_state: TaskState {
                interval_delay: Duration::from_secs(4),
                counter: 0,
                is_running: false,
            },
        }
    }
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let maybe_terminal_async = TerminalAsync::try_new("> ").await?;

    // If the terminal is not fully interactive, then return early.
    let mut terminal_async = match maybe_terminal_async {
        None => return Ok(()),
        _ => maybe_terminal_async.unwrap(),
    };

    // Pre-populate the readline's history with some entries.

    for command in Command::iter() {
        terminal_async
            .readline
            .add_history_entry(command.to_string());
    }

    // Initialize tracing w/ the "async stdout".
    tracing_setup::init(terminal_async.clone_shared_writer())?;

    // Start tasks.
    let mut state = State::default();
    let mut interval_1_task = interval(state.task_1_state.interval_delay);
    let mut interval_2_task = interval(state.task_2_state.interval_delay);

    terminal_async.println(get_info_message().to_string()).await;

    loop {
        select! {
            _ = interval_1_task.tick() => {
                task_1::tick(&mut state, &mut terminal_async.clone_shared_writer())?;
            },
            _ = interval_2_task.tick() => {
                task_2::tick(&mut state, &mut terminal_async.clone_shared_writer())?;
            },
            user_input = terminal_async.get_readline_event() => match user_input {
                Ok(readline_event) => {
                    let continuation = process_input_event::process_readline_event(
                        readline_event, &mut state, &mut terminal_async.clone_shared_writer(), &mut terminal_async.readline
                    ).await?;
                    if let ControlFlow::Break(_) = continuation { break }
                },
                Err(err) => {
                    let msg_1 = format!("Received err: {}", format!("{:?}",err).red());
                    let msg_2 = format!("{}", "Exiting...".red());
                    terminal_async.println(msg_1).await;
                    terminal_async.println(msg_2).await;
                    break;
                },
            }
        }
    }

    // Flush all writers to stdout
    let _ = terminal_async.flush().await;

    Ok(())
}

/// This task simply uses [writeln] and [SharedWriter] to print to stdout.
mod task_1 {
    use super::*;

    pub fn tick(state: &mut State, stdout: &mut SharedWriter) -> miette::Result<()> {
        if !state.task_1_state.is_running {
            return Ok(());
        };

        let counter_1 = state.task_1_state.counter;
        writeln!(stdout, "[{counter_1}] First interval went off!").into_diagnostic()?;
        state.task_1_state.counter += 1;

        Ok(())
    }
}

/// This task uses [tracing] to log to stdout (via [SharedWriter]).
mod task_2 {
    use super::*;

    pub fn tick(state: &mut State, _stdout: &mut SharedWriter) -> miette::Result<()> {
        if !state.task_2_state.is_running {
            return Ok(());
        };

        let counter_2 = state.task_2_state.counter;
        info!("[{counter_2}] Second interval went off!");
        state.task_2_state.counter += 1;

        Ok(())
    }
}

mod process_input_event {
    use std::str::FromStr;

    use super::*;

    pub async fn process_readline_event(
        readline_event: ReadlineEvent,
        state: &mut State,
        shared_writer: &mut SharedWriter,
        readline: &mut Readline,
    ) -> miette::Result<ControlFlow<()>> {
        match readline_event {
            ReadlineEvent::Line(user_input) => {
                process_user_input(user_input, state, shared_writer, readline).await
            }
            ReadlineEvent::Eof => {
                writeln!(shared_writer, "{}", "Exiting due to Eof...".red().bold())
                    .into_diagnostic()?;
                Ok(ControlFlow::Break(()))
            }
            ReadlineEvent::Interrupted => {
                writeln!(
                    shared_writer,
                    "{}",
                    "Exiting due to ^C pressed...".red().bold()
                )
                .into_diagnostic()?;
                Ok(ControlFlow::Break(()))
            }
        }
    }

    async fn process_user_input(
        user_input: String,
        state: &mut State,
        shared_writer: &mut SharedWriter,
        readline: &mut Readline,
    ) -> miette::Result<ControlFlow<()>> {
        // Add to history.
        let line = user_input.trim();
        readline.add_history_entry(line.to_string());

        // Convert line to command. And process it.
        let result_command = Command::from_str(&line.trim().to_lowercase());
        match result_command {
            Err(_) => {
                writeln!(shared_writer, "Unknown command!").into_diagnostic()?;
                return Ok(ControlFlow::Continue(()));
            }
            Ok(command) => match command {
                Command::Exit => {
                    writeln!(shared_writer, "{}", "Exiting due to exit command...".red())
                        .into_diagnostic()?;
                    readline.close().await;
                    return Ok(ControlFlow::Break(()));
                }
                Command::StartTask1 => {
                    state.task_1_state.is_running = true;
                    writeln!(shared_writer, "First task started! This prints to stdout.")
                        .into_diagnostic()?;
                }
                Command::StopTask1 => {
                    state.task_1_state.is_running = false;
                    writeln!(shared_writer, "First task stopped!").into_diagnostic()?;
                }
                Command::StartTask2 => {
                    state.task_2_state.is_running = true;
                    writeln!(
                        shared_writer,
                        "Second task started! This generates logs which print to stdout"
                    )
                    .into_diagnostic()?;
                }
                Command::StopTask2 => {
                    state.task_2_state.is_running = false;
                    writeln!(shared_writer, "Second task stopped!").into_diagnostic()?;
                }
                Command::StartPrintouts => {
                    writeln!(shared_writer, "Printouts started!").into_diagnostic()?;
                    readline.should_print_line_on(true, true).await;
                }
                Command::StopPrintouts => {
                    writeln!(shared_writer, "Printouts stopped!").into_diagnostic()?;
                    readline.should_print_line_on(false, false).await;
                }
                Command::Info => {
                    writeln!(shared_writer, "{}", get_info_message()).into_diagnostic()?;
                }
                Command::Spinner => {
                    writeln!(shared_writer, "Spinner started! Pausing terminal...")
                        .into_diagnostic()?;
                    long_running_task::spawn_task_that_shows_spinner(
                        shared_writer,
                        "Spinner task",
                        Duration::from_millis(100),
                    );
                }
                Command::Tree => {
                    let mut shared_writer_clone = shared_writer.clone();
                    tokio::spawn(async move {
                        let mut_shared_writer = &mut shared_writer_clone;
                        match file_walker::get_current_working_directory() {
                            Ok((root_path, _)) => {
                                match file_walker::display_tree(root_path, mut_shared_writer, true)
                                    .await
                                {
                                    Ok(_) => {}
                                    Err(_) => todo!(),
                                };
                            }
                            Err(_) => todo!(),
                        }
                    });
                }
            },
        }

        Ok(ControlFlow::Continue(()))
    }
}

mod long_running_task {
    use std::{io::stderr, sync::Arc};

    use r3bl_terminal_async::TokioMutex;

    use super::*;

    // Spawn a task that uses the shared writer to print to stdout, and pauses the spinner
    // at the start, and resumes it when it ends.
    pub fn spawn_task_that_shows_spinner(
        shared_writer: &mut SharedWriter,
        task_name: &str,
        delay: Duration,
    ) {
        let mut interval = interval(delay);
        let mut tick_counter = 0;
        let max_tick_count = 30;

        let line_sender = shared_writer.line_sender.clone();
        let task_name = task_name.to_string();

        let shared_writer_clone = shared_writer.clone();

        tokio::spawn(async move {
            // Create a spinner.
            let maybe_spinner = Spinner::try_start(
                format!(
                    "{} - This is a sample indeterminate progress message",
                    task_name
                ),
                Duration::from_millis(100),
                SpinnerStyle::default(),
                Arc::new(TokioMutex::new(stderr())),
                shared_writer_clone,
            )
            .await;

            loop {
                // Wait for the interval duration (one tick).
                interval.tick().await;

                // Don't print more than `max_tick_count` times.
                tick_counter += 1;
                if tick_counter >= max_tick_count {
                    break;
                }

                // Display a message at every tick.
                let msg = format!("[{task_name}] - [{tick_counter}] interval went off while spinner was spinning!\n");
                let _ = line_sender
                    .send(LineControlSignal::Line(msg.into_bytes()))
                    .await;
            }

            if let Ok(Some(mut spinner)) = maybe_spinner {
                let msg = format!("{} - Task ended. Resuming terminal and showing any output that was generated while spinner was active.", task_name);
                let _ = spinner.stop(msg.as_str()).await;
            }
        });
    }
}

pub mod file_walker {
    use super::*;
    use miette::miette;
    use r3bl_terminal_async::SendRawTerminal;
    use std::{
        fs,
        path::{self, PathBuf},
    };

    pub const FOLDER_DELIM: &str = std::path::MAIN_SEPARATOR_STR;
    pub const SPACE_CHAR: &str = " ";
    pub const INDENT_MULTIPLIER: usize = 4;

    /// - Get the current working directory. Eg:
    ///   `/home/nazmul/github/r3bl_terminal_async`.
    /// - Returns a tuple of `(path, name)`. Eg:
    ///   (`/home/nazmul/github/r3bl_terminal_async`, `r3bl_terminal_async`).
    pub fn get_current_working_directory() -> miette::Result<(/*path*/ String, /*name*/ String)> {
        let path = std::env::current_dir().into_diagnostic()?;

        let name = path
            .file_name()
            .ok_or_else(|| miette!("Could not get current working directory"))?;

        Ok((
            path.to_string_lossy().to_string(),
            name.to_string_lossy().to_string(),
        ))
    }

    #[derive(Debug, Clone)]
    pub struct Folder {
        name: String,
        full_path: String,
        depth: Option<usize>,
    }

    pub fn create_root(root_path: String) -> miette::Result<Folder> {
        // Validate that root_path is a directory.
        let metadata = fs::metadata(&root_path).into_diagnostic()?;
        if !metadata.is_dir() {
            return Err(miette!("The path is not a directory"));
        }

        // Get the folder name.
        let folder = PathBuf::from(&root_path);
        let name = folder
            .file_name()
            .ok_or_else(|| miette!("The root's full path is not a directory: {}", root_path))?;

        Ok(Folder {
            name: name.to_string_lossy().to_string(),
            full_path: root_path,
            depth: Some(0),
        })
    }

    pub async fn create_child_and_add_to(
        parent_node: &mut Folder,
        child_name: String,
    ) -> miette::Result<Folder> {
        let child_full_path = format!("{}{FOLDER_DELIM}{child_name}", parent_node.full_path);

        // Validate that child's new full path is a directory.
        let metadata = fs::metadata(&child_full_path).into_diagnostic()?;
        if !metadata.is_dir() {
            return Err(miette!(
                "The child's new full path is not a directory: {}",
                child_full_path
            ));
        }

        Ok(Folder {
            name: child_name,
            full_path: child_full_path,
            depth: Some(parent_node.depth.unwrap_or(0) + 1),
        })
    }

    /// Walk the current working directory. Display a tree formatted view of the
    /// sub-folders just like the `tree` command on Linux. This algorithm is a depth-first
    /// search (DFS) algorithm, which relies on the [fs], which is a tree, to get a list
    /// of folders (not files) contained in any given folder. There's no need to
    /// explicitly construct a tree, since the [fs] is a non-binary tree.
    ///
    /// The [fs::read_dir] function determines what the order of the traversal is.
    /// Non-binary trees, also known as N-ary trees, can be traversed in several ways.
    /// Here are the most common types of non-binary tree traversals: Depth-First Search
    /// (DFS): This traversal method explores as far as possible along each branch before
    /// backtracking. DFS itself can be further divided into three types:
    ///
    /// 1) Pre-Order Traversal: In this traversal method, the root is visited first, then
    /// the left subtree, and finally the right subtree.
    ///
    /// 2) In-Order Traversal: In this traversal method, the left subtree is visited
    /// first, then the root, and finally the right subtree. Note: This method is only for
    /// binary trees.
    ///
    /// 3) Post-Order Traversal: In this traversal method, the left subtree is visited
    /// first, then the right subtree, and finally the root. Breadth-First Search (BFS):
    /// Also known as level-order traversal, this method visits all the nodes of a level
    /// before going to the next level.
    ///
    /// 4) Spiral/Zigzag Order Traversal: This is a variant of BFS. In this traversal,
    /// levels are visited in alternating left-to-right and right-to-left order.
    ///
    /// Remember, the "left" and "right" in these traversal methods are just for
    /// explanation purposes. In a non-binary tree, a node can have more than two
    /// children. So, in actual implementation, "left" and "right" can be replaced with
    /// "first child", "second child", "third child", and so on.
    ///
    /// More info:
    /// 1. <https://developerlife.com/2018/08/16/algorithms-in-kotlin-3/>
    /// 2. <https://developerlife.com/2022/02/24/rust-non-binary-tree/>
    /// 3. <https://developerlife.com/2022/12/11/algo-ts-2/>
    pub async fn display_tree(
        root_path: String,
        shared_writer: &mut SharedWriter,
        delay_enable: bool,
    ) -> miette::Result<()> {
        let root = create_root(root_path)?;

        // Walk the root.
        let mut stack = vec![root.clone()];

        while let Some(mut current_node) = stack.pop() {
            // Print the current node.
            print_node(shared_writer, &current_node)?;
            if delay_enable {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            // Add node's sub-folders to the stack.
            let current_full_path = path::Path::new(&current_node.full_path);

            // Only get a list of folders (not files) contained in the node by scanning
            // the file system. We didn't specify the sorting order, so it's up to the
            // operating system to decide the order. This affects the ordering mentioned
            // in the docs above.
            let vec_folder_name: Vec<String> = fs::read_dir(current_full_path)
                .into_diagnostic()?
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect();

            // Add each sub-folder (contained in the current node) to the stack. And add
            // it to the current node's children.
            for sub_folder_name in vec_folder_name {
                stack.push(create_child_and_add_to(&mut current_node, sub_folder_name).await?);
            }
        }

        Ok(())
    }

    pub fn print_node(writer: &mut SendRawTerminal, node: &Folder) -> miette::Result<()> {
        writeln!(
            writer,
            "{}{}",
            SPACE_CHAR.repeat(INDENT_MULTIPLIER * node.depth.unwrap_or(0)),
            node.name
        )
        .into_diagnostic()?;

        Ok(())
    }

    #[tokio::test]
    async fn test_display_tree() -> miette::Result<()> {
        let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
        let mut shared_writer = SharedWriter {
            buffer: Vec::new(),
            line_sender,
        };

        let (path, _) = get_current_working_directory()?;

        display_tree(path, &mut shared_writer, false).await.unwrap();

        assert_eq!(shared_writer.buffer.len(), 0);

        // Print everything in line_receiver.
        let mut output_lines = vec![];
        loop {
            let it = line_receiver.try_recv().into_diagnostic();
            match it {
                Ok(LineControlSignal::Line(it)) => {
                    output_lines.push(String::from_utf8_lossy(&it).to_string());
                    print!("{}", String::from_utf8_lossy(&it));
                }
                _ => break,
            }
        }

        assert_ne!(output_lines.len(), 0);

        Ok(())
    }
}

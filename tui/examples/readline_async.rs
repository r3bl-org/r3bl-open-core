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
use std::{fs,
          io::Write,
          ops::ControlFlow,
          path::{self, PathBuf},
          str::FromStr,
          time::Duration};

use miette::{IntoDiagnostic, miette};
use r3bl_tui::{InlineVec, OutputDevice, SendRawTerminal, SharedWriter, SpinnerStyle,
               bold, fg_color, fg_red, fg_slate_gray, inline_string,
               log::{DisplayPreference, try_initialize_logging_global},
               readline_async::{Readline, ReadlineAsyncContext, ReadlineEvent, Spinner},
               rla_println, set_jemalloc_in_main, tui_color};
use smallvec::smallvec;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};
use tokio::{select, spawn,
            time::{interval, sleep}};

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
    let available_commands = &{
        let commands = Command::iter()
            .map(|it| it.to_string())
            .collect::<Vec<String>>();
        fg_color(tui_color!(lizard_green), &format!("{commands:?}")).to_string()
    };

    let info_message = &format!(
        "try Ctrl+D, Up, Down, Left, Right, Ctrl+left, Ctrl+right, `{}`, `{}`, `{}`, `{}`, and `{}`",
        Command::StartTask1,
        Command::Tree,
        Command::Spinner,
        Command::StartTask2,
        Command::StopPrintouts
    );

    format!(
        "{a}: \n{b}\n{c}",
        a = bold("Available commands"),
        b = available_commands,
        c = fg_color(tui_color!(frozen_blue), info_message)
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
#[allow(clippy::needless_return)]
async fn main() -> miette::Result<()> {
    set_jemalloc_in_main!();

    let prompt = {
        let prompt_seg_1 = fg_slate_gray("╭>╮").bg_moonlight_blue();
        let prompt_seg_2 = " ";
        format!("{prompt_seg_1}{prompt_seg_2}")
    };

    let maybe_rl_ctx = ReadlineAsyncContext::try_new(Some(prompt)).await?;

    // If the terminal is not fully interactive, then return early.
    let Some(mut rl_ctx) = maybe_rl_ctx else {
        return Ok(());
    };

    // Pre-populate the readline's history with some entries.
    for command in Command::iter() {
        rl_ctx.readline.add_history_entry(command.to_string());
    }

    // Initialize tracing w/ the "async stdout" (SharedWriter), and file writer.
    try_initialize_logging_global(DisplayPreference::SharedWriter(
        rl_ctx.clone_shared_writer(),
    ))?;

    // Start tasks.
    let mut state = State::default();
    let mut interval_1_task = interval(state.task_1_state.interval_delay);
    let mut interval_2_task = interval(state.task_2_state.interval_delay);

    rla_println!(rl_ctx, "{}", get_info_message());

    loop {
        select! {
            _ = interval_1_task.tick() => {
                task_1::tick(&mut state, &mut rl_ctx.clone_shared_writer())?;
            },
            _ = interval_2_task.tick() => {
                task_2::tick(&mut state, &mut rl_ctx.clone_shared_writer());
            },
            result_readline_event = rl_ctx.read_line() => {
                match result_readline_event {
                    Ok(readline_event) => {
                        match readline_event {
                            // User input event.
                            ReadlineEvent::Line(user_input) => {
                                let mut_state = &mut state;
                                let shared_writer = &mut rl_ctx.clone_shared_writer();
                                let readline = &mut rl_ctx.readline;
                                let control_flow = process_input_event::process(
                                    user_input, mut_state, shared_writer, readline)?;
                                if let ControlFlow::Break(()) = control_flow {
                                    rl_ctx.request_shutdown(Some("❪◕‿◕❫ Goodbye")).await?;
                                    rl_ctx.await_shutdown().await;
                                    break;
                                }
                            }
                            // Resize event.
                            ReadlineEvent::Resized => {
                                let shared_writer = &mut rl_ctx.clone_shared_writer();
                                writeln!(
                                    shared_writer,
                                    "{}",
                                    fg_color(tui_color!(frozen_blue), "Terminal resized!")
                                ).into_diagnostic()?;
                            }
                            // Ctrl+D, Ctrl+C.
                            ReadlineEvent::Eof | ReadlineEvent::Interrupted => {
                                rl_ctx.request_shutdown(Some("❪◕‿◕❫ Goodbye")).await?;
                                rl_ctx.await_shutdown().await;
                                break;
                            }
                        }
                    },
                    Err(err) => {
                        let msg_1 = format!("Received err: {}", fg_red(format!("{err:?}")));
                        let msg_2 = format!("{}", fg_red("Exiting..."));
                        rla_println!(rl_ctx, "{msg_1}");
                        rla_println!(rl_ctx, "{msg_2}");
                        break;
                    },
                }
            }
        }
    }

    // There's no need to close readline_async or readline. Drop will take care of
    // cleaning up (closing raw mode).

    Ok(())
}

/// This task simply uses [`writeln`] and [`SharedWriter`] to print to stdout.
mod task_1 {
    use super::{IntoDiagnostic, SharedWriter, State, Write};

    pub fn tick(state: &mut State, stdout: &mut SharedWriter) -> miette::Result<()> {
        if !state.task_1_state.is_running {
            return Ok(());
        }

        let counter_1 = state.task_1_state.counter;
        writeln!(stdout, "[{counter_1}] First interval went off!").into_diagnostic()?;
        state.task_1_state.counter += 1;

        Ok(())
    }
}

/// This task uses [tracing] to log to stdout (via [`SharedWriter`]).
mod task_2 {
    use super::{SharedWriter, State, inline_string};

    pub fn tick(state: &mut State, _stdout: &mut SharedWriter) {
        if !state.task_2_state.is_running {
            return;
        }

        // Display a log message with the current counter value.
        tracing::info!(
            message = %inline_string!("[{}] Second interval went off!", state.task_2_state.counter)
        );

        // Increment the counter.
        state.task_2_state.counter += 1;
    }
}

mod process_input_event {
    use super::{Command, ControlFlow, Duration, FromStr, IntoDiagnostic, Readline,
                SharedWriter, State, Write, file_walker, get_info_message,
                long_running_task, spawn};

    pub fn process(
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
                    return Ok(ControlFlow::Break(()));
                }
                Command::StartTask1 => {
                    state.task_1_state.is_running = true;
                    writeln!(
                        shared_writer,
                        "First task started! This prints to SharedWriter."
                    )
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
                        "Second task started! This generates logs which print to DisplayPreference (SharedWriter) and file."
                    )
                    .into_diagnostic()?;
                }
                Command::StopTask2 => {
                    state.task_2_state.is_running = false;
                    writeln!(shared_writer, "Second task stopped!").into_diagnostic()?;
                }
                Command::StartPrintouts => {
                    writeln!(shared_writer, "Printouts started!").into_diagnostic()?;
                    readline.should_print_line_on(true, true);
                }
                Command::StopPrintouts => {
                    writeln!(shared_writer, "Printouts stopped!").into_diagnostic()?;
                    readline.should_print_line_on(false, false);
                }
                Command::Info => {
                    writeln!(shared_writer, "{}", get_info_message())
                        .into_diagnostic()?;
                }
                Command::Spinner => {
                    writeln!(shared_writer, "Spinner started! Pausing terminal...")
                        .into_diagnostic()?;
                    long_running_task::spawn_task_that_shows_spinner(
                        shared_writer,
                        readline,
                        "Spinner task",
                        Duration::from_millis(100),
                    );
                }
                Command::Tree => {
                    let mut shared_writer_clone = shared_writer.clone();
                    spawn(async move {
                        let mut_shared_writer = &mut shared_writer_clone;
                        match file_walker::get_current_working_directory() {
                            Ok((root_path, _)) => {
                                match file_walker::display_tree(
                                    root_path,
                                    mut_shared_writer,
                                    true,
                                )
                                .await
                                {
                                    Ok(()) => {}
                                    Err(_) => todo!(),
                                }
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
    use super::{Duration, OutputDevice, Readline, SharedWriter, Spinner, SpinnerStyle,
                Write, interval, spawn};

    // Spawn a task that uses the shared writer to print to stdout, and pauses the spinner
    // at the start, and resumes it when it ends.
    pub fn spawn_task_that_shows_spinner(
        shared_writer: &mut SharedWriter,
        readline: &mut Readline,
        task_name: &str,
        delay: Duration,
    ) {
        let mut interval = interval(delay);
        let mut tick_counter = 0;
        let max_tick_count = 30;

        let task_name = task_name.to_string();

        let shared_writer_clone_1 = shared_writer.clone();
        let mut shared_writer_clone_2 = shared_writer.clone();

        if readline.safe_spinner_is_active.lock().unwrap().is_some() {
            // We don't care about the result of this operation.
            writeln!(
                shared_writer,
                "Spinner is already active, can't start another one"
            )
            .ok();
        }

        spawn(async move {
            // Try to create and start a spinner.
            let maybe_spinner = Spinner::try_start(
                format!("{task_name} - This is a sample indeterminate progress message"),
                format!(
                    "{task_name} - Task ended. Resuming terminal and showing any output that was generated while spinner was active."
                ),
                Duration::from_millis(100),
                SpinnerStyle::default(),
                OutputDevice::default(),
                Some(shared_writer_clone_1),
            )
            .await;

            loop {
                // Check for spinner shutdown (via interruption).
                if let Ok(Some(ref spinner)) = maybe_spinner
                    && spinner.is_shutdown()
                {
                    break;
                }

                // Wait for the interval duration (one tick).
                interval.tick().await;

                // Don't print more than `max_tick_count` times.
                tick_counter += 1;
                if tick_counter >= max_tick_count {
                    break;
                }

                // Display a message at every tick.
                // We don't care about the result of this operation.
                writeln!(
                    shared_writer_clone_2,
                    "[{task_name}] - [{tick_counter}] interval went off while spinner was spinning!"
                ).ok();
            }

            // Don't forget to stop the spinner.
            if let Ok(Some(mut spinner)) = maybe_spinner {
                spinner.request_shutdown();
                spinner.await_shutdown().await;
            }
        });
    }
}

pub mod file_walker {
    use super::{Duration, InlineVec, IntoDiagnostic, PathBuf, SendRawTerminal,
                SharedWriter, fs, miette, path, sleep, smallvec};

    pub const FOLDER_DELIM: &str = std::path::MAIN_SEPARATOR_STR;
    pub const SPACE_CHAR: &str = " ";
    pub const INDENT_MULTIPLIER: usize = 4;

    /// - Get the current working directory. Eg:
    ///   `/home/nazmul/github/r3bl_terminal_async`.
    /// - Returns a tuple of `(path, name)`. Eg:
    ///   (`/home/nazmul/github/r3bl_terminal_async`, `r3bl_terminal_async`).
    pub fn get_current_working_directory()
    -> miette::Result<(/* path */ String, /* name */ String)> {
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
        let name = folder.file_name().ok_or_else(|| {
            miette!("The root's full path is not a directory: {}", root_path)
        })?;

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
        let child_full_path =
            format!("{}{FOLDER_DELIM}{child_name}", parent_node.full_path);

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

    /// Walk the current working directory.
    ///
    /// Display a tree formatted view of the sub-folders just like the `tree` command on
    /// Linux. This algorithm is a depth-first search (DFS) algorithm, which relies on the
    /// [`fs`], which is a tree, to get a list of folders (not files) contained in any
    /// given folder. There's no need to explicitly construct a tree, since the [`fs`]
    /// is a non-binary tree.
    ///
    /// The [`fs::read_dir`] function determines what the order of the traversal is.
    /// Non-binary trees, also known as N-ary trees, can be traversed in several ways.
    /// Here are the most common types of non-binary tree traversals: Depth-First Search
    /// (DFS): This traversal method explores as far as possible along each branch before
    /// backtracking. DFS itself can be further divided into three types:
    ///
    /// 1) Pre-Order Traversal: In this traversal method, the root is visited first, then
    ///    the left subtree, and finally the right subtree.
    ///
    /// 2) In-Order Traversal: In this traversal method, the left subtree is visited
    ///    first, then the root, and finally the right subtree. Note: This method is only
    ///    for binary trees.
    ///
    /// 3) Post-Order Traversal: In this traversal method, the left subtree is visited
    ///    first, then the right subtree, and finally the root. Breadth-First Search
    ///    (BFS): Also known as level-order traversal, this method visits all the nodes of
    ///    a level before going to the next level.
    ///
    /// 4) Spiral/Zigzag Order Traversal: This is a variant of BFS. In this traversal,
    ///    levels are visited in alternating left-to-right and right-to-left order.
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
        let mut stack: InlineVec<Folder> = smallvec![root.clone()];

        while let Some(mut current_node) = stack.pop() {
            // Print the current node.
            print_node(shared_writer, &current_node)?;
            if delay_enable {
                sleep(Duration::from_millis(10)).await;
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
                stack.push(
                    create_child_and_add_to(&mut current_node, sub_folder_name).await?,
                );
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
}

#[tokio::test]
async fn test_display_tree() -> miette::Result<()> {
    let (line_sender, mut line_receiver) = tokio::sync::mpsc::channel(1_000);
    let mut shared_writer = SharedWriter::new(line_sender);

    let (path, _) = file_walker::get_current_working_directory()?;

    file_walker::display_tree(path, &mut shared_writer, false)
        .await
        .unwrap();

    assert_eq!(shared_writer.buffer.len(), 0);

    // Print everything in line_receiver.
    let mut output_lines: Vec<String> = vec![];
    loop {
        let it = line_receiver.try_recv().into_diagnostic();
        match it {
            Ok(r3bl_tui::LineStateControlSignal::Line(it)) => {
                let string = String::from_utf8_lossy(it.as_ref()).to_string();
                print!("{string}");
                output_lines.push(string);
            }
            _ => break,
        }
    }

    assert_ne!(output_lines.len(), 0);

    Ok(())
}

/*
 *   Copyright (c) 2023 R3BL LLC
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

//! For more information on how to use CLAP and Tuify, please read this tutorial:
//! <https://developerlife.com/2023/09/17/tuify-clap/>

use std::{io::Result, process::Command};

#[allow(unused_imports)]
use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use r3bl_ansi_color::{AnsiStyledText, Color, Style};
use r3bl_rs_utils_core::*;
use r3bl_tuify::*;

fn main() -> Result<()> {
    use clap_config::*;

    display_prompts::show_welcome_message();
    display_prompts::instruction_header();
    // If no args are passed, the following line will fail, and help will be printed
    // thanks to `arg_required_else_help(true)` in the `CliArgs` struct.
    let giti_app_args = GitiAppArgs::parse();

    let enable_logging = TRACE | giti_app_args.global_options.enable_logging;

    call_if_true!(enable_logging, {
        try_to_set_log_level(log::LevelFilter::Trace).ok();
        log_debug("Start logging...".to_string());
        log_debug(format!("og_size: {:?}", get_size()?).to_string());
        log_debug(format!("cli_args {:?}", giti_app_args));
    });

    match giti_app_args.command {
        CLICommands::Branch {
            selection_mode: _selection_mode,
            command_to_run_with_each_selection,
        } => match command_to_run_with_each_selection {
            Some(subcommand) => match subcommand {
                BranchSubcommands::Delete => {
                    branch_delete::tui_init();
                }
                BranchSubcommands::Show => todo!(),
                BranchSubcommands::Checkout => {
                    checkout_branch::tui_init();
                }
            },
            None => todo!(),
        },
        CLICommands::Add => {
            add_files_to_staging::tui_init();
        }
    }

    call_if_true!(enable_logging, {
        log_debug("Stop logging...".to_string());
    });

    Ok(())
}

mod display_prompts {
    use super::*;

    pub fn instruction_header() {
        AnsiStyledText {
            text: &format!(
                "{}{}{}",
                "Press Space : To Select or DeSelect branches\n",
                "Press Esc : To Exit\n",
                "Press Return: To Confirm Selection"
            ),
            style: &[Style::Bold, Style::Foreground(Color::Rgb(200, 1, 200))],
        }
        .println();
    }

    pub fn show_tuify(
        options: Vec<String>,
        header: String,
        selection_mode: SelectionMode,
    ) -> Option<Vec<String>> {
        let max_height_row_count = 50;
        let max_width_col_count = get_size().map(|it| it.col_count).unwrap_or(ch!(80)).into();
        let style = StyleSheet::default();
        let user_input = select_from_list(
            header,
            options,
            max_height_row_count,
            max_width_col_count,
            selection_mode,
            style,
        );
        user_input
    }

    pub fn show_exit_message() {
        let text = &{
            format!("Goodbye, {}! 👋🐈 Thank you for using giti. Please star r3bl-open-core repo on GitHub! 🌟", get_username())
        };
        AnsiStyledText {
            text,
            style: &[Style::Bold, Style::Foreground(Color::Rgb(1, 200, 200))],
        }
        .println();

        AnsiStyledText {
            text: "https://github.com/r3bl-org/r3bl-open-core",
            style: &[Style::Bold, Style::Foreground(Color::Rgb(200, 50, 100))],
        }
        .println();
    }

    pub fn get_username() -> String {
        std::env::var("USER").unwrap_or("unknown".to_string())
    }

    pub fn show_welcome_message() {
        let text = &{ format!("Hello, {}! 👋🐈", get_username()) };
        AnsiStyledText {
            text,
            style: &[Style::Bold, Style::Foreground(Color::Rgb(100, 200, 1))],
        }
        .println();
    }

    pub fn tui_exit<F: Fn()>(go_back_fn: F) {
        let options = vec!["Go Back".to_string(), "Exit".to_string()];
        let header = format!("Would you like to exit giti 🐈 ?");
        let selection_mode = SelectionMode::Single;
        let tuify_output = show_tuify(options, header, selection_mode);
        match tuify_output {
            Some(it) => {
                if it[0] == "Go Back".to_string() {
                    go_back_fn();
                } else if it[0] == "Exit".to_string() {
                    display_prompts::show_exit_message();
                }
            }
            None => tui_exit(go_back_fn),
        }
    }
}

/// More info:
/// - <https://docs.rs/clap/latest/clap/_derive/#overview>
/// - <https://developerlife.com/2023/09/17/tuify-clap/>
mod clap_config {
    use super::*;

    #[derive(Debug, Parser)]
    #[command(bin_name = "giti")]
    #[command(about = "Easy to use, interactive, tuified git", long_about = None)]
    #[command(version)]
    #[command(next_line_help = true)]
    #[command(arg_required_else_help(true))]
    pub struct GitiAppArgs {
        #[clap(subcommand)]
        pub command: CLICommands,

        #[clap(flatten)]
        pub global_options: GlobalOptions,
    }

    #[derive(Debug, Args)]
    pub struct GlobalOptions {
        /// Print debug output to log file (log.txt)
        #[arg(long, short = 'l')]
        pub enable_logging: bool,

        /// Optional maximum height of the TUI (rows)
        #[arg(value_name = "height", long, short = 'r')]
        pub tui_height: Option<usize>,

        /// Optional maximum width of the TUI (columns)
        #[arg(value_name = "width", long, short = 'c')]
        pub tui_width: Option<usize>,
    }

    #[derive(Debug, Subcommand)]
    pub enum CLICommands {
        /// Show TUI to allow you to select one or more local branches for deletion 🌿
        Branch {
            /// Would you like to select one or more items?
            #[arg(value_name = "mode", long, short = 's')]
            selection_mode: Option<SelectionMode>,

            /// Each selected item is passed to this command as an argument and executed in your shell.
            #[arg(value_name = "command")]
            command_to_run_with_each_selection: Option<BranchSubcommands>,
        },
        Add,
    }

    #[derive(Clone, Debug, ValueEnum)]
    pub enum BranchSubcommands {
        Delete,
        Checkout,
        Show,
    }

    #[allow(dead_code)]
    pub fn get_bin_name() -> String {
        let cmd = GitiAppArgs::command();
        cmd.get_bin_name().unwrap_or("this command").to_string()
    }
}

mod branch_delete {
    use super::*;
    use crate::display_prompts::{show_tuify, tui_exit};

    pub fn tui_init() {
        let options = git_commands::get_branches();
        let header = "Please select the branches you want to delete";
        let selection_mode = SelectionMode::Multiple;
        let tuify_output = show_tuify(options, header.to_string(), selection_mode);
        match tuify_output {
            Some(branches) => {
                let options = vec!["Yes".to_string(), "No".to_string(), "Cancel".to_string()];
                let header = format!("Are you sure you want to delete {:?}?", branches);
                let selection_mode = SelectionMode::Single;
                let tuify_output = show_tuify(options, header, selection_mode);
                match tuify_output {
                    Some(it) => {
                        if it[0] == "Yes".to_string() {
                            let mut command = Command::new("git");
                            command.arg("branch").arg("-D");
                            for branch in branches {
                                command.arg(branch.to_string());
                            }
                            let op = command.output().expect("failed to execute git branch");
                            if op.status.success() {
                                (AnsiStyledText {
                                    text: "Deleted Successfully !",
                                    style: &[Style::Bold, Style::Foreground(Color::Rgb(1, 200, 1))],
                                })
                                .println();
                                tui_exit(tui_init);
                            } else {
                                (AnsiStyledText {
                                    text: &format!(
                                        "Failed to delete branches !\n{:#?}",
                                        String::from_utf8(op.stderr).unwrap()
                                    ),
                                    style: &[Style::Bold, Style::Foreground(Color::Rgb(200, 1, 1))],
                                })
                                .println();
                            }
                        } else if it[0] == "No".to_string() {
                            tui_exit(tui_init);
                        } else {
                            tui_init();
                        }
                    }
                    None => tui_exit(tui_init),
                }
            }
            None => tui_exit(tui_init),
        }
    }
}

mod checkout_branch {
    use super::*;
    use crate::display_prompts::{show_tuify, tui_exit};

    pub fn tui_init() {
        let options = git_commands::get_branches();
        let header = "Please select the branch you want to checkout to";
        let selection_mode = SelectionMode::Single;
        let tuify_output = show_tuify(options, header.to_string(), selection_mode);
        match tuify_output {
            Some(branches) => {
                let options = vec!["Yes".to_string(), "No".to_string(), "Cancel".to_string()];
                let header = format!("Are you sure you want to checkout to {:?} ?", branches[0]);
                let selection_mode = SelectionMode::Single;
                let tuify_output = show_tuify(options, header, selection_mode);
                match tuify_output {
                    Some(it) => {
                        if it[0] == "Yes".to_string() {
                            let mut command = Command::new("git");
                            command.arg("checkout").arg(branches[0].to_string());
                            let op = command.output().expect("failed to execute git branch");
                            if op.status.success() {
                                (AnsiStyledText {
                                    text: &format!("You are now on branch {}", branches[0]),
                                    style: &[Style::Bold, Style::Foreground(Color::Rgb(1, 200, 1))],
                                })
                                .println();
                                tui_exit(tui_init);
                            } else {
                                (AnsiStyledText {
                                    text: &format!(
                                        "Failed to checkout branch !\n{:#?}",
                                        String::from_utf8(op.stderr).unwrap()
                                    ),
                                    style: &[Style::Bold, Style::Foreground(Color::Rgb(200, 1, 1))],
                                })
                                .println();
                            }
                        } else if it[0] == "No".to_string() {
                            tui_exit(tui_init);
                        } else {
                            tui_init();
                        }
                    }
                    None => tui_exit(tui_init),
                }
            }
            None => tui_exit(tui_init),
        }
    }
}

mod add_files_to_staging {
    use crate::display_prompts::{show_tuify, tui_exit};

    use super::*;

    pub fn tui_init() {
        let options = git_commands::get_changed_files();
        let header = "Please select the files you want to add to staging area.";
        let selection_mode = SelectionMode::Multiple;
        let tuify_output = show_tuify(options, header.to_string(), selection_mode);
        match tuify_output {
            Some(files) => {
                let mut command = Command::new("git");
                command.arg("add");
                for file in files {
                    command.arg(file);
                }
                let op = command.output().expect("failed to add file to staging.");
                if op.status.success() {
                    (AnsiStyledText {
                        text: &format!("All selected files were added to staging"),
                        style: &[Style::Bold, Style::Foreground(Color::Rgb(1, 200, 1))],
                    })
                    .println();
                    tui_exit(tui_init);
                } else {
                    (AnsiStyledText {
                        text: &format!(
                            "Failed to add file !\n{:#?}",
                            String::from_utf8(op.stderr).unwrap()
                        ),
                        style: &[Style::Bold, Style::Foreground(Color::Rgb(200, 1, 1))],
                    })
                    .println();
                }
            }
            None => tui_exit(tui_init),
        }
    }
}
mod git_commands {
    use super::*;

    pub fn get_branches() -> Vec<String> {
        let output = Command::new("git")
            .arg("branch")
            .arg("--format")
            .arg("%(refname:short)")
            .output()
            .expect("failed to execute git branch");

        let output = String::from_utf8(output.stdout).expect("failed to convert output to string");

        let mut branches = vec![];

        for line in output.lines() {
            branches.push(line.to_string());
        }
        branches
    }

    pub fn get_changed_files() -> Vec<String> {
        let output = Command::new("git")
            .arg("diff")
            .arg("--name-only")
            .arg("--relative")
            .output()
            .expect("failed to execute git diff");

        let output = String::from_utf8(output.stdout).expect("failed to convert output to string");

        let mut changed_files = vec![];

        for line in output.lines() {
            changed_files.push(line.to_string());
        }

        changed_files
    }
}

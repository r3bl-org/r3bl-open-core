/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use std::env::var;

use clap::Parser;
use r3bl_cmdr::{AnalyticsAction, edi::launcher, report_analytics, upgrade_check};
use r3bl_tui::{ColorWheel,
               CommonResult,
               DefaultIoDevices,
               GradientGenerationPolicy,
               InlineString,
               TextColorizationPolicy,
               choose,
               fg_lizard_green,
               fg_slate_gray,
               height,
               log::try_initialize_logging_global,
               readline_async::{HowToChoose, StyleSheet},
               set_jemalloc_in_main,
               throws,
               width};

use crate::clap_config::CLIArg;

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> CommonResult<()> {
    set_jemalloc_in_main!();

    throws!({
        // Parse CLI args.
        let cli_arg: CLIArg = CLIArg::parse();

        // Start logging.
        let enable_logging = cli_arg.global_options.enable_logging;

        enable_logging.then(|| {
            try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Start logging...",
                cli_arg = ?cli_arg
            );
        });

        // Check analytics reporting.
        if cli_arg.global_options.no_analytics {
            report_analytics::disable();
        }

        upgrade_check::start_task_to_check_for_updates();
        report_analytics::start_task_to_generate_event(
            "".to_string(),
            AnalyticsAction::EdiAppStart,
        );

        // Open the editor.
        match cli_arg.file_paths.len() {
            0 => {
                report_analytics::start_task_to_generate_event(
                    "".to_string(),
                    AnalyticsAction::EdiFileNew,
                );
                launcher::run_app(None).await?;
            }
            1 => {
                report_analytics::start_task_to_generate_event(
                    "".to_string(),
                    AnalyticsAction::EdiFileOpenSingle,
                );
                let maybe_file_path = Some(cli_arg.file_paths[0].as_str());
                launcher::run_app(maybe_file_path).await?;
            }
            _ => {
                if let Some(ref file_path) =
                    edi_ui_templates::handle_multiple_files_not_supported_yet(cli_arg)
                        .await
                {
                    report_analytics::start_task_to_generate_event(
                        "".to_string(),
                        AnalyticsAction::EdiFileOpenMultiple,
                    );
                    launcher::run_app(Some(file_path)).await?;
                }
            }
        }

        // Stop logging.
        enable_logging.then(|| {
            tracing::debug!(message = "Stop logging...");
        });

        // Exit message.
        edi_ui_templates::print_exit_message();
    })
}

pub mod edi_ui_templates {
    use super::*;

    pub async fn handle_multiple_files_not_supported_yet(
        cli_arg: CLIArg,
    ) -> Option<InlineString> {
        // Ask the user to select a file to edit.
        let mut default_io_devices = DefaultIoDevices::default();
        let maybe_user_choices = choose(
            "edi currently only allows you to edit one file at a time. Select one:",
            cli_arg
                .file_paths
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            Some(height(5)),
            Some(width(0)),
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .await
        .ok();

        // Return the single user choice, if there is one.
        if let Some(user_choices) = maybe_user_choices {
            if let Some(user_choice) = user_choices.first() {
                return Some(user_choice.clone());
            }
        }

        // Otherwise, return None.
        None
    }

    pub fn print_exit_message() {
        if upgrade_check::is_update_required() {
            println!("{}", {
                let msg_line_1 = {
                    ColorWheel::default().colorize_into_string(
                        "New version of edi is available 📦.",
                        GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                        TextColorizationPolicy::ColorEachCharacter(None),
                        None,
                    )
                };

                let msg_line_2 = {
                    format!(
                        "{a}{b}{c}",
                        a = fg_slate_gray("Run `"),
                        b = fg_lizard_green("cargo install r3bl-cmdr"),
                        c = fg_slate_gray("` to upgrade 🚀."),
                    )
                };

                format!("{}\n{}", msg_line_1, msg_line_2)
            });
        } else {
            println!("{}", {
                let goodbye_to_user = match var("USER") {
                    Ok(username) => {
                        format!("\n Goodbye, 👋 {}. Thanks for using 🦜 edi !", username)
                    }
                    Err(_) => "\n Goodbye 👋. Thanks for using 🦜 edi!".to_owned(),
                };

                let please_star_us = format!(
                    "{}: {}",
                    " Please star us and report issues on GitHub",
                    "🌟 🐞 https://github.com/r3bl-org/r3bl-open-core/issues/new/choose"
                );

                let plain_text_exit_msg = format!("{goodbye_to_user}\n{please_star_us}");

                ColorWheel::lolcat_into_string(&plain_text_exit_msg, None)
            });
        }
    }
}

mod clap_config {
    use clap::{Args, Parser};

    /// More info: <https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_2/index.html>
    #[derive(Debug, Parser)]
    #[command(bin_name = "edi")]
    #[command(
        about = "🦜 Edit Markdown with style 💖\n\x1b[38;5;206mEarly access preview \x1b[0m🐣"
    )]
    #[command(version)]
    #[command(next_line_help = true)]
    #[command(arg_required_else_help(false))]
    /// More info: <https://docs.rs/clap/latest/clap/struct.Command.html#method.help_template>
    #[command(
         /* cspell:disable-next-line */
         help_template = "{about}\nVersion: {bin} {version} 💻\n\nProvide file paths, separated by spaces, to edit in edi. Or no arguments to edit a new file.\nUSAGE 📓:\n  edi [\x1b[32mfile paths\x1b[0m] [\x1b[34moptions\x1b[0m]\n\n[options]\n{options}",
         subcommand_help_heading("Command")
     )]
    pub struct CLIArg {
        #[arg(name = "file paths")]
        pub file_paths: Vec<String>,

        #[command(flatten)]
        pub global_options: GlobalOption,
    }

    #[derive(Debug, Args)]
    pub struct GlobalOption {
        #[arg(
            global = true,
            long,
            short = 'l',
            help = "Log app output to a file named `log.txt` for debugging."
        )]
        pub enable_logging: bool,

        #[arg(
            global = true,
            long,
            short = 'n',
            help = "Disable anonymous data collection for analytics to improve the product; this data does not include IP addresses, or any other private user data, like user, branch, or repo names"
        )]
        pub no_analytics: bool,
    }
}

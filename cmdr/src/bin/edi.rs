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

use std::env::var;

use clap::Parser;
use r3bl_cmdr::edi::launcher;
use r3bl_rs_utils_core::{call_if_true,
                         log_debug,
                         throws,
                         try_to_set_log_level,
                         CommonResult,
                         UnicodeString};
use r3bl_tui::{ColorWheel, GradientGenerationPolicy, TextColorizationPolicy};

use crate::clap_config::CLIArg;

// 00: [_] handle analytics flag

#[tokio::main]
async fn main() -> CommonResult<()> {
    throws!({
        // Parse CLI args.
        let cli_arg: CLIArg = CLIArg::parse();

        // Start logging.
        let enable_logging = cli_arg.global_options.enable_logging;
        call_if_true!(enable_logging, {
            try_to_set_log_level(log::LevelFilter::Trace).ok();
            log_debug("Start logging...".to_string());
            log_debug(format!("cli_args {:?}", cli_arg));
        });

        // Open the editor.
        match cli_arg.file_paths.len() {
            0 => {
                launcher::run_app(None).await?;
            }
            1 => {
                launcher::run_app(Some(cli_arg.file_paths[0].clone())).await?;
            }
            _ => match edi_ui_templates::handle_multiple_files_not_supported_yet(cli_arg)
            {
                Some(file_path) => {
                    launcher::run_app(Some(file_path)).await?;
                }
                _ => {}
            },
        }

        // Stop logging.
        call_if_true!(enable_logging, {
            log_debug("Stop logging...".to_string());
        });

        // Exit message.
        edi_ui_templates::print_exit_message();
    })
}

pub mod edi_ui_templates {
    use r3bl_tuify::{select_from_list, SelectionMode, StyleSheet};

    use super::*;

    pub fn handle_multiple_files_not_supported_yet(cli_arg: CLIArg) -> Option<String> {
        // Ask the user to select a file to edit.
        let maybe_user_choices = select_from_list(
            "edi currently only allows you to edit one file at a time. Select one:"
                .to_string(),
            cli_arg.file_paths.clone(),
            5,
            0,
            SelectionMode::Single,
            StyleSheet::default(),
        );

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
        println!("{}", {
            let goodbye_to_user = match var("USER") {
                Ok(username) => {
                    format!("Goodbye, {} 👋 🦜. Thanks for using edi!", username)
                }
                Err(_) => "Thanks for using edi! 👋 🦜".to_owned(),
            };

            let please_star_us = format!(
                "{}\n{}",
                "Please star r3bl-open-core repo on GitHub!",
                "🌟 https://github.com/r3bl-org/r3bl-open-core"
            );

            let plain_text_exit_msg = format!("{goodbye_to_user}\n{please_star_us}");

            let unicode_string = UnicodeString::from(plain_text_exit_msg);
            let mut color_wheel = ColorWheel::default();
            let lolcat_exit_msg = color_wheel.colorize_into_string(
                &unicode_string,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
            );

            lolcat_exit_msg
        });
    }
}

mod clap_config {
    use clap::{Args, Parser};

    /// More info: <https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_2/index.html>
    #[derive(Debug, Parser)]
    #[command(bin_name = "edi")]
    #[command(
        about = "Edit Markdown with happiness 💖\n\x1b[38;5;206mEarly access preview \x1b[0m🐣"
    )]
    #[command(version)]
    #[command(next_line_help = true)]
    #[command(arg_required_else_help(false))]
    /// More info: <https://docs.rs/clap/latest/clap/struct.Command.html#method.help_template>
    #[command(
        /* cspell: disable-next-line */
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

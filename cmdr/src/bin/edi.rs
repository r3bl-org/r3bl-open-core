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
use r3bl_ansi_color::{AnsiStyledText, Color, Style};
use r3bl_rs_utils_core::{throws, CommonResult, UnicodeString};
use r3bl_tui::{ColorWheel, GradientGenerationPolicy, TextColorizationPolicy};

use crate::clap_config::CLIArgs;

#[tokio::main]
async fn main() -> CommonResult<()> {
    throws!({
        // 00: handle logging flag

        let cli_args = CLIArgs::parse();
        let inputs_to_string = cli_args.file_paths.join(", ");
        AnsiStyledText {
            text: &format!("Hello! 👋 🦜 args: [{}]", inputs_to_string),
            style: &[Style::Bold, Style::Foreground(Color::Rgb(100, 200, 1))],
        }
        .println();

        println!("{}", {
            let plain_text_exit_msg = match var("USER") {
                Ok(username) => {
                    format!("Goodbye, {} 👋 🦜. Thanks for using edi!", username)
                }
                Err(_) => "Thanks for using edi! 👋 🦜".to_owned(),
            };

            let unicode_string = UnicodeString::from(plain_text_exit_msg);
            let mut color_wheel = ColorWheel::default();
            let lolcat_exit_msg = color_wheel.colorize_into_string(
                &unicode_string,
                GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
                TextColorizationPolicy::ColorEachCharacter(None),
            );

            lolcat_exit_msg
        });
    })
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
        help_template = "{about}\nVersion: {bin} {version} 💻\n\nProvide file paths, separated by spaces, to edit in edi. Or no arguments to edit a new file.\nUSAGE 📓:\n  edi [\x1b[32mfile paths\x1b[0m] [\x1b[34moptions\x1b[0m]\n\n[options]\n{options}",
        subcommand_help_heading("Command")
    )]
    pub struct CLIArgs {
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

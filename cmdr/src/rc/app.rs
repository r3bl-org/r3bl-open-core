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

use std::{env::{args, Args},
          path::PathBuf};

use clap::{CommandFactory, Parser};
use miette::IntoDiagnostic as _;
use r3bl_core::{call_if_true, throws, CommonResult};
use r3bl_tui::ArgsToStrings as _;
use serde_json::json;
use tokio::io::{stdin, AsyncBufReadExt, AsyncRead, BufReader};

use crate::DEVELOPMENT_MODE;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    arg_required_else_help = true // Command is required, if none passed, show help.
)]
struct Cli {
    /// RC command to run
    command: String,
    /// Path to file to operate on
    path: PathBuf,
    #[arg(long = "json", short = 'j', help = "Print output as JSON")]
    json: bool,
    #[arg(long = "stdin", short = 's', help = "Read from stdin")]
    read_from_stdin: bool,
}

macro_rules! DEBUG_MSG_TEMPLATE {
    () => {
        r#"
--- DEBUG START ---
🐛🐛🐛 r3bl-cmdr -> src/entry_point/app.rs
- argv_without_process: {:?}
- result_cli_parsed: {:?}
- help_msg: ↓
{}
--- DEBUG END ---
"#
    };
}

pub async fn debug_cli_args() {
    call_if_true!(DEVELOPMENT_MODE, {
        // Get the command line arguments w/out the process name. Eg: `$ rc edit file --help` will
        // produce `edit file --help`.
        let argv_without_process: Vec<String> = args().filter_and_convert_to_strings();
        let argv_without_process: Vec<&str> = Args::as_str(&argv_without_process);
        let help_msg = format!("{}", Cli::command().render_help());

        // Handle the result.
        let result_cli_parsed = match Cli::try_parse() {
            Ok(cli_parsed) => {
                format!("🐛🐛🐛 Success: {cli_parsed:?}")
            }
            Err(error) => {
                format!("🚨🚨🚨 Error: {error:?}")
            }
        };

        println!(
            DEBUG_MSG_TEMPLATE!(),
            argv_without_process, result_cli_parsed, help_msg
        );
    });
}

pub async fn run_app() -> CommonResult<()> {
    throws!({
        // Print debug info.
        debug_cli_args().await;

        // Actually parse the command line arguments.
        let cli_parsed = Cli::parse();
        let cli_msg = format!("{cli_parsed:?}");

        let mut count = 0;

        // Test using `echo -e "foo\nbar" | cargo run -- edit foo.txt -s -j` or
        // `echo -e "foo\nbar" | cargo run -- edit foo.txt -s`
        if cli_parsed.read_from_stdin {
            let stdin = stdin();
            let buf_reader = BufReader::new(stdin);
            count = words_in_buf_reader(buf_reader).await?;
            println!("🚀🚀🚀 Run program w/ parsed args: {cli_msg} and count: {count:?}");
        }

        if cli_parsed.json {
            let json = if count > 0 {
                json!({
                    "type": "cli_arguments",
                    "content": cli_msg,
                    "count": count,
                })
            } else {
                json!({
                    "type": "cli_arguments",
                    "content": cli_msg,
                })
            };
            println!("{json}");
        }
    });
}

pub async fn words_in_buf_reader<R: AsyncRead + Unpin>(
    buf_reader: BufReader<R>,
) -> CommonResult<usize> {
    let mut lines = buf_reader.lines();
    let mut count = 0;
    while let Some(line) = lines.next_line().await.into_diagnostic()? {
        count += line.split(' ').count();
    }
    Ok(count)
}

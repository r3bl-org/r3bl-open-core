// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words mfile moptions

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
        help = "Log app output to a file named `/tmp/r3bl_tui/log.txt` for debugging."
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

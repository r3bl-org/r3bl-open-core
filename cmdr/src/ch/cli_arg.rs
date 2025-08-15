// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use clap::{Args, Parser};

/// Claude history browser - Select and copy previous Claude prompts to clipboard
#[derive(Debug, Parser)]
#[command(
    name = "ch",
    about = "📋 Select and copy previous \x1b[34mClaude Code\x1b[0m prompts to clipboard 💪\n\x1b[38;5;206mBrowse your prompt history \x1b[0m🔮"
)]
#[command(version)]
#[command(next_line_help = true)]
#[command(arg_required_else_help = false)]
#[command(
    help_template = "{about}\nVersion: {bin} {version} 💻\n\nUSAGE 📓:\n  ch [\x1b[34mOptions\x1b[0m]\n\n{all-args}\n"
)]
pub struct CLIArg {
    /// Global options (logging, analytics, etc.)
    #[command(flatten)]
    pub global_options: GlobalOption,
}

#[derive(Debug, Args)]
pub struct GlobalOption {
    #[arg(
        global = true,
        long,
        short = 'l',
        help = "Log app output to a file named `log.txt` for debugging"
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

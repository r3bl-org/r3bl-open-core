// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words mbranch mcommand moptions

use clap::{Args, Parser, Subcommand, ValueEnum};
use r3bl_tui::ItemsOwned;

#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn get_giti_command_subcommand_names(arg: CLICommand) -> ItemsOwned {
    match arg {
        CLICommand::Branch { .. } => BranchSubcommand::value_variants()
            .iter()
            .map(|subcommand| format!("{subcommand:?}").to_ascii_lowercase().into())
            .collect(),
        _ => unimplemented!(),
    }
}

#[derive(Debug, Parser)]
#[command(bin_name = "giti")]
#[command(
    about = "😺 Version control with confidence 💪\n\x1b[38;5;206mEarly access preview \x1b[0m🐣"
)]
#[command(version)] /* #[command(version = env!("CARGO_PKG_VERSION"))] */
#[command(next_line_help = true)]
#[command(arg_required_else_help(true))]
/// More info: <https://docs.rs/clap/latest/clap/struct.Command.html#method.help_template>
#[command(
    help_template = "{about}\nVersion: {bin} {version} 💻\n\nUSAGE 📓:\n  giti [\x1b[32mCommand\x1b[0m] [\x1b[34mOptions\x1b[0m]\n\n{all-args}\n",
    subcommand_help_heading("Command")
)]
/// More info:
/// - <https://docs.rs/clap/latest/clap/_derive/#overview>
/// - <https://developerlife.com/2023/09/17/tuify-clap/>
pub struct CLIArg {
    #[command(subcommand)]
    pub command: CLICommand,

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

#[derive(Debug, Subcommand)]
pub enum CLICommand {
    #[clap(
        about = "🌱 Manage your git branches with commands: `delete`, `checkout`, and `new`\n💡 Eg: `giti branch delete`"
    )]
    /// More info: <https://docs.rs/clap/latest/clap/struct.Command.html#method.help_template>
    #[command(
        help_template = "{about} \n\nUSAGE 📓:\n  giti branch [\x1b[34mcommand\x1b[0m] [\x1b[32mbranch_name\x1b[0m] [\x1b[32moptions\x1b[0m]\n\n{positionals}\n\n  [options]\n{options}"
    )]
    Branch {
        #[arg(
            value_name = "command",
            help = "In your shell, this command will execute, taking each selected item as an argument."
        )]
        /// Run this sub command with each selected item as an argument, if
        /// `maybe_branch_name` is not provided.
        sub_cmd: Option<BranchSubcommand>,
        #[arg(
            value_name = "branch_name",
            help = "Optional branch name to use with the sub command."
        )]
        /// Optional branch name to use with the sub command.
        maybe_branch_name: Option<String>,
    },

    #[clap(about = "TODO Commit help")]
    Commit {},

    #[clap(about = "TODO Remote help")]
    Remote {},
}

/// The ordering of these variants is important. The order in which they appear here is
/// the order in which they are enumerated (and in some cases, displayed to the user).
#[derive(Clone, Debug, ValueEnum)]
pub enum BranchSubcommand {
    #[clap(help = "Switch to the selected branch")]
    Checkout,
    #[clap(help = "Delete one or more selected branches")]
    Delete,
    #[clap(help = "TODO Create a new branch")]
    New,
}

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

use clap::{Args, Parser, Subcommand, ValueEnum};
use r3bl_tui::ItemsOwned;

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
#[command(version)]
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
            /* cSpell:disable-next-line */
            help_template = "{about} \n\nUSAGE 📓:\n  giti branch [\x1b[34mcommand\x1b[0m] [\x1b[32moptions\x1b[0m]\n\n{positionals}\n\n  [options]\n{options}"
        )]
    Branch {
        #[arg(
            value_name = "command",
            help = "In your shell, this command will execute, taking each selected item as an argument."
        )]
        command_to_run_with_each_selection: Option<BranchSubcommand>,
        maybe_branch_name: Option<String>,
    },

    #[clap(about = "TODO Commit help")]
    Commit {},

    #[clap(about = "TODO Remote help")]
    Remote {},
}

#[derive(Clone, Debug, ValueEnum)]
pub enum BranchSubcommand {
    #[clap(help = "Delete one or more selected branches")]
    Delete,
    #[clap(help = "Switch to the selected branch")]
    Checkout,
    #[clap(help = "TODO Create a new branch")]
    New,
}

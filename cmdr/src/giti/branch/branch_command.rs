/*
 *   Copyright (c) 2025 R3BL LLC
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

use clap::ValueEnum;
use r3bl_tui::{CommandRunResult,
               CommonResult,
               DefaultIoDevices,
               ast,
               ast_line,
               choose,
               height,
               new_style,
               readline_async::{HowToChoose, StyleSheet},
               tui_color};
use smallvec::smallvec;

use crate::giti::{BranchSubcommand,
                  CLICommand,
                  CommandRunDetails,
                  get_giti_command_subcommand_names,
                  handle_branch_checkout_command,
                  handle_branch_delete_command,
                  handle_branch_new_command,
                  ui_str,
                  ui_templates::single_select_instruction_header};

/// The main function to for `giti branch` command. This is the main routing function that
/// directs execution flow to the appropriate subcommand handler: `checkout`, `delete`,
/// `new`.
pub async fn handle_branch_command(
    sub_cmd: Option<BranchSubcommand>,
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandRunResult<CommandRunDetails>> {
    if let Some(subcommand) = sub_cmd {
        match subcommand {
            BranchSubcommand::Delete => {
                handle_branch_delete_command(maybe_branch_name).await
            }
            BranchSubcommand::Checkout => {
                handle_branch_checkout_command(maybe_branch_name).await
            }
            BranchSubcommand::New => handle_branch_new_command(maybe_branch_name).await,
        }
    } else {
        prompt_for_sub_command().await
    }
}

/// The user typed `giti branch` command with no subcommands. So prompt them for a
/// subcommand.
async fn prompt_for_sub_command() -> CommonResult<CommandRunResult<CommandRunDetails>> {
    let branch_subcommands = get_giti_command_subcommand_names(CLICommand::Branch {
        sub_cmd: None,
        maybe_branch_name: None,
    });

    let header = {
        let last_line = ast_line![ast(
            ui_str::please_select_branch_sub_command(),
            new_style!(
                color_fg: {tui_color!(frozen_blue)} color_bg: {tui_color!(moonlight_blue)}
            )
        )];
        single_select_instruction_header(smallvec![last_line])
    };

    let mut default_io_devices = DefaultIoDevices::default();
    let selected = choose(
        header,
        branch_subcommands,
        Some(height(20)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await?;

    if let Some(selected) = selected.first() {
        if let Ok(branch_subcommand) = BranchSubcommand::from_str(selected, true) {
            match branch_subcommand {
                BranchSubcommand::Delete => {
                    return handle_branch_delete_command(None).await;
                }
                BranchSubcommand::Checkout => {
                    return handle_branch_checkout_command(None).await;
                }
                BranchSubcommand::New => return handle_branch_new_command(None).await,
            }
        } else {
            unimplemented!();
        }
    };

    // User did not select anything.
    let it = CommandRunResult::Noop(ui_str::noop_message(), CommandRunDetails::Noop);
    Ok(it)
}

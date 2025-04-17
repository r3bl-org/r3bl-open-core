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
use r3bl_core::{CommonResult, ast, ast_line, height, new_style, tui_color};
use r3bl_tui::{DefaultIoDevices,
               choose,
               readline_async::{HowToChoose, StyleSheet}};
use smallvec::smallvec;

use crate::giti::{BranchSubcommand,
                  CLICommand,
                  CommandRunDetails,
                  CommandRunResult,
                  get_giti_command_subcommand_names,
                  try_checkout,
                  try_delete,
                  try_new,
                  ui_str,
                  ui_templates::single_select_instruction_header};

/// The main function to for `giti branch` command.
pub async fn try_main(
    command_to_run_with_each_selection: Option<BranchSubcommand>,
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandRunResult<CommandRunDetails>> {
    if let Some(subcommand) = command_to_run_with_each_selection {
        match subcommand {
            BranchSubcommand::Delete => try_delete().await,
            BranchSubcommand::Checkout => try_checkout(maybe_branch_name).await,
            BranchSubcommand::New => try_new(maybe_branch_name).await,
        }
    } else {
        user_typed_giti_branch().await
    }
}

async fn user_typed_giti_branch() -> CommonResult<CommandRunResult<CommandRunDetails>> {
    let branch_subcommands = get_giti_command_subcommand_names(CLICommand::Branch {
        command_to_run_with_each_selection: None,
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
                BranchSubcommand::Delete => return try_delete().await,
                BranchSubcommand::Checkout => return try_checkout(None).await,
                BranchSubcommand::New => return try_new(None).await,
            }
        } else {
            unimplemented!();
        }
    };

    // User did not select anything.
    let it = CommandRunResult::DidNotRun(ui_str::noop_message(), CommandRunDetails::Noop);
    Ok(it)
}

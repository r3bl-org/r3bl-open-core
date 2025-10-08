// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{common,
            common::ui_templates::prefix_single_select_instruction_header,
            giti::{BranchSubcommand, CLICommand, CommandRunDetails,
                   get_giti_command_subcommand_names, handle_branch_checkout_command,
                   handle_branch_delete_command, handle_branch_new_command, ui_str}};
use clap::ValueEnum;
use r3bl_tui::{CommandRunResult, CommonResult, DefaultIoDevices, ast, ast_line, choose,
               height, inline_vec,
               readline_async::{HowToChoose, StyleSheet}};

/// The main function to for `giti branch` command. This is the main routing function that
/// directs execution flow to the appropriate subcommand handler: `checkout`, `delete`,
/// `new`.
///
/// # Errors
///
/// Returns an error if any of the subcommands fail or if user interaction fails.
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
    let branch_subcommand_options =
        get_giti_command_subcommand_names(CLICommand::Branch {
            sub_cmd: None,
            maybe_branch_name: None,
        });
    let header_with_instructions = {
        let last_line = ast_line![ast(
            ui_str::please_select_branch_sub_command_msg_raw(),
            common::ui_templates::header_style_default()
        )];
        prefix_single_select_instruction_header(inline_vec![last_line])
    };
    let mut default_io_devices = DefaultIoDevices::default();
    let maybe_user_choice = choose(
        header_with_instructions,
        branch_subcommand_options,
        Some(height(20)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await?
    .into_iter()
    .next();

    // Early return if the user didn't select anything.
    let Some(user_choice) = maybe_user_choice else {
        return Ok(CommandRunResult::Noop(
            ui_str::noop_msg(),
            CommandRunDetails::Noop,
        ));
    };

    // Early return if the user-chosen branch subcommand is not valid (can't be parsed).
    let Ok(branch_subcommand) = BranchSubcommand::from_str(&user_choice, true) else {
        return Ok(CommandRunResult::Noop(
            ui_str::invalid_branch_sub_command_msg(),
            CommandRunDetails::Noop,
        ));
    };

    // Actually process the user-selected branch subcommand.
    match branch_subcommand {
        BranchSubcommand::Delete => handle_branch_delete_command(None).await,
        BranchSubcommand::Checkout => handle_branch_checkout_command(None).await,
        BranchSubcommand::New => handle_branch_new_command(None).await,
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{common,
            common::ui_templates::prefix_single_select_instruction_header,
            giti::{BranchSubcommand, CLICommand, CommandRunDetails,
                   get_giti_command_subcommand_names,
                   try_handle_branch_checkout_command,
                   try_handle_branch_delete_command, try_handle_branch_new_command,
                   ui_str}};
use clap::ValueEnum;
use r3bl_tui::{ChooseError, ChooseFutureExt, CommandRunError, CommandRunReport,
               DefaultIoDevices, ReadlineError, cli_text_inline, cli_text_line,
               inline_vec,
               readline_async::{HowToChoose, StyleSheet},
               try_choose, vp_height};

/// The main function for `giti branch` command. This is the main routing function that
/// directs execution flow to the appropriate subcommand handler: `checkout`, `delete`,
/// `new`.
///
/// # Errors
///
/// Returns [`GitiBranchError`] if pre-flight git queries or interactive terminal UI
/// operations fail.
pub async fn try_handle_branch_command(
    sub_cmd: Option<BranchSubcommand>,
    maybe_branch_name: Option<String>,
) -> Result<CommandRunReport<CommandRunDetails>, GitiBranchError> {
    if let Some(subcommand) = sub_cmd {
        match subcommand {
            BranchSubcommand::Delete => {
                try_handle_branch_delete_command(maybe_branch_name).await
            }
            BranchSubcommand::Checkout => {
                try_handle_branch_checkout_command(maybe_branch_name).await
            }
            BranchSubcommand::New => {
                try_handle_branch_new_command(maybe_branch_name).await
            }
        }
    } else {
        try_prompt_for_sub_command().await
    }
}

/// The user typed `giti branch` command with no subcommands. So prompt them for a
/// subcommand.
async fn try_prompt_for_sub_command()
-> Result<CommandRunReport<CommandRunDetails>, GitiBranchError> {
    let branch_subcommand_options =
        get_giti_command_subcommand_names(CLICommand::Branch {
            sub_cmd: None,
            maybe_branch_name: None,
        });
    let header_with_instructions = {
        let last_line = cli_text_line![cli_text_inline(
            ui_str::please_select_branch_sub_command_msg_raw(),
            common::ui_templates::header_style_default()
        )];
        prefix_single_select_instruction_header(inline_vec![last_line])
    };
    let mut default_io_devices = DefaultIoDevices::default();

    // Return the first item selected by the user.
    let maybe_user_choice = {
        try_choose(
            header_with_instructions,
            branch_subcommand_options,
            Some(vp_height(20)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .try_get_first_choice()
        .await
        .map_err(GitiBranchError::Choose)?
    };

    // Early return if the user didn't select anything.
    let Some(user_choice) = maybe_user_choice else {
        return Ok(CommandRunReport::Noop(
            ui_str::noop_msg(),
            CommandRunDetails::Noop,
        ));
    };

    // Early return if the user-chosen branch subcommand is not valid (can't be parsed).
    let Ok(branch_subcommand) = BranchSubcommand::from_str(&user_choice, true) else {
        return Ok(CommandRunReport::Noop(
            ui_str::invalid_branch_sub_command_msg(),
            CommandRunDetails::Noop,
        ));
    };

    // Actually process the user-selected branch subcommand.
    match branch_subcommand {
        BranchSubcommand::Delete => try_handle_branch_delete_command(None).await,
        BranchSubcommand::Checkout => try_handle_branch_checkout_command(None).await,
        BranchSubcommand::New => try_handle_branch_new_command(None).await,
    }
}

/// Unrecoverable infrastructure errors returned by `giti branch` command handlers before
/// a [`CommandRunReport`] can be produced.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum GitiBranchError {
    /// A pre-flight `git` query failed (for example, running outside a git repository).
    #[error(transparent)]
    #[diagnostic(transparent)]
    Git(CommandRunError),

    /// Interactive selection menu failed (non-interactive terminal or terminal I/O
    /// error).
    #[error(transparent)]
    #[diagnostic(transparent)]
    Choose(ChooseError),

    /// Async readline prompt failed (non-interactive terminal or terminal I/O error).
    #[error(transparent)]
    #[diagnostic(transparent)]
    Readline(ReadlineError),
}

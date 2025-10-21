// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use crate::{AnalyticsAction, common,
            common::ui_templates::{prefix_multi_select_instruction_header,
                                   prefix_single_select_instruction_header},
            giti::{BranchDeleteDetails, CommandRunDetails,
                   git::{self},
                   local_branch_ops::BranchExists,
                   ui_str::{self}},
            report_analytics};
use r3bl_tui::{CliText, CommandRunResult, CommonResult, DefaultIoDevices, InlineString,
               InlineVec, ItemsOwned, cli_text_line, choose, cli_text, height, inline_vec,
               readline_async::{HowToChoose, StyleSheet}};
use smallvec::smallvec;

/// The main function for `giti branch delete` command.
///
/// # Errors
///
/// Returns an error if:
/// - Git operations fail
/// - User interaction fails
/// - Branch deletion fails (protected branch, not found, etc.)
pub async fn handle_branch_delete_command(
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandRunResult<CommandRunDetails>> {
    report_analytics::start_task_to_generate_event(
        String::new(),
        AnalyticsAction::GitiBranchDelete,
    );

    // Handle case when branch name is provided directly.
    if let Some(branch_name) = maybe_branch_name {
        // Validate branch name; return early if:
        // - branch_name is empty.
        // - branch_name does not exist locally.
        // - branch_name is the same as the current branch.
        let branch_name: InlineString = branch_name.trim().into();
        let (res, _cmd) = git::local_branch_ops::try_get_local_branches().await;
        let (_, branch_info) = res?;
        if branch_name.is_empty()
            || branch_info.exists_locally(&branch_name) == BranchExists::No
            || branch_info.current_branch == branch_name
        {
            return Ok(CommandRunResult::Noop(
                ui_str::branch_delete_display::info_unable_to_msg(),
                details::empty(),
            ));
        }

        // Ask user for confirmation to delete the branch.
        let branch_to_delete = branch_name.into();
        let (confirm_header, confirm_options) =
            user_interaction::create_confirmation_prompt(&branch_to_delete);
        let selected_action =
            user_interaction::get_user_confirmation(confirm_header, confirm_options)
                .await?;

        match selected_action {
            // Actually delete the branch.
            parse_user_choice::Selection::Delete => {
                return command_execute::delete_selected_branches(&branch_to_delete)
                    .await;
            }
            // Do nothing.
            parse_user_choice::Selection::ExitProgram => {
                return Ok(CommandRunResult::Noop(
                    ui_str::branch_delete_display::info_chose_not_to_msg(),
                    details::empty(),
                ));
            }
        }
    }

    // Only proceed if some local branches exist (can't delete anything if there aren't
    // any).
    let (res, _cmd) = git::local_branch_ops::try_get_local_branches().await;
    if let Ok((_, branch_info)) = res
        && !branch_info.other_branches.is_empty()
    {
        let branches =
            user_interaction::select_branches_to_delete(branch_info.other_branches)
                .await?;

        // If the user didn't select any branches, we don't need to do anything.
        if branches.is_empty() {
            return Ok(CommandRunResult::Noop(
                ui_str::branch_delete_display::info_chose_not_to_msg(),
                details::empty(),
            ));
        }

        let (confirm_header, confirm_options) =
            user_interaction::create_confirmation_prompt(&branches);
        let selected_action =
            user_interaction::get_user_confirmation(confirm_header, confirm_options)
                .await?;

        if let parse_user_choice::Selection::Delete = selected_action {
            return command_execute::delete_selected_branches(&branches).await;
        }
    }

    Ok(CommandRunResult::Noop(
        ui_str::branch_delete_display::info_chose_not_to_msg(),
        details::empty(),
    ))
}

mod details {
    use super::{BranchDeleteDetails, CommandRunDetails, ItemsOwned};

    pub fn empty() -> CommandRunDetails {
        let it = BranchDeleteDetails {
            maybe_deleted_branches: None,
        };
        CommandRunDetails::BranchDelete(it)
    }

    pub fn with_details(branches: ItemsOwned) -> CommandRunDetails {
        if branches.is_empty() {
            empty()
        } else {
            let it = BranchDeleteDetails {
                maybe_deleted_branches: Some(branches),
            };
            CommandRunDetails::BranchDelete(it)
        }
    }
}

mod user_interaction {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub async fn select_branches_to_delete(
        branch_options: ItemsOwned,
    ) -> CommonResult<ItemsOwned> {
        let header_with_instructions = {
            let last_line = cli_text_line![cli_text(
                ui_str::branch_delete_display::select_branches_msg_raw(),
                common::ui_templates::header_style_default()
            )];
            prefix_multi_select_instruction_header(inline_vec![last_line])
        };

        let mut default_io_devices = DefaultIoDevices::default();
        choose(
            header_with_instructions,
            branch_options,
            Some(height(20)),
            None,
            HowToChoose::Multiple,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .await
    }

    pub fn create_confirmation_prompt(
        branches: &ItemsOwned,
    ) -> (InlineString, ItemsOwned) {
        debug_assert!(!branches.is_empty());

        let num_of_branches = branches.len();

        let mut confirm_deletion_options: ItemsOwned =
            ui_str::branch_delete_display::exit_msg_raw().into();

        if num_of_branches == 1 {
            let branch_name = &branches[0];
            confirm_deletion_options.insert(
                0,
                ui_str::branch_delete_display::yes_single_branch_msg_raw().into(),
            );

            // Return tuple.
            (
                ui_str::branch_delete_display::confirm_single_branch_msg(branch_name),
                confirm_deletion_options,
            )
        } else {
            confirm_deletion_options.insert(
                0,
                ui_str::branch_delete_display::yes_multiple_branches_msg_raw().into(),
            );

            // Return tuple.
            (
                ui_str::branch_delete_display::confirm_multiple_branches_msg(
                    num_of_branches,
                    branches,
                ),
                confirm_deletion_options,
            )
        }
    }

    pub async fn get_user_confirmation(
        header_text: InlineString,
        confirmation_options: ItemsOwned,
    ) -> CommonResult<parse_user_choice::Selection> {
        // Apply one style to the first line of the header, and another style to the rest
        // of the lines. Then prefix with the instruction header.
        let header_with_instructions = {
            let mut header_last_lines = header_text.lines();
            let mut header_last_lines_fmt: InlineVec<InlineVec<CliText>> = smallvec![];

            if let Some(first_line) = header_last_lines.next() {
                let first_line = cli_text_line![cli_text(
                    first_line,
                    crate::common::ui_templates::header_style_default()
                )];
                header_last_lines_fmt.push(first_line);
            }

            for line in header_last_lines {
                let line = cli_text_line![cli_text(
                    line,
                    crate::common::ui_templates::header_style_primary()
                )];
                header_last_lines_fmt.push(line);
            }

            prefix_single_select_instruction_header(header_last_lines_fmt)
        };

        let mut default_io_devices = DefaultIoDevices::default();
        let selected_option = choose(
            header_with_instructions,
            confirmation_options,
            Some(height(20)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .await?;

        Ok(parse_user_choice::Selection::from(selected_option))
    }
}

mod command_execute {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub async fn delete_selected_branches(
        branches: &ItemsOwned,
    ) -> CommonResult<CommandRunResult<CommandRunDetails>> {
        debug_assert!(!branches.is_empty());
        let (res_output, cmd) = git::try_delete_branches(branches).await;
        match res_output {
            Ok(()) => {
                let it = CommandRunResult::Run(
                    ui_str::branch_delete_display::info_success_msg(branches),
                    details::with_details(branches.clone()),
                    cmd,
                );
                Ok(it)
            }
            Err(report) => {
                let it = CommandRunResult::Fail(
                    ui_str::branch_delete_display::error_failed_msg(branches, None),
                    cmd,
                    report,
                );
                Ok(it)
            }
        }
    }
}

mod parse_user_choice {
    use super::{ItemsOwned, ui_str};

    pub enum Selection {
        Delete,
        ExitProgram,
    }

    impl From<ItemsOwned> for Selection {
        fn from(selected: ItemsOwned) -> Selection {
            let maybe_first = selected.into_iter().next();
            let Some(first) = maybe_first else {
                return Selection::ExitProgram;
            };

            if first == ui_str::branch_delete_display::yes_single_branch_msg_raw()
                || first == ui_str::branch_delete_display::yes_multiple_branches_msg_raw()
            {
                Selection::Delete
            } else {
                Selection::ExitProgram
            }
        }
    }
}

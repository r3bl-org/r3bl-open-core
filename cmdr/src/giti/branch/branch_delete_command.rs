// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{AnalyticsAction, common,
            common::ui_templates::{prefix_multi_select_instruction_header,
                                   prefix_single_select_instruction_header},
            giti::{BranchDeleteDetails, CommandRunDetails, GitiBranchError,
                   ui_str::{self}},
            report_analytics};
use r3bl_tui::{BranchExists, ChooseFutureExt, CliTextInline, CommandRunOutcome,
               CommandRunReport, DefaultIoDevices, InlineString, InlineVec, ItemsOwned,
               cli_text_inline, cli_text_line, inline_vec,
               readline_async::{HowToChoose, StyleSheet},
               try_choose, try_delete_branches, try_get_local_branches, vp_height};
use smallvec::smallvec;

/// The main function for `giti branch delete` command.
///
/// # Errors
///
/// Returns [`GitiBranchError`] if:
/// - Pre-flight git queries fail ([`GitiBranchError::Git`]).
/// - Interactive branch selection or confirmation fails ([`GitiBranchError::Choose`]).
pub async fn try_handle_branch_delete_command(
    maybe_branch_name: Option<String>,
) -> Result<CommandRunReport<CommandRunDetails>, GitiBranchError> {
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
        let (_, branch_info) = try_get_local_branches()
            .await
            .result
            .map_err(GitiBranchError::Git)?;
        if branch_name.is_empty()
            || branch_info.exists_locally(&branch_name) == BranchExists::No
            || branch_info.current_branch == branch_name
        {
            return Ok(CommandRunReport::Noop(
                ui_str::branch_delete_display::info_unable_to_msg(),
                details::empty(),
            ));
        }

        // Ask user for confirmation to delete the branch.
        let branch_to_delete = branch_name.into();
        let (confirm_header, confirm_options) =
            user_interaction::create_confirmation_prompt(&branch_to_delete);
        let selected_action =
            user_interaction::try_get_user_confirmation(confirm_header, confirm_options)
                .await?;

        match selected_action {
            // Actually delete the branch.
            parse_user_choice::Selection::Delete => {
                return Ok(
                    command_execute::delete_selected_branches(&branch_to_delete).await
                );
            }
            // Do nothing.
            parse_user_choice::Selection::ExitProgram => {
                return Ok(CommandRunReport::Noop(
                    ui_str::branch_delete_display::info_chose_not_to_msg(),
                    details::empty(),
                ));
            }
        }
    }

    // Only proceed if some local branches exist (can't delete anything if there aren't
    // any).
    if let Ok((_, branch_info)) = try_get_local_branches().await.result
        && !branch_info.other_branches.is_empty()
    {
        let maybe_branches =
            user_interaction::try_select_branches_to_delete(branch_info.other_branches)
                .await?;

        // If the user didn't select any branches, we don't need to do anything.
        let Some(branches) = maybe_branches else {
            return Ok(CommandRunReport::Noop(
                ui_str::branch_delete_display::info_chose_not_to_msg(),
                details::empty(),
            ));
        };

        let (confirm_header, confirm_options) =
            user_interaction::create_confirmation_prompt(&branches);
        let selected_action =
            user_interaction::try_get_user_confirmation(confirm_header, confirm_options)
                .await?;

        if let parse_user_choice::Selection::Delete = selected_action {
            return Ok(command_execute::delete_selected_branches(&branches).await);
        }
    }

    Ok(CommandRunReport::Noop(
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

    pub async fn try_select_branches_to_delete(
        branch_options: ItemsOwned,
    ) -> Result<Option<ItemsOwned>, GitiBranchError> {
        let header_with_instructions = {
            let last_line = cli_text_line![cli_text_inline(
                ui_str::branch_delete_display::select_branches_msg_raw(),
                common::ui_templates::header_style_default()
            )];
            prefix_multi_select_instruction_header(inline_vec![last_line])
        };

        let maybe_results = {
            let mut default_io_devices = DefaultIoDevices::default();
            try_choose(
                header_with_instructions,
                branch_options,
                Some(vp_height(20)),
                None,
                HowToChoose::Multiple,
                StyleSheet::default(),
                default_io_devices.as_mut_tuple(),
            )
            .try_get_all_choices()
            .await
            .map_err(GitiBranchError::Choose)?
        };

        Ok(maybe_results)
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

    pub async fn try_get_user_confirmation(
        header_text: InlineString,
        confirmation_options: ItemsOwned,
    ) -> Result<parse_user_choice::Selection, GitiBranchError> {
        // Apply one style to the first line of the header, and another style to the rest
        // of the lines. Then prefix with the instruction header.
        let header_with_instructions = {
            let mut header_last_lines = header_text.lines();
            let mut header_last_lines_fmt: InlineVec<InlineVec<CliTextInline>> =
                smallvec![];

            if let Some(first_line) = header_last_lines.next() {
                let first_line = cli_text_line![cli_text_inline(
                    first_line,
                    crate::common::ui_templates::header_style_default()
                )];
                header_last_lines_fmt.push(first_line);
            }

            for line in header_last_lines {
                let line = cli_text_line![cli_text_inline(
                    line,
                    crate::common::ui_templates::header_style_primary()
                )];
                header_last_lines_fmt.push(line);
            }

            prefix_single_select_instruction_header(header_last_lines_fmt)
        };

        let user_choice = {
            let mut default_io_devices = DefaultIoDevices::default();
            try_choose(
                header_with_instructions,
                confirmation_options,
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

        Ok(parse_user_choice::Selection::from(user_choice))
    }
}

mod command_execute {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub async fn delete_selected_branches(
        branches: &ItemsOwned,
    ) -> CommandRunReport<CommandRunDetails> {
        debug_assert!(!branches.is_empty());
        let CommandRunOutcome {
            result: res_output,
            command: cmd,
        } = try_delete_branches(branches).await;
        match res_output {
            Ok(()) => CommandRunReport::Run(
                ui_str::branch_delete_display::info_success_msg(branches),
                details::with_details(branches.clone()),
                cmd,
            ),
            Err(err) => CommandRunReport::Fail(
                ui_str::branch_delete_display::error_failed_msg(branches, None),
                cmd,
                err,
            ),
        }
    }
}

mod parse_user_choice {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub enum Selection {
        Delete,
        ExitProgram,
    }

    impl From<Option<InlineString>> for Selection {
        fn from(maybe_selected: Option<InlineString>) -> Selection {
            match maybe_selected {
                Some(first)
                    if first == ui_str::branch_delete_display::yes_single_branch_msg_raw()
                        || first == ui_str::branch_delete_display::yes_multiple_branches_msg_raw() =>
                {
                    Selection::Delete
                }
                _ => Selection::ExitProgram,
            }
        }
    }
}

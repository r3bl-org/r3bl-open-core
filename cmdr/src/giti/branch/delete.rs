/*
 *   Copyright (c) 2023-2025 R3BL LLC
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
use r3bl_tui::{AST,
               CommandRunResult,
               CommonResult,
               DefaultIoDevices,
               Header,
               InlineString,
               InlineVec,
               ItemsOwned,
               ast,
               ast_line,
               choose,
               height,
               new_style,
               readline_async::{HowToChoose, StyleSheet},
               tui_color};
use smallvec::smallvec;

use crate::{AnalyticsAction,
            giti::{BranchDeleteDetails,
                   CommandRunDetails,
                   git::{self},
                   ui_str::{self},
                   ui_templates::{multi_select_instruction_header,
                                  single_select_instruction_header}},
            report_analytics};

/// The main function for `giti branch delete` command.
pub async fn try_delete() -> CommonResult<CommandRunResult<CommandRunDetails>> {
    report_analytics::start_task_to_generate_event(
        "".to_string(),
        AnalyticsAction::GitiBranchDelete,
    );

    // Only proceed if some local branches exist (can't delete anything if there aren't
    // any).
    let (res, _cmd) =
        git::local_branch_ops::try_get_local_branch_names_with_current_marked().await;
    if let Ok(branches) = res
        && !branches.is_empty()
    {
        let branches = user_interaction::select_branches_to_delete(branches).await?;

        // If the user didn't select any branches, we don't need to do anything.
        if branches.is_empty() {
            return Ok(CommandRunResult::Noop(
                ui_str::branch_delete_display::info_no_branches_deleted(),
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
        ui_str::branch_delete_display::info_no_branches_deleted(),
        details::empty(),
    ))
}

mod details {
    use super::*;

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
    use super::*;

    pub fn create_multi_select_header() -> impl Into<Header> {
        let default_header_style = new_style!(
            color_fg: {tui_color!(frozen_blue)} color_bg: {tui_color!(moonlight_blue)}
        );
        let last_line = ast_line![ast(
            ui_str::branch_delete_display::please_select_branches_you_want_to_delete(),
            default_header_style
        )];
        multi_select_instruction_header(last_line)
    }

    pub async fn select_branches_to_delete(
        branches: ItemsOwned,
    ) -> CommonResult<ItemsOwned> {
        let header = create_multi_select_header();
        let mut default_io_devices = DefaultIoDevices::default();
        choose(
            header,
            branches,
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
            ui_str::branch_delete_display::exit_message().into();

        if num_of_branches == 1 {
            let branch_name = &branches[0];
            confirm_deletion_options.insert(
                0,
                ui_str::branch_delete_display::yes_delete_branch_message().into(),
            );

            // Return tuple.
            (
                ui_str::branch_delete_display::confirm_delete_one_branch(branch_name),
                confirm_deletion_options,
            )
        } else {
            confirm_deletion_options.insert(
                0,
                ui_str::branch_delete_display::yes_delete_branches_message().into(),
            );

            // Return tuple.
            (
                ui_str::branch_delete_display::confirm_deleting_multiple_branches(
                    num_of_branches,
                    branches,
                ),
                confirm_deletion_options,
            )
        }
    }

    pub async fn get_user_confirmation(
        header_text: InlineString,
        options: ItemsOwned,
    ) -> CommonResult<parse_user_choice::Selection> {
        // Define styles for the header first line, and subsequent lines (which are branches
        // that are selected for deletion).
        let default_header_style = new_style!(
            color_fg: {tui_color!(frozen_blue)} color_bg: {tui_color!(moonlight_blue)}
        );
        let branch_to_delete_style = new_style!(
            color_fg: {tui_color!(yellow)} color_bg: {tui_color!(moonlight_blue)}
        );

        // Apply one style to the first line of the header, and another style to the rest of
        // the lines. Then prefix with the instruction header.
        let header = {
            let mut header_last_lines = header_text.lines();
            let mut header_last_lines_fmt: InlineVec<InlineVec<AST>> = smallvec![];

            if let Some(first_line) = header_last_lines.next() {
                let first_line = ast_line![ast(first_line, default_header_style)];
                header_last_lines_fmt.push(first_line);
            }

            for line in header_last_lines {
                let line = ast_line![ast(line, branch_to_delete_style)];
                header_last_lines_fmt.push(line);
            }

            single_select_instruction_header(header_last_lines_fmt)
        };

        let mut default_io_devices = DefaultIoDevices::default();
        let selected_option = choose(
            header,
            options,
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
    use super::*;

    pub async fn delete_selected_branches(
        branches: &ItemsOwned,
    ) -> CommonResult<CommandRunResult<CommandRunDetails>> {
        debug_assert!(!branches.is_empty());
        let (res_output, cmd) = git::try_delete_branches(branches).await;
        match res_output {
            Ok(_) => {
                let it = CommandRunResult::Run(
                    ui_str::branch_delete_display::info_delete_success(branches),
                    details::with_details(branches.clone()),
                    cmd,
                );
                Ok(it)
            }
            Err(report) => {
                let it = CommandRunResult::Fail(
                    ui_str::branch_delete_display::error_failed_to_delete(branches, None),
                    cmd,
                    report,
                );
                Ok(it)
            }
        }
    }
}

mod parse_user_choice {
    use super::*;

    pub enum Selection {
        Delete,
        ExitProgram,
    }

    impl From<ItemsOwned> for Selection {
        fn from(selected: ItemsOwned) -> Selection {
            if selected.is_empty() {
                return Selection::ExitProgram;
            }

            let first = &selected[0];

            match (
                first == ui_str::branch_delete_display::yes_delete_branch_message(),
                first == ui_str::branch_delete_display::yes_delete_branches_message(),
                first == ui_str::branch_delete_display::exit_message(),
            ) {
                (true, _, _) | (_, true, _) => Selection::Delete,
                (_, _, true) => Selection::ExitProgram,
                _ => Selection::ExitProgram,
            }
        }
    }
}

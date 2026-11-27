// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::giti::{BranchNewDetails, CommandRunDetails, GitiBranchError, ui_str};
use r3bl_tui::{BranchExists, CommandRunOutcome, CommandRunReport, ReadlineAsyncContext,
               ReadlineEvent, try_create_and_switch_to_branch, try_get_local_branches};

/// The main function for `giti branch new` command.
///
/// # Errors
///
/// Returns [`GitiBranchError`] if:
/// - Pre-flight git queries fail ([`GitiBranchError::Git`]).
/// - Interactive branch name input fails ([`GitiBranchError::Readline`]).
pub async fn try_handle_branch_new_command(
    maybe_branch_name: Option<String>,
) -> Result<CommandRunReport<CommandRunDetails>, GitiBranchError> {
    match maybe_branch_name {
        Some(branch_name) => command_execute::try_create_new_branch(branch_name).await,
        None => user_interaction::try_prompt_for_branch_name().await,
    }
}

mod details {
    use super::{BranchNewDetails, CommandRunDetails};

    pub fn empty() -> CommandRunDetails {
        CommandRunDetails::BranchNew(BranchNewDetails {
            maybe_created_branch: None,
        })
    }

    pub fn with_details(created_branch: String) -> CommandRunDetails {
        CommandRunDetails::BranchNew(BranchNewDetails {
            maybe_created_branch: Some(created_branch),
        })
    }
}

mod command_execute {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub async fn try_create_new_branch(
        branch_name: String,
    ) -> Result<CommandRunReport<CommandRunDetails>, GitiBranchError> {
        let (_, branch_info) = try_get_local_branches()
            .await
            .result
            .map_err(GitiBranchError::Git)?;

        if let BranchExists::Yes = branch_info.exists_locally(&branch_name) {
            let string =
                ui_str::branch_create_display::info_branch_already_exists(&branch_name);
            let it = CommandRunReport::Noop(string, details::with_details(branch_name));
            return Ok(it);
        }

        let CommandRunOutcome {
            result: res_output,
            command: cmd,
        } = try_create_and_switch_to_branch(&branch_name).await;
        match res_output {
            Ok(()) => {
                let it = CommandRunReport::Run(
                    ui_str::branch_create_display::info_create_success(&branch_name),
                    details::with_details(branch_name),
                    cmd,
                );
                Ok(it)
            }
            Err(err) => {
                let string =
                    ui_str::branch_create_display::error_failed_to_create_new_branch(
                        &branch_name,
                    );
                let it = CommandRunReport::Fail(string, cmd, err);
                Ok(it)
            }
        }
    }
}

mod user_interaction {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub async fn try_prompt_for_branch_name()
    -> Result<CommandRunReport<CommandRunDetails>, GitiBranchError> {
        let prompt_text =
            ui_str::branch_create_display::enter_branch_name_you_want_to_create();

        let mut rl_ctx = ReadlineAsyncContext::try_new(Some(&prompt_text), None)
            .await
            .map_err(GitiBranchError::Readline)?;

        // The loop is just to handle the resize event.
        loop {
            let evt = rl_ctx
                .try_read_line()
                .await
                .map_err(GitiBranchError::Readline)?;
            match evt {
                ReadlineEvent::Line(branch_name) => {
                    rl_ctx
                        .try_request_shutdown(None)
                        .await
                        .map_err(GitiBranchError::Readline)?;
                    rl_ctx.await_shutdown().await;

                    return command_execute::try_create_new_branch(branch_name).await;
                }
                ReadlineEvent::Eof | ReadlineEvent::Interrupted => {
                    rl_ctx
                        .try_request_shutdown(None)
                        .await
                        .map_err(GitiBranchError::Readline)?;
                    rl_ctx.await_shutdown().await;

                    let it = CommandRunReport::Noop(
                        ui_str::branch_create_display::info_no_branch_created(),
                        details::empty(),
                    );

                    return Ok(it);
                }
                // Ignore navigation/function keys - not used in branch name input.
                ReadlineEvent::Resized(_)
                | ReadlineEvent::Tab
                | ReadlineEvent::BackTab
                | ReadlineEvent::PageUp
                | ReadlineEvent::PageDown
                | ReadlineEvent::Insert
                | ReadlineEvent::FnKey(_)
                | ReadlineEvent::UnhandledKey(_) => {}
            }
        }
    }
}

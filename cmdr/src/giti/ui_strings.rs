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

use strum_macros::Display;

// 01: use this constant in the `CurrentBranch` enum below after removing the enum
pub const CURRENT_PREFIX: &str = "(current) ";

// 01: remove the use of this UIStrings enum and just make these plain functions

#[derive(Display)]
pub enum UIStrings {
    #[strum(serialize = "Please select a branch subcommand")]
    PleaseSelectBranchSubCommand,

    #[strum(serialize = " Please select branches you want to delete")]
    PleaseSelectBranchesYouWantToDelete,

    #[strum(serialize = " Confirm deleting 1 branch: {branch_name}")]
    ConfirmDeletingOneBranch { branch_name: String },

    /// See [`display_confirm_deleting_multiple_branches()`] for the actual string.
    #[strum(
        serialize = "Please display_confirm_deleting_multiple_branches() instead of this"
    )]
    ConfirmDeletingMultipleBranches {
        num_of_branches: usize,
        branches_to_delete: Vec<String>,
    },

    #[strum(serialize = "Yes, delete branch")]
    YesDeleteBranch,

    #[strum(serialize = "Yes, delete branches")]
    YesDeleteBranches,

    #[strum(serialize = "Exit")]
    Exit,

    #[strum(serialize = " Failed to delete branch: {branch_name}!\n\n{error_message}")]
    FailedToDeleteBranch {
        branch_name: String,
        error_message: String,
    },

    #[strum(serialize = " Failed to delete branches:\n ╴{branches}!\n\n{error_message}")]
    FailedToDeleteBranches {
        branches: String,
        error_message: String,
    },

    #[strum(serialize = " Failed to run command to delete branches:\n ╴{branches}!")]
    FailedToRunCommandToDeleteBranches { branches: String },

    #[strum(serialize = "deleted")]
    Deleted,

    #[strum(serialize = "(current) {branch}")]
    CurrentBranch { branch: String },

    #[strum(serialize = " Select a branch to switch to")]
    SelectBranchToSwitchTo,

    #[strum(serialize = " You are already on branch ")]
    AlreadyOnCurrentBranch,

    #[strum(serialize = " Switched to branch ✅ ")]
    SwitchedToBranch,

    #[strum(serialize = " Failed to switch to branch '{branch}'!\n\n{error_message}")]
    FailedToSwitchToBranch {
        branch: String,
        error_message: String,
    },

    #[strum(serialize = " You chose not to delete any branches.")]
    NoBranchGotDeleted,

    #[strum(serialize = " No branch got checked out ... \n ╴{branch}!\n\n")]
    NoBranchGotCheckedOut { branch: String },

    #[strum(serialize = " No suitable branch is available for checkout.")]
    NoSuitableBranchIsAvailableForCheckout,

    #[strum(serialize = "\n Goodbye, 👋 {username}. Thanks for using 😺 giti!")]
    GoodbyeThanksForUsingGitiUsername { username: String },

    #[strum(serialize = "\n Goodbye 👋.\n\n 😺 giti!")]
    GoodbyeThanksForUsingGiti,

    #[strum(serialize = " Please report issues & star us on GitHub: 🌟 🐞 \
        https://github.com/r3bl-org/r3bl-open-core/issues/new/choose")]
    PleaseStarUs,

    #[strum(serialize = " Error executing command: '{program_name_to_string} \
        {command_args_to_string}'. Error: {command_output_error}")]
    ErrorExecutingCommand {
        program_name_to_string: String,
        command_args_to_string: String,
        command_output_error: miette::Report,
    },

    #[strum(serialize = " Could not run giti due to the following problem.\n{report}")]
    UnrecoverableErrorEncountered { report: String },

    #[strum(serialize = "Branch `{branch_name}` does not exist.")]
    BranchDoesNotExist { branch_name: String },

    #[strum(serialize = " You have a 📝 modified file on the current branch: ")]
    ModifiedFileOnCurrentBranch,

    #[strum(serialize = " You have 📝 modified files on the current branch: ")]
    ModifiedFilesOnCurrentBranch,

    #[strum(serialize = " Would you like to switch to branch '{branch_name}?'")]
    WouldYouLikeToSwitchToBranch { branch_name: String },

    #[strum(serialize = "Switch to branch and apply changes")]
    SwitchToBranchAndApplyChanges,

    #[strum(serialize = "Stay on current branch")]
    StayOnCurrentBranch,

    #[strum(serialize = " Staying on current branch ")]
    StayingOnCurrentBranch,

    #[strum(
        serialize = " Please commit your changes or stash them before you switch branches."
    )]
    PleaseCommitChangesBeforeSwitchingBranches,

    #[strum(serialize = " Branch {branch_name} already exists!")]
    BranchAlreadyExists { branch_name: String },

    #[strum(serialize = " You created and switched to branch ")]
    CreatedAndSwitchedToNewBranch,

    #[strum(serialize = " Failed to create and switch to branch {branch_name}")]
    FailedToCreateAndSwitchToBranch { branch_name: String },

    #[strum(serialize = " Failed to create new branch {branch_name}!\n\n{error_message}")]
    FailedToRunCommandToCreateBranch {
        branch_name: String,
        error_message: String,
    },

    #[strum(serialize = " Enter a branch name you want to create (Ctrl+C to exit): ")]
    EnterBranchNameYouWantToCreate,

    #[strum(serialize = " No new branch was created")]
    NoNewBranchWasCreated,
}

/// This is for [`UIStrings::ConfirmDeletingMultipleBranches`].
pub fn display_confirm_deleting_multiple_branches(
    num_of_branches: usize,
    branches_to_delete: Vec<String>,
) -> String {
    let prefixed_branches: Vec<String> = branches_to_delete
        .into_iter()
        .enumerate()
        .map(|(index, branch)| format!(" {}. {}", index + 1, branch))
        .collect();

    let mut acc = String::new();
    use std::fmt::Write as _;
    _ = write!(
        acc,
        " Confirm deleting {a} branches:\n{b}",
        a = num_of_branches,
        b = prefixed_branches.join("\n")
    );

    acc
}

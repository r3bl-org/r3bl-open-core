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

use std::fmt::{Display, Formatter};

use r3bl_core::{fg_lizard_green, fg_silver_metallic};

pub enum UIStrings {
    PleaseSelectBranchesYouWantToDelete,
    ConfirmDeletingOneBranch {
        branch_name: String,
    },
    ConfirmDeletingMultipleBranches {
        num_of_branches: usize,
        branches_to_delete: String,
    },
    YesDeleteBranch,
    YesDeleteBranches,
    Exit,
    FailedToDeleteBranch {
        branch_name: String,
        error_message: String,
    },
    FailedToDeleteBranches {
        branches: String,
        error_message: String,
    },
    FailedToRunCommandToDeleteBranches {
        branches: String,
    },
    Deleted,
    CurrentBranch {
        branch: String,
    },
    SelectBranchToSwitchTo,
    AlreadyOnCurrentBranch,
    SwitchedToBranch,
    FailedToSwitchToBranch {
        branch: String,
        error_message: String,
    },
    NoBranchGotCheckedOut {
        branch: String,
    },
    GoodbyeThanksForUsingGitiUsername {
        username: String,
    },
    GoodbyeThanksForUsingGiti,
    PleaseStarUs,
    ErrorExecutingCommand {
        program_name_to_string: String,
        command_args_to_string: String,
        command_output_error: miette::Report,
    },
    BranchDoesNotExist {
        branch_name: String,
    },
    ModifiedFileOnCurrentBranch,
    ModifiedFilesOnCurrentBranch,
    WouldYouLikeToSwitchToBranch {
        branch_name: String,
    },
    SwitchToBranchAndApplyChanges,
    StayOnCurrentBranch,
    StayingOnCurrentBranch,
    PleaseCommitChangesBeforeSwitchingBranches,
    BranchAlreadyExists {
        branch_name: String,
    },
    CreatedAndSwitchedToNewBranch,
    FailedToCreateAndSwitchToBranch {
        branch_name: String,
    },
    EnterBranchNameYouWantToCreate,
    NoNewBranchWasCreated,
}

impl Display for UIStrings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UIStrings::PleaseSelectBranchesYouWantToDelete => {
                write!(f, " Please select branches you want to delete")
            }
            UIStrings::ConfirmDeletingOneBranch { branch_name } => {
                write!(f, " Confirm deleting 1 branch: {branch_name}")
            }
            UIStrings::ConfirmDeletingMultipleBranches {
                num_of_branches,
                branches_to_delete,
            } => {
                write!(
                    f,
                    " Confirm deleting {} branches: {}?",
                    num_of_branches, branches_to_delete
                )
            }
            UIStrings::YesDeleteBranch => write!(f, "Yes, delete branch"),
            UIStrings::YesDeleteBranches => write!(f, "Yes, delete branches"),
            UIStrings::Exit => write!(f, "Exit"),
            UIStrings::FailedToDeleteBranch {
                branch_name,
                error_message,
            } => {
                write!(
                    f,
                    " Failed to delete branch: {}!\n\n{}",
                    branch_name, error_message
                )
            }
            UIStrings::FailedToDeleteBranches {
                branches,
                error_message,
            } => {
                write!(
                    f,
                    " Failed to delete branches:\n ╴{}!\n\n{}",
                    branches, error_message
                )
            }
            UIStrings::FailedToRunCommandToDeleteBranches { branches } => {
                write!(
                    f,
                    " Failed to run command to delete branches:\n ╴{branches}!"
                )
            }
            UIStrings::Deleted => write!(f, "deleted"),
            UIStrings::CurrentBranch { branch } => {
                write!(f, "(current) {branch}")
            }
            UIStrings::SelectBranchToSwitchTo => {
                write!(f, " Select a branch to switch to")
            }
            UIStrings::AlreadyOnCurrentBranch => {
                write!(f, " You are already on branch ")
            }
            UIStrings::SwitchedToBranch => write!(f, " Switched to branch ✅ "),
            UIStrings::FailedToSwitchToBranch {
                branch,
                error_message,
            } => {
                write!(
                    f,
                    " Failed to switch to branch '{branch}'!\n\n{}",
                    error_message
                )
            }
            UIStrings::NoBranchGotCheckedOut { branch } => {
                write!(f, " No branch got checked out ... \n ╴{branch}!\n\n")
            }
            UIStrings::GoodbyeThanksForUsingGitiUsername { username } => {
                write!(f, "\n Goodbye, 👋 {}. Thanks for using 😺 giti!", username)
            }
            UIStrings::GoodbyeThanksForUsingGiti => {
                write!(f, "\n Goodbye 👋.\n\n 😺 giti!")
            }
            UIStrings::PleaseStarUs => {
                write!(
                    f,
                    " Please star us and report issues on GitHub: 🌟 🐞 https://github.com/r3bl-org/r3bl-open-core/issues/new/choose"
                )
            }
            UIStrings::ErrorExecutingCommand {
                program_name_to_string,
                command_args_to_string,
                command_output_error,
            } => {
                write!(
                    f,
                    " Error executing command: '{program_name_to_string} {command_args_to_string}'. Error: {command_output_error}"
                )
            }
            UIStrings::BranchDoesNotExist { branch_name } => {
                write!(f, "Branch `{}` does not exist.", branch_name)
            }
            UIStrings::ModifiedFileOnCurrentBranch => {
                write!(f, " You have a 📝 modified file on the current branch: ")
            }
            UIStrings::ModifiedFilesOnCurrentBranch => {
                write!(f, " You have 📝 modified files on the current branch: ")
            }
            UIStrings::WouldYouLikeToSwitchToBranch { branch_name } => {
                write!(f, " Would you like to switch to branch '{branch_name}?'")
            }
            UIStrings::SwitchToBranchAndApplyChanges => {
                write!(f, "Switch to branch and apply changes")
            }
            UIStrings::StayOnCurrentBranch => write!(f, "Stay on current branch"),
            UIStrings::StayingOnCurrentBranch => {
                write!(f, " Staying on current branch ")
            }
            UIStrings::PleaseCommitChangesBeforeSwitchingBranches => write!(
                f,
                " Please commit your changes or stash them before you switch branches."
            ),
            UIStrings::BranchAlreadyExists { branch_name } => {
                write!(
                    f,
                    "{a}{b}{c}",
                    a = fg_silver_metallic(" Branch "),
                    b = fg_lizard_green(branch_name),
                    c = fg_silver_metallic(" already exists!"),
                )
            }
            UIStrings::CreatedAndSwitchedToNewBranch => {
                write!(f, " You created and switched to branch ")
            }
            UIStrings::FailedToCreateAndSwitchToBranch { branch_name } => {
                write!(f, " Failed to create and switch to branch {branch_name}")
            }
            UIStrings::EnterBranchNameYouWantToCreate => {
                write!(
                    f,
                    " Enter a branch name you want to create (Ctrl+C to exit): "
                )
            }
            UIStrings::NoNewBranchWasCreated => {
                write!(f, " No new branch was created")
            }
        }
    }
}

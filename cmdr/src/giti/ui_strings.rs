/*
 *   Copyright (c) 2024 R3BL LLC
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
        command_output_error: std::io::Error,
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

impl UIStrings {
    pub fn to_string(&self) -> String {
        match self {
            UIStrings::PleaseSelectBranchesYouWantToDelete => {
                String::from(" Please select branches you want to delete")
            }
            UIStrings::ConfirmDeletingOneBranch { branch_name } => {
                format!(" Confirm deleting 1 branch: {branch_name}")
            }
            UIStrings::ConfirmDeletingMultipleBranches {
                num_of_branches,
                branches_to_delete,
            } => {
                format!(
                    " Confirm deleting {} branches: {}?",
                    num_of_branches, branches_to_delete
                )
            }
            UIStrings::YesDeleteBranch => String::from("Yes, delete branch"),
            UIStrings::YesDeleteBranches => String::from("Yes, delete branches"),
            UIStrings::Exit => String::from("Exit"),
            UIStrings::FailedToDeleteBranch {
                branch_name,
                error_message,
            } => {
                format!(
                    " Failed to delete branch: {}!\n\n{}",
                    branch_name, error_message
                )
            }
            UIStrings::FailedToDeleteBranches {
                branches,
                error_message,
            } => {
                format!(
                    " Failed to delete branches:\n ╴{}!\n\n{}",
                    branches, error_message
                )
            }
            UIStrings::FailedToRunCommandToDeleteBranches { branches } => {
                format!(" Failed to run command to delete branches:\n ╴{branches}!")
            }
            UIStrings::Deleted => String::from("deleted"),
            UIStrings::CurrentBranch { branch } => {
                format!("(current) {branch}")
            }
            UIStrings::SelectBranchToSwitchTo => {
                String::from(" Select a branch to switch to")
            }
            UIStrings::AlreadyOnCurrentBranch => {
                String::from(" You are already on branch ")
            }
            UIStrings::SwitchedToBranch => String::from(" Switched to branch ✅ "),
            UIStrings::FailedToSwitchToBranch {
                branch,
                error_message,
            } => {
                format!(
                    " Failed to switch to branch '{branch}'!\n\n{}",
                    error_message
                )
            }
            UIStrings::NoBranchGotCheckedOut { branch } => {
                format!(" No branch got checked out ... \n ╴{branch}!\n\n")
            }
            UIStrings::GoodbyeThanksForUsingGitiUsername { username } => {
                format!("\n Goodbye, 👋 {}. Thanks for using 😺 giti!", username)
            }
            UIStrings::GoodbyeThanksForUsingGiti => {
                format!(
                    "\n Goodbye 👋. 
                
                 😺 giti!"
                )
            }
            UIStrings::PleaseStarUs => {
                format!(
                    "{}: {}",
                    " Please star us and report issues on GitHub",
                    "🌟 🐞 https://github.com/r3bl-org/r3bl-open-core/issues/new/choose"
                )
            }
            UIStrings::ErrorExecutingCommand {
                program_name_to_string,
                command_args_to_string,
                command_output_error,
            } => {
                format!(
                    " Error executing command: '{program_name_to_string} {command_args_to_string}'. Error: {command_output_error}"
                )
            }
            UIStrings::BranchDoesNotExist { branch_name } => {
                format!("Branch `{}` does not exist.", branch_name)
            }
            UIStrings::ModifiedFileOnCurrentBranch => {
                format!(" You have a 📝 modified file on the current branch: ")
            }
            UIStrings::ModifiedFilesOnCurrentBranch => {
                format!(" You have 📝 modified files on the current branch: ")
            }
            UIStrings::WouldYouLikeToSwitchToBranch { branch_name } => {
                format!(" Would you like to switch to branch '{branch_name}?'")
            }
            UIStrings::SwitchToBranchAndApplyChanges => {
                String::from("Switch to branch and apply changes")
            }
            UIStrings::StayOnCurrentBranch => String::from("Stay on current branch"),
            UIStrings::StayingOnCurrentBranch => {
                String::from(" Staying on current branch ")
            }
            UIStrings::PleaseCommitChangesBeforeSwitchingBranches => String::from(
                " Please commit your changes or stash them before you switch branches.",
            ),
            UIStrings::BranchAlreadyExists { branch_name } => {
                format!(" Branch {branch_name} already exists!")
            }
            UIStrings::CreatedAndSwitchedToNewBranch => {
                format!(" You created and switched to branch ")
            }
            UIStrings::FailedToCreateAndSwitchToBranch { branch_name } => {
                format!(" Failed to create and switch to branch {branch_name}")
            }
            UIStrings::EnterBranchNameYouWantToCreate => {
                format!(" Enter a branch name you want to create (Ctrl+C to exit) ")
            }
            UIStrings::NoNewBranchWasCreated => {
                String::from(" No new branch was created")
            }
        }
    }
}

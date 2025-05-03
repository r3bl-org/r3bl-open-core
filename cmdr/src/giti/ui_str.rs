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
use std::process::Output;

use r3bl_tui::{InlineString, ItemsOwned, inline_string, join_fmt};

use crate::common::fmt;

pub const CURRENT_BRANCH_PREFIX: &str = "(◕‿◕)";

pub fn unrecoverable_error_msg(report: miette::Report) -> InlineString {
    inline_string!(
        "{a}{b}\n{c}",
        a = fmt::error("Could not run giti due to the following problem"),
        b = fmt::colon(),
        c = fmt::error(report)
    )
}

pub fn noop_msg() -> InlineString { fmt::normal("Nothing was selected to run.") }

pub fn invalid_branch_sub_command_msg() -> InlineString {
    fmt::normal("Nothing was selected to run, since the branch sub command is invalid.")
}

/// This is unformatted text. The formatting is applied by the caller.
pub fn please_select_branch_sub_command_msg_raw() -> &'static str {
    "Please select a branch subcommand:"
}

pub mod modified_files_display {
    use super::*;

    pub fn warn_modified_files_exist_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a}{b} {c}{d}",
            a = fmt::error("You have 📝 modified files on the current branch"),
            b = fmt::colon(),
            c = fmt::emphasis(branch_name),
            d = fmt::period()
        )
    }
}

pub mod branch_checkout_display {
    use super::*;

    /// This is unformatted text. The formatted is applied by the caller.
    pub fn select_branch_to_switch_to_msg_raw() -> &'static str {
        "Select a branch to switch to:"
    }

    pub fn info_checkout_success_msg(
        branch_name: &str,
        current_branch: &str,
    ) -> InlineString {
        if branch_name == current_branch {
            info_already_on_current_branch_msg(branch_name)
        } else {
            info_switched_to_branch_msg(branch_name)
        }
    }

    pub fn no_suitable_branch_available_msg() -> InlineString {
        fmt::normal("No suitable branch is available for checkout.")
    }

    pub fn info_already_on_current_branch_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} {b}{c}",
            a = fmt::normal("You are already on branch"),
            b = fmt::emphasis(branch_name),
            c = fmt::period()
        )
    }

    pub fn info_switched_to_branch_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} {b}{c}",
            a = fmt::normal("Switched to branch ✅"),
            b = fmt::emphasis(branch_name),
            c = fmt::period()
        )
    }

    pub fn error_branch_does_not_exist_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a}{b} {c}{d}",
            a = fmt::error("Branch does not exist"),
            b = fmt::colon(),
            c = fmt::emphasis(branch_name),
            d = fmt::period()
        )
    }

    pub fn error_failed_to_checkout_branch_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a}{b} {c}{d}",
            a = fmt::error("Failed to switch to branch"),
            b = fmt::colon(),
            c = fmt::emphasis(branch_name),
            d = fmt::exclamation()
        )
    }
}

pub mod branch_create_display {
    use super::*;

    pub fn enter_branch_name_you_want_to_create() -> InlineString {
        inline_string!(
            "{a}{b}{c} ",
            a = fmt::prompt_seg_normal("Branch name to create "),
            b = fmt::prompt_seg_bail("(Ctrl+C exits)"),
            c = fmt::colon()
        )
    }

    /// This is the [r3bl_tui::CommandRunResult::Noop] message.
    pub fn info_no_branch_created() -> InlineString {
        fmt::normal("No new branch was created.")
    }

    pub fn info_create_success(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} ✅ {b}.",
            a = fmt::normal("You created and switched to branch"),
            b = fmt::emphasis(branch_name)
        )
    }

    pub fn info_branch_already_exists(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} {b} {c}!",
            a = fmt::normal("Branch"),
            b = fmt::emphasis(branch_name),
            c = fmt::normal("already exists")
        )
    }

    pub fn error_failed_to_create_new_branch(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} {b}!",
            a = fmt::error("Failed to create and switch to"),
            b = fmt::emphasis(branch_name)
        )
    }
}

pub mod branch_delete_display {
    use super::*;

    pub fn info_unable_to_msg() -> InlineString {
        fmt::normal("Branch not found or is currently checked out.")
    }

    pub fn info_chose_not_to_msg() -> InlineString {
        fmt::normal("You chose not to delete any branches.")
    }

    /// Put each deleted branch in a separate line.
    pub fn info_success_msg(branches: &ItemsOwned) -> InlineString {
        debug_assert!(!branches.is_empty());

        let mut acc = InlineString::new();
        for branch_name in branches {
            use std::fmt::Write as _;
            _ = writeln!(
                acc,
                "✅ {a} {b}{c}",
                a = fmt::emphasis(branch_name),
                b = fmt::error("deleted"),
                c = fmt::colon()
            );
        }

        acc
    }

    pub fn error_failed_msg(
        branches: &ItemsOwned,
        maybe_output: Option<Output>,
    ) -> InlineString {
        debug_assert!(!branches.is_empty());

        let prefix = fmt::dim("\n  - ");
        let delim = fmt::dim(",\n  - ");
        let colon = fmt::colon();

        match maybe_output {
            Some(output) => {
                let std_err = &String::from_utf8_lossy(&output.stderr);
                match branches.len() {
                    1 => {
                        let branch_name = &branches[0];
                        inline_string!(
                            "{a}{b} {c}!\n\n{d}",
                            a = fmt::error("Failed to delete branch"),
                            b = colon,
                            c = branch_name,
                            d = std_err
                        )
                    }
                    _ => {
                        // Join the branch names with a specific delimiter, adding a
                        // bullet point before each subsequent branch.
                        let mut joined_branches = InlineString::new();
                        join_fmt!(
                            fmt: joined_branches,
                            from: branches,
                            each: it,
                            // Delimiter includes newline and bullet for subsequent items.
                            delim: delim,
                            format: "{}", fmt::error(it),
                        );

                        // Construct the final error message.
                        inline_string!(
                            "{a}{b} {c}{d}\n{e}",
                            a = fmt::error("Failed to delete branches"),
                            b = colon,
                            c = prefix,
                            d = joined_branches,
                            e = std_err
                        )
                    }
                }
            }
            None => {
                // Join the branch names with a specific delimiter, adding a bullet point before
                // each subsequent branch.
                let mut joined_branches = InlineString::new();
                join_fmt!(
                    fmt: joined_branches,
                    from: branches,
                    each: it,
                    // Delimiter includes newline and bullet for subsequent items.
                    delim: delim,
                    format: "{}", fmt::error(it),
                );

                // Construct the final error message.
                inline_string!(
                    "{a}{b} {c}{d}",
                    a = fmt::error("Failed to run command to delete branches"),
                    b = colon,
                    // Add bullet prefix for the first item in the list.
                    c = prefix,
                    d = joined_branches
                )
            }
        }
    }

    /// This is unformatted text. The formatting is applied by the caller.
    pub fn yes_single_branch_msg_raw() -> &'static str { "Yes, delete branch" }

    /// This is unformatted text. The formatting is applied by the caller.
    pub fn yes_multiple_branches_msg_raw() -> &'static str { "Yes, delete branches" }

    /// This is unformatted text. The formatting is applied by the caller.
    pub fn exit_msg_raw() -> &'static str { "Exit" }

    /// This is unformatted text. The formatting is applied by the caller.
    pub fn select_branches_msg_raw() -> &'static str {
        "Please select branches you want to delete:"
    }

    pub fn confirm_single_branch_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a}{b} {c}",
            a = fmt::normal("Confirm deleting 1 branch"),
            b = fmt::colon(),
            c = fmt::emphasis_delete(branch_name)
        )
    }

    pub fn confirm_multiple_branches_msg(
        num_of_branches: usize,
        branches_to_delete: &ItemsOwned,
    ) -> InlineString {
        debug_assert!(branches_to_delete.len() > 1);

        use std::fmt::Write as _;

        let mut prefixed_branches_joined = InlineString::new();
        for (index, branch) in branches_to_delete.iter().enumerate() {
            _ = writeln!(
                prefixed_branches_joined,
                "{a}{b} {c}",
                a = fmt::dim(inline_string!("{}", index + 1)),
                b = fmt::colon(),
                c = fmt::emphasis_delete(branch)
            );
        }

        let mut acc = InlineString::new();
        _ = write!(
            acc,
            "{a} {b} {c}{d}\n{e}",
            a = fmt::normal("Confirm deleting"),
            b = fmt::emphasis_delete(inline_string!("{}", num_of_branches)),
            c = fmt::normal("branches"),
            d = fmt::colon(),
            e = prefixed_branches_joined
        );
        acc
    }
}

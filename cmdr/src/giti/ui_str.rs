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

use r3bl_tui::{InlineString,
               ItemsOwned,
               fg_frozen_blue,
               fg_lizard_green,
               fg_medium_gray,
               fg_pink,
               fg_silver_metallic,
               fg_slate_gray,
               fg_soft_pink,
               inline_string,
               join_fmt};

pub const CURRENT_BRANCH_PREFIX: &str = "(◕‿◕)";

pub fn unrecoverable_error_message(report: miette::Report) -> InlineString {
    let text = inline_string!(
        "{a}:\n{b}",
        a = "Could not run giti due to the following problem",
        b = report
    );
    fg_pink(&text).to_small_str()
}

pub fn noop_message() -> InlineString {
    fg_silver_metallic("Nothing was selected to run.").to_small_str()
}

pub fn invalid_branch_sub_command_message() -> InlineString {
    fg_silver_metallic(
        "Nothing was selected to run, since the branch sub command is invalid.",
    )
    .to_small_str()
}

pub fn please_select_branch_sub_command() -> &'static str {
    "Please select a branch subcommand:"
}

pub mod modified_files_display {
    use super::*;

    pub fn warn_modified_files_exist_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a}: {b}.",
            a = fg_pink("You have 📝 modified files on the current branch"),
            b = fg_lizard_green(branch_name)
        )
    }
}

pub mod branch_checkout_display {
    use super::*;

    pub fn select_branch_to_switch_to_msg() -> &'static str {
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
        fg_silver_metallic("No suitable branch is available for checkout.").to_small_str()
    }

    pub fn info_already_on_current_branch_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} {b}.",
            a = fg_silver_metallic("You are already on branch"),
            b = fg_lizard_green(branch_name)
        )
    }

    pub fn info_switched_to_branch_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} {b}.",
            a = fg_silver_metallic("Switched to branch ✅"),
            b = fg_lizard_green(branch_name)
        )
    }

    pub fn error_branch_does_not_exist_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a}: {b}.",
            a = fg_pink("Branch does not exist"),
            b = fg_lizard_green(branch_name)
        )
    }

    pub fn error_failed_to_checkout_branch_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a}: {b}!",
            a = fg_pink("Failed to switch to branch"),
            b = fg_lizard_green(branch_name)
        )
    }
}

pub mod branch_create_display {
    use super::*;

    pub fn enter_branch_name_you_want_to_create() -> InlineString {
        inline_string!(
            "{a}{b}{c}",
            a = fg_frozen_blue("Branch name to create "),
            b = fg_soft_pink("(Ctrl+C exits)").italic().bg_moonlight_blue(),
            c = fg_frozen_blue(": ")
        )
    }

    /// This is the [r3bl_tui::CommandRunResult::Noop] message.
    pub fn info_no_branch_created() -> InlineString {
        fg_silver_metallic("No new branch was created.").to_small_str()
    }

    pub fn info_create_success(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} ✅ {b}.",
            a = fg_silver_metallic("You created and switched to branch"),
            b = fg_lizard_green(branch_name)
        )
    }

    pub fn info_branch_already_exists(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} {b} {c}!",
            a = fg_silver_metallic("Branch"),
            b = fg_lizard_green(branch_name),
            c = fg_silver_metallic("already exists")
        )
    }

    pub fn error_failed_to_create_new_branch(branch_name: &str) -> InlineString {
        inline_string!(
            "{a} {b}!",
            a = fg_pink("Failed to create and switch to"),
            b = fg_lizard_green(branch_name)
        )
    }
}

pub mod branch_delete_display {
    use super::*;

    pub fn info_unable_to_msg() -> InlineString {
        fg_silver_metallic("Branch not found or is currently checked out.").to_small_str()
    }

    pub fn info_chose_not_to_msg() -> InlineString {
        fg_silver_metallic("You chose not to delete any branches.").to_small_str()
    }

    /// Put each deleted branch in a separate line.
    pub fn info_success_msg(branches: &ItemsOwned) -> InlineString {
        debug_assert!(!branches.is_empty());

        let mut acc = InlineString::new();
        for branch in branches {
            use std::fmt::Write as _;
            _ = writeln!(
                acc,
                "✅ {a} {b}{c}",
                a = fg_lizard_green(branch),
                b = fg_soft_pink("deleted"),
                c = fg_slate_gray(".")
            );
        }

        acc
    }

    pub fn error_failed_msg(
        branches: &ItemsOwned,
        maybe_output: Option<Output>,
    ) -> InlineString {
        debug_assert!(!branches.is_empty());

        let prefix = fg_slate_gray("\n  - ");
        let delim = fg_slate_gray(",\n  - ");
        let colon = fg_slate_gray(": ");

        match maybe_output {
            Some(output) => {
                let std_err = &String::from_utf8_lossy(&output.stderr);
                match branches.len() {
                    1 => {
                        let branch_name = &branches[0];
                        inline_string!(
                            "{a}{b}{c}!\n\n{d}",
                            a = fg_pink("Failed to delete branch"),
                            b = colon,
                            c = branch_name,
                            d = std_err
                        )
                    }
                    _ => {
                        // Join the branch names with a specific delimiter, adding a bullet point before
                        // each subsequent branch.
                        let mut joined_branches = InlineString::new();
                        join_fmt!(
                            fmt: joined_branches,
                            from: branches,
                            each: it,
                            // Delimiter includes newline and bullet for subsequent items.
                            delim: delim,
                            format: "{}", fg_soft_pink(it),
                        );

                        // Construct the final error message.
                        inline_string!(
                            "{a}{b}{c}{d}\n{e}",
                            a = fg_pink("Failed to delete branches"),
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
                    format: "{}", fg_soft_pink(it),
                );

                // Construct the final error message.
                inline_string!(
                    "{a}{b}{c}{d}",
                    a = fg_pink("Failed to run command to delete branches"),
                    // Add bullet prefix for the first item in the list.
                    c = prefix,
                    d = joined_branches,
                    b = colon
                )
            }
        }
    }

    pub fn yes_single_branch_msg() -> &'static str { "Yes, delete branch" }

    pub fn yes_multiple_branches_msg() -> &'static str { "Yes, delete branches" }

    pub fn exit_msg() -> &'static str { "Exit" }

    pub fn select_branches_msg() -> &'static str {
        "Please select branches you want to delete:"
    }

    pub fn confirm_single_branch_msg(branch_name: &str) -> InlineString {
        inline_string!(
            "{a}: {b}",
            a = fg_silver_metallic("Confirm deleting 1 branch"),
            b = fg_soft_pink(branch_name)
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
                "{a}. {b}",
                a = fg_medium_gray(inline_string!("{}", index + 1)),
                b = fg_soft_pink(branch)
            );
        }

        let mut acc = InlineString::new();
        _ = write!(
            acc,
            "{a} {b} {c}:\n{d}",
            a = fg_silver_metallic("Confirm deleting"),
            b = fg_soft_pink(inline_string!("{}", num_of_branches)),
            c = fg_silver_metallic("branches"),
            d = prefixed_branches_joined
        );
        acc
    }
}

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

use r3bl_tui::{ColorWheel,
               GradientGenerationPolicy,
               InlineString,
               ItemsOwned,
               TextColorizationPolicy,
               fg_frozen_blue,
               fg_lizard_green,
               fg_pink,
               fg_silver_metallic,
               inline_string};

pub const CURRENT_PREFIX: &str = "(current) ";

pub fn unrecoverable_error_message(report: miette::Report) -> String {
    let text = format!(" Could not run giti due to the following problem.\n{report}");
    fg_pink(&text).to_string()
}

pub fn noop_message() -> String { fg_silver_metallic(" No command was run.").to_string() }

pub fn please_select_branch_sub_command() -> &'static str {
    "Please select a branch subcommand"
}

pub fn upgrade_required_message() -> InlineString {
    let plain_text_exit_msg = inline_string!(
        "{}\n{}",
        "💿 A new version of giti is available.",
        "Run `cargo install r3bl-cmdr` to upgrade 🙌."
    );

    ColorWheel::default().colorize_into_string(
        &plain_text_exit_msg,
        GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
        TextColorizationPolicy::ColorEachCharacter(None),
        None,
    )
}

pub fn goodbye_thanks_for_using_giti() -> InlineString {
    let goodbye = match std::env::var("USER") {
        Ok(username) => {
            inline_string!("\n Goodbye, 👋 {username}. Thanks for using 😺 giti!")
        }
        Err(_) => inline_string!("\n Goodbye 👋.\n\n 😺 giti!"),
    };

    let please_star_us = inline_string!(
        " Please report issues & star us on GitHub: 🌟 🐞 \
        https://github.com/r3bl-org/r3bl-open-core/issues/new/choose"
    );

    let combined = inline_string!("{goodbye}\n{please_star_us}");

    ColorWheel::lolcat_into_string(&combined, None)
}

pub mod modified_files_display {
    use super::*;

    pub fn text_modified_file_on_current_branch_exists() -> &'static str {
        " You have a 📝 modified file on the current branch: "
    }

    pub fn text_modified_files_on_current_branch_exist() -> &'static str {
        " You have 📝 modified files on the current branch: "
    }

    pub fn warn_modified_files_on_current_branch_exist(branch_name: &str) -> String {
        format!(
            "{a}{b}",
            a = fg_pink(text_modified_files_on_current_branch_exist()),
            b = fg_lizard_green(branch_name)
        )
    }
}

pub mod branch_checkout_display {
    use super::*;

    pub fn select_branch_to_switch_to() -> &'static str {
        " Select a branch to switch to"
    }

    pub fn info_checkout_success(branch_name: &str, current_branch: &str) -> String {
        if branch_name == current_branch {
            info_already_on_current_branch(branch_name)
        } else {
            info_switched_to_branch(branch_name)
        }
    }

    pub fn info_no_suitable_branch_is_available_for_checkout() -> String {
        fg_silver_metallic(" No suitable branch is available for checkout.").to_string()
    }

    pub fn info_already_on_current_branch(branch_name: &str) -> String {
        format!(
            "{a}{b}",
            a = fg_silver_metallic(" You are already on branch "),
            b = fg_lizard_green(branch_name)
        )
    }

    pub fn info_switched_to_branch(branch_name: &str) -> String {
        format!(
            "{a}{b}",
            a = fg_silver_metallic(" Switched to branch ✅ "),
            b = fg_lizard_green(branch_name)
        )
    }

    pub fn error_branch_does_not_exist(branch_name: &str) -> String {
        let text = format!("Branch `{branch_name}` does not exist.");
        fg_pink(&text).to_string()
    }

    pub fn error_failed_to_checkout_branch(branch_name: &str) -> String {
        let text = format!(" Failed to switch to branch '{branch_name}'!");
        fg_pink(&text).to_string()
    }
}

pub mod branch_create_display {
    use super::*;

    pub fn enter_branch_name_you_want_to_create() -> String {
        fg_frozen_blue(" Enter a branch name you want to create (Ctrl+C to exit): ")
            .to_string()
    }

    /// This is the [r3bl_tui::CommandRunResult::DidNotRun] message.
    pub fn info_no_branch_created() -> String {
        fg_silver_metallic(" No new branch was created").to_string()
    }

    pub fn info_create_success(branch_name: &str) -> String {
        format!(
            "{a}{b}",
            a = fg_silver_metallic(" You created and switched to branch "),
            b = fg_lizard_green(&format!("✅ {branch_name}"))
        )
    }

    pub fn info_branch_already_exists(branch_name: &str) -> String {
        let text = format!(" Branch {branch_name} already exists!");
        fg_silver_metallic(&text).to_string()
    }

    pub fn error_failed_to_create_new_branch(branch_name: &str) -> String {
        let text = format!(" Failed to create and switch to new branch {branch_name}!");
        fg_pink(&text).to_string()
    }
}

pub mod branch_delete_display {
    use super::*;

    pub fn info_no_branches_deleted() -> String {
        fg_silver_metallic(" You chose not to delete any branches.").to_string()
    }

    pub fn info_delete_success(branches: &ItemsOwned) -> String {
        debug_assert!(!branches.is_empty());

        if branches.len() == 1 {
            let branch_name = &branches[0].to_string();
            format!(
                " ✅ {a} {b}",
                a = fg_lizard_green(branch_name),
                b = fg_silver_metallic("deleted"),
            )
        } else {
            branches
                .iter()
                .map(|branch| {
                    format!(
                        " ✅ {a} {b}",
                        a = fg_lizard_green(branch),
                        b = fg_silver_metallic("deleted"),
                    )
                })
                .collect::<String>()
        }
    }

    pub fn error_failed_to_delete(
        branches: &ItemsOwned,
        maybe_output: Option<Output>,
    ) -> String {
        debug_assert!(!branches.is_empty());

        match maybe_output {
            Some(output) => {
                let std_err = &String::from_utf8_lossy(&output.stderr);
                let text = match branches.len() {
                    1 => {
                        let branch_name = &branches[0];
                        format!(" Failed to delete branch: {branch_name}!\n\n{std_err}")
                    }
                    _ => {
                        let branches = branches.join(",\n ╴");
                        format!(" Failed to delete branches:\n ╴{branches}!\n\n{std_err}")
                    }
                };
                fg_pink(&text).to_string()
            }
            None => {
                let branches = branches.join(",\n ╴");
                let text =
                    format!(" Failed to run command to delete branches:\n ╴{branches}!");
                fg_pink(&text).to_string()
            }
        }
    }

    pub fn yes_delete_branch_message() -> &'static str { "Yes, delete branch" }

    pub fn yes_delete_branches_message() -> &'static str { "Yes, delete branches" }

    pub fn exit_message() -> &'static str { "Exit" }

    pub fn confirm_delete_one_branch(branch_name: &str) -> InlineString {
        inline_string!(" Confirm deleting 1 branch: {branch_name}")
    }

    pub fn please_select_branches_you_want_to_delete() -> &'static str {
        " Please select branches you want to delete"
    }

    pub fn confirm_deleting_multiple_branches(
        num_of_branches: usize,
        branches_to_delete: &ItemsOwned,
    ) -> InlineString {
        let prefixed_branches: Vec<String> = branches_to_delete
            .into_iter()
            .enumerate()
            .map(|(index, branch)| format!(" {}. {}", index + 1, branch))
            .collect();

        let mut acc = InlineString::new();

        use std::fmt::Write as _;
        _ = write!(
            acc,
            " Confirm deleting {a} branches:\n{b}",
            a = num_of_branches,
            b = prefixed_branches.join("\n")
        );

        acc
    }
}

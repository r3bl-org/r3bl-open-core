/*
 *   Copyright (c) 2025 R3BL LLC
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

use std::{env::var, io::Error, process::ExitStatus};

use r3bl_tui::{ColorWheel,
               GradientGenerationPolicy,
               InlineString,
               TextColorizationPolicy,
               glyphs,
               inline_string};

use super::upgrade_check::{get_self_bin_name, get_self_crate_name};
use crate::get_self_bin_emoji;

pub mod upgrade_install {
    use super::*;

    /// Ran `cargo install ...` and this process exited with zero exit code.
    pub fn install_success_msg() -> InlineString {
        inline_string!("\n✅ Updated {} successfully.", get_self_crate_name())
    }

    /// Ran `cargo install ...` but this process exited with non-zero exit code.
    pub fn install_not_success_msg(status: ExitStatus) -> InlineString {
        inline_string!(
            "\n❌ Failed to update {} (exit code {:?}).",
            get_self_crate_name(),
            status.code()
        )
    }

    /// Could not run `cargo install $crate_name` itself.
    pub fn install_failed_to_run_command_msg(err: Error) -> InlineString {
        inline_string!(
            "\n❌ Failed to run `cargo install {}`: {}",
            get_self_crate_name(),
            err
        )
    }

    pub fn tokio_blocking_task_failed_msg(err_str: impl AsRef<str>) -> String {
        format!(
            "Blocking task for installation failed: {}",
            err_str.as_ref()
        )
    }
}

pub mod upgrade_spinner {
    use super::*;

    pub fn stop_msg() -> &'static str { "Finished installation!" }

    pub fn indeterminate_progress_msg() -> String {
        format!("Installing {}... ", get_self_crate_name())
    }

    pub fn readline_async_exit_msg() -> InlineString {
        inline_string!("{} is installed 🎉.", get_self_crate_name())
    }
}

pub mod upgrade_check {
    use super::*;

    pub fn yes_msg() -> &'static str { "Yes, upgrade now" }

    pub fn no_msg() -> &'static str { "No, thanks" }

    pub fn ask_user_msg() -> InlineString {
        inline_string!("Would you like to upgrade {} now?", get_self_crate_name())
    }

    pub fn upgrade_is_required_msg() -> InlineString {
        let plain_text_exit_msg = inline_string!(
            "\n{a}\n{b}\n",
            a = inline_string!(
                "🎁 A new version of {} is available.",
                get_self_bin_name()
            ),
            b = inline_string!(
                "{} You can run `cargo install {}` to upgrade.",
                glyphs::PROMPT,
                get_self_crate_name()
            )
        );

        ColorWheel::default().colorize_into_string(
            &plain_text_exit_msg,
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
            TextColorizationPolicy::ColorEachCharacter(None),
            None,
        )
    }
}

pub mod goodbye_greetings {
    use super::*;

    pub fn thanks_msg() -> InlineString {
        let goodbye_msg = match var("USER") {
            Ok(username) => inline_string!(
                "Goodbye, 👋 {a}. Thanks for using {b} {c}!",
                a = username,
                b = get_self_bin_emoji(),
                c = get_self_bin_name()
            ),
            Err(_) => inline_string!(
                "Goodbye, 👋. Thanks for using {a} {b}!",
                a = get_self_bin_emoji(),
                b = get_self_bin_name()
            ),
        };

        let star_us_msg = inline_string!(
            "{a}\n{b}",
            a = "Please report issues & star us on GitHub: 🌟 🐞",
            b = "https://github.com/r3bl-org/r3bl-open-core/issues/new/choose"
        );

        let combined = inline_string!("\n{goodbye_msg}\n{star_us_msg}");

        ColorWheel::lolcat_into_string(&combined, None)
    }
}

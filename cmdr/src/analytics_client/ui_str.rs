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

use std::{env::var, fmt::Display, io::Error, process::ExitStatus};

use r3bl_tui::{ColorWheel,
               GradientGenerationPolicy,
               InlineString,
               TextColorizationPolicy,
               glyphs,
               inline_string};

use super::upgrade_check::{get_self_bin_name, get_self_crate_name};
use crate::{common::fmt, get_self_bin_emoji};

pub mod upgrade_install {
    use super::*;

    /// Ran `cargo install ...` and this process exited with zero request_shutdown code.
    pub fn install_success_msg() -> InlineString {
        inline_string!(
            "✅ {a} {b} {c}{d}\n",
            a = fmt::normal("Upgraded"),
            b = fmt::emphasis(get_self_crate_name()),
            c = fmt::normal("successfully"),
            d = fmt::period()
        )
    }

    /// Ran `cargo install ...` but this process exited with non-zero request_shutdown
    /// code.
    pub fn install_not_success_msg(status: ExitStatus) -> InlineString {
        inline_string!(
            "❌ {a} {b} {c}{d}\n",
            a = fmt::error("Failed to upgrade"),
            b = fmt::emphasis(get_self_crate_name()),
            c = fmt::emphasis_delete(inline_string!(
                "(request_shutdown code {:?})",
                status.code()
            )),
            d = fmt::period()
        )
    }

    /// Could not run `cargo install $crate_name` itself.
    pub fn install_failed_to_run_command_msg(err: Error) -> InlineString {
        inline_string!(
            "❌ {a}{b} {c}{e} {d}{b}\n{f}",
            a = fmt::error("Failed to run"),
            b = fmt::colon(),
            c = fmt::emphasis(inline_string!("cargo install {}", get_self_crate_name())),
            d = fmt::normal("due to"),
            e = fmt::comma(),
            f = fmt::normal(err),
        )
    }

    pub fn tokio_blocking_task_failed_msg(err_str_arg: impl Display) -> InlineString {
        let err_str = inline_string!("{}", err_str_arg);
        inline_string!(
            "{a}{b}\n{c}",
            a = fmt::error("Blocking task for installation failed"),
            b = fmt::colon(),
            c = fmt::error(err_str)
        )
    }

    pub fn fail_send_sigint_msg(err: Error) -> InlineString {
        inline_string!(
            "{a}{b}\n{c}",
            a = fmt::error("Failed to send kill signal to install process"),
            b = fmt::colon(),
            c = fmt::error(err)
        )
    }

    pub fn send_sigint_msg() -> InlineString {
        inline_string!(
            "{a}{c}",
            a = fmt::error("Kill signal sent to install process"),
            c = fmt::period(),
        )
    }

    pub fn stop_msg() -> InlineString {
        inline_string!("{a}", a = fmt::dim("Installation ended."))
    }

    /// No formatting on this string, since the spinner will apply its own animated lolcat
    /// formatting.
    pub fn indeterminate_progress_msg_raw() -> String {
        format!("Installing {a}... ", a = get_self_crate_name())
    }

    pub fn readline_async_exit_msg() -> InlineString {
        inline_string!(
            "{a} {b} {c}{d} 🎉",
            a = fmt::normal("Crate"),
            b = fmt::emphasis(get_self_crate_name()),
            c = fmt::normal("is installed"),
            d = fmt::period()
        )
    }
}

pub mod upgrade_check {
    use super::*;

    pub fn yes_msg_raw() -> &'static str { "Yes, upgrade now" }

    pub fn no_msg_raw() -> &'static str { "No, thanks" }

    /// No formatting on this string, as the formatting is applied by the caller.
    pub fn ask_user_msg_raw() -> InlineString {
        inline_string!(
            "Would you like to upgrade {a} now?",
            a = get_self_crate_name()
        )
    }

    pub fn upgrade_is_required_msg() -> InlineString {
        let plain_text_exit_msg = inline_string!(
            "\n{a}\n{b}\n",
            a = inline_string!("A new version of {} is available.", get_self_bin_name()),
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
            a = "Please report issues & star us on GitHub:",
            b = "https://github.com/r3bl-org/r3bl-open-core/issues/new/choose"
        );

        let combined = inline_string!("{goodbye_msg}\n{star_us_msg}");

        ColorWheel::lolcat_into_string(&combined, None)
    }
}

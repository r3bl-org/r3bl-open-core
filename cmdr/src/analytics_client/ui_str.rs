// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::upgrade_check::{get_self_bin_name, get_self_crate_name};
use crate::{common::fmt, get_self_bin_emoji};
use r3bl_tui::{ColorWheel, GradientGenerationPolicy, InlineString,
               TextColorizationPolicy, glyphs, inline_string};
use std::{env::var, fmt::Display, io::Error, process::ExitStatus};

pub mod upgrade_install {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Ran `cargo install ...` and this process exited with zero `request_shutdown` code.
    #[must_use]
    pub fn install_success_msg() -> InlineString {
        inline_string!(
            "✅ {a} {b} {c}{d}\n",
            a = fmt::normal("Upgraded"),
            b = fmt::emphasis(get_self_crate_name()),
            c = fmt::normal("successfully"),
            d = fmt::period()
        )
    }

    /// Ran `cargo install ...` but this process exited with non-zero `request_shutdown`
    /// code.
    #[must_use]
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
    #[must_use]
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

    #[must_use]
    pub fn fail_send_sigint_msg(err: Error) -> InlineString {
        inline_string!(
            "{a}{b}\n{c}",
            a = fmt::error("Failed to send kill signal to install process"),
            b = fmt::colon(),
            c = fmt::error(err)
        )
    }

    #[must_use]
    pub fn send_sigint_msg() -> InlineString {
        inline_string!(
            "{a}{c}",
            a = fmt::error("Kill signal sent to install process"),
            c = fmt::period(),
        )
    }

    #[must_use]
    pub fn stop_msg() -> InlineString {
        inline_string!("{a}", a = fmt::dim("Installation ended."))
    }

    /// No formatting on this string, since the spinner will apply its own animated lolcat
    /// formatting.
    #[must_use]
    pub fn indeterminate_progress_msg_raw() -> String {
        format!("Installing {a}... ", a = get_self_crate_name())
    }

    #[must_use]
    pub fn readline_async_exit_msg() -> InlineString {
        inline_string!(
            "{a} {b} {c}{d} 🎉",
            a = fmt::normal("Crate"),
            b = fmt::emphasis(get_self_crate_name()),
            c = fmt::normal("is installed"),
            d = fmt::period()
        )
    }

    /// No formatting on this string, since the spinner will apply its own animated lolcat
    /// formatting.
    #[must_use]
    pub fn rustup_update_msg_raw() -> String { "Updating Rust toolchain...".to_string() }

    /// No formatting on this string, since the spinner will apply its own animated lolcat
    /// formatting.
    #[must_use]
    pub fn install_with_progress_msg_raw(crate_name: &str, percentage: u8) -> String {
        format!("Installing {crate_name}... {percentage}%")
    }

    /// No formatting on this string, since the spinner will apply its own animated lolcat
    /// formatting.
    #[must_use]
    pub fn install_building_msg_raw(crate_name: &str) -> String {
        format!("Installing {crate_name}... (building)")
    }
}

pub mod upgrade_check {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[must_use]
    pub fn yes_msg_raw() -> &'static str { "Yes, upgrade now" }

    #[must_use]
    pub fn no_msg_raw() -> &'static str { "No, thanks" }

    /// No formatting on this string, as the formatting is applied by the caller.
    #[must_use]
    pub fn ask_user_msg_raw() -> InlineString {
        inline_string!(
            "Would you like to upgrade {a} now?",
            a = get_self_crate_name()
        )
    }

    #[must_use]
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

        InlineString::from(ColorWheel::default().colorize_into_string(
            &plain_text_exit_msg,
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
            TextColorizationPolicy::ColorEachCharacter(None),
            None,
        ))
    }
}

pub mod goodbye_greetings {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[must_use]
    pub fn thanks_msg_simple() -> InlineString {
        let goodbye_msg = common_msg();

        InlineString::from(ColorWheel::lolcat_into_string(&goodbye_msg, None))
    }

    #[must_use]
    pub fn thanks_msg_with_github() -> InlineString {
        let goodbye_msg = common_msg();

        let star_us_msg = inline_string!(
            "{a}\n{b}",
            a = "Please report issues & star us on GitHub:",
            b = "https://github.com/r3bl-org/r3bl-open-core/issues/new/choose"
        );

        let combined = inline_string!("{goodbye_msg}\n{star_us_msg}");

        InlineString::from(ColorWheel::lolcat_into_string(&combined, None))
    }

    /// Helper function to generate the goodbye message with optional username.
    #[must_use]
    fn common_msg() -> InlineString {
        if let Ok(username) = var("USER") {
            inline_string!(
                "Goodbye, 👋 {a}. Thanks for using {b} {c}!",
                a = username,
                b = get_self_bin_emoji(),
                c = get_self_bin_name()
            )
        } else {
            inline_string!(
                "Goodbye, 👋. Thanks for using {a} {b}!",
                a = get_self_bin_emoji(),
                b = get_self_bin_name()
            )
        }
    }
}

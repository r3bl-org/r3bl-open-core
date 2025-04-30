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

use r3bl_tui::{ColorWheel,
               GradientGenerationPolicy,
               InlineString,
               TextColorizationPolicy,
               glyphs,
               inline_string};

pub fn upgrade_required_message() -> InlineString {
    let bin_name = super::upgrade_check::get_self_bin_name();
    let crate_name = super::upgrade_check::get_self_crate_name();

    let plain_text_exit_msg = inline_string!(
        "\n{}\n{}",
        inline_string!(" 🎁 A new version of {} is available.", bin_name),
        inline_string!(
            " {} You can run `cargo install {}` to upgrade.",
            glyphs::PROMPT,
            crate_name
        )
    );

    ColorWheel::default().colorize_into_string(
        &plain_text_exit_msg,
        GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
        TextColorizationPolicy::ColorEachCharacter(None),
        None,
    )
}

pub fn goodbye_thanks_for_using_message() -> InlineString {
    let bin_name = super::upgrade_check::get_self_bin_name();

    let goodbye = match std::env::var("USER") {
        Ok(username) => super::ui_str::goodbye_message_with_username(username, bin_name),
        Err(_) => super::ui_str::goodbye_message_no_username(bin_name),
    };

    let please_star_us = super::ui_str::please_file_issues_or_star_us_on_github();

    let combined = inline_string!("{goodbye}\n{please_star_us}");

    ColorWheel::lolcat_into_string(&combined, None)
}

pub fn upgrade_available_message(crate_name: &str) -> InlineString {
    inline_string!("Would you like to upgrade {} now?", crate_name)
}

pub fn upgrade_yes() -> &'static str { " Yes, upgrade now" }

pub fn upgrade_no() -> &'static str { " No, thanks" }

pub fn please_file_issues_or_star_us_on_github() -> InlineString {
    inline_string!(
        " Please report issues & star us on GitHub: 🌟 🐞 \
            \n https://github.com/r3bl-org/r3bl-open-core/issues/new/choose"
    )
}

pub fn goodbye_message_no_username(bin_name: impl AsRef<str>) -> InlineString {
    inline_string!("\n Goodbye �.\n\n 😺 {}!", bin_name.as_ref())
}

pub fn goodbye_message_with_username(
    username: impl AsRef<str>,
    bin_name: impl AsRef<str>,
) -> InlineString {
    inline_string!(
        "\n Goodbye, 👋 {}. Thanks for using 😺 {}!",
        username.as_ref(),
        bin_name.as_ref()
    )
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Interactive exit prompt asking the user whether to perform an upgrade.

use super::ui_strings;
use crate::{DefaultIoDevices, HowToChoose, StyleSheet, TerminalInteractiveStatus,
            TuiAvailabilityChooseExt, check_is_terminal_interactive, choose,
            cli_text_inline, cli_text_line, new_style, vp_height};
use smallvec::smallvec;

/// Interactively prompts the user via a modal single-choice dialog asking whether to
/// upgrade now. Returns `true` if the user selects `"Yes, upgrade now"`.
///
/// Returns `false` immediately if the terminal is non-interactive (e.g. running in a CI
/// pipeline or headless script).
pub async fn prompt_user_to_upgrade(crate_name: &str, remote_version_str: &str) -> bool {
    if let TerminalInteractiveStatus::NotAvailable(_) = check_is_terminal_interactive() {
        return false;
    }

    let yes_no_options = &[
        ui_strings::yes_upgrade_now_raw(),
        ui_strings::no_thanks_raw(),
    ];
    let prompt_header =
        ui_strings::format_interactive_prompt_header(crate_name, remote_version_str);
    let header_lines = smallvec![cli_text_line![cli_text_inline(
        prompt_header,
        new_style!(bold)
    )]];
    let mut io = DefaultIoDevices::default();

    let maybe_user_choice = choose(
        header_lines,
        yes_no_options,
        Some(vp_height(yes_no_options.len())),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        io.as_mut_tuple(),
    )
    .get_first_result()
    .await
    .ok()
    .flatten();

    maybe_user_choice.is_some_and(|user_choice| user_choice == yes_no_options[0])
}

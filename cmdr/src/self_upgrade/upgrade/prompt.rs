// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::ui_strings;
use r3bl_tui::{DefaultIoDevices, HowToChoose, StyleSheet, TerminalInteractiveStatus,
            TuiAvailabilityChooseExt, check_is_terminal_interactive, choose,
            cli_text_inline, cli_text_line, new_style, vp_height};
use smallvec::smallvec;

/// Decision returned by [`prompt_user_to_upgrade`] when asking whether to install an
/// available upgrade at application exit.
///
/// Replaces a raw `bool` return to eliminate boolean blindness and distinguish explicit
/// user rejection ([`Self::DeclinedByUser`]) from non-interactive sessions
/// ([`Self::SkippedNonInteractive`]) where a modal [`choose`] dialog cannot be rendered.
///
/// [`choose`]: r3bl_tui::choose
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptUpgradeDecision {
    /// User explicitly selected `"Yes, upgrade now"`.
    UpgradeNow,

    /// User selected `"No, thanks"` or dismissed/canceled the dialog in an interactive
    /// terminal.
    DeclinedByUser,

    /// Terminal is non-interactive (for example, piped streams or a headless `CI`
    /// pipeline), so the modal prompt was skipped immediately without blocking.
    SkippedNonInteractive,
}

impl PromptUpgradeDecision {
    /// Maps the optional selection returned by [`choose`] into a typed
    /// [`PromptUpgradeDecision`].
    ///
    /// [`choose`]: r3bl_tui::choose
    #[must_use]
    pub fn from_user_choice(maybe_choice: Option<&str>) -> Self {
        match maybe_choice {
            Some(choice) if choice == ui_strings::yes_upgrade_now_raw() => {
                Self::UpgradeNow
            }
            Some(_) | None => Self::DeclinedByUser,
        }
    }
}

/// Interactively prompts the user via a modal single-choice dialog asking whether to
/// upgrade now.
///
/// Returns [`PromptUpgradeDecision::SkippedNonInteractive`] immediately if the terminal
/// is non-interactive (e.g., running in a `CI` pipeline or headless script).
#[must_use]
pub async fn prompt_user_to_upgrade(
    crate_name: &str,
    remote_version_str: &str,
) -> PromptUpgradeDecision {
    if let TerminalInteractiveStatus::NotAvailable(_) = check_is_terminal_interactive() {
        return PromptUpgradeDecision::SkippedNonInteractive;
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

    PromptUpgradeDecision::from_user_choice(maybe_user_choice.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_prompt_upgrade_decision_from_user_choice() {
        assert_eq2!(
            PromptUpgradeDecision::from_user_choice(Some(
                ui_strings::yes_upgrade_now_raw()
            )),
            PromptUpgradeDecision::UpgradeNow
        );
        assert_eq2!(
            PromptUpgradeDecision::from_user_choice(Some(ui_strings::no_thanks_raw())),
            PromptUpgradeDecision::DeclinedByUser
        );
        assert_eq2!(
            PromptUpgradeDecision::from_user_choice(Some("other")),
            PromptUpgradeDecision::DeclinedByUser
        );
        assert_eq2!(
            PromptUpgradeDecision::from_user_choice(None),
            PromptUpgradeDecision::DeclinedByUser
        );
    }
}

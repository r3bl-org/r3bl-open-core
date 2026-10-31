// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{UpgradeError, ui_strings,
            super::{CrateDistribution, UpgradeConfig, UserFeedbackChannel}};
use r3bl_tui::{OutputDevice, PaintMode, StdMutex};
use std::sync::Arc;

/// Constructs an [`OutputDevice`] matching the requested [`UserFeedbackChannel`].
#[must_use]
pub fn create_output_device_for_feedback_channel(
    channel: UserFeedbackChannel,
) -> OutputDevice {
    match channel {
        UserFeedbackChannel::Stdout => OutputDevice::new_stdout(),
        UserFeedbackChannel::Stderr => OutputDevice {
            resource: Arc::new(StdMutex::new(std::io::stderr())),
            paint_mode: PaintMode::Real,
        },
        UserFeedbackChannel::Quiet => OutputDevice {
            resource: Arc::new(StdMutex::new(std::io::sink())),
            paint_mode: PaintMode::Mock,
        },
    }
}

/// Formats and logs errors encountered during [`UpgradeSource`] resolution.
///
/// [`UpgradeSource`]: crate::self_upgrade::UpgradeSource
pub fn report_upgrade_source_error(
    config: &UpgradeConfig,
    distribution: CrateDistribution,
    bin_name: &str,
    err: &UpgradeError,
) {
    match err {
        UpgradeError::NoViableUpgradeSource { .. } => {
            if distribution == CrateDistribution::ClosedSource {
                config.get_user_feedback_channel().eprintln(
                    ui_strings::format_closed_source_unreachable_error(bin_name),
                );
            } else {
                config.get_user_feedback_channel().eprintln(
                    ui_strings::format_open_source_no_toolchain_error(bin_name),
                );
            }
        }
        other => {
            config.get_user_feedback_channel().eprintln(
                ui_strings::format_upgrade_failed_retry_msg(bin_name, &other.to_string()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::super::{CooldownPolicy, CrateIdentity, CrateName, CrateVersion,
                               UpgradePolicy},
                *};
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_create_output_device_for_feedback_channel() {
        let stdout_dev =
            create_output_device_for_feedback_channel(UserFeedbackChannel::Stdout);
        assert_eq2!(stdout_dev.paint_mode, PaintMode::Real);

        let stderr_dev =
            create_output_device_for_feedback_channel(UserFeedbackChannel::Stderr);
        assert_eq2!(stderr_dev.paint_mode, PaintMode::Real);

        let quiet_dev =
            create_output_device_for_feedback_channel(UserFeedbackChannel::Quiet);
        assert_eq2!(quiet_dev.paint_mode, PaintMode::Mock);
    }

    #[test]
    fn test_report_upgrade_source_error_branches() {
        let version = CrateVersion::try_from("0.0.25").unwrap();
        let config = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", version),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::default(),
            None,
            None,
            CooldownPolicy::Disabled,
        );

        let no_source_err = UpgradeError::NoViableUpgradeSource {
            crate_name: CrateName::from("r3bl-cmdr"),
            crate_distribution: CrateDistribution::OpenSource,
            reason: "test reason".to_string(),
        };

        // Exercises ClosedSource, OpenSource, and Other branches without panicking.
        report_upgrade_source_error(
            &config,
            CrateDistribution::ClosedSource,
            "giti",
            &no_source_err,
        );
        report_upgrade_source_error(
            &config,
            CrateDistribution::OpenSource,
            "giti",
            &no_source_err,
        );
        report_upgrade_source_error(
            &config,
            CrateDistribution::OpenSource,
            "giti",
            &UpgradeError::Cancelled,
        );
    }
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Parameterized user-facing messages for the binary self-upgrade engine.

use crate::Pc;

pub mod rustup_progress {
    /// Prefix stripped from `rustup` [`stdout`] lines to extract progress information.
    ///
    /// [`stdout`]: std::io::stdout
    pub const INFO_PREFIX: &str = "info: ";

    /// Suffix appended to truncated progress output.
    pub const ELLIPSIS: &str = "...";

    /// Maximum character length of progress message shown in spinner display.
    pub const MAX_DISPLAY_LEN: usize = 50;

    /// Truncation slice length before appending [`ELLIPSIS`].
    pub const TRUNCATE_LEN: usize = MAX_DISPLAY_LEN - ELLIPSIS.len();
}

/// Interactive prompt option for confirming an upgrade.
#[must_use]
pub const fn yes_upgrade_now_raw() -> &'static str { "Yes, upgrade now" }

/// Interactive prompt option for declining an upgrade.
#[must_use]
pub const fn no_thanks_raw() -> &'static str { "No, thanks" }

/// Prompt header asking the user whether to upgrade `crate_name`.
#[must_use]
pub fn format_interactive_prompt_header(crate_name: &str, version: &str) -> String {
    format!("Would you like to upgrade {crate_name} to v{version} now?")
}

/// Notification message when a newer version is available.
#[must_use]
pub fn format_upgrade_available_notice(
    bin_name: &str,
    local_version: &str,
    remote_version: &str,
) -> String {
    format!(
        "A new version of {bin_name} is available (v{local_version} -> v{remote_version}). \
         Run `{bin_name} --upgrade` to upgrade."
    )
}

/// Message shown when binary is already up to date during standalone `--upgrade`.
#[must_use]
pub fn format_already_up_to_date_msg(crate_name: &str, local_version: &str) -> String {
    format!("✅ {crate_name} (v{local_version}) is already up to date.")
}

/// Initial spinner message when downloading a pre-compiled binary release archive.
#[must_use]
pub fn format_download_initial_msg(crate_name: &str, version: &str) -> String {
    format!("Downloading {crate_name} v{version}...")
}

/// Spinner progress message showing downloaded percentage and megabytes.
#[must_use]
pub fn format_download_progress_msg(
    crate_name: &str,
    version: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) -> String {
    let downloaded_mb = bytes_to_megabytes_str(downloaded_bytes);
    if let Some(total) = total_bytes
        && total > 0
    {
        let pct = downloaded_bytes.saturating_mul(100) / total;
        let total_mb = bytes_to_megabytes_str(total);
        format!(
            "Downloading {crate_name} v{version}... {pct}% ({downloaded_mb} / {total_mb} MB)"
        )
    } else {
        format!("Downloading {crate_name} v{version}... {downloaded_mb} MB")
    }
}

/// Converts a byte count to a `X.Y` megabyte string using integer arithmetic (avoiding
/// lossy float casts).
#[must_use]
pub fn bytes_to_megabytes_str(bytes: u64) -> String {
    const BYTES_PER_MB: u64 = 1_048_576;
    let whole_mb = bytes / BYTES_PER_MB;
    let remainder = bytes % BYTES_PER_MB;
    let tenths = remainder.saturating_mul(10) / BYTES_PER_MB;
    format!("{whole_mb}.{tenths}")
}

/// Spinner message while verifying `.sha256` checksum and unpacking archive.
#[must_use]
pub fn format_extract_and_replace_msg(crate_name: &str, version: &str) -> String {
    format!("Verifying checksum and installing {crate_name} v{version}...")
}

/// Notice printed when falling back from GitHub Release binary download to `crates.io`
/// source compilation.
#[must_use]
pub fn format_fallback_to_source_notice(crate_name: &str, version: &str) -> String {
    format!(
        "⚠️  Pre-compiled binary for {crate_name} v{version} unavailable; \
         falling back to source compilation from crates.io..."
    )
}

/// Initial spinner message for `rustup` toolchain update.
#[must_use]
pub const fn rustup_update_initial_msg() -> &'static str { "Updating Rust toolchain..." }

/// Spinner message during `rustup` toolchain update.
#[must_use]
pub fn format_rustup_progress_msg(progress_info: &str) -> String {
    format!("Updating Rust toolchain... {progress_info}")
}

/// Spinner message for `cargo +nightly install` with `OSC 9;4` percentage.
#[must_use]
pub fn format_cargo_progress_msg(
    crate_name: &str,
    version: &str,
    percentage: Pc,
) -> String {
    format!("Installing {crate_name} v{version}... {percentage:?}")
}

/// Spinner message for indeterminate building phase of `cargo +nightly install`.
#[must_use]
pub fn format_cargo_building_msg(crate_name: &str, version: &str) -> String {
    format!("Installing {crate_name} v{version}... (building)")
}

/// Spinner message for initial or cleared progress phase of `cargo +nightly install`.
#[must_use]
pub fn format_cargo_installing_msg(crate_name: &str, version: &str) -> String {
    format!("Installing {crate_name} v{version}...")
}

/// Spinner message when a build error event is received during `cargo +nightly install`.
#[must_use]
pub fn format_cargo_build_error_msg(crate_name: &str, version: &str) -> String {
    format!("Installing {crate_name} v{version}... (error occurred)")
}

/// Final message passed to `Spinner` when installation or download ends.
#[must_use]
pub const fn installation_ended_msg() -> &'static str { "Installation ended." }

/// Success message after upgrading `crate_name` to `version`.
#[must_use]
pub fn format_upgrade_success_msg(crate_name: &str, version: &str) -> String {
    format!("✅ Upgraded {crate_name} to v{version} successfully.")
}

/// Clean error message when upgrade fails for an open-source crate with no local
/// toolchain.
#[must_use]
pub fn format_open_source_no_toolchain_error(bin_name: &str) -> String {
    format!(
        "❌ Upgrade failed: GitHub release unreachable and no local Rust build toolchain found. \
         Please retry later: {bin_name} --upgrade"
    )
}

/// Clean error message when upgrade fails for a closed-source crate.
#[must_use]
pub fn format_closed_source_unreachable_error(bin_name: &str) -> String {
    format!(
        "❌ Upgrade failed: Binary release server unreachable. \
         Please retry later: {bin_name} --upgrade"
    )
}

/// Clean error message for general upgrade failure with retry instructions.
#[must_use]
pub fn format_upgrade_failed_retry_msg(bin_name: &str, reason: &str) -> String {
    format!("❌ Upgrade failed ({reason}). Please retry later: {bin_name} --upgrade")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, pc};

    #[test]
    fn test_bytes_to_megabytes_str() {
        assert_eq2!(bytes_to_megabytes_str(0), "0.0");
        assert_eq2!(bytes_to_megabytes_str(1_048_576), "1.0");
        assert_eq2!(bytes_to_megabytes_str(1_572_864), "1.5");
        assert_eq2!(bytes_to_megabytes_str(10_485_760), "10.0");
    }

    #[test]
    fn test_format_download_progress_msg() {
        let with_total = format_download_progress_msg(
            "r3bl-cmdr",
            "0.0.25",
            1_048_576,
            Some(2_097_152),
        );
        assert_eq2!(
            with_total,
            "Downloading r3bl-cmdr v0.0.25... 50% (1.0 / 2.0 MB)"
        );

        let without_total =
            format_download_progress_msg("r3bl-cmdr", "0.0.25", 1_048_576, None);
        assert_eq2!(without_total, "Downloading r3bl-cmdr v0.0.25... 1.0 MB");
    }

    #[test]
    fn test_format_cargo_messages() {
        let p = pc!(75).unwrap();
        let msg = format_cargo_progress_msg("r3bl-cmdr", "0.0.25", p);
        assert!(msg.contains("Installing r3bl-cmdr v0.0.25..."));
        assert!(msg.contains("75%"));

        assert_eq2!(
            format_open_source_no_toolchain_error("giti"),
            "❌ Upgrade failed: GitHub release unreachable and no local Rust build toolchain found. Please retry later: giti --upgrade"
        );
        assert_eq2!(
            format_closed_source_unreachable_error("r3bl-pro"),
            "❌ Upgrade failed: Binary release server unreachable. Please retry later: r3bl-pro --upgrade"
        );
    }
}

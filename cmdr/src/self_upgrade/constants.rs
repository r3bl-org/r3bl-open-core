// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::time::Duration;

/// URL for R3BL crate versions and distribution visibility. See [`JsoncManifest`] for the
/// complete JSONC schema specification and example.
///
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
pub const R3BL_VERSIONS_JSONC_URL: &str = "https://r3bl.com/versions.jsonc";

/// Property keys used in the [`R3BL_VERSIONS_JSONC_URL`] manifest.
///
/// [`R3BL_VERSIONS_JSONC_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSONC_URL
pub mod versions_manifest_keys {
    /// Top-level base URL for downloading binary release archives
    /// (`"binaries_release_base_url"`).
    pub const BINARIES_RELEASE_BASE_URL: &str = "binaries_release_base_url";

    /// Top-level crates map key (`"crates"`).
    pub const CRATES: &str = "crates";

    /// Crate semantic version key (`"version"`).
    pub const VERSION: &str = "version";

    /// Crate distribution model key (`"crate_distribution"`).
    pub const CRATE_DISTRIBUTION: &str = "crate_distribution";

    /// Crate UTC release timestamp key (`"released_on"`).
    pub const RELEASED_ON: &str = "released_on";
}

/// Wire string values for [`CrateDistribution`] in `versions.jsonc`.
///
/// [`CrateDistribution`]: crate::self_upgrade::CrateDistribution
pub mod crate_distribution_values {
    /// Open-source distribution value (`"open-source"`).
    pub const OPEN_SOURCE: &str = "open-source";

    /// Closed-source distribution value (`"closed-source"`).
    pub const CLOSED_SOURCE: &str = "closed-source";
}

/// Toolchain binary names used on `$PATH` and in [`PTY`] sessions.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub mod toolchain_bin {
    /// Binary name for `rustup`.
    pub const RUSTUP: &str = "rustup";

    /// Binary name for `cargo`.
    pub const CARGO: &str = "cargo";
}

/// Binary name constants and fallbacks.
pub mod upgrade_bin_names {
    /// Default fallback binary name when [`std::env::current_exe`] cannot resolve.
    pub const DEFAULT_FALLBACK_BIN_NAME: &str = "r3bl-binary";
}

/// CLI arguments and flags passed to toolchain binaries (`rustup`, `cargo`).
pub mod toolchain_cli_args {
    /// `--version` flag used to probe binary presence on `$PATH` and specify crate
    /// version.
    pub const VERSION: &str = "--version";

    /// `--force` flag passed to force overwrite or installation.
    pub const FORCE: &str = "--force";

    /// `toolchain` subcommand for `rustup`.
    pub const TOOLCHAIN: &str = "toolchain";

    /// `install` subcommand for `rustup` and `cargo`.
    pub const INSTALL: &str = "install";

    /// `nightly` toolchain identifier for `rustup`.
    pub const NIGHTLY_TOOLCHAIN: &str = "nightly";

    /// `+nightly` toolchain override flag for `cargo`.
    pub const NIGHTLY_FLAG: &str = "+nightly";
}

/// CLI arguments and flags recognized by binaries for self-upgrade operations.
pub mod upgrade_cli_args {
    /// Long argument name (`"upgrade"`) used by [`clap`] for `--upgrade`.
    ///
    /// [`clap`]: mod@clap
    pub const UPGRADE_ARG_LONG: &str = "upgrade";

    /// Long argument name (`"upgrade-behavior"`) used by [`clap`] for
    /// `--upgrade-behavior`.
    ///
    /// [`clap`]: mod@clap
    pub const UPGRADE_BEHAVIOR_ARG_LONG: &str = "upgrade-behavior";
}

/// Timeouts and cooldown windows for manifest version checks.
pub mod manifest_timeouts {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// HTTP timeout (`5s`) when fetching `versions.jsonc` during explicit `--upgrade`
    /// ([`UPGRADE_ARG_LONG`]) runs.
    ///
    /// [`UPGRADE_ARG_LONG`]: crate::self_upgrade::upgrade_cli_args::UPGRADE_ARG_LONG
    pub const IMMEDIATE_MANIFEST_TIMEOUT: Duration = Duration::from_secs(5);

    /// HTTP timeout (`2s`) when fetching `versions.jsonc` on the dedicated check thread
    /// configured via `--upgrade-behavior` ([`UPGRADE_BEHAVIOR_ARG_LONG`]).
    ///
    /// [`UPGRADE_BEHAVIOR_ARG_LONG`]: crate::self_upgrade::upgrade_cli_args::UPGRADE_BEHAVIOR_ARG_LONG
    pub const DEDICATED_THREAD_MANIFEST_TIMEOUT: Duration = Duration::from_secs(2);

    /// Cooldown window (`1h`) after a verified [`UpToDate`] check or completed upgrade
    /// before the dedicated check thread queries `versions.jsonc` again.
    ///
    /// [`UpToDate`]: crate::self_upgrade::UpToDate
    pub const DEDICATED_THREAD_CHECK_COOLDOWN: Duration = Duration::from_hours(1);
}

/// OS thread naming constants for the [`self_upgrade`] [Control Plane] (see [Thread
/// Model]).
///
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
pub mod check_thread {
    /// Thread name assigned to the dedicated version-check OS thread spawned by the
    /// [Control Plane] (see [Thread Model]).
    ///
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    /// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
    pub const CHECK_THREAD_NAME: &str = "self-upgrade-check";
}

/// File extensions, delimiters, and network timeouts for pre-compiled release archives
/// and temporary extraction files.
pub mod archive {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// HTTP `HEAD` timeout (`5s`) when probing the release archive server for
    /// pre-compiled binary archives.
    pub const HEAD_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

    /// Standard archive extension for pre-compiled releases (`"tar.gz"`).
    pub const TAR_GZ_EXT: &str = "tar.gz";

    /// Checksum file extension (`"sha256"`).
    pub const SHA256_EXT: &str = "sha256";

    /// Separator between crate name, version tag, and target triple in release tags and
    /// archive filenames (`'-'`).
    pub const COMPONENT_SEPARATOR: char = '-';

    /// Delimiter preceding file extensions in archive and checksum filenames (`'.'`).
    pub const EXTENSION_DELIMITER: char = '.';

    /// Path segment separator for release asset URLs (`'/'`).
    pub const URL_PATH_SEPARATOR: char = '/';

    /// Temporary extraction directory name under temp folder.
    pub const EXTRACTED_DIR_NAME: &str = "extracted";

    /// Temporary upgrade file infix/suffix for sibling binary replacement.
    pub const UPGRADE_TMP_SUFFIX: &str = "upgrade.tmp";

    /// Non-binary metadata file extensions to ignore during sibling binary replacement.
    pub const NON_BINARY_EXTENSIONS: [&str; 5] = ["md", "txt", "json", "jsonc", "sha256"];
}

/// Descriptive reason strings for [`UpgradeError::NoViableUpgradeSource`] and retry
/// messages.
///
/// [`UpgradeError::NoViableUpgradeSource`]: super::UpgradeError::NoViableUpgradeSource
pub mod upgrade_reasons {
    /// Reason when closed-source binary release assets cannot be reached.
    pub const BINARY_RELEASE_UNREACHABLE: &str = "Binary release server unreachable";

    /// Reason when binary release archives are unreachable and no local Rust toolchain
    /// exists.
    pub const NO_LOCAL_TOOLCHAIN: &str =
        "Binary release server unreachable and no local Rust build toolchain found";

    /// Reason when binary release download fails mid-stream for closed-source binaries.
    pub const BINARY_DOWNLOAD_FAILED: &str = "Binary release download failed";

    /// Reason when an upgrade step is canceled cooperatively by the user.
    pub const CANCELLED_BY_USER: &str = "cancelled by user";
}

/// Progress parsing constants for `rustup` terminal output.
pub mod upgrade_rustup_progress {
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

/// Semantic version formatting and parsing constants.
pub mod semver {
    /// Delimiter separating `MAJOR.MINOR.PATCH` version components (`'.'`).
    pub const DELIMITER: char = '.';

    /// Prefix character on version tags and archive version segments (`'v'`).
    pub const VERSION_PREFIX: char = 'v';

    /// Base-10 radix used for parsing and formatting version segments.
    pub const RADIX: u32 = 10;

    /// Maximum segments collected by [`str::splitn`] (`3` valid `MAJOR.MINOR.PATCH`
    /// segments + `1` overflow slot to detect trailing extra segments without
    /// heap-allocating).
    pub const MAX_SPLIT_SEGMENTS: usize = 4;
}

/// Allowed URL scheme constants for [`EndpointUrl`] validation.
///
/// [`EndpointUrl`]: crate::self_upgrade::EndpointUrl
pub mod url_schemes {
    /// Plain HTTP scheme (`"http"`, used by local loopback mock servers in unit tests).
    pub const HTTP: &str = "http";

    /// TLS HTTPS scheme (`"https"`, used by production manifest and release endpoints).
    pub const HTTPS: &str = "https";
}

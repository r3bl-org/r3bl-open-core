// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared constants for binary self-upgrade operations, toolchain probes, and archive
//! handling.

/// URL for R3BL crate versions and distribution visibility.
pub const R3BL_VERSIONS_JSON_URL: &str = "https://r3bl.com/versions.jsonc";

/// Base URL for pre-compiled release assets in the dedicated history-free binary
/// repository.
pub const R3BL_BINARIES_RELEASE_BASE_URL: &str =
    "https://github.com/r3bl-org/r3bl-binaries/releases/download";

/// Toolchain binary names used on `$PATH` and in [`PTY`] sessions.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub mod toolchain_bin {
    /// Binary name for `rustup`.
    pub const RUSTUP: &str = "rustup";

    /// Binary name for `cargo`.
    pub const CARGO: &str = "cargo";
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

/// File extensions and names for pre-compiled release archives and temporary extraction
/// files.
pub mod archive {
    /// Standard archive extension for pre-compiled releases (`"tar.gz"`).
    pub const TAR_GZ_EXT: &str = "tar.gz";

    /// Checksum file extension (`"sha256"`).
    pub const SHA256_EXT: &str = "sha256";

    /// Default fallback archive filename when URL parsing fails.
    pub const DEFAULT_ARCHIVE_FILENAME: &str = "release_archive.tar.gz";

    /// Temporary extraction directory name under temp folder.
    pub const EXTRACTED_DIR_NAME: &str = "extracted";

    /// Temporary upgrade file infix/suffix for sibling binary replacement.
    pub const UPGRADE_TMP_SUFFIX: &str = "upgrade.tmp";

    /// Non-binary metadata file extensions to ignore during sibling binary replacement.
    pub const NON_BINARY_EXTENSIONS: [&str; 5] = ["md", "txt", "json", "jsonc", "sha256"];
}

/// Property keys used in the [`R3BL_VERSIONS_JSON_URL`] manifest.
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::core::script::upgrade::R3BL_VERSIONS_JSON_URL
pub mod versions_manifest_keys {
    /// Top-level schema version key (`"schema_version"`).
    pub const SCHEMA_VERSION: &str = "schema_version";

    /// Top-level crates map key (`"crates"`).
    pub const CRATES: &str = "crates";

    /// Crate semantic version key (`"version"`).
    pub const VERSION: &str = "version";

    /// Crate distribution model key (`"crate_distribution"`).
    pub const CRATE_DISTRIBUTION: &str = "crate_distribution";

    /// Crate UTC release timestamp key (`"released_on"`).
    pub const RELEASED_ON: &str = "released_on";
}

/// Descriptive reason strings for [`UpgradeError::NoViableUpgradeSource`] and retry
/// messages.
///
/// [`UpgradeError::NoViableUpgradeSource`]: super::UpgradeError::NoViableUpgradeSource
pub mod upgrade_reasons {
    /// Reason when closed-source binary release assets cannot be reached.
    pub const BINARY_RELEASE_UNREACHABLE: &str = "Binary release server unreachable";

    /// Reason when GitHub releases are unreachable and no local Rust toolchain exists.
    pub const NO_LOCAL_TOOLCHAIN: &str =
        "GitHub release unreachable and no local Rust build toolchain found";

    /// Reason when binary release download fails mid-stream for closed-source binaries.
    pub const BINARY_DOWNLOAD_FAILED: &str = "Binary release download failed";

    /// Reason when an upgrade step is canceled cooperatively by the user.
    pub const CANCELLED_BY_USER: &str = "cancelled by user";
}

/// Progress parsing constants for `rustup` terminal output.
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

/// Semantic version formatting and parsing constants.
pub mod semver {
    /// Delimiter separating `MAJOR.MINOR.PATCH` version components (`'.'`).
    pub const DELIMITER: char = '.';
}

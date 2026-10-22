// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Streaming archive download and SHA-256 checksum verification.
//!
//! Provides the primary download entry point [`download_and_verify_archive`] and its
//! synchronous variant [`download_and_verify_archive_sync`].
//!
//! Underlying logic is encapsulated into inner modules:
//! - [`sha256_checksum`]: Checksum parsing, computation, and verification.
//! - [`archive_stream`]: Streaming HTTP download with live spinner updates.

use super::{super::UpgradeError, ui_strings};
use crate::{Spinner, script::http_client::create_client_with_user_agent};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::{fs::{self, File},
          io::Write,
          path::Path};
use tokio::signal;

// -------------------------------------------------------------------------------------------------
// Top-level Entry Points (Runtime Call-Site Adapters)
// -------------------------------------------------------------------------------------------------

/// Downloads and verifies the checksum of the pre-compiled archive.
///
/// # Errors
///
/// Returns [`UpgradeError`] on download failure, cancellation, or SHA-256 verification
/// failure.
pub async fn download_and_verify_archive(
    crate_name: &str,
    pinned_version: &str,
    archive_url: &str,
    sha256_url: &str,
    archive_path: &Path,
    maybe_spinner: Option<&Spinner>,
) -> Result<(), UpgradeError> {
    let client = create_client_with_user_agent(None)
        .map_err(|e| UpgradeError::Other(e.to_string()))?;

    let expected_sha256 =
        sha256_checksum::fetch_expected_sha256(&client, sha256_url).await?;
    archive_stream::stream_archive_to_disk(
        &client,
        archive_url,
        archive_path,
        crate_name,
        pinned_version,
        maybe_spinner,
    )
    .await?;

    if let Some(spinner) = maybe_spinner {
        spinner.update_message(ui_strings::format_extract_and_replace_msg(
            crate_name,
            pinned_version,
        ));
    }

    sha256_checksum::verify_file_sha256(archive_path, &expected_sha256)?;
    Ok(())
}

/// Synchronous variant of [`download_and_verify_archive`].
///
/// Builds a private `current_thread` Tokio runtime to drive the download and
/// verification.
///
/// # Errors
///
/// Returns [`UpgradeError`] on download failure, cancellation, or SHA-256 verification
/// failure.
pub fn download_and_verify_archive_sync(
    crate_name: &str,
    pinned_version: &str,
    archive_url: &str,
    sha256_url: &str,
    archive_path: &Path,
    maybe_spinner: Option<&Spinner>,
) -> Result<(), UpgradeError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| UpgradeError::Other(e.to_string()))?;
    rt.block_on(download_and_verify_archive(
        crate_name,
        pinned_version,
        archive_url,
        sha256_url,
        archive_path,
        maybe_spinner,
    ))
}

// -------------------------------------------------------------------------------------------------
// Inner Modules (Underlying Implementation & Domain Logic)
// -------------------------------------------------------------------------------------------------

/// SHA-256 digest calculation, formatting, and file verification.
pub mod sha256_checksum {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Parses the expected hex SHA-256 digest from the contents of a `.sha256` checksum
    /// file (handles both `<hex>  <filename>` and plain `<hex>` formats).
    #[must_use]
    pub fn parse_sha256_checksum_text(text: &str) -> Option<String> {
        let token = text.split_whitespace().next()?.trim();
        if token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()) {
            Some(token.to_ascii_lowercase())
        } else {
            None
        }
    }

    /// Computes the lower-case hex SHA-256 digest of a byte slice.
    #[must_use]
    pub fn compute_sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        let mut hex_str = String::with_capacity(64);
        for byte in digest {
            use std::fmt::Write as _;
            let _unused = write!(hex_str, "{byte:02x}");
        }
        hex_str
    }

    /// Verifies that the SHA-256 digest of `file_path` matches `expected_hex`.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::ChecksumMismatch`] if the computed digest differs from
    /// `expected_hex`, or [`UpgradeError::Other`] on file read failure.
    pub fn verify_file_sha256(
        file_path: &Path,
        expected_hex: &str,
    ) -> Result<(), UpgradeError> {
        let bytes =
            fs::read(file_path).map_err(|e| UpgradeError::Other(e.to_string()))?;
        let actual_hex = compute_sha256_hex(&bytes);
        let expected_lower = expected_hex.trim().to_ascii_lowercase();
        if actual_hex != expected_lower {
            return Err(UpgradeError::ChecksumMismatch {
                expected: expected_lower,
                actual: actual_hex,
            });
        }
        Ok(())
    }

    /// Fetches the expected SHA-256 checksum string from `sha256_url`.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError`] on network failure, cancellation, or invalid checksum
    /// text.
    pub async fn fetch_expected_sha256(
        client: &reqwest::Client,
        sha256_url: &str,
    ) -> Result<String, UpgradeError> {
        let sha_resp = tokio::select! {
            _ = signal::ctrl_c() => return Err(UpgradeError::Cancelled),
            res = client.get(sha256_url).send() => {
                res.map_err(|e| UpgradeError::Other(e.to_string()))?
            }
        };
        let sha_resp = sha_resp
            .error_for_status()
            .map_err(|e| UpgradeError::Other(e.to_string()))?;
        let sha_text = tokio::select! {
            _ = signal::ctrl_c() => return Err(UpgradeError::Cancelled),
            res = sha_resp.text() => {
                res.map_err(|e| UpgradeError::Other(e.to_string()))?
            }
        };
        parse_sha256_checksum_text(&sha_text).ok_or_else(|| {
            UpgradeError::Other(format!("Invalid .sha256 file format: {sha_text}"))
        })
    }
}

/// HTTP streaming archive downloads to local storage.
pub mod archive_stream {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Streams the binary archive from `archive_url` to disk at `archive_path`, updating
    /// `maybe_spinner` with progress percentage and megabytes.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError`] on network failure, file writing failure, or user
    /// cancellation.
    pub async fn stream_archive_to_disk(
        client: &reqwest::Client,
        archive_url: &str,
        archive_path: &Path,
        crate_name: &str,
        pinned_version: &str,
        maybe_spinner: Option<&Spinner>,
    ) -> Result<(), UpgradeError> {
        let archive_resp = tokio::select! {
            _ = signal::ctrl_c() => return Err(UpgradeError::Cancelled),
            res = client.get(archive_url).send() => {
                res.map_err(|e| UpgradeError::Other(e.to_string()))?
            }
        };
        let archive_resp = archive_resp
            .error_for_status()
            .map_err(|e| UpgradeError::Other(e.to_string()))?;
        let total_bytes = archive_resp.content_length();

        let mut dest_file =
            File::create(archive_path).map_err(|e| UpgradeError::Other(e.to_string()))?;
        let mut downloaded_bytes: u64 = 0;
        let mut stream = archive_resp.bytes_stream();

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    return Err(UpgradeError::Cancelled);
                }
                maybe_chunk = stream.next() => {
                    match maybe_chunk {
                        Some(Ok(chunk)) => {
                            dest_file
                                .write_all(&chunk)
                                .map_err(|e| UpgradeError::Other(e.to_string()))?;
                            downloaded_bytes = downloaded_bytes.saturating_add(
                                u64::try_from(chunk.len()).unwrap_or(u64::MAX),
                            );
                            if let Some(spinner) = maybe_spinner {
                                spinner.update_message(
                                    ui_strings::format_download_progress_msg(
                                        crate_name,
                                        pinned_version,
                                        downloaded_bytes,
                                        total_bytes,
                                    ),
                                );
                            }
                        }
                        Some(Err(err)) => {
                            return Err(UpgradeError::Other(err.to_string()));
                        }
                        None => break,
                    }
                }
            }
        }

        dest_file
            .flush()
            .map_err(|e| UpgradeError::Other(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, script::temp_dir::try_create_temp_dir};

    #[test]
    fn test_parse_sha256_checksum_text() {
        let sha_line = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  r3bl-cmdr-v0.0.25-x86_64-unknown-linux-gnu.tar.gz\n";
        assert_eq2!(
            sha256_checksum::parse_sha256_checksum_text(sha_line).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        let plain_hex =
            "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855";
        assert_eq2!(
            sha256_checksum::parse_sha256_checksum_text(plain_hex).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        assert!(sha256_checksum::parse_sha256_checksum_text("not_a_valid_sha").is_none());
        assert!(sha256_checksum::parse_sha256_checksum_text("").is_none());
    }

    #[test]
    fn test_compute_and_verify_file_sha256() {
        let temp_dir = try_create_temp_dir().unwrap();
        let file_path = temp_dir.join("empty.bin");
        fs::write(&file_path, b"").unwrap();

        // Known SHA-256 of empty byte array.
        let empty_sha =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq2!(sha256_checksum::compute_sha256_hex(b""), empty_sha);
        assert!(sha256_checksum::verify_file_sha256(&file_path, empty_sha).is_ok());
        assert!(matches!(
            sha256_checksum::verify_file_sha256(
                &file_path,
                "0000000000000000000000000000000000000000000000000000000000000000"
            ),
            Err(UpgradeError::ChecksumMismatch { .. })
        ));
    }
}

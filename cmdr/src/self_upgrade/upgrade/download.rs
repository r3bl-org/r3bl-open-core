// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::UpgradeError, Sha256Digest, ui_strings};
use futures_util::StreamExt;
use r3bl_tui::{Spinner, create_client_with_user_agent};
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
///
/// [`UpgradeError`]: crate::self_upgrade::UpgradeError
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

    sha256_checksum::verify_file_sha256(archive_path, expected_sha256)?;
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Inner Modules (Underlying Implementation & Domain Logic)
// -------------------------------------------------------------------------------------------------

/// SHA-256 checksum fetching and file verification.
pub mod sha256_checksum {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Verifies that the SHA-256 digest of `file_path` matches `expected`.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::ChecksumMismatch`] if the computed digest differs from
    /// `expected`, or [`UpgradeError::Other`] on file read failure.
    ///
    /// [`UpgradeError::ChecksumMismatch`]: crate::self_upgrade::UpgradeError::ChecksumMismatch
    /// [`UpgradeError::Other`]: crate::self_upgrade::UpgradeError::Other
    pub fn verify_file_sha256(
        file_path: &Path,
        expected: Sha256Digest,
    ) -> Result<(), UpgradeError> {
        let bytes =
            fs::read(file_path).map_err(|e| UpgradeError::Other(e.to_string()))?;
        let actual = Sha256Digest::from_bytes(&bytes);
        if actual != expected {
            return Err(UpgradeError::ChecksumMismatch { expected, actual });
        }
        Ok(())
    }

    /// Fetches and parses the expected [`Sha256Digest`] from `sha256_url`.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError`] on network failure, cancellation, or invalid checksum
    /// text.
    ///
    /// [`Sha256Digest`]: crate::self_upgrade::Sha256Digest
    /// [`UpgradeError`]: crate::self_upgrade::UpgradeError
    pub async fn fetch_expected_sha256(
        client: &reqwest::Client,
        sha256_url: &str,
    ) -> Result<Sha256Digest, UpgradeError> {
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
        Sha256Digest::try_parse_hex(&sha_text).ok_or_else(|| {
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
    use r3bl_tui::{assert_eq2, try_create_temp_dir};

    #[test]
    fn test_verify_file_sha256() {
        let temp_dir = try_create_temp_dir().unwrap();
        let file_path = temp_dir.join("empty.bin");
        fs::write(&file_path, b"").unwrap();

        let empty_digest = Sha256Digest::from_bytes(b"");
        assert!(sha256_checksum::verify_file_sha256(&file_path, empty_digest).is_ok());

        let zero_digest = Sha256Digest::try_parse_hex(
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        assert_eq2!(
            sha256_checksum::verify_file_sha256(&file_path, zero_digest),
            Err(UpgradeError::ChecksumMismatch {
                expected: zero_digest,
                actual: empty_digest,
            })
        );
    }

    #[test]
    fn test_verify_file_sha256_nonexistent_file() {
        let temp_dir = try_create_temp_dir().unwrap();
        let nonexistent = temp_dir.join("nonexistent.bin");
        let result = sha256_checksum::verify_file_sha256(
            &nonexistent,
            Sha256Digest::from_bytes(b""),
        );
        assert!(matches!(result, Err(UpgradeError::Other(_))));
    }

    #[tokio::test]
    async fn test_fetch_expected_sha256_unreachable_url() {
        let client = reqwest::Client::new();
        let result = sha256_checksum::fetch_expected_sha256(
            &client,
            "http://127.0.0.1:1/nonexistent.sha256",
        )
        .await;
        assert!(matches!(result, Err(UpgradeError::Other(_))));
    }
}

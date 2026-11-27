// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::script::http_client::{HttpClientError, create_http_client};
use std::{fs,
          io::Write,
          path::{Path, PathBuf}};

/// Error returned by [`try_download_file_overwrite_existing`] when HTTP client creation,
/// downloading, or writing to disk fails.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum DownloadFileError {
    /// Failed to construct the shared [`reqwest::Client`].
    #[error("Failed to create HTTP client")]
    HttpClientCreateFailed(#[source] HttpClientError),

    /// HTTP request, status check, or response body download failed for `url`.
    #[error("HTTP download from '{url}' failed")]
    HttpRequestFailed {
        /// Source URL being downloaded.
        url: String,
        /// Underlying [`reqwest::Error`].
        #[source]
        src_err: reqwest::Error,
    },

    /// Creating or writing the destination file at `path` failed.
    #[error("Failed to write downloaded file '{}'", path.display())]
    FileIo {
        /// Destination file path on disk.
        path: PathBuf,
        /// Underlying OS I/O error.
        #[source]
        src_err: std::io::Error,
    },
}

/// # Errors
///
/// Returns [`DownloadFileError`] if:
/// - The HTTP client cannot be created ([`DownloadFileError::HttpClientCreateFailed`])
/// - The network request fails or returns a non-2xx status
///   ([`DownloadFileError::HttpRequestFailed`])
/// - The destination file cannot be created or written ([`DownloadFileError::FileIo`])
pub async fn try_download_file_overwrite_existing(
    source_url: &str,
    destination_file: impl AsRef<Path>,
) -> Result<(), DownloadFileError> {
    let destination = destination_file.as_ref();

    let client =
        create_http_client().map_err(DownloadFileError::HttpClientCreateFailed)?;
    let response = client.get(source_url).send().await.map_err(|src_err| {
        DownloadFileError::HttpRequestFailed {
            url: source_url.to_owned(),
            src_err,
        }
    })?;
    let response = response.error_for_status().map_err(|src_err| {
        DownloadFileError::HttpRequestFailed {
            url: source_url.to_owned(),
            src_err,
        }
    })?;
    let response =
        response
            .bytes()
            .await
            .map_err(|src_err| DownloadFileError::HttpRequestFailed {
                url: source_url.to_owned(),
                src_err,
            })?;

    let mut dest_file =
        fs::File::create(destination).map_err(|src_err| DownloadFileError::FileIo {
            path: destination.to_path_buf(),
            src_err,
        })?;
    dest_file
        .write_all(&response)
        .map_err(|src_err| DownloadFileError::FileIo {
            path: destination.to_path_buf(),
            src_err,
        })?;

    ok!()
}

#[cfg(test)]
mod tests_download {
    use super::*;
    use crate::try_create_temp_dir;

    #[tokio::test]
    async fn test_download_file_creates_and_overwrites_existing() {
        let mut server = mockito::Server::new_async().await;
        let root = try_create_temp_dir().unwrap();
        let destination_file = root.join("artifact.bin");

        // Initial download (no pre-existing file).
        let v1_mock = server
            .mock("GET", "/artifact-v1")
            .with_status(200)
            .with_body(b"initial-payload-v1")
            .expect(1)
            .create_async()
            .await;

        try_download_file_overwrite_existing(
            &format!("{}/artifact-v1", server.url()),
            &destination_file,
        )
        .await
        .unwrap();
        assert!(destination_file.exists());
        assert_eq2!(fs::read(&destination_file).unwrap(), b"initial-payload-v1");
        v1_mock.assert_async().await;

        // Second download overwrites existing file with updated payload bytes.
        let v2_mock = server
            .mock("GET", "/artifact-v2")
            .with_status(200)
            .with_body(b"updated-binary-payload-v2-longer")
            .expect(1)
            .create_async()
            .await;

        try_download_file_overwrite_existing(
            &format!("{}/artifact-v2", server.url()),
            &destination_file,
        )
        .await
        .unwrap();
        assert_eq2!(
            fs::read(&destination_file).unwrap(),
            b"updated-binary-payload-v2-longer"
        );
        v2_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_download_file_propagates_http_404_and_500_errors() {
        let mut server = mockito::Server::new_async().await;
        let root = try_create_temp_dir().unwrap();

        let not_found_mock = server
            .mock("GET", "/missing")
            .with_status(404)
            .expect(1)
            .create_async()
            .await;
        let server_error_mock = server
            .mock("GET", "/boom")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;

        let missing_dest = root.join("missing.bin");
        let boom_dest = root.join("boom.bin");

        let url_404 = format!("{}/missing", server.url());
        let res_404 = try_download_file_overwrite_existing(&url_404, &missing_dest).await;
        assert!(matches!(
            res_404,
            Err(DownloadFileError::HttpRequestFailed { ref url, ref src_err })
                if url == &url_404 && src_err.status() == Some(reqwest::StatusCode::NOT_FOUND)
        ));
        assert!(!missing_dest.exists());
        not_found_mock.assert_async().await;

        let url_500 = format!("{}/boom", server.url());
        let res_500 = try_download_file_overwrite_existing(&url_500, &boom_dest).await;
        assert!(matches!(
            res_500,
            Err(DownloadFileError::HttpRequestFailed { ref url, ref src_err })
                if url == &url_500 && src_err.status() == Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
        ));
        assert!(!boom_dest.exists());
        server_error_mock.assert_async().await;
    }
}

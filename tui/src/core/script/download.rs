// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::script::http_client::create_http_client;
use miette::IntoDiagnostic;
use std::{fs, io::Write, path::Path};

/// # Errors
///
/// Returns an error if:
/// - The HTTP client cannot be created
/// - The network request fails
/// - The server returns an error status
/// - The destination file cannot be created (permissions, invalid path)
/// - Writing to the file fails
pub async fn try_download_file_overwrite_existing(
    source_url: &str,
    destination_file: impl AsRef<Path>,
) -> miette::Result<()> {
    let destination = destination_file.as_ref();

    let client = create_http_client()?;
    let response = client.get(source_url).send().await.into_diagnostic()?;
    let response = response.error_for_status().into_diagnostic()?;
    let response = response.bytes().await.into_diagnostic()?;

    let mut dest_file = fs::File::create(destination).into_diagnostic()?;
    dest_file.write_all(&response).into_diagnostic()?;

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

        let res_404 = try_download_file_overwrite_existing(
            &format!("{}/missing", server.url()),
            &missing_dest,
        )
        .await;
        assert!(res_404.is_err());
        assert!(!missing_dest.exists());
        not_found_mock.assert_async().await;

        let res_500 = try_download_file_overwrite_existing(
            &format!("{}/boom", server.url()),
            &boom_dest,
        )
        .await;
        assert!(res_500.is_err());
        assert!(!boom_dest.exists());
        server_error_mock.assert_async().await;
    }
}

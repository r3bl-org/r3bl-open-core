// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{http_client::create_client_with_user_agent, ok};
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

    let client = create_client_with_user_agent(None)?;
    let response = client.get(source_url).send().await.into_diagnostic()?;
    let response = response.error_for_status().into_diagnostic()?;
    let response = response.bytes().await.into_diagnostic()?;

    let mut dest_file = fs::File::create(destination).into_diagnostic()?;
    dest_file.write_all(&response).into_diagnostic()?;

    ok!()
}

#[cfg(test)]
mod tests_download {
    // cspell::ignore cfssljson

    use super::*;
    use crate::try_create_temp_dir;
    use std::time::Duration;
    use tokio::time::timeout;

    const TIMEOUT: Duration = Duration::from_secs(1);

    #[tokio::test]
    async fn test_download_file_overwrite_existing() {
        // Create the root temp dir.
        let root = try_create_temp_dir().unwrap();

        let new_dir = root.join("test_download_file_overwrite_existing");
        fs::create_dir_all(&new_dir).unwrap();

        let source_url = "https://github.com/cloudflare/cfssl/releases/download/v1.6.5/cfssljson_1.6.5_linux_amd64";
        let destination_file = new_dir.join("cfssljson");

        // Download file (no pre-existing file).
        match timeout(
            TIMEOUT,
            try_download_file_overwrite_existing(source_url, &destination_file),
        )
        .await
        {
            Ok(Ok(())) => {
                assert!(destination_file.exists());
            }
            Ok(Err(err)) => {
                // Re-throw the error and fail the test.
                panic!("Error: {err:?}");
            }
            Err(_) => {
                // Timeout does not mean that test has failed. Github is probably slow.
                println!("Timeout");
                return;
            }
        }

        let meta_data = destination_file.metadata().unwrap();
        let og_file_size = meta_data.len();

        // Download file again (overwrite existing).
        match timeout(
            TIMEOUT,
            try_download_file_overwrite_existing(source_url, &destination_file),
        )
        .await
        {
            Ok(Ok(())) => {
                assert!(destination_file.exists());
            }
            Ok(Err(err)) => {
                // Re-throw the error and fail the test.
                panic!("Error: {err:?}");
            }
            Err(_) => {
                // Timeout does not mean that test has failed. Github is probably slow.
                println!("Timeout");
                return;
            }
        }

        // Ensure that the file sizes are the same.
        let meta_data = destination_file.metadata().unwrap();
        let new_file_size = meta_data.len();
        assert_eq!(og_file_size, new_file_size);
    }
}

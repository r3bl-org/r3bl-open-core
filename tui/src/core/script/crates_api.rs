// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DEBUG_SCRIPT, fg_magenta, script::http_client};
use miette::IntoDiagnostic;

mod constants {
    pub const CRATE: &str = "crate";
    pub const MAX_VERSION: &str = "max_version";
    pub const CRATE_INFO: &str = "https://crates.io/api/v1/crates/{crate_name}";
}

/// # Errors
///
/// Returns an error if:
/// - The HTTP client cannot be created
/// - The network request fails
/// - The crates.io API returns an error status
/// - The response JSON is malformed or missing expected fields
pub async fn try_get_latest_release_version_from_crates_io(
    crate_name: &str,
) -> miette::Result<String> {
    let url = constants::CRATE_INFO.replace("{crate_name}", crate_name);
    try_get_latest_release_version_from_url(&url).await
}

/// Fetches crate metadata JSON from `url` and extracts `crate.max_version`.
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP client cannot be created
/// - The network request fails
/// - The endpoint returns a non-2xx status
/// - The response JSON is malformed or missing `crate.max_version`
pub(super) async fn try_get_latest_release_version_from_url(
    url: &str,
) -> miette::Result<String> {
    use self::constants::{CRATE, MAX_VERSION};

    DEBUG_SCRIPT.then(|| {
        tracing::debug!(
            message = "Fetching latest version from crates.io",
            url = %fg_magenta(url)
        );
    });

    let client = http_client::create_client_with_user_agent(None)?;
    let response = client.get(url).send().await.into_diagnostic()?;
    let response = response.error_for_status().into_diagnostic()?;
    let response: serde_json::Value = response.json().await.into_diagnostic()?;

    let Some(version) = response[CRATE][MAX_VERSION].as_str() else {
        return Err(miette::miette!(
            "Failed to get version from JSON: {:?}",
            response
        ));
    };

    ok!(version.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_latest_version_extracts_max_version() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/v1/crates/serde")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"crate": {"max_version": "1.0.228"}}"#)
            .expect(1)
            .create_async()
            .await;

        let version = try_get_latest_release_version_from_url(&format!(
            "{}/api/v1/crates/serde",
            server.url()
        ))
        .await
        .unwrap();
        assert_eq2!(version, "1.0.228");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_latest_version_missing_max_version_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/v1/crates/serde")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"crate": {"name": "serde"}}"#)
            .expect(1)
            .create_async()
            .await;

        let err = try_get_latest_release_version_from_url(&format!(
            "{}/api/v1/crates/serde",
            server.url()
        ))
        .await
        .unwrap_err();
        assert!(err.to_string().contains("Failed to get version from JSON"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_latest_version_http_500_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/v1/crates/serde")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;

        let res = try_get_latest_release_version_from_url(&format!(
            "{}/api/v1/crates/serde",
            server.url()
        ))
        .await;
        assert!(res.is_err());
        mock.assert_async().await;
    }
}

// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DEBUG_SCRIPT, fg_magenta,
            script::http_client::{self, HttpClientError}};

/// Error returned by [`try_get_latest_release_version_from_crates_io`] when HTTP client
/// creation, the crates.io API request, or JSON field extraction fails.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CratesApiError {
    /// Failed to construct the shared [`reqwest::Client`].
    #[error("Failed to create HTTP client")]
    HttpClientCreateFailed(#[source] HttpClientError),

    /// Network error, non-2xx HTTP status, or JSON decoding failure for `url`.
    #[error("crates.io API request to '{url}' failed")]
    HttpRequestFailed {
        /// Request URL sent to the crates.io API endpoint.
        url: String,
        /// Underlying [`reqwest::Error`].
        #[source]
        src_err: reqwest::Error,
    },

    /// JSON response was valid JSON, and lacked a string `crate.max_version` field.
    #[error("Missing 'crate.max_version' string field in crates.io JSON from '{url}'")]
    MissingMaxVersion {
        /// Endpoint URL whose response lacked `crate.max_version`.
        url: String,
    },
}

mod constants {
    pub const CRATE: &str = "crate";
    pub const MAX_VERSION: &str = "max_version";
    pub const CRATE_INFO: &str = "https://crates.io/api/v1/crates/{crate_name}";
}

/// # Errors
///
/// Returns [`CratesApiError`] if:
/// - The HTTP client cannot be created ([`CratesApiError::HttpClientCreateFailed`])
/// - The network request fails, returns a non-2xx status, or returns malformed JSON
///   ([`CratesApiError::HttpRequestFailed`])
/// - The response JSON is missing `crate.max_version`
///   ([`CratesApiError::MissingMaxVersion`])
pub async fn try_get_latest_release_version_from_crates_io(
    crate_name: &str,
) -> Result<String, CratesApiError> {
    let url = constants::CRATE_INFO.replace("{crate_name}", crate_name);
    try_get_latest_release_version_from_url(&url).await
}

/// Fetches crate metadata JSON from `url` and extracts `crate.max_version`.
///
/// # Errors
///
/// Returns [`CratesApiError`] if:
/// - The HTTP client cannot be created ([`CratesApiError::HttpClientCreateFailed`])
/// - The network request fails, returns a non-2xx status, or returns malformed JSON
///   ([`CratesApiError::HttpRequestFailed`])
/// - The response JSON is missing `crate.max_version`
///   ([`CratesApiError::MissingMaxVersion`])
pub(super) async fn try_get_latest_release_version_from_url(
    url: &str,
) -> Result<String, CratesApiError> {
    use self::constants::{CRATE, MAX_VERSION};

    DEBUG_SCRIPT.then(|| {
        tracing::debug!(
            message = "Fetching latest version from crates.io",
            url = %fg_magenta(url)
        );
    });

    let client = http_client::create_http_client()
        .map_err(CratesApiError::HttpClientCreateFailed)?;
    let response = client.get(url).send().await.map_err(|src_err| {
        CratesApiError::HttpRequestFailed {
            url: url.to_owned(),
            src_err,
        }
    })?;
    let response = response.error_for_status().map_err(|src_err| {
        CratesApiError::HttpRequestFailed {
            url: url.to_owned(),
            src_err,
        }
    })?;
    let response: serde_json::Value = response.json().await.map_err(|src_err| {
        CratesApiError::HttpRequestFailed {
            url: url.to_owned(),
            src_err,
        }
    })?;

    let Some(version) = response[CRATE][MAX_VERSION].as_str() else {
        return Err(CratesApiError::MissingMaxVersion {
            url: url.to_owned(),
        });
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

        let req_url = format!("{}/api/v1/crates/serde", server.url());
        let err = try_get_latest_release_version_from_url(&req_url)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            CratesApiError::MissingMaxVersion { ref url } if url == &req_url
        ));
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

        let req_url = format!("{}/api/v1/crates/serde", server.url());
        let res = try_get_latest_release_version_from_url(&req_url).await;
        assert!(matches!(
            res,
            Err(CratesApiError::HttpRequestFailed { ref url, ref src_err })
                if url == &req_url && src_err.status() == Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
        ));
        mock.assert_async().await;
    }
}

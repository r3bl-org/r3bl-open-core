// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DEBUG_SCRIPT, fg_magenta,
            script::http_client::{self, HttpClientError}};

/// Error returned by [`try_get_latest_release_tag_from_github`] when HTTP client
/// creation, the GitHub API request, or JSON field extraction fails.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum GitHubApiError {
    /// Failed to construct the shared [`reqwest::Client`].
    #[error("Failed to create HTTP client")]
    HttpClientCreateFailed(#[source] HttpClientError),

    /// Network error, non-2xx HTTP status, or JSON decoding failure for `url`.
    #[error("GitHub API request to '{url}' failed")]
    HttpRequestFailed {
        /// Request URL sent to the GitHub releases endpoint.
        url: String,
        /// Underlying [`reqwest::Error`].
        #[source]
        src_err: reqwest::Error,
    },

    /// JSON response was valid JSON, and lacked a string `tag_name` field.
    #[error("Missing 'tag_name' string field in GitHub release JSON from '{url}'")]
    MissingTagName {
        /// Endpoint URL whose response lacked `tag_name`.
        url: String,
    },
}

mod constants {
    pub const TAG_NAME: &str = "tag_name";
    pub const VERSION_PREFIX: &str = "v";
}

mod urls {
    pub const REPO_LATEST_RELEASE: &str =
        "https://api.github.com/repos/{org}/{repo}/releases/latest";
}

/// # Errors
///
/// Returns [`GitHubApiError`] if:
/// - The HTTP client cannot be created ([`GitHubApiError::HttpClientCreateFailed`])
/// - The network request fails, returns a non-2xx status, or returns malformed JSON
///   ([`GitHubApiError::HttpRequestFailed`])
/// - The response JSON is missing the string `tag_name` field
///   ([`GitHubApiError::MissingTagName`])
pub async fn try_get_latest_release_tag_from_github(
    org: &str,
    repo: &str,
) -> Result<String, GitHubApiError> {
    let url = urls::REPO_LATEST_RELEASE
        .replace("{org}", org)
        .replace("{repo}", repo);

    try_get_latest_release_tag_from_url(&url).await
}

/// Fetches release JSON from `url` and returns the tag string without a leading `"v"`
/// prefix.
///
/// # Errors
///
/// Returns [`GitHubApiError`] if:
/// - The HTTP client cannot be created ([`GitHubApiError::HttpClientCreateFailed`])
/// - The network request fails, returns a non-2xx status, or returns malformed JSON
///   ([`GitHubApiError::HttpRequestFailed`])
/// - The response JSON is missing the string `tag_name` field
///   ([`GitHubApiError::MissingTagName`])
pub(super) async fn try_get_latest_release_tag_from_url(
    url: &str,
) -> Result<String, GitHubApiError> {
    DEBUG_SCRIPT.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "Fetching latest release tag from GitHub",
            url = %fg_magenta(url)
        );
    });

    let client = http_client::create_http_client()
        .map_err(GitHubApiError::HttpClientCreateFailed)?;
    let response = client.get(url).send().await.map_err(|src_err| {
        GitHubApiError::HttpRequestFailed {
            url: url.to_owned(),
            src_err,
        }
    })?;
    let response = response.error_for_status().map_err(|src_err| {
        GitHubApiError::HttpRequestFailed {
            url: url.to_owned(),
            src_err,
        }
    })?; // Return an error if the status != 2xx.
    let response: serde_json::Value = response.json().await.map_err(|src_err| {
        GitHubApiError::HttpRequestFailed {
            url: url.to_owned(),
            src_err,
        }
    })?;

    let tag_name = match response[constants::TAG_NAME].as_str() {
        Some(tag_name) => tag_name.trim_start_matches(constants::VERSION_PREFIX),
        None => {
            return Err(GitHubApiError::MissingTagName {
                url: url.to_owned(),
            });
        }
    };

    ok!(tag_name.to_owned())
}

#[cfg(test)]
mod tests_github_api {
    use super::*;

    #[tokio::test]
    async fn test_get_latest_tag_strips_v_prefix() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name": "v1.6.5"}"#)
            .expect(1)
            .create_async()
            .await;

        let tag = try_get_latest_release_tag_from_url(&format!(
            "{}/releases/latest",
            server.url()
        ))
        .await
        .unwrap();
        assert_eq2!(tag, "1.6.5");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_latest_tag_missing_tag_name_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"name": "no-tag"}"#)
            .expect(1)
            .create_async()
            .await;

        let req_url = format!("{}/releases/latest", server.url());
        let err = try_get_latest_release_tag_from_url(&req_url)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            GitHubApiError::MissingTagName { ref url } if url == &req_url
        ));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_latest_tag_http_500_returns_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/releases/latest")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;

        let req_url = format!("{}/releases/latest", server.url());
        let res = try_get_latest_release_tag_from_url(&req_url).await;
        assert!(matches!(
            res,
            Err(GitHubApiError::HttpRequestFailed { ref url, ref src_err })
                if url == &req_url && src_err.status() == Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
        ));
        mock.assert_async().await;
    }
}

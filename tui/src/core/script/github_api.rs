// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DEBUG_SCRIPT, fg_magenta, script::http_client};
use miette::IntoDiagnostic;

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
/// Returns an error if:
/// - The HTTP client cannot be created
/// - The network request fails
/// - The GitHub API returns an error status
/// - The response JSON is malformed or missing the `tag_name` field
pub async fn try_get_latest_release_tag_from_github(
    org: &str,
    repo: &str,
) -> miette::Result<String> {
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
/// Returns an error if:
/// - The HTTP client cannot be created
/// - The network request fails
/// - The endpoint returns a non-2xx status
/// - The response JSON is malformed or missing the `tag_name` field
pub(super) async fn try_get_latest_release_tag_from_url(
    url: &str,
) -> miette::Result<String> {
    DEBUG_SCRIPT.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "Fetching latest release tag from GitHub",
            url = %fg_magenta(url)
        );
    });

    let client = http_client::create_client_with_user_agent(None)?;
    let response = client.get(url).send().await.into_diagnostic()?;
    let response = response.error_for_status().into_diagnostic()?; // Return an error if the status != 2xx.
    let response: serde_json::Value = response.json().await.into_diagnostic()?;

    let tag_name = match response[constants::TAG_NAME].as_str() {
        Some(tag_name) => tag_name.trim_start_matches(constants::VERSION_PREFIX),
        None => {
            return Err(miette::miette!(
                "Failed to get tag name from JSON: {:?}",
                response
            ));
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

        let err = try_get_latest_release_tag_from_url(&format!(
            "{}/releases/latest",
            server.url()
        ))
        .await
        .unwrap_err();
        assert!(err.to_string().contains("Failed to get tag name from JSON"));
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

        let res = try_get_latest_release_tag_from_url(&format!(
            "{}/releases/latest",
            server.url()
        ))
        .await;
        assert!(res.is_err());
        mock.assert_async().await;
    }
}

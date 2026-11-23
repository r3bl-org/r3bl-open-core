// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use miette::IntoDiagnostic;

/// Global default HTTP `User-Agent` header sent by R3BL apps and libraries.
pub const R3BL_USER_AGENT: &str = "r3bl/1.0 (https://r3bl.com)";

/// The **only** canonical place in the codebase to construct a [`reqwest::Client`].
///
/// Creates a [`reqwest::Client`] configured with [`R3BL_USER_AGENT`] as its `User-Agent`
/// header. All other code in the workspace must call this function instead of
/// [`reqwest::Client::new`], [`reqwest::Client::builder`],
/// [`reqwest::ClientBuilder::new`], or [`reqwest::get`] (enforced via
/// `clippy::disallowed_methods` in `clippy.toml`), guaranteeing that no outbound HTTP
/// request is ever sent without a `User-Agent` header.
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP client builder fails to build.
/// - TLS backend initialization fails.
pub fn create_http_client() -> miette::Result<reqwest::Client> {
    // Canonical constructor: allowed to call `reqwest::Client::builder` directly.
    #[allow(clippy::disallowed_methods)]
    let it = reqwest::Client::builder()
        .user_agent(R3BL_USER_AGENT)
        .build();
    it.into_diagnostic()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[tokio::test]
    async fn test_create_http_client_sends_r3bl_user_agent() {
        let mut server = mockito::Server::new_async().await;

        let mock = server
            .mock("GET", "/default-ua")
            .match_header("user-agent", R3BL_USER_AGENT)
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let client = create_http_client().unwrap();
        let resp = client
            .get(format!("{}/default-ua", server.url()))
            .send()
            .await
            .unwrap();
        assert_eq2!(resp.status(), StatusCode::OK);
        mock.assert_async().await;
    }
}

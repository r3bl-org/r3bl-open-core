// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use miette::IntoDiagnostic;

mod constants {
    pub const USER_AGENT: &str = "scripting.rs/1.0";
}

/// # Errors
///
/// Returns an error if:
/// - The HTTP client builder fails to build
/// - TLS backend initialization fails
pub fn create_client_with_user_agent(
    user_agent: Option<&str>,
) -> miette::Result<reqwest::Client> {
    let it = reqwest::Client::builder()
        .user_agent(match user_agent {
            Some(ua) => ua.to_owned(),
            None => constants::USER_AGENT.to_owned(),
        })
        .build();
    it.into_diagnostic()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[tokio::test]
    async fn test_create_client_with_default_and_custom_user_agent() {
        let mut server = mockito::Server::new_async().await;

        let default_mock = server
            .mock("GET", "/default-ua")
            .match_header("user-agent", constants::USER_AGENT)
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let default_client = create_client_with_user_agent(None).unwrap();
        let default_resp = default_client
            .get(format!("{}/default-ua", server.url()))
            .send()
            .await
            .unwrap();
        assert_eq2!(default_resp.status(), StatusCode::OK);
        default_mock.assert_async().await;

        let custom_mock = server
            .mock("GET", "/custom-ua")
            .match_header("user-agent", "custom-agent/2.0")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let custom_client =
            create_client_with_user_agent(Some("custom-agent/2.0")).unwrap();
        let custom_resp = custom_client
            .get(format!("{}/custom-ua", server.url()))
            .send()
            .await
            .unwrap();
        assert_eq2!(custom_resp.status(), StatusCode::OK);
        custom_mock.assert_async().await;
    }
}

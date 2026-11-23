// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::DEBUG_ANALYTICS_CLIENT;
use miette::IntoDiagnostic;
use r3bl_tui::{create_http_client, inline_string};
use reqwest::Response;

/// # Errors
///
/// Returns an error if the HTTP client cannot be created or the HTTP GET request fails
/// or returns a non-success status.
pub async fn make_get_request(url: &str) -> miette::Result<Response> {
    let client = create_http_client()?;
    let response = client.get(url).send().await.into_diagnostic()?;
    if response.status().is_success() {
        // Handle successful response.
        DEBUG_ANALYTICS_CLIENT.then(|| {
            tracing::debug!(
                message = "GET request succeeded.",
                response = %inline_string!("{response:#?}")
            );
        });
        Ok(response)
    } else {
        // Handle error response.
        // % is Display, ? is Debug.
        tracing::error!(
            message = "GET request failed.",
            response = %inline_string!("{response:#?}")
        );
        response.error_for_status().into_diagnostic()
    }
}

/// # Errors
///
/// Returns an error if the HTTP client cannot be created or the HTTP POST request fails
/// or returns a non-success status.
pub async fn make_post_request(
    url: &str,
    data: &serde_json::Value,
) -> miette::Result<Response> {
    let client = create_http_client()?;
    let response = client.post(url).json(data).send().await.into_diagnostic()?;
    if response.status().is_success() {
        // Handle successful response.
        DEBUG_ANALYTICS_CLIENT.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "POST request succeeded.",
                response = %inline_string!("{response:#?}")
            );
        });
        Ok(response)
    } else {
        // Handle error response.
        tracing::error!(
            message = "POST request failed.",
            response = %inline_string!("{response:#?}")
        );
        response.error_for_status().into_diagnostic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::{R3BL_USER_AGENT, assert_eq2};
    use reqwest::StatusCode;
    use serde_json::json;

    #[tokio::test]
    async fn test_make_get_request_success_and_http_500() {
        let mut server = mockito::Server::new_async().await;

        let ok_mock = server
            .mock("GET", "/ok")
            .match_header("user-agent", R3BL_USER_AGENT)
            .with_status(200)
            .with_body("ok-payload")
            .expect(1)
            .create_async()
            .await;

        let ok_response = make_get_request(&format!("{}/ok", server.url()))
            .await
            .unwrap();
        assert_eq2!(ok_response.status(), StatusCode::OK);
        assert_eq2!(ok_response.text().await.unwrap(), "ok-payload");
        ok_mock.assert_async().await;

        let err_mock = server
            .mock("GET", "/err")
            .match_header("user-agent", R3BL_USER_AGENT)
            .with_status(500)
            .with_body("server failure")
            .expect(1)
            .create_async()
            .await;

        let err = make_get_request(&format!("{}/err", server.url()))
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains(&StatusCode::INTERNAL_SERVER_ERROR.to_string())
        );
        err_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_make_post_request_json_payload_success_and_http_500() {
        let mut server = mockito::Server::new_async().await;
        let payload = json!({"event": "session_start", "count": 1});

        let ok_mock = server
            .mock("POST", "/events")
            .match_header("user-agent", R3BL_USER_AGENT)
            .match_header("content-type", "application/json")
            .match_body(mockito::Matcher::Json(payload.clone()))
            .with_status(200)
            .with_body("accepted")
            .expect(1)
            .create_async()
            .await;

        let ok_response =
            make_post_request(&format!("{}/events", server.url()), &payload)
                .await
                .unwrap();
        assert_eq2!(ok_response.status(), StatusCode::OK);
        assert_eq2!(ok_response.text().await.unwrap(), "accepted");
        ok_mock.assert_async().await;

        let err_mock = server
            .mock("POST", "/events-err")
            .match_header("user-agent", R3BL_USER_AGENT)
            .match_header("content-type", "application/json")
            .match_body(mockito::Matcher::Json(payload.clone()))
            .with_status(500)
            .expect(1)
            .create_async()
            .await;

        let err = make_post_request(&format!("{}/events-err", server.url()), &payload)
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains(&StatusCode::INTERNAL_SERVER_ERROR.to_string())
        );
        err_mock.assert_async().await;
    }
}

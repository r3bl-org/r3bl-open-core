// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{AnalyticsAction, proxy_machine_id};
use crate::{DEBUG_ANALYTICS_CLIENT, analytics_endpoint};
use r3bl_analytics_schema::AnalyticsEvent;
use r3bl_tui::{AtomicBoolExt, HttpClientError, create_http_client, inline_string};
use reqwest::Response;
use std::sync::atomic::AtomicBool;

/// Runtime gate for outbound analytics event reporting.
///
/// Defaults to `false` (disabled) until the new R3BL telemetry endpoint is deployed.
static ANALYTICS_REPORTING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Disables outbound analytics event reporting for the current process (e.g., when
/// `--no-analytics` is passed).
pub fn disable() { ANALYTICS_REPORTING_ENABLED.set(false); }

/// Spawns a background task to serialize and `POST` an anonymized [`AnalyticsEvent`]
/// when analytics reporting is enabled.
pub fn start_task_to_generate_event(proxy_user_id: String, action: AnalyticsAction) {
    if !ANALYTICS_REPORTING_ENABLED.get() {
        return;
    }

    tokio::spawn(async move {
        let proxy_machine_id =
            proxy_machine_id::load_id_from_file_or_generate_and_save_it().to_string();

        let event =
            AnalyticsEvent::new(proxy_user_id, proxy_machine_id, action.to_string());
        let result_event_json = serde_json::to_value(&event);
        match result_event_json {
            Ok(json) => {
                let result = post_event_impl::try_post_analytics_event(
                    analytics_endpoint::ADD_EVENT_URL,
                    &json,
                )
                .await;
                match result {
                    Ok(_) => {
                        DEBUG_ANALYTICS_CLIENT.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug!(
                                message = "Successfully reported analytics event to analytics backend.",
                                json = %inline_string!("{json:#?}")
                            );
                        });
                    }
                    Err(error) => {
                        // % is Display, ? is Debug.
                        tracing::error!(
                            message = "Could not report analytics event to analytics backend.",
                            error = ?error
                        );
                    }
                }
            }
            Err(error) => {
                // % is Display, ? is Debug.
                tracing::error!(
                    message = "Could not serialize analytics event to JSON.",
                    error = ?error
                );
            }
        }
    });
}

mod post_event_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Errors returned when sending an analytics event `POST` request.
    #[derive(Debug, thiserror::Error)]
    pub enum PostAnalyticsEventError {
        /// Failed to construct the shared [`reqwest::Client`].
        #[error("Failed to create HTTP client")]
        HttpClientCreateFailed(#[source] HttpClientError),

        /// Network transport failure or unsuccessful HTTP status code for `url`.
        #[error("HTTP POST request to '{url}' failed")]
        RequestFailed {
            /// Target request URL.
            url: String,
            /// Underlying [`reqwest::Error`].
            #[source]
            src_err: reqwest::Error,
        },
    }

    /// Sends an HTTP `POST` request with a JSON `payload` to `url` and returns the
    /// [`Response`] if the server responds with a successful HTTP status code (200–299).
    pub async fn try_post_analytics_event(
        url: &str,
        payload: &serde_json::Value,
    ) -> Result<Response, PostAnalyticsEventError> {
        let client = create_http_client()
            .map_err(PostAnalyticsEventError::HttpClientCreateFailed)?;
        let response =
            client
                .post(url)
                .json(payload)
                .send()
                .await
                .map_err(|src_err| PostAnalyticsEventError::RequestFailed {
                    url: url.to_owned(),
                    src_err,
                })?;
        if response.status().is_success() {
            DEBUG_ANALYTICS_CLIENT.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "POST request succeeded.",
                    response = %inline_string!("{response:#?}")
                );
            });
            Ok(response)
        } else {
            tracing::error!(
                message = "POST request failed.",
                response = %inline_string!("{response:#?}")
            );
            response.error_for_status().map_err(|src_err| {
                PostAnalyticsEventError::RequestFailed {
                    url: url.to_owned(),
                    src_err,
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use post_event_impl::{PostAnalyticsEventError, try_post_analytics_event};
    use r3bl_tui::{R3BL_USER_AGENT, assert_eq2};
    use reqwest::StatusCode;
    use serde_json::json;

    #[test]
    fn test_analytics_reporting_parked_by_default() {
        assert!(!ANALYTICS_REPORTING_ENABLED.get());
    }

    #[tokio::test]
    async fn test_post_analytics_event_json_payload_success_and_http_500() {
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
            try_post_analytics_event(&format!("{}/events", server.url()), &payload)
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

        let err_url = format!("{}/events-err", server.url());
        let err = try_post_analytics_event(&err_url, &payload)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            PostAnalyticsEventError::RequestFailed {
                ref url,
                ref src_err,
            } if url == &err_url && src_err.status() == Some(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        err_mock.assert_async().await;
    }
}

/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use r3bl_tui::inline_string;
use reqwest::{Client, Response};

use crate::DEBUG_ANALYTICS_CLIENT_MOD;

/// # Errors
///
/// Returns an error if the HTTP GET request fails or returns a non-success status.
pub async fn make_get_request(
    url: &str,
) -> core::result::Result<Response, reqwest::Error> {
    let client = Client::new();
    let response = client.get(url).send().await?;
    if response.status().is_success() {
        // Handle successful response.
        DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
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
        response.error_for_status()
    }
}

/// # Errors
///
/// Returns an error if the HTTP POST request fails or returns a non-success status.
pub async fn make_post_request(
    url: &str,
    data: &serde_json::Value,
) -> core::result::Result<Response, reqwest::Error> {
    let client = Client::new();
    let response = client.post(url).json(data).send().await?;
    if response.status().is_success() {
        // Handle successful response.
        DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
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
        response.error_for_status()
    }
}

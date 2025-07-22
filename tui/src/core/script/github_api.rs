/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use miette::IntoDiagnostic;

use crate::{fg_magenta, http_client, ok, SCRIPT_MOD_DEBUG};

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
/// - The response JSON is malformed or missing the tag_name field
pub async fn try_get_latest_release_tag_from_github(
    org: &str,
    repo: &str,
) -> miette::Result<String> {
    let url = urls::REPO_LATEST_RELEASE
        .replace("{org}", org)
        .replace("{repo}", repo);

    SCRIPT_MOD_DEBUG.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "Fetching latest release tag from GitHub",
            url = %fg_magenta(&url)
        );
    });

    let client = http_client::create_client_with_user_agent(None)?;
    let response = client.get(url).send().await.into_diagnostic()?;
    let response = response.error_for_status().into_diagnostic()?; // Return an error if the status != 2xx.
    let response: serde_json::Value = response.json().await.into_diagnostic()?;

    let tag_name = match response[constants::TAG_NAME].as_str() {
        Some(tag_name) => tag_name.trim_start_matches(constants::VERSION_PREFIX),
        None => miette::bail!("Failed to get tag name from JSON: {:?}", response),
    };

    ok!(tag_name.to_owned())
}

#[cfg(test)]
mod tests_github_api {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;
    use crate::{return_if_not_interactive_terminal, TTYResult};

    const TIMEOUT: Duration = Duration::from_secs(1);

    /// Do not run this in CI/CD since it makes API calls to github.com.
    #[tokio::test]
    async fn test_get_latest_tag_from_github() {
        return_if_not_interactive_terminal!();

        let org = "cloudflare";
        let repo = "cfssl";

        // Original code w/out timeout.
        // let tag = try_get_latest_release_tag_from_github(org, repo)
        //     .await
        //     .unwrap();

        match timeout(TIMEOUT, try_get_latest_release_tag_from_github(org, repo)).await {
            Ok(Ok(tag)) => {
                assert!(!tag.is_empty());
                println!("Latest tag: {}", fg_magenta(&tag));
            }
            Ok(Err(err)) => {
                // Re-throw the error and fail the test.
                panic!("Error: {err:?}");
            }
            Err(_) => {
                // Timeout does not mean that test has failed. Github is probably slow.
                println!("Timeout");
            }
        }
    }
}

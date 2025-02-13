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

use crossterm::style::Stylize as _;
use miette::IntoDiagnostic;
use r3bl_core::ok;

use crate::http_client;

mod constants {
    pub const TAG_NAME: &str = "tag_name";
    pub const VERSION_PREFIX: &str = "v";
}

pub mod urls {
    pub const REPO_LATEST_RELEASE: &str =
        "https://api.github.com/repos/{org}/{repo}/releases/latest";
}

pub async fn try_get_latest_release_tag_from_github(
    org: &str,
    repo: &str,
) -> miette::Result<String> {
    let url = urls::REPO_LATEST_RELEASE
        .replace("{org}", org)
        .replace("{repo}", repo);

    // % is Display, ? is Debug.
    tracing::debug!(
        "Fetching latest release tag from GitHub" = %url.to_string().magenta()
    );

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
    use r3bl_ansi_color::{TTYResult, is_fully_uninteractive_terminal};

    use super::*;
    use crate::github_api::try_get_latest_release_tag_from_github;

    /// Do not run this in CI/CD since it makes API calls to github.com.
    #[tokio::test]
    async fn test_get_latest_tag_from_github() {
        // This is for CI/CD.
        if let TTYResult::IsNotInteractive = is_fully_uninteractive_terminal() {
            return;
        }
        let org = "cloudflare";
        let repo = "cfssl";
        let tag = try_get_latest_release_tag_from_github(org, repo)
            .await
            .unwrap();
        assert!(!tag.is_empty());
        println!("Latest tag: {}", tag.magenta());
    }
}

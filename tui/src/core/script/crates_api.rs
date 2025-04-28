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

use crate::{crates_api::constants::{CRATE, MAX_VERSION},
            fg_magenta,
            http_client,
            ok,
            SCRIPT_MOD_DEBUG};

mod constants {
    pub const CRATE: &str = "crate";
    pub const MAX_VERSION: &str = "max_version";
}

mod urls {
    pub const CRATE_INFO: &str = "https://crates.io/api/v1/crates/{crate_name}";
}

pub async fn try_get_latest_release_version_from_crates_io(
    crate_name: &str,
) -> miette::Result<String> {
    let url = urls::CRATE_INFO.replace("{crate_name}", crate_name);

    SCRIPT_MOD_DEBUG.then(|| {
        tracing::debug!(
            message = "Fetching latest version from crates.io",
            url = %fg_magenta(&url)
        );
    });

    let client = http_client::create_client_with_user_agent(None)?;
    let response = client.get(url).send().await.into_diagnostic()?;
    let response = response.error_for_status().into_diagnostic()?;
    let response: serde_json::Value = response.json().await.into_diagnostic()?;

    let version = match response[CRATE][MAX_VERSION].as_str() {
        Some(version) => version,
        None => miette::bail!("Failed to get version from JSON: {:?}", response),
    };

    ok!(version.to_owned())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nom::{character::complete::{char, digit0},
              combinator::map_res,
              IResult,
              Parser};
    use tokio::time::timeout;

    use super::*;
    use crate::{console_log, return_if_not_interactive_terminal, TTYResult};

    const TIMEOUT: Duration = Duration::from_secs(1);

    fn version_parser(input: &str) -> IResult<&str, (u64, u64, u64)> {
        (
            map_res(digit0, str::parse),
            char('.'),
            map_res(digit0, str::parse),
            char('.'),
            map_res(digit0, str::parse),
        )
            .parse(input)
            .map(|(remaining, (major, _, minor, _, patch))| {
                (remaining, (major, minor, patch))
            })
    }

    #[tokio::test]
    async fn test_get_latest_version_from_crates_io() {
        return_if_not_interactive_terminal!();

        let crate_name = "serde";

        match timeout(
            TIMEOUT,
            try_get_latest_release_version_from_crates_io(crate_name),
        )
        .await
        {
            Ok(Ok(version)) => {
                assert!(!version.is_empty());
                let parsed_version = version_parser(&version)
                    .map_err(|e| format!("Failed to parse version: {e}"))
                    .expect("Version should be in format X.Y.Z");

                let (remaining, (major, minor, patch)) = parsed_version;
                assert!(remaining.is_empty(), "No remaining characters expected");
                console_log!(version);
                console_log!(major);
                console_log!(minor);
                console_log!(patch);
            }
            Ok(Err(err)) => {
                panic!("Error: {err:?}");
            }
            Err(_) => {
                panic!("Timeout");
            }
        }
    }
}

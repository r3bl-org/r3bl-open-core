// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{SCRIPT_MOD_DEBUG, fg_magenta, ok, script::http_client};
use miette::IntoDiagnostic;

mod constants {
    pub const CRATE: &str = "crate";
    pub const MAX_VERSION: &str = "max_version";
    pub const CRATE_INFO: &str = "https://crates.io/api/v1/crates/{crate_name}";
}

/// # Errors
///
/// Returns an error if:
/// - The HTTP client cannot be created
/// - The network request fails
/// - The crates.io API returns an error status
/// - The response JSON is malformed or missing expected fields
pub async fn try_get_latest_release_version_from_crates_io(
    crate_name: &str,
) -> miette::Result<String> {
    use self::constants::{CRATE, CRATE_INFO, MAX_VERSION};

    let url = CRATE_INFO.replace("{crate_name}", crate_name);

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

    let Some(version) = response[CRATE][MAX_VERSION].as_str() else {
        miette::bail!("Failed to get version from JSON: {:?}", response)
    };

    ok!(version.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TTYResult, console_log, is_partially_uninteractive_terminal};
    use nom::{IResult, Parser,
              character::complete::{char, digit0},
              combinator::map_res};
    use std::time::Duration;
    use tokio::time::timeout;

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
        if let TTYResult::IsNotInteractive = is_partially_uninteractive_terminal() {
            return;
        }

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
            Err(e) => {
                panic!("Timeout: {e:?}");
            }
        }
    }
}

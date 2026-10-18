// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! GitHub Release asset URL formatting and HTTP `HEAD` availability verification against
//! `https://github.com/r3bl-org/r3bl-binaries`.

use super::{get_archive_extension_for_target, R3BL_BINARIES_RELEASE_BASE_URL};
use crate::{SCRIPT_MOD_DEBUG, fg_magenta, script::http_client};
use std::time::Duration;

/// Timeout for the lightweight HTTP `HEAD` check against GitHub's release CDN.
const HEAD_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Formats the release tag for a crate and version (`v<version>-<crate_name>`).
#[must_use]
pub fn format_release_tag(crate_name: &str, version: &str) -> String {
    let clean_ver = version.trim_start_matches('v');
    format!("v{clean_ver}-{crate_name}")
}

/// Formats the archive filename (`<crate_name>-v<version>-<target>.tar.gz`).
#[must_use]
pub fn format_archive_filename(
    crate_name: &str,
    version: &str,
    target_triple: &str,
) -> String {
    let clean_ver = version.trim_start_matches('v');
    let ext = get_archive_extension_for_target(target_triple);
    format!("{crate_name}-v{clean_ver}-{target_triple}.{ext}")
}

/// Formats the direct download URLs `(archive_url, sha256_url)` for a pre-compiled
/// binary release on `https://github.com/r3bl-org/r3bl-binaries`.
///
/// Pattern:
/// `https://github.com/r3bl-org/r3bl-binaries/releases/download/v{ver}-{crate}/{crate}-v{ver}-{target}.{ext}`
#[must_use]
pub fn format_github_release_asset_urls(
    crate_name: &str,
    version: &str,
    target_triple: &str,
) -> (String, String) {
    format_github_release_asset_urls_with_base(
        R3BL_BINARIES_RELEASE_BASE_URL,
        crate_name,
        version,
        target_triple,
    )
}

/// Formats the direct download URLs `(archive_url, sha256_url)` using a customizable
/// base URL (useful for testing or enterprise mirrors).
#[must_use]
pub fn format_github_release_asset_urls_with_base(
    base_url: &str,
    crate_name: &str,
    version: &str,
    target_triple: &str,
) -> (String, String) {
    let tag = format_release_tag(crate_name, version);
    let archive_name = format_archive_filename(crate_name, version, target_triple);
    let base = base_url.trim_end_matches('/');
    let archive_url = format!("{base}/{tag}/{archive_name}");
    let sha256_url = format!("{archive_url}.sha256");
    (archive_url, sha256_url)
}

/// Performs a lightweight HTTP `HEAD` request to check if the pre-compiled release
/// archive exists and is reachable on GitHub's CDN.
///
/// Follows redirects (GitHub redirects release asset URLs with `302` to
/// `objects.githubusercontent.com`) and returns `true` if the final status is `2xx`.
pub async fn check_github_release_asset_available(archive_url: &str) -> bool {
    SCRIPT_MOD_DEBUG.then(|| {
        tracing::debug!(
            message = "Checking GitHub release asset availability via HEAD",
            url = %fg_magenta(archive_url)
        );
    });

    let Ok(client) = http_client::create_client_with_user_agent(None) else {
        return false;
    };

    let Ok(Ok(response)) =
        tokio::time::timeout(HEAD_CHECK_TIMEOUT, client.head(archive_url).send()).await
    else {
        return false;
    };

    response.status().is_success()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_format_release_tag() {
        assert_eq2!(
            format_release_tag("r3bl-cmdr", "0.0.25"),
            "v0.0.25-r3bl-cmdr"
        );
        assert_eq2!(
            format_release_tag("r3bl-cmdr", "v0.0.25"),
            "v0.0.25-r3bl-cmdr"
        );
    }

    #[test]
    fn test_format_github_release_asset_urls_unix() {
        let (archive_url, sha256_url) = format_github_release_asset_urls(
            "r3bl-cmdr",
            "0.0.25",
            "x86_64-unknown-linux-gnu",
        );
        assert_eq2!(
            archive_url,
            "https://github.com/r3bl-org/r3bl-binaries/releases/download/v0.0.25-r3bl-cmdr/r3bl-cmdr-v0.0.25-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq2!(
            sha256_url,
            "https://github.com/r3bl-org/r3bl-binaries/releases/download/v0.0.25-r3bl-cmdr/r3bl-cmdr-v0.0.25-x86_64-unknown-linux-gnu.tar.gz.sha256"
        );
    }

    #[test]
    fn test_format_github_release_asset_urls_windows() {
        let (archive_url, sha256_url) = format_github_release_asset_urls(
            "r3bl-build-infra",
            "0.1.0",
            "x86_64-pc-windows-msvc",
        );
        assert_eq2!(
            archive_url,
            "https://github.com/r3bl-org/r3bl-binaries/releases/download/v0.1.0-r3bl-build-infra/r3bl-build-infra-v0.1.0-x86_64-pc-windows-msvc.tar.gz"
        );
        assert_eq2!(
            sha256_url,
            "https://github.com/r3bl-org/r3bl-binaries/releases/download/v0.1.0-r3bl-build-infra/r3bl-build-infra-v0.1.0-x86_64-pc-windows-msvc.tar.gz.sha256"
        );
    }
}

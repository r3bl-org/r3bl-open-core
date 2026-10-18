// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{archive, semver},
            ReleaseTarget};
use crate::DEBUG_SELF_UPGRADE;
use r3bl_tui::{create_client_with_user_agent, fg_magenta};

/// Sentinel type serving as a namespace for release archive helpers.
///
/// Given a [`ReleaseTarget`], builds the Git tag, `.tar.gz` filename, and download URLs
/// for a pre-compiled binary release, and checks whether that archive exists on the
/// server.
///
/// [`ReleaseTarget`]: crate::self_upgrade::ReleaseTarget
#[derive(Debug)]
pub struct ReleaseAsset;

impl ReleaseAsset {
    /// Formats the release tag for a crate and version (`v<version>-<crate_name>`).
    #[must_use]
    pub fn format_tag(crate_name: &str, version: &str) -> String {
        let clean_ver = version.trim_start_matches(semver::VERSION_PREFIX);
        format!(
            "{}{clean_ver}{}{crate_name}",
            semver::VERSION_PREFIX,
            archive::COMPONENT_SEPARATOR,
        )
    }

    /// Formats the archive filename (`<crate_name>-v<version>-<target>.tar.gz`).
    #[must_use]
    pub fn format_archive_filename(
        crate_name: &str,
        version: &str,
        target: ReleaseTarget,
    ) -> String {
        let clean_ver = version.trim_start_matches(semver::VERSION_PREFIX);
        let ext = target.archive_extension();
        let sep = archive::COMPONENT_SEPARATOR;
        let ver_prefix = semver::VERSION_PREFIX;
        let ext_delim = archive::EXTENSION_DELIMITER;
        format!("{crate_name}{sep}{ver_prefix}{clean_ver}{sep}{target}{ext_delim}{ext}")
    }

    /// Formats the `(archive_url, sha256_url)` paths or download URLs for
    /// [`UpgradeSource::ReleaseArchive`] under `base_prefix` (trimming any trailing `'/'`
    /// from `base_prefix` and any leading `'v'` from `version`).
    ///
    /// # URL Anatomy
    ///
    /// ```text
    /// <base_prefix>/v0.0.25-r3bl-cmdr/r3bl-cmdr-v0.0.25-x86_64-unknown-linux-gnu.tar.gz[.sha256]
    /// └──── 1 ────┘└─────── 2 ──────┘└─────────────────────── 3 ──────────────────────┘└── 4 ──┘
    /// ```
    ///
    /// 1. `base_prefix`: Either:
    ///    - A full HTTP(S) base URL in production (via [`binaries_release_base_url`][1],
    ///      an [`EndpointUrl`], e.g.,
    ///      `"https://github.com/r3bl-org/r3bl-binaries/releases/download"`), yielding
    ///      full download URLs for [`reqwest`].
    ///    - An origin-relative path in mock server tests (e.g., `"/releases/download"`),
    ///      yielding relative request paths for [`mockito`].
    /// 2. Git tag ([`Self::format_tag`]): Repeats version + crate so tags stay unique
    ///    across multiple crates sharing one release repository.
    /// 3. Archive filename ([`Self::format_archive_filename`]): Repeats crate + version +
    ///    [`ReleaseTarget`] so downloaded files remain self-identifying on disk.
    /// 4. Checksum suffix ([`SHA256_EXT`]): Appended to the archive path/URL to form the
    ///    companion `.sha256` path/URL.
    ///
    /// [1]: crate::self_upgrade::JsoncManifest::binaries_release_base_url
    /// [`EndpointUrl`]: crate::self_upgrade::EndpointUrl
    /// [`mockito`]: https://docs.rs/mockito
    /// [`ReleaseTarget`]: crate::self_upgrade::ReleaseTarget
    /// [`reqwest`]: mod@reqwest
    /// [`SHA256_EXT`]: crate::self_upgrade::archive::SHA256_EXT
    /// [`UpgradeSource::ReleaseArchive`]: crate::self_upgrade::UpgradeSource::ReleaseArchive
    #[must_use]
    pub fn format_urls(
        base_prefix: &str,
        crate_name: &str,
        version: &str,
        target: ReleaseTarget,
    ) -> (String, String) {
        let tag = Self::format_tag(crate_name, version);
        let archive_name = Self::format_archive_filename(crate_name, version, target);
        let path_sep = archive::URL_PATH_SEPARATOR;
        let base = base_prefix.trim_end_matches(path_sep);
        let archive_url = format!("{base}{path_sep}{tag}{path_sep}{archive_name}");
        let sha256_url = format!(
            "{archive_url}{}{}",
            archive::EXTENSION_DELIMITER,
            archive::SHA256_EXT,
        );
        (archive_url, sha256_url)
    }

    /// Performs a lightweight HTTP `HEAD` request to check if the pre-compiled release
    /// archive exists and is reachable on the remote release CDN.
    ///
    /// Follows redirects and returns `true` if the final status is `2xx`.
    pub async fn check_available(archive_url: &str) -> bool {
        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Checking release asset availability via HEAD",
                url = %fg_magenta(archive_url)
            );
        });

        let Ok(client) = create_client_with_user_agent(None) else {
            return false;
        };

        let Ok(Ok(response)) = tokio::time::timeout(
            archive::HEAD_CHECK_TIMEOUT,
            client.head(archive_url).send(),
        )
        .await
        else {
            return false;
        };

        response.status().is_success()
    }
}

#[cfg(test)]
mod tests {
    use super::{super::super::{EndpointUrl, JsoncScalar},
                *};
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_format_tag() {
        assert_eq2!(
            ReleaseAsset::format_tag("r3bl-cmdr", "0.0.25"),
            "v0.0.25-r3bl-cmdr"
        );
        assert_eq2!(
            ReleaseAsset::format_tag("r3bl-cmdr", "v0.0.25"),
            "v0.0.25-r3bl-cmdr"
        );
    }

    #[test]
    fn test_format_urls_unix() {
        let base_url = EndpointUrl::try_parse(
            "https://github.com/r3bl-org/r3bl-binaries/releases/download",
        )
        .unwrap();
        let (archive_url, sha256_url) = ReleaseAsset::format_urls(
            &base_url,
            "r3bl-cmdr",
            "0.0.25",
            ReleaseTarget::LinuxX86_64Gnu,
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
    fn test_format_urls_windows() {
        let base_url = EndpointUrl::try_parse(
            "https://github.com/r3bl-org/r3bl-binaries/releases/download",
        )
        .unwrap();
        let (archive_url, sha256_url) = ReleaseAsset::format_urls(
            &base_url,
            "r3bl-build-infra",
            "0.1.0",
            ReleaseTarget::WindowsX86_64Msvc,
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

    #[tokio::test]
    async fn test_check_available_unreachable() {
        let available =
            ReleaseAsset::check_available("http://127.0.0.1:1/nonexistent.tar.gz").await;
        assert!(!available);
    }
}

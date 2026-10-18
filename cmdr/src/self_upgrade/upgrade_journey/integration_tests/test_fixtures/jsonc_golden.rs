// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Golden JSONC manifest generators for test fixtures.

use super::constants::*;
use crate::{crate_distribution_values::{CLOSED_SOURCE, OPEN_SOURCE},
            self_upgrade::{EndpointUrl, versions_manifest_keys::*}};

/// Manifest JSONC payload where `r3bl-cmdr` is at `0.0.25` (`open-source`), matching the
/// local test version `0.0.25` ([`UpToDate`]).
///
/// [`UpToDate`]: crate::self_upgrade::UpToDate
#[must_use]
pub fn manifest_up_to_date() -> String {
    make_manifest_jsonc(
        blocks::TEST_CRATE_NAME,
        blocks::TEST_CURRENT_VERSION,
        OPEN_SOURCE,
        PLACEHOLDER_RELEASE_BASE_URL,
    )
}

/// Manifest JSONC payload where `r3bl-cmdr` has a newer `9.9.9` (`open-source`) release
/// available pointing to `release_base_url`.
#[must_use]
pub fn manifest_upgrade_available_open_source(release_base_url: &EndpointUrl) -> String {
    make_manifest_jsonc(
        blocks::TEST_CRATE_NAME,
        blocks::TEST_UPGRADE_AVAILABLE_VERSION,
        OPEN_SOURCE,
        release_base_url,
    )
}

/// Manifest JSONC payload where `r3bl-cmdr` has a newer `9.9.9` (`closed-source`) release
/// available pointing to `release_base_url`.
#[must_use]
pub fn manifest_upgrade_available_closed_source(
    release_base_url: &EndpointUrl,
) -> String {
    make_manifest_jsonc(
        blocks::TEST_CRATE_NAME,
        blocks::TEST_UPGRADE_AVAILABLE_VERSION,
        CLOSED_SOURCE,
        release_base_url,
    )
}

/// Generates a valid JSONC manifest string from structured arguments.
#[must_use]
fn make_manifest_jsonc(
    crate_name: &str,
    version: &str,
    crate_distribution: &str,
    release_base_url: &str,
) -> String {
    let released_on = blocks::TEST_RELEASED_ON_TIMESTAMP;
    format!(
        r#"
        {{
            "{BINARIES_RELEASE_BASE_URL}": "{release_base_url}",
            "{CRATES}": {{
                "{crate_name}": {{
                "{VERSION}": "{version}",
                "{CRATE_DISTRIBUTION}": "{crate_distribution}",
                "{RELEASED_ON}": "{released_on}"
                }}
            }}
            }}
        "#,
    )
}

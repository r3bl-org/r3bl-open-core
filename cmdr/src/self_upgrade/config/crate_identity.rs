// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::self_upgrade::{CrateName, CrateVersion};

/// Identity of the running binary crate (`crate_name` and `current_version`).
///
/// In binary crates, construct [`UpgradeJourney`] via the [`upgrade_journey!`] macro,
/// which captures `CARGO_PKG_NAME` and `CARGO_PKG_VERSION` from the calling crate's
/// `Cargo.toml` at compile time into a [`CrateIdentity`].
///
/// Programmatic callers (such as unit and integration tests) can also construct this
/// struct directly via [`Self::new()`] when passing it to [`UpgradeConfig::new()`].
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
/// [`upgrade_journey!`]: crate::upgrade_journey
/// [`UpgradeConfig::new()`]: crate::self_upgrade::UpgradeConfig::new
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateIdentity {
    /// Name of the crate as registered in [`R3BL_VERSIONS_JSON_URL`] (e.g.,
    /// `"r3bl-cmdr"`).
    ///
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
    pub crate_name: CrateName,

    /// Current version of the binary compiled from `Cargo.toml` which you can get from
    /// [`env!("CARGO_PKG_VERSION")`][env-macro], which returns something like `0.0.25`.
    /// This is typically parsed into a [`CrateVersion`] via [`CrateVersion::try_from`].
    ///
    /// [env-macro]: std::env!
    pub current_version: CrateVersion,
}

impl CrateIdentity {
    /// Creates a new [`CrateIdentity`] from an explicit crate name and semantic
    /// version.
    ///
    /// In binary crates, prefer the [`upgrade_journey!`] macro so `CARGO_PKG_NAME`
    /// and `CARGO_PKG_VERSION` are extracted automatically at compile time.
    ///
    /// [`upgrade_journey!`]: crate::upgrade_journey
    #[must_use]
    pub fn new(crate_name: impl Into<CrateName>, current_version: CrateVersion) -> Self {
        Self {
            crate_name: crate_name.into(),
            current_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_crate_identity_new() {
        let identity = CrateIdentity::new(
            "r3bl-cmdr",
            CrateVersion {
                major: 1,
                minor: 2,
                patch: 3,
            },
        );

        assert_eq2!(identity.crate_name, CrateName::from("r3bl-cmdr"));
        assert_eq2!(
            identity.current_version,
            CrateVersion {
                major: 1,
                minor: 2,
                patch: 3,
            }
        );
    }
}

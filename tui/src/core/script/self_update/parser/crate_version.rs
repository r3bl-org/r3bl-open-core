// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::semver;
use serde::Deserialize;
use std::{cmp::Ordering,
          fmt::{Display, Formatter},
          str::FromStr};

/// Semantic version of a crate (`MAJOR.MINOR.PATCH`, e.g. `"0.0.25"`).
///
/// Strict [`SemVer`] 3-component format where `major`, `minor`, and `patch` are each
/// unsigned 16-bit integers (`u16`). Pre-release suffixes (such as `"-alpha.1"`), build
/// metadata (such as `"+build"`), and tag prefixes (such as `"v0.1.0"`) are strictly
/// disallowed in `versions.jsonc`.
///
/// [`SemVer`] is used for semantic versioning of crates and comparing versions in strict
/// precedence order (`major`, then `minor`, then `patch`).
///
/// [`SemVer`]: https://semver.org/
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct CrateVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

mod impl_crate_version {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Compares two [`CrateVersion`] values in strict [`SemVer`] precedence order:
    /// `major` first, then `minor`, then `patch`.
    ///
    /// [`SemVer`]: https://semver.org/
    impl Ord for CrateVersion {
        fn cmp(&self, other: &Self) -> Ordering {
            // Compare (major, minor, patch) in SemVer precedence order.
            (self.major, self.minor, self.patch).cmp(&(
                other.major,
                other.minor,
                other.patch,
            ))
        }
    }

    /// Delegates `<`, `<=`, `>`, `>=` comparisons to the total [`Ord`] implementation.
    impl PartialOrd for CrateVersion {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
    }

    /// Formats the version as `"MAJOR.MINOR.PATCH"` (e.g., `"0.0.25"`) for UI strings,
    /// release URLs, and JSONC CST updates.
    impl Display for CrateVersion {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}{}{}{}{}",
                self.major,
                semver::DELIMITER,
                self.minor,
                semver::DELIMITER,
                self.patch
            )
        }
    }

    /// Primary parser for [`CrateVersion`], used both directly by Rust callers
    /// (`"0.0.25".parse::<CrateVersion>()`) and indirectly by [`serde`] deserialization
    /// via [`TryFrom<String>`].
    impl FromStr for CrateVersion {
        type Err = String;

        fn from_str(version_str: &str) -> Result<Self, Self::Err> {
            let parts: Vec<&str> = version_str.trim().split(semver::DELIMITER).collect();

            match parts.as_slice() {
                [major_str, minor_str, patch_str] => {
                    let major = major_str.parse::<u16>().map_err(|_| {
                        format!("Invalid major version in '{version_str}'")
                    })?;
                    let minor = minor_str.parse::<u16>().map_err(|_| {
                        format!("Invalid minor version in '{version_str}'")
                    })?;
                    let patch = patch_str.parse::<u16>().map_err(|_| {
                        format!("Invalid patch version in '{version_str}'")
                    })?;
                    Ok(Self {
                        major,
                        minor,
                        patch,
                    })
                }
                _ => Err(format!(
                    "Invalid semantic version '{version_str}': expected 'MAJOR.MINOR.PATCH'"
                )),
            }
        }
    }

    /// Delegates to [`FromStr`].
    impl TryFrom<&str> for CrateVersion {
        type Error = String;

        fn try_from(version_str: &str) -> Result<Self, Self::Error> {
            version_str.parse()
        }
    }

    /// Invoked by [`serde`] via `#[serde(try_from = "String")]` during JSONC
    /// deserialization; delegates to [`TryFrom<&str>`].
    impl TryFrom<String> for CrateVersion {
        type Error = String;

        fn try_from(s: String) -> Result<Self, Self::Error> { Self::try_from(s.as_str()) }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::JsoncManifest, *};
    use crate::assert_eq2;

    #[test]
    fn test_crate_version_parsing_and_comparison() {
        let v_0_0_9 = CrateVersion::try_from("0.0.9").unwrap();
        let v_0_0_25 = CrateVersion::try_from("0.0.25").unwrap();
        let v_0_1_0 = CrateVersion::try_from("0.1.0").unwrap();
        let v_1_2_3 = CrateVersion::try_from("1.2.3").unwrap();

        assert_eq2!(
            v_0_0_25,
            CrateVersion {
                major: 0,
                minor: 0,
                patch: 25
            }
        );
        assert_eq2!(v_0_0_25.to_string(), "0.0.25");
        assert_eq2!(
            v_1_2_3,
            CrateVersion {
                major: 1,
                minor: 2,
                patch: 3
            }
        );

        // Numerical SemVer ordering (where string comparison would fail on 0.0.9 vs
        // 0.0.25).
        assert!(v_0_0_25 > v_0_0_9);
        assert!(v_0_1_0 > v_0_0_25);
        assert!(v_1_2_3 > v_0_1_0);
        assert_eq2!(
            v_0_0_25.cmp(&CrateVersion::try_from("0.0.25").unwrap()),
            Ordering::Equal
        );

        // Invalid segment counts are rejected by slice pattern matching.
        assert!(CrateVersion::try_from("invalid").is_err());
        assert!(CrateVersion::try_from("1.2").is_err());
        assert!(CrateVersion::try_from("1.2.3.4").is_err());

        // Non-numeric major, minor, or patch segments are rejected.
        assert!(CrateVersion::try_from("x.2.3").is_err());
        assert!(CrateVersion::try_from("1.x.3").is_err());
        assert!(CrateVersion::try_from("1.2.x").is_err());

        // Strict SemVer: v-prefixes, pre-release tags, and build metadata are rejected.
        assert!(CrateVersion::try_from("v0.1.0").is_err());
        assert!(CrateVersion::try_from("1.2.3-alpha.1").is_err());
        assert!(CrateVersion::try_from("1.2.3+build.42").is_err());
        assert!(CrateVersion::try_from("1.2.3-alpha.1+build.42").is_err());

        // Malformed version in JSONC is rejected at deserialization time.
        let bad_jsonc = r#"{
            "schema_version": 1,
            "crates": {
                "r3bl-cmdr": {
                    "version": "not-a-semver",
                    "crate_distribution": "open-source",
                    "released_on": "2026-10-17T00:00:00Z"
                }
            }
        }"#;
        assert!(JsoncManifest::try_parse_jsonc(bad_jsonc).is_err());
    }
}

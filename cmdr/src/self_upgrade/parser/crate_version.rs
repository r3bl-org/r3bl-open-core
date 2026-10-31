// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::semver::{self, MAX_SPLIT_SEGMENTS, RADIX};
use serde::Deserialize;
use smallvec::SmallVec;
use std::{cmp::Ordering,
          fmt::{Display, Formatter},
          str::FromStr};

/// Semantic version of a published R3BL crate (`MAJOR.MINOR.PATCH`, e.g., `"0.0.25"`).
///
/// This struct is stored in [`CrateMetadata::version`] and enforces a strict 3-component
/// [`SemVer`] format where `major`, `minor`, and `patch` are unsigned 16-bit integers
/// (`u16`). Pre-release suffixes (such as `"-alpha.1"`), build metadata (such as
/// `"+build"`), and tag prefixes (such as `"v0.1.0"`) are strictly disallowed in
/// `versions.jsonc`.
///
/// See [`CrateMetadata`] and [`JsoncManifest`] for the complete JSONC schema
/// specification and example.
///
/// [`CrateMetadata::version`]: crate::self_upgrade::CrateMetadata::version
/// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
/// [`JsoncManifest`]: crate::self_upgrade::JsoncManifest
/// [`SemVer`]: https://semver.org/
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct CrateVersion {
    /// Major version number (`MAJOR` in `MAJOR.MINOR.PATCH`), incremented for
    /// incompatible API changes.
    pub major: u16,

    /// Minor version number (`MINOR` in `MAJOR.MINOR.PATCH`), incremented for
    /// backwards-compatible functionality.
    pub minor: u16,

    /// Patch version number (`PATCH` in `MAJOR.MINOR.PATCH`), incremented for
    /// backwards-compatible bug fixes.
    pub patch: u16,
}

/// Failure reason returned by [`CrateVersion::try_parse`], [`FromStr`], and
/// [`TryFrom`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, miette::Diagnostic)]
pub enum VersionParseErrorKind {
    /// Input does not have strictly 3 `'.'`-separated segments (`MAJOR.MINOR.PATCH`).
    #[error("Invalid semantic version: expected 'MAJOR.MINOR.PATCH'")]
    WrongSegmentCount,

    /// `MAJOR` segment is not a valid `u16`.
    #[error("Invalid u16 major version segment")]
    InvalidMajor,

    /// `MINOR` segment is not a valid `u16`.
    #[error("Invalid u16 minor version segment")]
    InvalidMinor,

    /// `PATCH` segment is not a valid `u16` (or contains `-pre` / `+build` suffixes).
    #[error("Invalid u16 patch version segment")]
    InvalidPatch,
}

impl CrateVersion {
    /// Strictly compares `self` against `other` via [`SemVer`] precedence order,
    /// returning `true` if and only if `self > other`.
    ///
    /// If `other` is equal to or ahead of `self`, returns `false` to prevent
    /// accidental downgrades.
    ///
    /// [`SemVer`]: https://semver.org/
    #[must_use]
    pub fn is_newer_than(&self, other: &Self) -> bool { self > other }

    /// Constructs a [`CrateVersion`] at compile time (not runtime) from Cargo's pre-split
    /// `CARGO_PKG_VERSION_*` environment variables inside [`upgrade_journey!`].
    ///
    /// # Design notes
    ///
    /// 1. **Cargo pre-validates `Cargo.toml`**: Cargo validates `[package].version`
    ///    against [`SemVer`] before invoking `rustc`, guaranteeing numeric `major`,
    ///    `minor`, and `patch` strings.
    /// 2. **`const`-compatible parsing**: [`env!`] expands to `&'static str` literals,
    ///    and neither [`str::parse`] nor [`Result::unwrap`] is `const`. Thus,
    ///    [`u16::from_str_radix`] is used.
    /// 3. **Rejecting pre-releases**: Cargo allows pre-release suffixes (e.g.,
    ///    `"0.1.0-alpha.1"`), which `versions.jsonc` disallows, so catch it here.
    ///
    /// For runtime manifest parsing (`versions.jsonc` and tests), use
    /// [`Self::try_parse`] (or [`FromStr`] / [`TryFrom`]) instead.
    ///
    /// # Panics
    ///
    /// Panics (at compile time when called inside `const { ... }`) if `pre` is non-empty
    /// (or if a segment exceeds [`u16::MAX`]).
    ///
    /// [`env!`]: std::env!
    /// [`SemVer`]: https://semver.org/
    /// [`upgrade_journey!`]: crate::upgrade_journey
    #[must_use]
    pub const fn from_cargo_parts(
        major: &str,
        minor: &str,
        patch: &str,
        pre: &str,
    ) -> Self {
        assert!(
            pre.is_empty(),
            "CARGO_PKG_VERSION_PRE must be empty (pre-release versions are disallowed)"
        );
        let Ok(major) = u16::from_str_radix(major, RADIX) else {
            panic!("CARGO_PKG_VERSION_MAJOR must be a valid u16");
        };
        let Ok(minor) = u16::from_str_radix(minor, RADIX) else {
            panic!("CARGO_PKG_VERSION_MINOR must be a valid u16");
        };
        let Ok(patch) = u16::from_str_radix(patch, RADIX) else {
            panic!("CARGO_PKG_VERSION_PATCH must be a valid u16");
        };
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parses a `"MAJOR.MINOR.PATCH"` string slice (from `versions.jsonc` or tests) into
    /// a [`CrateVersion`] at runtime.
    ///
    /// [`FromStr`], [`TryFrom<&str>`], and [`TryFrom<String>`] delegate to this
    /// function.
    ///
    /// # Errors
    ///
    /// Returns [`VersionParseErrorKind`] if `version_str` does not contain strictly 3
    /// `u16` segments delimited by `.` (`MAJOR.MINOR.PATCH`).
    ///
    /// [`TryFrom<&str>`]: TryFrom
    /// [`TryFrom<String>`]: TryFrom
    pub fn try_parse(version_str: &str) -> Result<Self, VersionParseErrorKind> {
        let segments: SmallVec<[&str; MAX_SPLIT_SEGMENTS]> = version_str
            .trim_ascii()
            .splitn(MAX_SPLIT_SEGMENTS, semver::DELIMITER)
            .collect();
        let [major_str, minor_str, patch_str] = segments.as_slice() else {
            return Err(VersionParseErrorKind::WrongSegmentCount);
        };
        Ok(Self {
            major: major_str
                .parse()
                .map_err(|_| VersionParseErrorKind::InvalidMajor)?,
            minor: minor_str
                .parse()
                .map_err(|_| VersionParseErrorKind::InvalidMinor)?,
            patch: patch_str
                .parse()
                .map_err(|_| VersionParseErrorKind::InvalidPatch)?,
        })
    }
}

/// Compares two [`CrateVersion`] values in strict [`SemVer`] precedence order:
/// `major` first, then `minor`, then `patch`.
///
/// [`SemVer`]: https://semver.org/
impl Ord for CrateVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare (major, minor, patch) in SemVer precedence order.
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

/// Delegates `<`, `<=`, `>`, `>=` comparisons to the total [`Ord`] implementation.
impl PartialOrd for CrateVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

/// Formats the version as `"MAJOR.MINOR.PATCH"` (e.g., `"0.0.25"`) for UI strings,
/// release URLs, and JSONC CST updates in
/// [`JsoncManifest::try_update_crate_in_jsonc()`].
///
/// [`JsoncManifest::try_update_crate_in_jsonc()`]: crate::self_upgrade::JsoncManifest::try_update_crate_in_jsonc
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

/// Parses a `"MAJOR.MINOR.PATCH"` string slice via [`str::parse()`].
impl FromStr for CrateVersion {
    type Err = VersionParseErrorKind;

    fn from_str(version_str: &str) -> Result<Self, Self::Err> {
        Self::try_parse(version_str)
    }
}

/// Used by tests.
impl TryFrom<&str> for CrateVersion {
    type Error = VersionParseErrorKind;

    fn try_from(version_str: &str) -> Result<Self, Self::Error> {
        Self::try_parse(version_str)
    }
}

/// Invoked by [`serde`] via `#[serde(try_from = "String")]` during JSONC deserialization.
impl TryFrom<String> for CrateVersion {
    type Error = VersionParseErrorKind;

    fn try_from(s: String) -> Result<Self, Self::Error> { Self::try_parse(s.as_str()) }
}

#[cfg(test)]
mod tests {
    use super::{super::JsoncManifest, *};
    use r3bl_tui::assert_eq2;

    #[test]
    fn test_crate_version_parsing_and_comparison() {
        let v_0_0_9 = CrateVersion::try_from("0.0.9").unwrap();
        let v_0_0_25 = const { CrateVersion::from_cargo_parts("0", "0", "25", "") };
        let v_0_1_0 = CrateVersion::try_from("0.1.0").unwrap();
        let v_1_2_3: CrateVersion = "1.2.3".parse().unwrap();

        assert_eq2!(
            v_0_0_25,
            CrateVersion {
                major: 0,
                minor: 0,
                patch: 25
            }
        );
        assert_eq2!(CrateVersion::from_cargo_parts("0", "0", "25", ""), v_0_0_25);
        assert_eq2!(v_0_0_25.to_string(), "0.0.25");
        assert_eq2!(
            v_1_2_3,
            CrateVersion {
                major: 1,
                minor: 2,
                patch: 3
            }
        );
        assert_eq2!(CrateVersion::try_from("  1.2.3\n"), Ok(v_1_2_3));

        // Numerical SemVer ordering (where string comparison would fail on 0.0.9 vs
        // 0.0.25).
        assert!(v_0_0_25 > v_0_0_9);
        assert!(v_0_1_0 > v_0_0_25);
        assert!(v_1_2_3 > v_0_1_0);
        assert_eq2!(
            v_0_0_25.cmp(&CrateVersion::try_from("0.0.25").unwrap()),
            Ordering::Equal
        );

        // Invalid segment counts (including dot-containing pre-release/build suffixes)
        // are rejected by slice pattern matching.
        for input in [
            "invalid",
            "1.2",
            "1.2.3.4",
            "1.2.3-alpha.1",
            "1.2.3+build.42",
            "1.2.3-alpha.1+build.42",
        ] {
            assert_eq2!(
                CrateVersion::try_from(input),
                Err(VersionParseErrorKind::WrongSegmentCount)
            );
        }

        // Non-numeric major, minor, or patch segments (and v-prefixes, pre-release tags,
        // or build metadata) return the specific segment error kind.
        for input in ["x.2.3", "v0.1.0"] {
            assert_eq2!(
                CrateVersion::try_from(input),
                Err(VersionParseErrorKind::InvalidMajor)
            );
        }
        assert_eq2!(
            CrateVersion::try_from("1.x.3"),
            Err(VersionParseErrorKind::InvalidMinor)
        );
        for input in ["1.2.x", "1.2.3-alpha", "1.2.3+build"] {
            assert_eq2!(
                CrateVersion::try_from(input),
                Err(VersionParseErrorKind::InvalidPatch)
            );
        }

        // Malformed version in JSONC is rejected at deserialization time.
        let bad_jsonc = r#"{
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

    #[test]
    #[should_panic(expected = "CARGO_PKG_VERSION_PRE must be empty")]
    fn test_from_cargo_parts_rejects_prerelease() {
        let _ = CrateVersion::from_cargo_parts("0", "1", "0", "alpha.1");
    }

    #[test]
    fn test_is_newer_than() {
        let v_0_0_25 = CrateVersion::try_from("0.0.25").unwrap();
        let v_0_0_26 = CrateVersion::try_from("0.0.26").unwrap();
        let v_0_1_0 = CrateVersion::try_from("0.1.0").unwrap();
        let v_0_2_0 = CrateVersion::try_from("0.2.0").unwrap();
        let v_1_0_0 = CrateVersion::try_from("1.0.0").unwrap();
        let v_2_0_0 = CrateVersion::try_from("2.0.0").unwrap();

        // Strictly newer -> true
        assert!(v_0_0_26.is_newer_than(&v_0_0_25));
        assert!(v_0_1_0.is_newer_than(&v_0_0_25));
        assert!(v_1_0_0.is_newer_than(&v_0_1_0));

        // Equal -> false
        assert!(!v_0_0_25.is_newer_than(&v_0_0_25));

        // Current version ahead of remote -> false (prevents downgrade!)
        assert!(!v_0_0_25.is_newer_than(&v_0_0_26));
        assert!(!v_0_1_0.is_newer_than(&v_0_2_0));
        assert!(!v_1_0_0.is_newer_than(&v_2_0_0));
    }
}

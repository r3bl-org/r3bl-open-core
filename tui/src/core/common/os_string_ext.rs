// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::type_aliases::EnvMap;
use std::{env::VarsOs, ffi::OsString, path::PathBuf};

/// Extension trait providing zero-allocation for [`OsString`] and [`PathBuf`] containing
/// valid [`UTF-8`] with allocation fallback for invalid [`UTF-8`].
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
pub trait OsStringExt {
    /// Converts `self` into a [`String`].
    ///
    /// Consumes the underlying buffer with zero allocations when the string contains
    /// valid [`UTF-8`]. If invalid [`UTF-8`] bytes are encountered, it safely falls back
    /// to lossy conversion (substituting replacement characters) without panicking.
    ///
    /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
    fn into_string_lossy(self) -> String;
}

mod impl_os_string_ext {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl OsStringExt for OsString {
        fn into_string_lossy(self) -> String {
            self.into_string()
                .unwrap_or_else(|os| os.to_string_lossy().into_owned())
        }
    }

    impl OsStringExt for PathBuf {
        fn into_string_lossy(self) -> String { self.into_os_string().into_string_lossy() }
    }
}

pub trait VarsOsExt {
    fn into_env_map(self) -> EnvMap;
}

mod impl_vars_os_ext {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl VarsOsExt for VarsOs {
        fn into_env_map(self) -> EnvMap {
            self.map(|tuple| (tuple.0.into_string_lossy(), tuple.1.into_string_lossy()))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests_vars_os_ext {
    use super::*;

    #[test]
    fn test_vars_os_into_env_map() {
        let env_map: EnvMap = std::env::vars_os().into_env_map();
        // Verify our iterator mapping accurately converts all keys and values.
        for (key, val) in std::env::vars_os() {
            let key_str = key.into_string_lossy();
            let val_str = val.into_string_lossy();
            assert_eq!(env_map.get(&key_str), Some(&val_str));
        }
    }
}

#[cfg(test)]
mod tests_os_string_ext {
    use super::*;

    #[test]
    fn test_os_string_valid_utf8() {
        let os = OsString::from("hello_world");
        assert_eq!(os.into_string_lossy(), "hello_world");
    }

    #[cfg(unix)]
    #[test]
    fn test_os_string_invalid_utf8_fallback() {
        use std::os::unix::ffi::OsStringExt as _;
        // Triggers the Err fallback branch in unwrap_or_else.
        let os = OsString::from_vec(vec![b'f', 0xFF, b'o']);
        assert_eq!(os.into_string_lossy(), "f\u{FFFD}o");
    }

    #[cfg(windows)]
    #[test]
    fn test_os_string_invalid_utf8_fallback() {
        use std::os::windows::ffi::OsStringExt as _;
        // Triggers the Err fallback branch in unwrap_or_else on Windows (unpaired
        // surrogate).
        let os = OsString::from_wide(&[0x0066, 0xD800, 0x006F]);
        assert_eq!(os.into_string_lossy(), "f\u{FFFD}o");
    }

    #[test]
    fn test_path_buf() {
        let path = PathBuf::from("/path/to/file.txt");
        assert_eq!(path.into_string_lossy(), "/path/to/file.txt");
    }
}

// cspell:words FFFD

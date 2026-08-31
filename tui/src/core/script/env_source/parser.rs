// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::EnvMap;
use crate::NULL_CHAR;

/// Parses a null-delimited (`\0`) byte sequence of environment variables (such as output
/// from [`env -0`]) into an [`EnvMap`].
///
/// Each non-empty record is expected to be in the form `KEY=VALUE`. If multiple `=`
/// characters appear, only the first `=` is used as the delimiter. Records without an `=`
/// character or empty records are skipped.
///
/// # Diagram
///
/// ```text
/// Raw byte buffer from [`env -0`]:
/// ┌─────────┬────┬────────────────────────┬────┬─────────────────┬────┐
/// │ FOO=BAR │ \0 │ MULTI=line1\nline2     │ \0 │ KEY=val=with=eq │ \0 │
/// └────┬────┴────┴───────────┬────────────┴────┴───────┬─────────┴────┘
///      │                     │                         │
///      ▼                     ▼                         ▼
/// ┌─────────┐      ┌───────────────────┐    ┌─────────────────┐
/// │ FOO     │      │ MULTI             │    │ KEY             │
/// │   =     │      │   =               │    │   =             │
/// │ BAR     │      │ line1\nline2      │    │ val=with=eq     │
/// └─────────┘      └───────────────────┘    └─────────────────┘
///  (basic)          (preserves newlines)     (keeps extra '=')
///      │                     │                         │
///      └─────────────────────┼─────────────────────────┘
///                            ▼
///                   EnvMap (FxHashMap)
///             ┌─────────┬───────────────────┐
///             │ Key     │ Value             │
///             ├─────────┼───────────────────┤
///             │ "FOO"   │ "BAR"             │
///             │ "KEY"   │ "val=with=eq"     │
///             │ "MULTI" │ "line1\nline2"    │
///             └─────────┴───────────────────┘
/// ```
///
/// [`env -0`]: https://www.gnu.org/software/coreutils/manual/html_node/env.html
/// [`EnvMap`]: crate::EnvMap
#[must_use]
pub fn parse_env_unix(bytes: &[u8]) -> EnvMap {
    let mut map = EnvMap::default();
    let text = String::from_utf8_lossy(bytes);

    for record in text.split(NULL_CHAR) {
        if record.is_empty() {
            continue;
        }

        if let Some((key, value)) = record.split_once(EQUAL_DELIM) {
            map.insert(key.into(), value.into());
        }
    }

    map
}

/// Parses newline-delimited (`\r\n` or `\n`) environment output from Windows `cmd.exe`
/// `set` into an [`EnvMap`].
///
/// Each non-empty line is expected to be in the form `KEY=VALUE`.
/// - Lines starting with `=` (such as Windows hidden drive variables like `=C:`) are
///   skipped.
/// - The first `=` separates the key from the value.
/// - Trailing `\r` carriage returns are stripped.
///
/// [`EnvMap`]: crate::EnvMap
#[cfg(any(windows, doc))]
#[must_use]
pub fn parse_env_windows(bytes: &[u8]) -> EnvMap {
    let mut map = EnvMap::default();
    let text = String::from_utf8_lossy(bytes);

    for line in text.lines() {
        if line.is_empty() || line.starts_with(EQUAL_DELIM) {
            continue;
        }

        if let Some((key, value)) = line.split_once(EQUAL_DELIM) {
            map.insert(key.into(), value.into());
        }
    }

    map
}

/// Delimiter character (`'='`) separating environment variable keys from values.
pub const EQUAL_DELIM: char = '=';

#[cfg(test)]
mod tests_parser {
    use super::*;

    #[test]
    fn test_parse_env_unix_basic() {
        let input = b"KEY1=VALUE1\0KEY2=VALUE2\0";
        let result = parse_env_unix(input);

        assert_eq!(result.len(), 2);
        assert_eq!(result.get("KEY1"), Some(&"VALUE1".to_string()));
        assert_eq!(result.get("KEY2"), Some(&"VALUE2".to_string()));
    }

    #[test]
    fn test_parse_env_unix_no_trailing_null() {
        let input = b"FOO=BAR\0BAZ=QUX";
        let result = parse_env_unix(input);

        assert_eq!(result.len(), 2);
        assert_eq!(result.get("FOO"), Some(&"BAR".to_string()));
        assert_eq!(result.get("BAZ"), Some(&"QUX".to_string()));
    }

    #[test]
    fn test_parse_env_unix_multiple_equals() {
        let input = b"CONNECTION_STRING=host=localhost;port=5432\0KEY=value=extra\0";
        let result = parse_env_unix(input);

        assert_eq!(
            result.get("CONNECTION_STRING"),
            Some(&"host=localhost;port=5432".to_string())
        );
        assert_eq!(result.get("KEY"), Some(&"value=extra".to_string()));
    }

    #[test]
    fn test_parse_env_unix_multiline_value() {
        let input = b"MULTILINE=line1\nline2\nline3\0SINGLE=one\0";
        let result = parse_env_unix(input);

        assert_eq!(
            result.get("MULTILINE"),
            Some(&"line1\nline2\nline3".to_string())
        );
        assert_eq!(result.get("SINGLE"), Some(&"one".to_string()));
    }

    #[test]
    fn test_parse_env_unix_empty_value() {
        let input = b"EMPTY_VAR=\0OTHER=val\0";
        let result = parse_env_unix(input);

        assert_eq!(result.get("EMPTY_VAR"), Some(&String::new()));
        assert_eq!(result.get("OTHER"), Some(&"val".to_string()));
    }

    #[test]
    fn test_parse_env_unix_empty_input() {
        let input = b"";
        let result = parse_env_unix(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_env_unix_skips_invalid_records() {
        let input = b"NO_EQUALS\0VALID=yes\0\0ANOTHER_NO_EQUALS\0";
        let result = parse_env_unix(input);

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("VALID"), Some(&"yes".to_string()));
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_env_windows_basic() {
        let input = b"ALLUSERSPROFILE=C:\\ProgramData\r\nAPPDATA=C:\\Users\\test\\AppData\\Roaming\r\n";
        let result = parse_env_windows(input);

        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("ALLUSERSPROFILE"),
            Some(&"C:\\ProgramData".to_string())
        );
        assert_eq!(
            result.get("APPDATA"),
            Some(&"C:\\Users\\test\\AppData\\Roaming".to_string())
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_env_windows_skips_pseudo_vars() {
        let input = b"=C:=C:\\Users\\test\r\n=ExitCode=00000000\r\nPATH=C:\\Windows\\system32\r\n";
        let result = parse_env_windows(input);

        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("PATH"),
            Some(&"C:\\Windows\\system32".to_string())
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_env_windows_multiple_equals() {
        let input = b"FOO=a=b=c\r\nBAR=baz\n";
        let result = parse_env_windows(input);

        assert_eq!(result.get("FOO"), Some(&"a=b=c".to_string()));
        assert_eq!(result.get("BAR"), Some(&"baz".to_string()));
    }
}

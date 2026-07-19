// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension trait for [`serde_json::Value`] providing ergonomic parameter extraction and
//! inspection.

use crate::error::McpServerError;
use serde_json::Value;

/// Extension trait providing ergonomic query and parameter extraction methods on
/// [`serde_json::Value`].
pub trait ValueExt {
    /// Returns `true` if this JSON value is [`Value::Null`], an empty [`Value::Array`],
    /// or an empty [`Value::Object`].
    #[must_use]
    fn is_empty_or_null(&self) -> bool;

    /// Extracts a mandatory string parameter from JSON arguments.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerError::InvalidParam`] if the key is missing or not a string.
    fn get_str_param<'a>(&'a self, key: &str) -> Result<&'a str, McpServerError>;

    /// Extracts a mandatory [`u32`] parameter from JSON arguments safely without
    /// overflow.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerError::InvalidParam`] if the key is missing, not numeric, or
    /// exceeds [`u32::MAX`].
    fn get_u32_param(&self, key: &str) -> Result<u32, McpServerError>;
}

impl ValueExt for Value {
    fn is_empty_or_null(&self) -> bool {
        match self {
            Self::Null => true,
            Self::Array(arr) => arr.is_empty(),
            Self::Object(map) => map.is_empty(),
            _ => false,
        }
    }

    fn get_str_param<'a>(&'a self, key: &str) -> Result<&'a str, McpServerError> {
        self.get(key)
            .and_then(Self::as_str)
            .ok_or_else(|| McpServerError::InvalidParam {
                key: key.to_string(),
                reason: "missing or not a string".to_string(),
            })
    }

    fn get_u32_param(&self, key: &str) -> Result<u32, McpServerError> {
        let raw = self.get(key).and_then(Self::as_u64).ok_or_else(|| {
            McpServerError::InvalidParam {
                key: key.to_string(),
                reason: "missing or not a numeric value".to_string(),
            }
        })?;
        u32::try_from(raw).map_err(|e| McpServerError::InvalidParam {
            key: key.to_string(),
            reason: format!("exceeds u32 range: {e}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_empty_or_null() {
        assert!(Value::Null.is_empty_or_null());
        assert!(json!([]).is_empty_or_null());
        assert!(json!({}).is_empty_or_null());

        assert!(!json!([1, 2, 3]).is_empty_or_null());
        assert!(!json!({"key": "val"}).is_empty_or_null());
        assert!(!json!("").is_empty_or_null());
        assert!(!json!("hello").is_empty_or_null());
        assert!(!json!(0).is_empty_or_null());
        assert!(!json!(false).is_empty_or_null());
    }

    #[test]
    fn test_get_str_param_success() {
        let args = json!({
            "file_path": "src/main.rs",
            "name": "foo"
        });

        assert_eq!(args.get_str_param("file_path").unwrap(), "src/main.rs");
        assert_eq!(args.get_str_param("name").unwrap(), "foo");
    }

    #[test]
    fn test_get_str_param_missing_key() {
        let args = json!({ "other": "value" });
        let err = args.get_str_param("file_path").unwrap_err();
        match err {
            McpServerError::InvalidParam { key, reason } => {
                assert_eq!(key, "file_path");
                assert_eq!(reason, "missing or not a string");
            }
            _ => panic!("Expected InvalidParam error"),
        }
    }

    #[test]
    fn test_get_str_param_wrong_type() {
        let args = json!({
            "num": 42,
            "bool": true,
            "null_val": null,
            "arr": ["a"],
            "obj": {"a": 1}
        });

        assert!(matches!(
            args.get_str_param("num"),
            Err(McpServerError::InvalidParam { .. })
        ));
        assert!(matches!(
            args.get_str_param("bool"),
            Err(McpServerError::InvalidParam { .. })
        ));
        assert!(matches!(
            args.get_str_param("null_val"),
            Err(McpServerError::InvalidParam { .. })
        ));
        assert!(matches!(
            args.get_str_param("arr"),
            Err(McpServerError::InvalidParam { .. })
        ));
        assert!(matches!(
            args.get_str_param("obj"),
            Err(McpServerError::InvalidParam { .. })
        ));
    }

    #[test]
    fn test_get_u32_param_success() {
        let args = json!({
            "line": 0,
            "character": 42,
            "max_val": u32::MAX
        });

        assert_eq!(args.get_u32_param("line").unwrap(), 0);
        assert_eq!(args.get_u32_param("character").unwrap(), 42);
        assert_eq!(args.get_u32_param("max_val").unwrap(), u32::MAX);
    }

    #[test]
    fn test_get_u32_param_missing_key() {
        let args = json!({ "other": 10 });
        let err = args.get_u32_param("line").unwrap_err();
        match err {
            McpServerError::InvalidParam { key, reason } => {
                assert_eq!(key, "line");
                assert_eq!(reason, "missing or not a numeric value");
            }
            _ => panic!("Expected InvalidParam error"),
        }
    }

    #[test]
    fn test_get_u32_param_wrong_type_and_negative() {
        let args = json!({
            "str_num": "42",
            "bool_val": true,
            "null_val": null,
            "neg_num": -5
        });

        assert!(matches!(
            args.get_u32_param("str_num"),
            Err(McpServerError::InvalidParam { .. })
        ));
        assert!(matches!(
            args.get_u32_param("bool_val"),
            Err(McpServerError::InvalidParam { .. })
        ));
        assert!(matches!(
            args.get_u32_param("null_val"),
            Err(McpServerError::InvalidParam { .. })
        ));
        assert!(matches!(
            args.get_u32_param("neg_num"),
            Err(McpServerError::InvalidParam { .. })
        ));
    }

    #[test]
    fn test_get_u32_param_overflow() {
        let overflow_val = u64::from(u32::MAX) + 1;
        let args = json!({
            "overflow": overflow_val,
            "max_u64": u64::MAX
        });

        let err = args.get_u32_param("overflow").unwrap_err();
        match err {
            McpServerError::InvalidParam { key, reason } => {
                assert_eq!(key, "overflow");
                assert!(reason.contains("exceeds u32 range"));
            }
            _ => panic!("Expected InvalidParam error"),
        }

        let err2 = args.get_u32_param("max_u64").unwrap_err();
        match err2 {
            McpServerError::InvalidParam { key, reason } => {
                assert_eq!(key, "max_u64");
                assert!(reason.contains("exceeds u32 range"));
            }
            _ => panic!("Expected InvalidParam error"),
        }
    }
}

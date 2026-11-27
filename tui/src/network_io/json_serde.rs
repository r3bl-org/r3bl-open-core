// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module provides JSON serialization helpers for the network protocol.
//!
//! It wraps [`serde_json`] to provide a consistent interface for serializing and
//! deserializing data structures to/from bytes for network transmission.

use crate::{Buffer, BufferAtom};
use serde::{Deserialize, Serialize};

/// Error returned by [`try_serialize`] and [`try_deserialize`] when JSON encoding or
/// decoding fails.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum JsonSerdeError {
    /// Failed to serialize a value into JSON bytes.
    #[error("Failed to serialize JSON payload")]
    SerializeFailed(#[source] serde_json::Error),

    /// Failed to deserialize a value from JSON bytes.
    #[error("Failed to deserialize JSON payload")]
    DeserializeFailed(#[source] serde_json::Error),
}

/// Serialize the payload to JSON bytes. Returns a [Buffer]. `T` must implement the
/// [Serialize] trait.
///
/// # Arguments
///
/// * `data` - The data to serialize.
///
/// # Errors
///
/// Returns [`JsonSerdeError::SerializeFailed`] if the data cannot be serialized to JSON.
pub fn try_serialize<T: Serialize>(data: &T) -> Result<Buffer, JsonSerdeError> {
    serde_json::to_vec(data).map_err(JsonSerdeError::SerializeFailed)
}

/// Deserialize a JSON byte buffer into type `T`.
///
/// # Arguments
///
/// * `buffer` - The buffer to deserialize.
/// * `T` - The type to deserialize to. Must implement the [Deserialize] trait.
///
/// # Errors
///
/// Returns [`JsonSerdeError::DeserializeFailed`] if the buffer contains invalid JSON or
/// cannot be deserialized into `T`.
pub fn try_deserialize<T: for<'de> Deserialize<'de>>(
    buffer: &[BufferAtom],
) -> Result<T, JsonSerdeError> {
    serde_json::from_slice(buffer).map_err(JsonSerdeError::DeserializeFailed)
}

#[cfg(test)]
mod tests_json_serde {
    use super::{try_deserialize, try_serialize};
    use crate::Buffer;
    use pretty_assertions::assert_eq;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestPayload {
        pub id: f32,
        pub description: String,
        pub data: Buffer,
    }

    #[test]
    fn test_json_serde() -> miette::Result<()> {
        let value = TestPayload {
            id: 12.0,
            description: "foo bar".to_string(),
            data: vec![0, 1, 2],
        };

        // Struct (MyValueType) -> Bytes (Buffer).
        let res_struct_to_bytes = try_serialize(&value);

        assert!(res_struct_to_bytes.is_ok());
        let struct_to_bytes: Buffer = res_struct_to_bytes?;
        println!("{struct_to_bytes:?}");

        // Bytes (Buffer) -> Struct (MyValueType).
        let res = try_deserialize::<TestPayload>(&struct_to_bytes);
        assert!(res.is_ok());
        let result_struct_from_bytes = res?;
        let struct_from_bytes: TestPayload = result_struct_from_bytes;
        println!("{struct_from_bytes:?}");

        assert_eq!(value, struct_from_bytes);

        ok!()
    }
}

// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module provides JSON serialization helpers for the network protocol.
//!
//! It wraps [`serde_json`] to provide a consistent interface for serializing and
//! deserializing data structures to/from bytes for network transmission.

use crate::{Buffer, BufferAtom};
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};

/// Serialize the payload to JSON bytes. Returns a [Buffer]. `T` must implement the
/// [Serialize] trait.
///
/// # Arguments
///
/// * `data` - The data to serialize.
///
/// # Errors
///
/// Returns an error if:
/// - The data cannot be serialized to JSON format
/// - The serialization encounters an I/O error
pub fn try_serialize<T: Serialize>(data: &T) -> miette::Result<Buffer> {
    serde_json::to_vec(data).into_diagnostic()
}

/// Deserialize a JSON byte buffer into type `T`. Returns a [`miette::Result`] of `T`.
///
/// # Arguments
///
/// * `buffer` - The buffer to deserialize.
/// * `T` - The type to deserialize to. Must implement the [Deserialize] trait.
///
/// # Errors
///
/// Returns an error if:
/// - The buffer contains invalid JSON data
/// - The data cannot be deserialized into type T
/// - The buffer is corrupted or incomplete
pub fn try_deserialize<T: for<'de> Deserialize<'de>>(
    buffer: &[BufferAtom],
) -> miette::Result<T> {
    serde_json::from_slice(buffer).into_diagnostic()
}

#[cfg(test)]
mod tests_json_serde {
    use crate::{Buffer, json_serde};
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
        let res_struct_to_bytes = json_serde::try_serialize(&value);

        assert!(res_struct_to_bytes.is_ok());
        let struct_to_bytes: Buffer = res_struct_to_bytes?;
        println!("{struct_to_bytes:?}");

        // Bytes (Buffer) -> Struct (MyValueType).
        let res = json_serde::try_deserialize::<TestPayload>(&struct_to_bytes);
        assert!(res.is_ok());
        let result_struct_from_bytes = res?;
        let struct_from_bytes: TestPayload = result_struct_from_bytes;
        println!("{struct_from_bytes:?}");

        assert_eq!(value, struct_from_bytes);

        Ok(())
    }
}

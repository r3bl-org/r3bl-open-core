/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! This is a module to make it easier to use `bincode` with `serde`.
//!
//! More info:
//! - [bincode v2.x migration guide](https://github.com/bincode-org/bincode/blob/trunk/docs/migration_guide.md)

use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};

use crate::{Buffer, BufferAtom};

/// Serialize the payload using the [bincode] crate. Returns a [Buffer]. `T` must
/// implement the [Serialize] trait.
///
/// # Arguments
///
/// * `data` - The data to serialize.
pub fn try_serialize<T: Serialize>(data: &T) -> miette::Result<Buffer> {
    let buffer = bincode::serde::encode_to_vec(data, get_config()).into_diagnostic()?;
    Ok(buffer)
}

/// You must provide the `T` type to deserialize the payload. Deserialize the payload
/// (of &[Buffer]) using the [bincode] crate. Returns a [`miette::Result`] of `T`.
///
/// # Arguments
///
/// * `buffer` - The buffer to deserialize.
/// * `T` - The type to deserialize to. Must implement the [Deserialize] trait.
pub fn try_deserialize<T: for<'de> Deserialize<'de>>(
    buffer: &[BufferAtom],
) -> miette::Result<T> {
    let res = bincode::serde::decode_from_slice::<T, _>(buffer, get_config());
    match res {
        Ok((payload, _bytes_read)) => Ok(payload),
        Err(err) => {
            let err_msg = format!("{err:?}");
            miette::bail!("Failed to deserialize: {}", err_msg)
        }
    }
}

fn get_config() -> bincode::config::Configuration { bincode::config::standard() }

/// More info:
/// - [what is bincode](https://docs.rs/bincode/latest/bincode/)
/// - [what is codec](https://g.co/bard/share/cbf732b548c7)
///
/// [bincode] is a crate for encoding and decoding using a tiny binary serialization
/// strategy. Using it, you can easily go from having an struct / object in memory,
/// quickly serialize it to bytes, and then deserialize it back just as fast!
#[cfg(test)]
mod tests_bincode_serde {
    use pretty_assertions::assert_eq;
    use serde::{Deserialize, Serialize};

    use crate::{bincode_serde, Buffer};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestPayload {
        pub id: f32,
        pub description: String,
        pub data: Buffer,
    }

    #[test]
    fn test_bincode_serde() -> miette::Result<()> {
        let value = TestPayload {
            id: 12.0,
            description: "foo bar".to_string(),
            data: vec![0, 1, 2],
        };

        // Struct (MyValueType) -> Bytes (Buffer).
        let res_struct_to_bytes = bincode_serde::try_serialize(&value);

        assert!(res_struct_to_bytes.is_ok());
        let struct_to_bytes: Buffer = res_struct_to_bytes?;
        println!("{struct_to_bytes:?}");

        // Bytes (Buffer) -> Struct (MyValueType).
        let res = bincode_serde::try_deserialize::<TestPayload>(&struct_to_bytes);
        assert!(res.is_ok());
        let result_struct_from_bytes = res?;
        let struct_from_bytes: TestPayload = result_struct_from_bytes;
        println!("{struct_from_bytes:?}");

        assert_eq!(value, struct_from_bytes);

        Ok(())
    }
}

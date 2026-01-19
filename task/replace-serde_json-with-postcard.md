<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

**Table of Contents** _generated with [DocToc](https://github.com/thlorenz/doctoc)_

- [Replace serde_json with Postcard in Network Protocol](#replace-serde_json-with-postcard-in-network-protocol)
  - [Overview](#overview)
  - [Why Postcard?](#why-postcard)
  - [Implementation Plan](#implementation-plan)
    - [Step 0: Add Postcard Dependency](#step-0-add-postcard-dependency)
    - [Step 1: Rename Module](#step-1-rename-module)
    - [Step 2: Update postcard_serde.rs Implementation](#step-2-update-postcard_serders-implementation)
    - [Step 3: Update mod.rs](#step-3-update-modrs)
    - [Step 4: Verification](#step-4-verification)
  - [File Changes Summary](#file-changes-summary)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Replace serde_json with Postcard in Network Protocol

## Overview

This task replaces `serde_json` with `postcard` for the network protocol serialization in
`tui/src/network_io/`.

**Note:** This is a follow-up to the bincode removal task (`task/done/remove-bincode.md`) which
initially replaced bincode with serde_json. We're now replacing serde_json with postcard for better
performance and consistency with our syntect fork.

**Scope:** Only the network protocol layer. KV storage continues to use serde_json (via
`kv::Json`), which is fine for that use case.

## Why Postcard?

1. **Binary format** - Compact binary output (smaller than JSON, similar to bincode)
2. **Performance** - Faster serialization/deserialization than text-based JSON
3. **Active maintenance & funding** - Unlike bincode (which was abandoned due to maintainer
   harassment), Postcard has strong maintainer support:
   - **Author:** James Munns (OneVariable UG), founding member of Rust Embedded Working Group
   - **Mozilla funded:** The 1.0 release was sponsored by Mozilla Corporation
   - **Current version:** v1.1.3 (July 2025), with 378 commits and active development
   - **Community adoption:** 1.3k GitHub stars, used by 7,800+ projects
   - **Ecosystem involvement:** James is active in Rust governance and speaks at major conferences
4. **Serde compatible** - Drop-in replacement at the serialization layer
5. **Consistency** - Matches our syntect fork (see `task/pending/fork-syntect-and-remove-bincode-dep.md`)

## Implementation Plan

### Step 0: Add Postcard Dependency

In `tui/Cargo.toml`, add postcard:

```toml
postcard = { version = "1.0", features = ["use-std"] }
```

### Step 1: Rename Module

```bash
mv tui/src/network_io/json_serde.rs tui/src/network_io/postcard_serde.rs
```

### Step 2: Update postcard_serde.rs Implementation

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module provides Postcard serialization helpers for the network protocol.
//!
//! It wraps [`postcard`] to provide a consistent interface for serializing and
//! deserializing data structures to/from bytes for network transmission.

use crate::{Buffer, BufferAtom};
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};

/// Serialize the payload to Postcard bytes. Returns a [Buffer]. `T` must implement the
/// [Serialize] trait.
///
/// # Arguments
///
/// * `data` - The data to serialize.
///
/// # Errors
///
/// Returns an error if:
/// - The data cannot be serialized to Postcard format
/// - The serialization encounters an I/O error
pub fn try_serialize<T: Serialize>(data: &T) -> miette::Result<Buffer> {
    postcard::to_stdvec(data).into_diagnostic()
}

/// Deserialize a Postcard byte buffer into type `T`. Returns a [`miette::Result`] of `T`.
///
/// # Arguments
///
/// * `buffer` - The buffer to deserialize.
/// * `T` - The type to deserialize to. Must implement the [Deserialize] trait.
///
/// # Errors
///
/// Returns an error if:
/// - The buffer contains invalid Postcard data
/// - The data cannot be deserialized into type T
/// - The buffer is corrupted or incomplete
pub fn try_deserialize<T: for<'de> Deserialize<'de>>(
    buffer: &[BufferAtom],
) -> miette::Result<T> {
    postcard::from_bytes(buffer).into_diagnostic()
}

#[cfg(test)]
mod tests_postcard_serde {
    use crate::{Buffer, postcard_serde};
    use pretty_assertions::assert_eq;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestPayload {
        pub id: f32,
        pub description: String,
        pub data: Buffer,
    }

    #[test]
    fn test_postcard_serde() -> miette::Result<()> {
        let value = TestPayload {
            id: 12.0,
            description: "foo bar".to_string(),
            data: vec![0, 1, 2],
        };

        // Struct (MyValueType) -> Bytes (Buffer).
        let res_struct_to_bytes = postcard_serde::try_serialize(&value);

        assert!(res_struct_to_bytes.is_ok());
        let struct_to_bytes: Buffer = res_struct_to_bytes?;
        println!("{struct_to_bytes:?}");

        // Bytes (Buffer) -> Struct (MyValueType).
        let res = postcard_serde::try_deserialize::<TestPayload>(&struct_to_bytes);
        assert!(res.is_ok());
        let result_struct_from_bytes = res?;
        let struct_from_bytes: TestPayload = result_struct_from_bytes;
        println!("{struct_from_bytes:?}");

        assert_eq!(value, struct_from_bytes);

        Ok(())
    }
}
```

### Step 3: Update mod.rs

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach.
pub mod compress;
pub mod postcard_serde;
pub mod length_prefix_protocol;
pub mod protocol_types;

// Re-export.
pub use compress::*;
pub use postcard_serde::*;
pub use length_prefix_protocol::*;
pub use protocol_types::*;
```

### Step 4: Verification

```bash
# Build
cargo check -p r3bl_tui

# Run tests
cargo test -p r3bl_tui

# Check clippy
cargo clippy -p r3bl_tui

# Build docs
cargo doc -p r3bl_tui --no-deps
```

## File Changes Summary

| File                               | Action                             |
| ---------------------------------- | ---------------------------------- |
| `tui/Cargo.toml`                   | Add `postcard` dependency          |
| `tui/src/network_io/json_serde.rs` | Rename to `postcard_serde.rs`      |
| `tui/src/network_io/mod.rs`        | Update module name and re-export   |

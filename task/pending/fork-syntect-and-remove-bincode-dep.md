<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

**Table of Contents** _generated with [DocToc](https://github.com/thlorenz/doctoc)_

- [Fork Syntect and Remove Bincode Dependency](#fork-syntect-and-remove-bincode-dependency)
  - [Overview](#overview)
  - [Why Fork?](#why-fork)
  - [Implementation Plan](#implementation-plan)
    - [Step 0: Vendor Syntect Source](#step-0-vendor-syntect-source)
    - [Step 1: Add to Workspace](#step-1-add-to-workspace)
    - [Step 2: Modify Syntect's Cargo.toml](#step-2-modify-syntects-cargotoml)
    - [Step 3: Modify dumps.rs](#step-3-modify-dumpsrs)
      - [3.1 Update Imports](#31-update-imports)
      - [3.2 Update dump_to_uncompressed_file](#32-update-dump_to_uncompressed_file)
      - [3.3 Update from_uncompressed_dump_file](#33-update-from_uncompressed_dump_file)
      - [3.4 Update dump_to_file (compressed)](#34-update-dump_to_file-compressed)
      - [3.5 Update from_dump_file (compressed)](#35-update-from_dump_file-compressed)
      - [3.6 Update from_binary (for embedded assets)](#36-update-from_binary-for-embedded-assets)
      - [3.7 Update from_uncompressed_data](#37-update-from_uncompressed_data)
    - [Step 4: Regenerate Embedded Assets](#step-4-regenerate-embedded-assets)
      - [4.1 Create Asset Regeneration Script](#41-create-asset-regeneration-script)
      - [4.2 Run Regeneration](#42-run-regeneration)
    - [Step 5: Update tui/Cargo.toml](#step-5-update-tuicargotoml)
    - [Step 6: Verification](#step-6-verification)
  - [File Changes Summary](#file-changes-summary)
  - [Risks and Mitigations](#risks-and-mitigations)
  - [Estimated Effort](#estimated-effort)
  - [Future Considerations](#future-considerations)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Fork Syntect and Remove Bincode Dependency

## Overview

Syntect is a Rust library for syntax highlighting using Sublime Text syntax definitions. It
currently depends on `bincode v1.3.3` for serializing/deserializing pre-compiled syntax and theme
dumps. Since bincode has been abandoned, we want to fork syntect and replace bincode with
`serde_json`.

**Repository:** https://github.com/trishume/syntect
**License:** MIT
**Current version used:** 5.3.0
**Upstream issue:** https://github.com/trishume/syntect/issues/606

**Approach:** Vendor syntect source directly into our workspace as a local crate. This is NOT
published to crates.io - it's purely for internal use to eliminate the bincode dependency.

## Why Fork?

The `dump-load` and `dump-create` features in syntect use bincode to:

1. Serialize `SyntaxSet` and `ThemeSet` structs to binary format
2. Compress them with flate2 (zlib)
3. Embed them as `include_bytes!` assets in the library

These features are transitively required by `default-syntaxes`, `default-themes`, and even `parsing`
— making it impossible to disable bincode while using syntect's core functionality.

## Implementation Plan

### Step 0: Vendor Syntect Source

Clone syntect v5.3.0 and remove its git history (we don't need subtree complexity):

```bash
cd /home/nazmul/github/roc

# Clone specific version
git clone --depth 1 --branch v5.3.0 \
    https://github.com/trishume/syntect.git \
    syntect-fork

# Remove git history - this becomes a vendored copy
rm -rf syntect-fork/.git
```

Final structure:

```
syntect-fork/
├── Cargo.toml      (modified syntect Cargo.toml)
├── src/
│   ├── lib.rs
│   ├── dumps.rs    (modified to use serde_json)
│   └── ...
├── assets/         (regenerated as JSON)
└── README.md
```

### Step 1: Add to Workspace

In the root `Cargo.toml`, add the vendored crate to the workspace:

```toml
[workspace]
members = ["analytics_schema", "cmdr", "tui", "build-infra", "syntect-fork"]
```

Add a patch to redirect all `syntect` dependencies to our fork:

```toml
[patch.crates-io]
syntect = { path = "./syntect-fork" }
```

### Step 2: Modify Syntect's Cargo.toml

In `syntect-fork/Cargo.toml`:

1. Change the package name to avoid confusion:

```toml
[package]
name = "syntect"  # Keep as "syntect" so patch works seamlessly
version = "5.3.0-fork"  # Add -fork suffix to indicate modification
```

2. Replace bincode with serde_json:

```toml
# OLD
bincode = { version = "1.0", optional = true }

# NEW
serde_json = { version = "1.0", optional = true }
```

3. Update the feature flags:

```toml
# OLD
dump-load = ["flate2", "bincode"]
dump-create = ["flate2", "bincode"]

# NEW
dump-load = ["flate2", "serde_json"]
dump-create = ["flate2", "serde_json"]
```

4. Add workspace lints:

```toml
[lints]
workspace = true
```

### Step 3: Modify dumps.rs

Location: `syntect-fork/src/dumps.rs`

#### 3.1 Update Imports

```rust
// OLD
use bincode::Result as BincodeResult;
use bincode::{deserialize_from, serialize_into};

// NEW
use serde_json::Result as JsonResult;
use std::io::{BufReader, BufWriter};
```

#### 3.2 Update dump_to_uncompressed_file

```rust
// OLD
pub fn dump_to_uncompressed_file<T: Serialize, P: AsRef<Path>>(
    o: &T,
    path: P,
) -> BincodeResult<()> {
    let f = File::create(path)?;
    serialize_into(f, o)
}

// NEW
pub fn dump_to_uncompressed_file<T: Serialize, P: AsRef<Path>>(
    o: &T,
    path: P,
) -> JsonResult<()> {
    let f = File::create(path)?;
    let writer = BufWriter::new(f);
    serde_json::to_writer(writer, o)
}
```

#### 3.3 Update from_uncompressed_dump_file

```rust
// OLD
pub fn from_uncompressed_dump_file<T: DeserializeOwned, P: AsRef<Path>>(
    path: P,
) -> BincodeResult<T> {
    let f = File::open(path)?;
    deserialize_from(f)
}

// NEW
pub fn from_uncompressed_dump_file<T: DeserializeOwned, P: AsRef<Path>>(
    path: P,
) -> JsonResult<T> {
    let f = File::open(path)?;
    let reader = BufReader::new(f);
    serde_json::from_reader(reader)
}
```

#### 3.4 Update dump_to_file (compressed)

```rust
// OLD
pub fn dump_to_file<T: Serialize, P: AsRef<Path>>(o: &T, path: P) -> BincodeResult<()> {
    let f = File::create(path)?;
    let mut encoder = ZlibEncoder::new(f, Compression::best());
    serialize_into(&mut encoder, o)?;
    encoder.finish()?;
    Ok(())
}

// NEW
pub fn dump_to_file<T: Serialize, P: AsRef<Path>>(o: &T, path: P) -> JsonResult<()> {
    let f = File::create(path)?;
    let encoder = ZlibEncoder::new(f, Compression::best());
    let writer = BufWriter::new(encoder);
    serde_json::to_writer(writer, o)?;
    // Note: BufWriter flush happens on drop, encoder finish happens on drop
    Ok(())
}
```

#### 3.5 Update from_dump_file (compressed)

```rust
// OLD
pub fn from_dump_file<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> BincodeResult<T> {
    let f = File::open(path)?;
    let decoder = ZlibDecoder::new(f);
    deserialize_from(decoder)
}

// NEW
pub fn from_dump_file<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> JsonResult<T> {
    let f = File::open(path)?;
    let decoder = ZlibDecoder::new(f);
    let reader = BufReader::new(decoder);
    serde_json::from_reader(reader)
}
```

#### 3.6 Update from_binary (for embedded assets)

```rust
// OLD
pub fn from_binary<T: DeserializeOwned>(v: &[u8]) -> T {
    deserialize_from(v).unwrap()
}

// NEW
pub fn from_binary<T: DeserializeOwned>(v: &[u8]) -> T {
    serde_json::from_slice(v).unwrap()
}
```

#### 3.7 Update from_uncompressed_data

```rust
// OLD
pub fn from_uncompressed_data<T: DeserializeOwned>(v: &[u8]) -> BincodeResult<T> {
    deserialize_from(v)
}

// NEW
pub fn from_uncompressed_data<T: DeserializeOwned>(v: &[u8]) -> JsonResult<T> {
    serde_json::from_slice(v)
}
```

### Step 4: Regenerate Embedded Assets

The embedded assets in `syntect-fork/assets/` are bincode-serialized. We need to regenerate them as
JSON.

#### 4.1 Create Asset Regeneration Script

Create `syntect-fork/examples/regenerate_assets.rs`:

```rust
//! Regenerate embedded assets as JSON-serialized dumps.
//!
//! Run with: cargo run --example regenerate_assets --features dump-create

use syntect::dumps::dump_to_file;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

fn main() {
    // Regenerate syntax sets
    let ss = SyntaxSet::load_defaults_newlines();
    dump_to_file(&ss, "assets/default_newlines.packdump").unwrap();
    println!("Regenerated: assets/default_newlines.packdump");

    let ss_no_newlines = SyntaxSet::load_defaults_nonewlines();
    dump_to_file(&ss_no_newlines, "assets/default_nonewlines.packdump").unwrap();
    println!("Regenerated: assets/default_nonewlines.packdump");

    // Regenerate theme set
    let ts = ThemeSet::load_defaults();
    dump_to_file(&ts, "assets/default.themedump").unwrap();
    println!("Regenerated: assets/default.themedump");

    println!("\nAll assets regenerated successfully!");
}
```

#### 4.2 Run Regeneration

```bash
cd syntect-fork
cargo run --example regenerate_assets --features dump-create,default-syntaxes,default-themes
```

### Step 5: Update tui/Cargo.toml

No changes needed! The `[patch.crates-io]` in the workspace root automatically redirects:

```toml
# This stays the same - patch handles the redirect
syntect = "5.3.0"
```

Remove the comment about bincode since it will no longer apply:

```toml
# OLD
# Syntax highlighting.
# Note: syntect depends on bincode v1.3.3 for loading embedded syntax/theme dumps.
# This is unavoidable without replacing syntect entirely.
syntect = "5.3.0"

# NEW
# Syntax highlighting (using vendored fork with serde_json instead of bincode).
syntect = "5.3.0"
```

### Step 6: Verification

```bash
# Check no bincode in dependency tree
cargo tree -p r3bl_tui -i bincode
# Should output: "bincode not found in r3bl_tui"

# Run tests
cargo test -p r3bl_tui

# Run syntect's own tests
cargo test -p syntect

# Check clippy
cargo clippy -p r3bl_tui
cargo clippy -p syntect

# Build docs
cargo doc -p syntect --no-deps
```

## File Changes Summary

| File                                 | Action                                     |
| ------------------------------------ | ------------------------------------------ |
| `Cargo.toml` (workspace)             | Add `syntect-fork` to members, add patch   |
| `syntect-fork/Cargo.toml`            | Replace `bincode` with `serde_json`        |
| `syntect-fork/src/dumps.rs`          | Replace bincode calls with serde_json      |
| `syntect-fork/examples/regenerate_assets.rs` | Create asset regeneration script   |
| `syntect-fork/assets/*.packdump`     | Regenerate as JSON                         |
| `syntect-fork/assets/*.themedump`    | Regenerate as JSON                         |
| `tui/Cargo.toml`                     | Update comment (optional)                  |

## Risks and Mitigations

| Risk                            | Likelihood | Mitigation                                      |
| ------------------------------- | ---------- | ----------------------------------------------- |
| JSON assets larger than bincode | High       | flate2 compression mitigates; test actual size  |
| Slower deserialization          | Medium     | Acceptable for one-time startup load            |
| Upstream syntect updates        | Low        | Pin to v5.3.0; merge upstream changes as needed |
| Serde compatibility issues      | Low        | syntect already uses serde traits               |

## Estimated Effort

| Task                           | Time          |
| ------------------------------ | ------------- |
| Vendor syntect source          | 15 min        |
| Modify Cargo.toml and dumps.rs | 1 hour        |
| Regenerate assets              | 30 min        |
| Testing and debugging          | 1-2 hours     |
| **Total**                      | **2-3 hours** |

## Future Considerations

1. **Upstream PR**: Consider submitting a PR to syntect adding `serde_json` as an alternative
   serialization backend (feature-gated). Reference issue #606.

2. **Asset size monitoring**: Track the size difference between bincode and JSON assets.

3. **Lazy loading**: If startup time becomes an issue, consider lazy-loading syntax definitions.

4. **Upstream merge**: If syntect ever adds JSON support upstream, we can switch back to the
   published crate and remove our fork.

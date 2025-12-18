# Plan: Replace Bincode with JSON Serialization

## Background

The `bincode` crate has been abandoned (maintainer harassment led to discontinuation). Version 3.0.0
is a "tombstone" release. We need to remove all bincode dependencies and switch to JSON.

**Two areas affected:**

1. `tui/src/network_io/` - Direct bincode usage for protocol serialization
2. `tui/src/core/storage/kv.rs` - Indirect bincode via `kv` crate's `bincode-value` feature

**Decision:** Use `serde_json` (already a dependency) for both. No data migration needed.

---

## Implementation Steps

### Phase 1: Network Protocol (`network_io/`)

#### Step 1.1: Rename module

- `tui/src/network_io/bincode_serde.rs` → `tui/src/network_io/json_serde.rs`

#### Step 1.2: Update `json_serde.rs` implementation

Replace bincode calls with serde_json:

```rust
// OLD
bincode::serde::encode_to_vec(data, get_config())
bincode::serde::decode_from_slice::<T, _>(buffer, get_config())

// NEW
serde_json::to_vec(data)
serde_json::from_slice(buffer)
```

Remove `get_config()` function (bincode-specific).

#### Step 1.3: Update `mod.rs`

```rust
// OLD
pub mod bincode_serde;
pub use bincode_serde::*;

// NEW
pub mod json_serde;
pub use json_serde::*;
```

#### Step 1.4: Update doc comments

- Module-level docs: reference JSON instead of bincode
- Function docs: update error descriptions

#### Step 1.5: Update tests

- Rename `tests_bincode_serde` → `tests_json_serde`
- Rename `test_bincode_serde` → `test_json_serde`

---

### Phase 2: KV Storage (`core/storage/`)

#### Step 2.1: Update `Cargo.toml`

```toml
# OLD
kv = { version = "0.24.0", features = ["json-value", "bincode-value"] }

# NEW
kv = { version = "0.24.0", features = ["json-value"] }
```

#### Step 2.2: Update `kv.rs` imports and types

```rust
// OLD
use kv::{Bincode, Config, Store};
pub type KVBucket<'a, KeyT, ValueT> = kv::Bucket<'a, KeyT, Bincode<ValueT>>;

// NEW
use kv::{Json, Config, Store};
pub type KVBucket<'a, KeyT, ValueT> = kv::Bucket<'a, KeyT, Json<ValueT>>;
```

#### Step 2.3: Update doc comments

- Replace all references to "Bincode" with "JSON" in rustdoc comments
- Update the CBOR comparison section

---

### Phase 3: Remove Bincode Dependency

#### Step 3.1: Update `Cargo.toml`

```toml
# REMOVE this line entirely:
bincode = { version = "2.0.1", features = ["serde"] }
```

---

### Phase 4: Verification

- [ ] `cargo check -p r3bl_tui`
- [ ] `cargo test -p r3bl_tui`
- [ ] `cargo clippy -p r3bl_tui`
- [ ] `cargo doc -p r3bl_tui --no-deps`

---

## Files to Modify

| File | Change |
|------|--------|
| `tui/Cargo.toml` | Remove `bincode`, update `kv` features |
| `tui/src/network_io/bincode_serde.rs` | Rename to `json_serde.rs`, replace implementation |
| `tui/src/network_io/mod.rs` | Update module name and re-export |
| `tui/src/core/storage/kv.rs` | Change `Bincode` → `Json` |

## No Changes Needed

- `length_prefix_protocol.rs` - Only imports `bincode_serde` module, no direct bincode usage
- `compress.rs` - Unaffected (gzip compression layer)
- `protocol_types.rs` - Unaffected (type aliases only)

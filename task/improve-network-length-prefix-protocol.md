# Task: Improve Network Length-Prefix Protocol

## Overview

This task outlines improvements for `tui/src/network_io/length_prefix_protocol.rs`.

## Planned Improvements

### 1. Header Size Optimization (u32 or VarInt LEB128 instead of u64)

- **Current State**: Uses `u64` (8-byte header) for the length prefix.
- **Observation**: `MAX_PAYLOAD_SIZE` is capped at `10_000_000` (10 MB).
- **Improvement**: `u32` can address up to 4.29 GB. Changing `LengthPrefixType` to `u32`
  cuts the header size in half (from 8 bytes to 4 bytes per message). Alternatively,
  VarInt / LEB128 encoding uses only 1 byte for small payloads under 128 bytes, 2 bytes
  for under 16 KB, and scales dynamically.

### 2. Read Timeout Guard for `try_read`

- **Current State**: `handshake` has a 1-second timeout (`try_connect_or_timeout`), but
  `try_read` has no timeout.
- **Vulnerability**: If a client sends a valid length header (such as 5 MB) and then halts
  or drops TCP packets, `buf_reader.read_exact(&mut payload_buffer).await` will hang
  indefinitely, leaking the connection and task.
- **Improvement**: Add a configurable read/idle timeout wrapper (`try_read_or_timeout`):

```rust
pub async fn try_read_or_timeout<R: AsyncRead + Unpin, T: for<'d> Deserialize<'d>>(
    buf_reader: &mut BufReader<R>,
    timeout_duration: Duration,
) -> miette::Result<T> {
    tokio::time::timeout(timeout_duration, try_read(buf_reader))
        .await
        .map_err(|_| miette!("Read operation timed out"))?
}
```

### 3. Buffer Allocation & Memory Zeroing Optimization

- **Current State**: `let mut payload_buffer = vec![0; size_of_payload];`
- **Observation**: `vec![0; size]` allocates memory on the heap and fills it with zeros
  via `memset`, only for `read_exact` to immediately overwrite those zeros.
- **Improvement**: Use `bytes::BytesMut` or a buffer pool to avoid unnecessary
  zero-filling overhead for high-frequency or large messages.

### 4. Flexible Handshake Version Negotiation

- **Current State**: Strict equality check for
  `received_protocol_version == PROTOCOL_VERSION`.
- **Improvement**: Strict equality means any minor version bump instantly rejects existing
  clients. Supporting a version range (`MIN_SUPPORTED_VERSION..=CURRENT_VERSION`) during
  the handshake allows backwards-compatible client and server upgrades without breaking
  existing deployments.

## Priority Matrix

| Priority | Improvement                           | Primary Benefit                                         |
| -------- | ------------------------------------- | ------------------------------------------------------- |
| High     | Read Timeout (`try_read_or_timeout`)  | Prevents hanging/stuck TCP connections & resource leaks |
| Medium   | Switch `u64` header to `u32` / VarInt | Reduces framing header byte overhead per frame          |
| Medium   | Handshake Version Range Check         | Enables backwards-compatible client/server upgrades     |
| Low      | `BytesMut` buffer reuse               | Eliminates `memset` zeroing overhead on reads           |

## Mandatory Manual Review Checkbox List

- [ ] [length_prefix_protocol.rs](file:///home/nazmul/github/roc/tui/src/network_io/length_prefix_protocol.rs)

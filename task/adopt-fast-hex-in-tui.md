# Task: Adopt `HexSlice` Utilities in `core/ansi`

<!-- BEGIN mktoc -->

- [Task: Adopt `HexSlice` Utilities in `core/ansi`](#task-adopt-hexslice-utilities-in-coreansi)
- [Overview](#overview)
    - [Target `HexSlice` API](#target-hexslice-api)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: `HexSlice` Namespace Refactor](#phase-1-hexslice-namespace-refactor)
    - [Phase 2: Input Parser Tracing](#phase-2-input-parser-tracing)
    - [Phase 3: OSC Hex Operations](#phase-3-osc-hex-operations)

<!-- END mktoc -->

## Overview

`tui/src/core/common/fast_strings/hex.rs` currently exposes four free functions
(`decode_hex_nibble`, `encode_hex_nibble`, `try_parse_hex_bytes`, and
`fmt_lower_hex_bytes`) at the `r3bl_tui` crate root via barrel re-exports, while keeping
`HexSlice` private inside `hex::tests`.

Co-locating all hex operations onto `HexSlice` (and renaming `hex.rs` to `hex_slice.rs`)
reduces the crate-root export surface from five symbols down to **one**
(`r3bl_tui::HexSlice`), gives callers `HexSlice::*` IDE discoverability (following
`design-philosophy` Type-Centric Co-location), and eliminates `fmt_lower_hex_bytes` as a
separate public function because `HexSlice(bytes).fmt(f)` delegates directly with zero
wrapper or `Formatter` state-machine overhead.

With `HexSlice` in place, three areas in `tui/src/core/ansi/` can drop avoidable heap
allocations, `std::fmt::Formatter` dispatches, and `from_str_radix` calls:

1. **Heap-allocating hex diagnostics in `vt_100_terminal_input_parser`:**
   `terminal_events.rs` (`raw_osc_hex`) and `accumulator_struct.rs` (`discarded_hex`) call
   `%format!("{:02X?}", ...)` inside `tracing::warn!`, allocating a temporary `String`
   instead of streaming hex directly to the subscriber formatter. Switching to
   `%HexSlice(...)` eliminates that allocation (changing the logged representation from
   uppercase debug list `"[1B, 5D, ...]"` to compact lowercase hex `"1b5d..."`).
2. **`Formatter` dispatch inside `FastStringify` (`osc_codes.rs`):**
   `OscSequence::ColorReport` in `FastStringify::write_to_buf` calls
   `write!(acc, "{prefix}{r:02x}{r:02x}{sep}{g:02x}{g:02x}{sep}{b:02x}{b:02x}")`,
   re-entering `std::fmt::Formatter` inside a trait designed to bypass it.
3. **UTF-8 check and `u16::from_str_radix` in `osc_color.rs`:**
   `try_parse_hex_channel(slice: &[u8]) -> Option<u8>` runs `std::str::from_utf8(slice)`
   followed by `u16::from_str_radix(hex_str, 16)` on 1 to 4 byte color channels. Aside
   from extra work, `from_str_radix` accepts a leading `'+'` sign (for example `"+f"`
   parses as `15`), which is invalid in XParseColor / `#RGB` payloads.

### Target `HexSlice` API

In `tui/src/core/common/fast_strings/hex_slice.rs` (re-exported as `r3bl_tui::HexSlice`
via `fast_strings/mod.rs`):

```rust
/// Zero-cost `Display` wrapper and namespace for stack-only hexadecimal encoding and
/// decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexSlice<'a>(pub &'a [u8]);

/// Stateless associated functions live on `HexSlice<'static>` so callers write
/// `HexSlice::encode_byte(b)` or `HexSlice::try_parse_bytes::<32>(s)` without
/// unconstrained lifetime inference issues.
impl HexSlice<'static> {
    #[must_use]
    #[inline]
    pub const fn decode_nibble(byte: u8) -> Option<u8>;

    #[must_use]
    #[inline]
    pub fn encode_nibble(nibble: u8) -> u8;

    #[must_use]
    #[inline]
    pub fn encode_byte(byte: u8) -> [u8; 2];

    #[must_use]
    pub fn try_parse_bytes<const N: usize>(hex_str: &str) -> Option<[u8; N]>;
}

impl std::fmt::Display for HexSlice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}
```

_(Optional follow-up out of scope for `core/ansi/`: `color_to_hex_string` in
`tui/src/core/color_wheel/gradients/truecolor.rs` can similarly replace `write!(acc, ...)`
with `HexSlice::encode_byte` if desired later.)_

## Implementation Plan

### Phase 1: `HexSlice` Namespace Refactor

Rename `hex.rs` to `hex_slice.rs`, scope all hex helpers under `HexSlice`, and update
`Sha256Digest` in `cmdr` to use `HexSlice`.

- [ ] Rename `tui/src/core/common/fast_strings/hex.rs` to
      `tui/src/core/common/fast_strings/hex_slice.rs` (`git mv` or create + delete).
- [ ] In `tui/src/core/common/fast_strings/hex_slice.rs`, promote
      `pub struct HexSlice<'a>(pub &'a [u8])` to the primary public type, inline the
      chunked stack formatter directly into `impl Display for HexSlice<'_>`, and move
      `decode_nibble`, `encode_nibble`, `encode_byte`, and `try_parse_bytes` onto
      `impl HexSlice<'static>`.
- [ ] In `tui/src/core/common/fast_strings/mod.rs`, replace `hex` module attachment and
      re-export with `hex_slice` (`#[cfg(any(test, doc))] pub mod hex_slice;`,
      `#[cfg(not(any(test, doc)))] mod hex_slice;`, `pub use hex_slice::*;`) and mention
      `HexSlice` in the module-level docs.
- [ ] In `cmdr/src/self_upgrade/upgrade/sha256_digest.rs`, update imports to
      `use r3bl_tui::HexSlice;`, delegate `try_parse_hex` to
      `HexSlice::try_parse_bytes::<{ constants::DIGEST_BYTES }>(hex)`, and delegate
      `Display::fmt` to `HexSlice(&self.inner).fmt(f)`.
- [ ] Update and expand unit tests in `tui/src/core/common/fast_strings/hex_slice.rs` to
      cover `HexSlice::decode_nibble`, `HexSlice::encode_nibble`, `HexSlice::encode_byte`,
      `HexSlice::try_parse_bytes`, and `Display` across empty, single-chunk, and
      multi-chunk slices.
- [ ] Run `./check.fish --check`, `./check.fish --fmt`,
      `cargo test -p r3bl_tui --lib fast_strings::hex_slice::tests -q --message-format=short`,
      and `cargo test -p r3bl-cmdr --lib sha256_digest::tests -q --message-format=short`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/common/fast_strings/hex_slice.rs`
    - [ ] `tui/src/core/common/fast_strings/mod.rs`
    - [ ] `cmdr/src/self_upgrade/upgrade/sha256_digest.rs`

### Phase 2: Input Parser Tracing

Replace heap-allocating `format!("{:02X?}", ...)` calls in `vt_100_terminal_input_parser`
diagnostic logs with zero-allocation `%HexSlice(...)`.

- [ ] In
      `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/terminal_events.rs`,
      import `crate::HexSlice` and replace
      `raw_osc_hex = %format!("{:02X?}", &chunk[..len])` with
      `raw_osc_hex = %HexSlice(&chunk[..len])`.
- [ ] In
      `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/accumulator/accumulator_struct.rs`,
      import `crate::HexSlice` and replace
      `discarded_hex = %format!("{:02X?}", self.buffer)` with
      `discarded_hex = %HexSlice(&self.buffer)`.
- [ ] Run `./check.fish --check`, `./check.fish --fmt`, and targeted
      `vt_100_terminal_input_parser` unit tests.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/terminal_events.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_framer/accumulator/accumulator_struct.rs`

### Phase 3: OSC Hex Operations

Eliminate `Formatter` overhead in `OscSequence::ColorReport` formatting and replace
`from_utf8` + `u16::from_str_radix` in `osc_color.rs` with `HexSlice::decode_nibble`.

- [ ] In `tui/src/core/ansi/osc/osc_codes.rs` (`OscSequence::ColorReport` arm of
      `FastStringify::write_to_buf`), replace `write!(acc, ...)` with direct pushes into
      `acc: &mut BufTextStorage` using `crate::HexSlice::encode_byte` (`acc.push_str` for
      `OSC_COLOR_SPEC_RGB_PREFIX`, `acc.push(OSC_COLOR_SPEC_CHANNEL_SEPARATOR_CHAR)`
      between channels, and pushing `[hi, lo, hi, lo]` via `char::from(...)` for each of
      `red`, `green`, `blue`).
- [ ] In `tui/src/core/ansi/osc/osc_color.rs`, rewrite
      `fn try_parse_hex_channel(slice: &[u8]) -> Option<u8>` to validate
      `(1..=4).contains(&slice.len())` and fold nibbles directly into a `u16` via
      `crate::HexSlice::decode_nibble`, removing `std::str::from_utf8` and
      `u16::from_str_radix`.
- [ ] Add a unit test in `tui/src/core/ansi/osc/osc_color.rs` verifying that leading `'+'`
      signs in channel payloads (such as `rgb:+f/+f/+f` and `#+f0000`) return `None`, and
      verify existing `osc_codes` and `osc_color` round-trip tests pass.
- [ ] Run `./check.fish --check`, `./check.fish --fmt`, and targeted `osc` unit tests
      (`cargo test -p r3bl_tui --lib core::ansi::osc -q --message-format=short`).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `tui/src/core/ansi/osc/osc_codes.rs`
    - [ ] `tui/src/core/ansi/osc/osc_color.rs`

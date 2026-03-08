# Task: ANSI Constants Architecture Cleanup

## Overview

The `define_ansi_const!` macro was originally designed for both "foundational parts" (single
bytes/chars) and "composed sequences" (prefix + value strings). This conflated two distinct
abstraction levels, leading to prefix-only variants (`@esc_prefix_char`, `@csi_prefix_bytes`)
that obscure simple values behind macro indirection.

The fix: split constants into two tiers based on what they represent.

**Tier 1 - Foundational Parts (manual `pub const`):**
Single bytes, characters, and fixed byte slices that represent protocol building blocks.
These use a Standardized Doc Template applied manually for transparency and discoverability.

**Tier 2 - Composed Sequences (macro):**
Strings that concatenate a prefix (`ESC`, `CSI`, `SGR`, etc.) with a value at compile time.
The macro handles the `concat!("\x1b[", $val)` magic and enforces a consistent doc template
with `$std_name : $desc`.

**Tier 3 - Dynamic Sequences (generator functions):**
Complex compositions that require runtime values and cannot be `const`. Already handled by
`EscSequence`, `CsiSequence`, etc. No changes needed.

## Implementation Plan

### Phase 1: Remove prefix variants from macro and convert to manual constants

- [x] Remove `@esc_prefix_char` variant from `macros.rs`
- [x] Remove `@csi_prefix_bytes` variant from `macros.rs`
- [x] Convert `ESC_START` in `esc.rs` to manual `pub const`
- [x] Convert `CSI_PREFIX` in `input_sequences.rs` to manual `pub const`
- [x] Keep `CSI_PREFIX_LEN = CSI_PREFIX.len()` (derived from source of truth)

### Phase 2: Remove `@csi_bytes` variant and convert call sites to manual constants

- [x] `mouse.rs`: `MOUSE_SGR_PREFIX`, `MOUSE_X10_PREFIX`
- [x] `sgr.rs`: `SGR_RESET_BYTES`
- [x] `csi.rs`: `SCP_SAVE_CURSOR_BYTES`, `RCP_RESTORE_CURSOR_BYTES`
- [x] `input_sequences.rs`: `DECCKM_ENABLE_BYTES`, `DECCKM_DISABLE_BYTES`

### Phase 3: Update macro call sites to new `$std_name : $desc` signature

- [x] `generic.rs`: 4 `@csi_str` calls
- [x] `sgr.rs`: 24 `@sgr_str` calls
- [x] `csi.rs`: 4 `@csi_str` calls
- [x] `dsr.rs`: 5 `@dsr_str` calls
- [x] `osc_codes.rs`: 7 `@osc_str`/`@esc_str` calls

### Phase 4: Apply Standardized Doc Template to foundational constants

- [x] `esc.rs`
- [x] `input_sequences.rs` (74 constants across 13 sections)
- [x] `mouse.rs` (22 constants)
- [x] `utf8.rs` (20 constants)
- [x] `csi.rs` (47 constants)
- [x] `dsr.rs` (3 constants)
- [x] `sgr.rs` (2 constants)

### Phase 5: Clean up `mod.rs` Design section

- [x] Write final Design section prose with three-tier architecture
- [x] Add code examples for Tier 1 and Tier 2

### Phase 6: Validation

- [x] `./check.fish --check` (typecheck passes)
- [x] `./check.fish --test` (all tests pass)
- [x] `./check.fish --doc` (passes, fixed unresolved `DECSTBM` link in `csi.rs`)
- [x] `./check.fish --clippy` (lint clean)

## Also completed in this session (prior to this task)

These doc improvements were made earlier in the conversation, before the task was created:

- Simplified link styles in `generic.rs`: `[DEC mode N][VT510 Programmer Reference]` to
  `[`DECCKM - DEC mode 1`]` (merged abbreviation + mode number into single linked term)
- Fixed `[`ESC`]` link targets: changed 3 instances in `mouse.rs` and `input_sequences.rs`
  from `crate::EscSequence` to `crate::ANSI_ESC` where ESC referred to the byte, not the enum
- Backticked all bare hex/dec byte values across `utf8.rs` (~22), `input_sequences.rs` (~66),
  `esc.rs` (3), `mouse.rs` (1)
- Backticked key combos (`Ctrl+X`, `Shift+Tab`, etc.) in `input_sequences.rs`
- Removed stale backward-compat re-export (`pub use mouse::*`) from `input_sequences.rs`
- Renamed macro variants: `@esc_char` to `@esc_prefix_char`, `@csi_prefix` to
  `@csi_prefix_bytes` (consistent `{protocol}_{role}_{type}` naming)
- Added `@csi_prefix_bytes` variant to macro (later removed in Phase 1)
- Fixed `SAVE_CURSOR_DEC` doc: corrected `ESC [ 7` to `ESC 7`, added `[`DECSC`]` link

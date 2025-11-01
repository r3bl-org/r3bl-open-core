# Task: Fix ParamsExt and Add RGB/256-Color Support

## Problem Statement

The current VT100 ANSI parser has a critical gap in color support:

1. **ParamsExt Limitation**: The trait only extracts the first sub-parameter from each position,
   making it impossible to handle complex sequences like `ESC[38:5:196m` (256-color) or
   `ESC[38:2:255:128:0m` (RGB).

2. **Missing Color Support**: The codebase ignores extended color sequences entirely
   (`vt_100_shim_sgr_ops.rs:184`), only supporting basic 16-color ANSI.

3. **Misleading Documentation**: The docs use `ESC[38:5:196m` as an example, but the implementation
   can't actually handle it.

## Background: How VT100 Parameters Work

```text
Simple sequence:    ESC[1;31m      → params: [[1], [31]]
Complex sequence:   ESC[38:5:196m  → params: [[38, 5, 196]]
                    ESC[38;5;196m  → params: [[38], [5], [196]] (different!)
```

The `vte::Params` type stores each semicolon-separated position as a `&[u16]` slice that can contain
colon-separated sub-parameters.

## Implementation Plan

### Phase 1: Enhance ParamsExt Trait ✅

**File**: `tui/src/core/pty_mux/vt_100_ansi_parser/protocols/csi_codes/params.rs`

- [x] Rename `extract_nth_non_zero()` → `extract_nth_single_non_zero()`
- [x] Rename `extract_nth_opt_raw()` → `extract_nth_single_opt_raw()`
- [x] Add new method:

```rust
/// Extract all sub-parameters at position n.
///
/// # Examples
/// - `ESC[38:5:196m` at position 0 → `Some(&[38, 5, 196])`
/// - `ESC[5A` at position 0 → `Some(&[5])`
fn extract_nth_all(&self, arg_n: impl Into<Index>) -> Option<&[u16]> {
    let n: Index = arg_n.into();
    self.iter().nth(n.as_usize())
}
```

- [x] Update documentation to explain `_single_` vs `_all` distinction
- [x] Move `ESC[38:5:196m` example to `extract_nth_all()` section

### Phase 2: Add Color Constants

**File**: `tui/src/core/pty_mux/vt_100_ansi_parser/protocols/csi_codes/constants.rs`

Add after line 283:

```rust
// Extended Color Support

/// Extended foreground color (SGR 38)
pub const SGR_FG_EXTENDED: u16 = 38;

/// Extended background color (SGR 48)
pub const SGR_BG_EXTENDED: u16 = 48;

/// 256-color mode indicator
pub const SGR_COLOR_MODE_256: u16 = 5;

/// RGB color mode indicator
pub const SGR_COLOR_MODE_RGB: u16 = 2;
```

### Phase 3: Create Color Sequence Types

**New File**: `tui/src/core/pty_mux/vt_100_ansi_parser/protocols/csi_codes/color_sequences.rs`

```rust
// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extended color sequence parsing for SGR parameters.

use super::constants::{SGR_COLOR_MODE_256, SGR_COLOR_MODE_RGB, SGR_FG_EXTENDED, SGR_BG_EXTENDED};

/// Extended color sequences for 256-color and RGB support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendedColorSequence {
    /// 256-color palette index (0-255)
    Ansi256 { index: u8 },
    /// RGB color values
    Rgb { r: u8, g: u8, b: u8 },
}

impl ExtendedColorSequence {
    /// Parse extended color from a parameter slice.
    ///
    /// # Formats
    /// - `[38, 5, n]` - 256-color foreground
    /// - `[48, 5, n]` - 256-color background
    /// - `[38, 2, r, g, b]` - RGB foreground
    /// - `[48, 2, r, g, b]` - RGB background
    pub fn parse_from_slice(params: &[u16]) -> Option<(Self, bool)> {
        match params {
            [fg_or_bg, mode, rest @ ..] if *fg_or_bg == SGR_FG_EXTENDED || *fg_or_bg == SGR_BG_EXTENDED => {
                let is_background = *fg_or_bg == SGR_BG_EXTENDED;
                match (*mode, rest) {
                    (SGR_COLOR_MODE_256, [index, ..]) if *index <= 255 => {
                        Some((Self::Ansi256 { index: *index as u8 }, is_background))
                    }
                    (SGR_COLOR_MODE_RGB, [r, g, b, ..]) if *r <= 255 && *g <= 255 && *b <= 255 => {
                        Some((Self::Rgb { r: *r as u8, g: *g as u8, b: *b as u8 }, is_background))
                    }
                    _ => None
                }
            }
            _ => None
        }
    }
}
```

**Update** `tui/src/core/pty_mux/vt_100_ansi_parser/protocols/csi_codes/mod.rs`:

- [x] Add `pub mod color_sequences;`
- [x] Add `pub use color_sequences::*;`

### Phase 4: Update SGR Handler

**File**: `tui/src/core/pty_mux/vt_100_ansi_parser/operations/vt_100_shim_sgr_ops.rs`

Replace the `set_graphics_rendition` function (lines 188-195):

```rust
use crate::vt_100_ansi_parser::protocols::csi_codes::color_sequences::ExtendedColorSequence;

/// Handle SGR (Select Graphic Rendition) parameters.
pub fn set_graphics_rendition(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    use crate::ParamsExt;

    let mut idx = 0;
    while let Some(param_slice) = params.extract_nth_all(idx) {
        // Check for extended color sequences first
        if let Some((color, is_background)) = ExtendedColorSequence::parse_from_slice(param_slice) {
            match color {
                ExtendedColorSequence::Ansi256 { index } => {
                    if is_background {
                        performer.ofs_buf.set_background_ansi256(index);
                    } else {
                        performer.ofs_buf.set_foreground_ansi256(index);
                    }
                }
                ExtendedColorSequence::Rgb { r, g, b } => {
                    if is_background {
                        performer.ofs_buf.set_background_rgb(r, g, b);
                    } else {
                        performer.ofs_buf.set_foreground_rgb(r, g, b);
                    }
                }
            }
        } else if let Some(&first_param) = param_slice.first() {
            // Handle single parameters (existing behavior)
            apply_sgr_param(performer, first_param);
        }
        idx += 1;
    }
}
```

### Phase 5: Add Color Conversion Support

**File**: `tui/src/core/pty_mux/vt_100_ansi_parser/ansi_to_tui_color.rs`

Add new functions:

```rust
use crate::{AnsiValue, RgbValue};

/// Convert 256-color index to TuiColor.
#[must_use]
pub fn ansi256_to_tui_color(index: u8) -> TuiColor {
    TuiColor::Ansi(AnsiValue::new(index))
}

/// Convert RGB values to TuiColor.
#[must_use]
pub fn rgb_to_tui_color(r: u8, g: u8, b: u8) -> TuiColor {
    TuiColor::Rgb(RgbValue::from_u8(r, g, b))
}
```

### Phase 6: Update Offscreen Buffer Implementation

**Note**: The `OffscreenBuffer` implementation needs methods:

- [x] `set_foreground_ansi256(index: u8)`
- [x] `set_background_ansi256(index: u8)`
- [x] `set_foreground_rgb(r: u8, g: u8, b: u8)`
- [x] `set_background_rgb(r: u8, g: u8, b: u8)`

These should use the conversion functions from Phase 5.

### Phase 7: Update All Call Sites

Files to update with new method names:

- [x] `MovementCount` usages → `extract_nth_single_non_zero()`
- [x] `AbsolutePosition` usages → `extract_nth_single_non_zero()`
- [x] `CursorPositionRequest` usages → `extract_nth_single_non_zero()`
- [x] `MarginRequest` usages (if any) → `extract_nth_single_opt_raw()`

### Phase 8: Add Tests

**New test file**:
`tui/src/core/pty_mux/vt_100_ansi_parser/vt_100_ansi_conformance_tests/tests/vt_100_test_extended_colors.rs`

```rust
// Test 256-color sequences
#[test]
fn test_256_color_foreground() {
    // ESC[38;5;196m - semicolon format
    // ESC[38:5:196m - colon format
}

#[test]
fn test_256_color_background() {
    // ESC[48;5;196m
}

// Test RGB sequences
#[test]
fn test_rgb_foreground() {
    // ESC[38;2;255;128;0m
    // ESC[38:2:255:128:0m
}

#[test]
fn test_rgb_background() {
    // ESC[48;2;255;128;0m
}

// Test invalid sequences
#[test]
fn test_invalid_color_sequences() {
    // ESC[38;5;256m - index out of range
    // ESC[38;2;256;0;0m - RGB value out of range
    // ESC[38;3;100m - invalid mode
}
```

## Testing Checklist

- [x] All existing tests pass after renaming
- [x] New ParamsExt methods have unit tests
- [x] 256-color sequences work in both formats (`:` and `;`)
- [x] RGB sequences work in both formats
- [x] Invalid sequences are gracefully ignored
- [x] Mixed sequences work: `ESC[1;31;38:5:196m` (bold + red + 256-color)

## Implementation Notes

1. **Backward Compatibility**: The renaming preserves all existing functionality, just with clearer
   names.

2. **Colon vs Semicolon**: Modern terminals support both `ESC[38:5:196m` (colon) and `ESC[38;5;196m`
   (semicolon). The colon format arrives as one parameter `[38, 5, 196]`, while semicolon arrives as
   three `[38], [5], [196]`. Our implementation handles both.

3. **Performance**: Using `&[u16]` avoids allocations - we're just returning references to data
   already parsed by VTE.

4. **Type Safety**: The `ExtendedColorSequence` enum ensures we only create valid color values.

## Progress Tracking

- [x] Phase 1: ParamsExt enhancements
- [x] Phase 2: Add constants
- [x] Phase 3: Color sequence types
- [x] Phase 4: Update SGR handler
- [x] Phase 5: Color conversion
- [x] Phase 6: Offscreen buffer support
- [x] Phase 7: Update call sites
- [x] Phase 8: Add tests

## References

- VT100 Specification: https://vt100.net/docs/vt100-ug/chapter3.html
- 256 Color Cheat Sheet: https://jonasjacek.github.io/colors/
- ANSI Escape Codes: https://en.wikipedia.org/wiki/ANSI_escape_code#SGR

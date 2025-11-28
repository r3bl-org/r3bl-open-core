---
name: human-readable-constants
description: Apply human-readable numeric literal conventions for constants. Use bitmasks as binary, printable chars as byte literals, and non-printables as decimal. Use proactively when writing byte/u8 constants in code - apply the convention from the start, not as a later fix.
---

# Human-Readable Constants

## When to Use

- Writing new byte/u8 constants (ANSI sequences, control characters, ASCII values)
- Reviewing terminal input/output parsing code
- Working in `tui/src/core/ansi/constants/` or parser modules
- Before creating commits with byte-level constants
- When user asks about "magic numbers", "hex literals", "readability"

## The Problem

Hex literals are spec-friendly but not human-friendly:

```rust
// ❌ Bad - What is 0x1B? What is 0x5B?
pub const ANSI_ESC: u8 = 0x1B;
pub const ANSI_CSI_BRACKET: u8 = 0x5B;
pub const CTRL_TO_LOWERCASE_MASK: u8 = 0x60;
```

Developers must mentally convert hex to understand the code.

## The Solution

Use a 3-tier convention based on the constant's purpose:

| Type | Format | Why |
|------|--------|-----|
| **Bitmasks** (used in `&`, `\|`, `^`) | Binary `0b0110_0000` | Shows which bits are set |
| **Printable ASCII** | Byte literal `b'['` | Self-documenting character |
| **Non-printable bytes** | Decimal `27` | Humans read decimal naturally |
| **Comments** | Show hex `(0x1B in hex)` | For spec cross-reference |

## Quick Reference

```rust
// ✅ Good - Human-readable constants

// Bitmasks → Binary (shows bit pattern)
pub const CTRL_TO_LOWERCASE_MASK: u8 = 0b0110_0000;  // 0x60 in hex
pub const CTRL_TO_UPPERCASE_MASK: u8 = 0b0100_0000;  // 0x40 in hex

// Printable characters → Byte literals (self-documenting)
pub const ANSI_CSI_BRACKET: u8 = b'[';   // 0x5B in hex
pub const PRINTABLE_ASCII_MIN: u8 = b' '; // 0x20 in hex (space)
pub const PRINTABLE_ASCII_MAX: u8 = b'~'; // 0x7E in hex (tilde)

// Non-printable bytes → Decimal (intuitive for humans)
pub const ANSI_ESC: u8 = 27;              // 0x1B in hex
pub const CONTROL_BACKSPACE: u8 = 8;      // 0x08 in hex
pub const ASCII_DEL: u8 = 127;            // 0x7F in hex
```

## Decision Guide

Ask yourself:

1. **Is this used in bitwise operations (`&`, `|`, `^`)?**
   - Yes → Use binary: `0b0110_0000`

2. **Is this a printable ASCII character?**
   - Yes → Use byte literal: `b'['`, `b'~'`, `b' '`

3. **Is this a non-printable byte value?**
   - Yes → Use decimal: `27`, `8`, `127`

4. **Always add hex in comments** for cross-referencing specs.

## Common Files

This convention applies to:

- `tui/src/core/ansi/constants/input_sequences.rs`
- `tui/src/core/ansi/constants/esc.rs`
- `tui/src/core/ansi/constants/utf8.rs` (already uses binary)
- `tui/src/core/ansi/constants/mouse.rs` (already uses binary)
- Any parser module with byte constants

## Examples

### ❌ Before (Spec-centric)

```rust
/// ESC byte (27 in decimal, 0x1B in hex)
pub const ANSI_ESC: u8 = 0x1B;

/// Mask to convert control character to lowercase letter
pub const CTRL_TO_LOWERCASE_MASK: u8 = 0x60;

/// Printable ASCII minimum (space, 0x20)
pub const PRINTABLE_ASCII_MIN: u8 = 0x20;
```

### ✅ After (Human-centric)

```rust
/// ESC byte (0x1B in hex).
pub const ANSI_ESC: u8 = 27;

/// Mask to convert control character to lowercase letter (0x60 in hex).
pub const CTRL_TO_LOWERCASE_MASK: u8 = 0b0110_0000;

/// Printable ASCII minimum: space (0x20 in hex).
pub const PRINTABLE_ASCII_MIN: u8 = b' ';
```

## Verification

After applying changes, check for remaining hex literals:

```bash
# Find any remaining 0x literals in constants files
grep -n "0x[0-9a-fA-F]" tui/src/core/ansi/constants/*.rs
```

Note: Hex in **documentation examples** (showing raw bytes) is acceptable.

## Reporting Results

When applying this convention:

- ✅ All constants use human-readable format → "Constants follow human-readable convention!"
- 🔧 Converted hex to appropriate format → Report conversions made
- 📝 Added hex comments → List where comments were added

## Related Skills

- `check-code-quality` - Includes verifying constant conventions
- `write-documentation` - For documenting constant purposes

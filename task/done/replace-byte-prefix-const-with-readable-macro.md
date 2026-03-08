# Task: Replace byte-prefix constants with readable macro

## Overview

The current ANSI/VT constants (CSI, SGR, OSC, DSR) use redundant `\x1b[` prefixes and
repetitive Rustdoc link definitions. This creates visual noise in the source code and
increases the risk of "copy-paste" errors (e.g., using a CSI prefix for an OSC sequence).

Design decision: Use a declarative macro (`macro_rules!`) with a domain-specific language (DSL)
to semantically define these constants. The macro will automatically:
1.  Prepend the correct escape prefix (`\x1b[`, `\x1b]`, etc.) at compile-time.
2.  Generate a standardized doc comment.
3.  Include the necessary reference-style links for Rustdoc.

## Final Macro DSL Design

The `define_ansi_const!` macro uses a "colon-equals-bracket" syntax that isolates the
protocol data and makes it highly readable: `@tag : Name = [Val] => Desc`.

```rust
#[macro_export]
macro_rules! define_ansi_const {
    // === CSI (Control Sequence Introducer) ===
    // &str variant
    (@csi_str : $name:ident = [$val:literal] => $desc:literal) => {
        #[doc = concat!("[`CSI`] ", $val, ": ", $desc)]
        #[doc = ""]
        #[doc = "[`CSI`]: crate::CsiSequence"]
        pub const $name: &str = concat!("\x1b[", $val);
    };
    // &[u8] variant
    (@csi_bytes : $name:ident = [$val:literal] => $desc:literal) => {
        #[doc = concat!("[`CSI`] ", $val, " (bytes): ", $desc)]
        #[doc = ""]
        #[doc = "[`CSI`]: crate::CsiSequence"]
        pub const $name: &[u8] = concat!("\x1b[", $val).as_bytes();
    };

    // === ESC (Escape Sequence) ===
    // char variant (always '\x1b')
    (@esc_char : $name:ident => $desc:literal) => {
        #[doc = concat!("[`ESC`] ", $desc)]
        #[doc = ""]
        #[doc = "[`ESC`]: crate::EscSequence"]
        pub const $name: char = '\x1b';
    };
    // &str variant
    (@esc_str : $name:ident = [$val:literal] => $desc:literal) => {
        #[doc = concat!("[`ESC`] ", $val, ": ", $desc)]
        #[doc = ""]
        #[doc = "[`ESC`]: crate::EscSequence"]
        pub const $name: &str = concat!("\x1b", $val);
    };

    // === SGR (Select Graphic Rendition) - Subset of CSI ===
    // &str variant
    (@sgr_str : $name:ident = [$val:literal] => $desc:literal) => {
        #[doc = concat!("[`SGR`] ", $val, ": ", $desc)]
        #[doc = ""]
        #[doc = "[`CSI`]: crate::CsiSequence"]
        #[doc = "[`SGR`]: crate::SgrCode"]
        pub const $name: &str = concat!("\x1b[", $val);
    };

    // === OSC (Operating System Command) ===
    // &str variant
    (@osc_str : $name:ident = [$val:literal] => $desc:literal) => {
        #[doc = concat!("[`OSC`] ", $val, ": ", $desc)]
        #[doc = ""]
        #[doc = "[`OSC`]: crate::osc_codes::OscSequence"]
        pub const $name: &str = concat!("\x1b]", $val);
    };

    // === DSR (Device Status Report) - Subset of CSI ===
    // &str variant
    (@dsr_str : $name:ident = [$val:literal] => $desc:literal) => {
        #[doc = concat!("[`DSR`] ", $val, ": ", $desc)]
        #[doc = ""]
        #[doc = "[`CSI`]: crate::CsiSequence"]
        #[doc = "[`DSR`]: crate::DsrSequence"]
        pub const $name: &str = concat!("\x1b[", $val);
    };
}
```

## Migration Examples

### CSI String (`csi.rs`)
- **Before**: `define_ansi_const!(CSI_START, csi, str, "", "Start")`
- **After**: `define_ansi_const!(@csi_str : CSI_START = [""] => "Sequence start: `ESC [`");`

### CSI Bytes (`sgr.rs`)
- **Before**: `define_ansi_const!(SGR_RESET_BYTES, csi, bytes, "0m", "Reset")`
- **After**: `define_ansi_const!(@csi_bytes : SGR_RESET_BYTES = ["0m"] => "Reset sequence bytes.");`

### DSR String (`dsr.rs`)
- **Before**: `define_ansi_const!(DSR_CURSOR_POS, dsr, str, "6n", "Request")`
- **After**: `define_ansi_const!(@dsr_str : DSR_CURSOR_POSITION_REQUEST = ["6n"] => "Cursor position request.");`

### OSC String (`osc_codes.rs`)
- **Before**: `define_ansi_const!(OSC_START, osc, str, "", "Start")`
- **After**: `define_ansi_const!(@osc_str : OSC_START = [""] => "Generic start: `ESC ]`" );`

### ESC Char (`esc.rs`)
- **Before**: `pub const ESC_START: char = '\x1b';`
- **After**: `define_ansi_const!(@esc_char : ESC_START => "Start byte: the escape character (27 dec, 1B hex).");`

### ESC String (`esc.rs`)
- **Before**: (no `&str` equivalent existed)
- **After**: `define_ansi_const!(@esc_str : ESC_STR = [""] => "Start string: the escape character (27 dec, 1B hex).");`

## Implementation Plan

### Phase 1: Core Macro Definition
- [x] Create `tui/src/core/ansi/constants/macros.rs`
- [x] Implement `define_ansi_const!` with final `@tag : Name = [Val] => Desc` DSL
- [x] Add `@esc_char` variant for bare ESC character constant (`char` type, no suffix)
- [x] Re-export the macro in `tui/src/core/ansi/constants/mod.rs`

### Phase 2: Workspace Migration
- [x] Migrate `dsr.rs`
- [x] Migrate `csi.rs`
- [x] Migrate `sgr.rs`
- [x] Migrate `osc_codes.rs`
- [x] Migrate `mouse.rs` (byte-array constants)
- [x] Migrate `input_sequences.rs` (byte-array constants)
- [x] Migrate `esc.rs` — `ESC_START: char` via `@esc_char` (zero call-site changes)
- [x] Add `ESC_STR: &str` via `@esc_str` in `esc.rs` — `&str` variant for string operations like `.replace()`

### Phase 3: Validation
- [x] Run full workspace checks: `./check.fish --check`
- [x] Ensure generated docs are correct
- [x] Audit all usages to ensure no regressions
- [x] Fix type mismatch in `osc_codes.rs` (use `.push()` for `OSC_DELIMITER`)

## Follow-up Tasks to Complete the Plan

These tasks address the remaining gaps where raw ANSI literals are still used in production code, tests, and examples.

### 1. Complete `csi.rs` Migration
The `csi.rs` file contains many constants that were not fully migrated to the macro DSL.
- [x] Migrate `char` and `u16` constants to macro-defined string/byte versions where appropriate (e.g., `SCP_SAVE_CURSOR`, `RCP_RESTORE_CURSOR`, `ED_CLEAR_SCREEN`).
- [x] Update backends like `crossterm_paint_render_op_impl.rs` to use these new constants instead of `b"\x1b[s"` and `b"\x1b[u"`.

### 2. Expand `sgr.rs` with Common Formatting
Common text attributes are missing high-level string/byte constants.
- [x] Add `define_ansi_const!` entries for `SGR_BOLD_STR`, `SGR_ITALIC_STR`, `SGR_UNDERLINE_STR`, etc.
- [x] Update `pixel_char_renderer.rs` tests to use these constants.

### 3. Clean up Tests and Examples
Most tests and examples were skipped in the initial migration.
- [x] Update `osc_codes.rs` tests to use constants (e.g., `OSC_START`, `OSC_TERMINATOR_BEL`).
- [x] Update PTY and spinner examples to use semantic color constants instead of hardcoded `\x1b[93m`, etc.
  - [x] Update `stdout_mock.rs` tests to use `SGR_FG_RED_STR` and `SGR_RESET_STR`.
  - [x] Update `ansi_sequence_generator_output.rs` doc examples to use `CSI_START`.
- [x] **Exemption (Ground Truth)**: DO NOT migrate "Validation Tests" (e.g., `input_parser_validation_test.rs`). These tests use empirical byte sequences captured from real terminals (via `showkey -a`) to establish ground truth. Using constants here would create a circular dependency where a typo in a constant could cause both the generator and the test to be wrong while still passing.
- [x] **Exemption (Conformance Tests)**: DO NOT migrate "Conformance Tests" (e.g., `vt_100_pty_output_conformance_tests`). These are integration tests that validate the entire pipeline against the specification. They should continue to use either literal sequences or their existing type-safe builders to remain independent of the constants used by the production generator.
- [x] Update `text_operations.rs` integration tests to use `CSI_START` instead of `"\x1b["` (not ground truth — these are high-level "output contains CSI" checks).

### 4. Standardize Protocol Markers
- [x] Migrate `OSC_TERMINATOR_BEL` (`\x07`) and other "magic" protocol bytes to macro-defined constants for consistency.
- [x] Replace remaining bare `\x1b` and `\u{1b}` in assertions and logic with `ESC_START` where it improves readability without breaking "ground truth" requirements.
  - [x] Update `pty_input_event.rs` to use `CSI_START` in `format!` macros for sequence generation.
- [x] Remove private `const CSI: &str = "\x1b["` and `const SGR: &str = "m"` from `sgr_code.rs` generator — use `CSI_START` and `SGR_SET_GRAPHICS` from constants module.
- [x] Replace `"\x1b"` with `ESC_STR` in `cross_platform_commands.rs` PowerShell `.replace()` call.

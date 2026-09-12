# Task: Fix Shift+Home and Unrecognized ANSI Sequence Input Freeze

## Overview

When running any TUI example on Linux, pressing `Shift + Home` (or `Shift + End`, or any
unrecognized CSI sequence) permanently freezes the input event handling of the main event
loop. Subsequent keystrokes are ignored and never processed.

## Problem Analysis and Root Causes

### 1. Missing Key Decoding in `parse_csi_parameters` (`keyboard.rs`)

When `Shift + Home` is pressed, terminal emulators (xterm, GNOME Terminal, Alacritty,
Kitty, WezTerm) emit the standard xterm modified sequence: `ESC [ 1 ; 2 H` (`\x1b[1;2H`).

In `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs`, the CSI parameter parser
only matches `(2, final_byte) if params[0] == 1` for arrow keys:

- `ARROW_UP_FINAL` (`b'A'`)
- `ARROW_DOWN_FINAL` (`b'B'`)
- `ARROW_RIGHT_FINAL` (`b'C'`)
- `ARROW_LEFT_FINAL` (`b'D'`)

It omits:

- `SPECIAL_HOME_FINAL` (`b'H'`)
- `SPECIAL_END_FINAL` (`b'F'`)
- `SS3_F1_FINAL` through `SS3_F4_FINAL` (`b'P'` through `b'S'`)

Because `params = [1, 2]` and `final_byte = b'H'`, the match falls through to `_ => None`.

### 2. Stateful Parser Poisoning in `StatefulInputParser::advance` (`stateful_parser.rs`)

In `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs`,
`StatefulInputParser::advance` accumulates incoming bytes into `self.buffer`:

```rust
self.buffer.push(*byte);
match try_parse_input_event(&self.buffer, more) {
    Some((event, _bytes_consumed)) => {
        self.internal_events.push_back(event);
        self.buffer.clear();
    }
    None => {
        // Keeps buffer and continues accumulating.
    }
}
```

In ANSI / ECMA-48, any byte in `0x40..=0x7E` is the terminating final byte of a CSI
sequence. Once `b'H'` arrives, the CSI sequence has finished. Adding more bytes can never
turn this sequence into a valid CSI sequence.

Because `try_parse_input_event` returned `None`, `self.buffer` retains `b"\x1b[1;2H"`.
When the user subsequently types any key (such as `'a'`), the new byte is appended:
`b"\x1b[1;2Ha"`.

The parser continues attempting to parse the head of `self.buffer`, encounters the
unsupported sequence `\x1b[1;2H`, and returns `None`. `self.buffer` is never cleared. All
future keystrokes are blocked forever, freezing the main event loop.

### 3. Generator Asymmetry in `ansi_input.rs`

In `tui/src/core/ansi/generator/ansi_input.rs`, `VT100KeyCodeIR::Home` and `End` generate
`generate_simple_csi(SPECIAL_HOME_FINAL)`, ignoring modifiers rather than producing
`ESC [ 1 ; <mod> H/F`.

## Implementation Plan

### Step 1: Add Modified Key Decoding in `keyboard.rs`

Update `parse_csi_parameters` in
`tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs`:

- Extend `(2, final_byte) if params[0] == 1` to support:
    - `SPECIAL_HOME_FINAL` (`b'H'`) -> `VT100KeyCodeIR::Home`
    - `SPECIAL_END_FINAL` (`b'F'`) -> `VT100KeyCodeIR::End`
    - `SS3_F1_FINAL` (`b'P'`) -> `VT100KeyCodeIR::Function(1)`
    - `SS3_F2_FINAL` (`b'Q'`) -> `VT100KeyCodeIR::Function(2)`
    - `SS3_F3_FINAL` (`b'R'`) -> `VT100KeyCodeIR::Function(3)`
    - `SS3_F4_FINAL` (`b'S'`) -> `VT100KeyCodeIR::Function(4)`
- Also support single-parameter sequences where `params[0] <= 1` for `SPECIAL_HOME_FINAL`,
  `SPECIAL_END_FINAL`, and `BACKTAB_FINAL` (`b'Z'`).

### Step 2: Update Generator in `ansi_input.rs`

In `tui/src/core/ansi/generator/ansi_input.rs`:

- Update `VT100KeyCodeIR::Home` and `VT100KeyCodeIR::End` to use `generate_arrow_key`
  (which formats `ESC [ 1 ; <mod> <final>` when modifiers are present, and `ESC [ <final>`
  when absent).
- Rename helper if appropriate or document its dual use for navigation keys.

### Step 3: Resilient Recovery in `StatefulInputParser::advance` (`stateful_parser.rs`)

In `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs`:

- Add detection for completed or invalid escape sequences when `try_parse_input_event`
  returns `None`.
    - If `self.buffer` starts with `ESC [` and contains a final byte (`0x40..=0x7E`) after
      `ESC [`, the CSI sequence has terminated but could not be parsed. Discard it by
      clearing `self.buffer`.
    - If `self.buffer` starts with `ESC O` and length is >= 3, discard it by clearing
      `self.buffer`.
    - If `self.buffer` exceeds a safety length threshold (64 bytes), clear `self.buffer`.
- This guarantees that any unrecognized or unsupported escape sequence never locks up
  future input event processing.

### Step 4: Add Unit and Integration Tests

- In `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs`:
    - Test parsing `Shift + Home` (`\x1b[1;2H`).
    - Test parsing `Ctrl + Home` (`\x1b[1;5H`).
    - Test parsing `Shift + End` (`\x1b[1;2F`).
    - Test recovery: unrecognized sequence followed by regular character does not block
      the character.
- In
  `tui/src/core/ansi/vt_100_terminal_input_parser/unit_tests/generator_round_trip_tests.rs`:
    - Test round-trip generation and parsing for modified Home and End keys.
- In `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`:
    - Add `Shift+Home`, `Ctrl+Home`, `Shift+End`, and `Ctrl+End` test sequences to verify
      parity between Crossterm and DirectToAnsi backends.

### Step 5: Verification

- Run `./check.fish --check`.
- Run `./check.fish --clippy`.
- Run `./check.fish --test`.
- Run `./check.fish --fmt`.

### Step 6: Mandatory Manual Review

- [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs`
- [ ] `tui/src/core/ansi/generator/ansi_input.rs`
- [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs`
- [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/unit_tests/generator_round_trip_tests.rs`
- [ ] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`

### Known Limitations (Out of Scope)

- **OSC Sequences (`ESC ]`)**: The 64-byte fallback in `StatefulInputParser::advance`
  handles extreme cases of malformed CSI/SS3 inputs. If a terminal sends an OSC sequence
  (e.g., `ESC ] 0 ; title ST`), the router currently parses the `ESC ]` immediately as
  `Alt+]`. This clears the buffer, causing the rest of the OSC sequence to spill out as
  raw text. Fixing OSC parsing is a pre-existing limitation and is intentionally out of
  scope for fixing this CSI lockup.

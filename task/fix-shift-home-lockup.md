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

## [ ] Implementation Plan

### [x] Step 1: Add Modified Key Decoding in `keyboard.rs`

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

### [x] Step 2: Update Generator in `ansi_input.rs`

In `tui/src/core/ansi/generator/ansi_input.rs`:

- Update `VT100KeyCodeIR::Home` and `VT100KeyCodeIR::End` to use `generate_arrow_key`
  (which formats `ESC [ 1 ; <mod> <final>` when modifiers are present, and `ESC [ <final>`
  when absent).
- Rename helper if appropriate or document its dual use for navigation keys.

### [x] Step 3: Resilient Recovery in `StatefulInputParser::advance` (`stateful_parser.rs`)

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

### [x] Step 4: Add Unit and Integration Tests

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

### [x] Step 5: Verification

- [x] Run `./check.fish --check`.
- [x] Run `./check.fish --clippy`.
- [x] Run `./check.fish --test`.
- [x] Run `./check.fish --fmt`.

### [x] Step 6: Mandatory Manual Review

- [x] `tui/src/core/ansi/constants/input_sequences.rs`
- [x] `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs`
- [x] `tui/src/core/ansi/generator/ansi_input.rs`
- [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs`
- [x] `tui/src/core/ansi/vt_100_terminal_input_parser/unit_tests/generator_round_trip_tests.rs`
- [x] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`

### [x] Step 7: Consolidate Stream Availability and Zero-Latency ESC Disambiguation into `MaybeMore`

- **Problem Analysis**:
    - **The Ad-Hoc Boolean**: Previously, stream availability was represented across the
      pipeline as a loose boolean flag (`is_more_anticipated: bool` or `more: bool`),
      leading to ambiguity between deterministic user-space slice boundaries and
      probabilistic kernel read queue status.
    - **The SSH Misconception**: Documentation in `input_device_public_api.rs` previously
      claimed that a lone `ESC` byte arriving in a fragmented packet over SSH would
      accumulate with subsequent packets across read boundaries. In reality, when a packet
      arrives with a lone `ESC` and `bytes_read < buffer_size`, the zero-latency heuristic
      treats the stream as drained and emits `ESC` immediately (clearing the accumulator).
      Subsequent bytes (e.g. `[ A`) are parsed as literal characters.
    - **Lack of Unified Reference**: The architectural trade-offs, packet fragmentation
      scenarios, and escape sequence collision behaviors (`ESC`, `Alt+]`, `Alt+[`) were
      fragmented across multiple files without a single source of truth.

- [x] **Phase 7.1: Implement `MaybeMore` Enum & State Machine** in
      `tui/src/core/ansi/vt_100_terminal_input_parser/maybe_more.rs`:
    - Create `MaybeMore` enum with variants:
        - `KernelDrained`: Read returned fewer bytes than capacity; kernel queue drained.
        - `KernelMayHaveMore`: Read saturated buffer; more data likely in transit.
    - Implement `MaybeMore::from_read_count(bytes_read, buffer_capacity)`.
    - Establish the Single Source of Truth (SSOT) documentation:
        - **The Disambiguation Matrix** (`ESC`, `Alt+]`, `Alt+[`).
        - **The Two Levels of Stream Availability** (Deterministic User-Space vs
          Probabilistic Kernel Queue).
        - **Zero-Latency Heuristic vs Fixed Timer Delays** (0ms vs ncurses `ESCDELAY` /
          Vim `ttimeoutlen`).
        - **Packet Fragmentation & Buffer Boundary Scenarios** (Scenarios 1, 2, and 3).

- [x] **Phase 7.2: Refactor Input Pipeline to Use `MaybeMore`**:
    - In `tui/src/core/ansi/vt_100_terminal_input_parser/router.rs`:
        - Update `try_parse_input_event` to take `maybe_more: MaybeMore`.
    - In `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs`:
        - Update `advance` signature to take `maybe_more: MaybeMore`.
    - In
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs`:
        - Evaluate `MaybeMore::from_read_count` at the I/O read boundary.

- [x] **Phase 7.3: Documentation & Cross-Reference Harmonization**:
    - In `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`:
        - Add `### Escape Sequence Disambiguation` to `## Architecture`.
        - Add `### [`maybe_more`]` to `## Module Responsibilities`.
        - Add intra-doc links for `[`maybe_more`]` and `[`MaybeMore`]`.
    - In
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs`:
        - Fix SSH text to accurately describe prefix reassembly vs lone `ESC` trade-off.
        - Add `# Event Parsing & Disambiguation` section heading to `next()` doc comment
        - with links to `StatefulInputParser` and `MaybeMore`.

- [x] **Phase 7.4: Verification**:
    - [x] Run `./check.fish --check`.
    - [x] Run `./check.fish --clippy`.
    - [x] Run `./check.fish --test`.
    - [x] Run `./check.fish --fmt`.
    - [x] Run `./check.fish --quick-doc`.

- [x] **Phase 7.5: Mandatory Manual Review**:
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/maybe_more.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/router.rs`
    - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs`
    - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/handler_stdin.rs`
    - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_public_api.rs`

### [x] Step 8: Fix OSC Input Sequence Handling and Disambiguate `Alt+]`

- **Problem Analysis**:
    - **The Ambiguity**: In VT-100 / ECMA-48 terminals, both the human keystroke `Alt+]`
      and Operating System Command (OSC) responses written by terminal emulators begin
      with the identical 2-byte prefix `ESC ]` (`\x1b]`, `0x1B 0x5D`).
    - **The Current Defect**: In `router.rs`, `[ANSI_ESC, _, ..]` routes `ESC ]` to
      `keyboard::parse_alt_letter()`. Because `]` is printable ASCII (`0x5D`), it
      immediately consumes 2 bytes and emits `Alt+]`. If an OSC response arrives (such as
      a background color response `\x1b]11;rgb:1e1e/1e1e/1e1e\x07`), the router consumes
      `\x1b]`, clears the buffer, and the remaining payload bytes (`11;rgb:...`) spill
      into the application as raw user keystrokes.
    - **The Deadlock Risk on Incomplete OSC**: If the router were to blindly wait for an
      OSC terminator whenever `ESC ] <digit>` is seen, a human typing `Alt+] 5` would
      never send an OSC terminator (`BEL` or `ST`). If the router returns `None` without
      checking `maybe_more == MaybeMore::KernelMayHaveMore`, all future user keystrokes
      would be trapped in the accumulator until reaching the 1MB limit, permanently
      locking up the input event loop and wiping out user input.
    - **The UTF-8 Corruption Risk**: In ECMA-48, 8-bit `ST` is `0x9C`. However, in UTF-8,
      `0x9C` is a valid continuation byte found in common characters like `£`
      (`0xC2 0x9C`), `œ` (`0xC5 0x93`), and `✓` (`0xE2 0x9C 0x93`). Matching `0x9C` as a
      terminator in UTF-8 streams causes premature truncation and leaks subsequent bytes.
      Modern terminals in UTF-8 mode exclusively use 7-bit `ST` (`\x1b\\`) or `BEL`
      (`\x07`).
    - **The Buffer Truncation Defect**: `StatefulInputParser::advance` unconditionally
      clears its buffer on any parsed event (`self.buffer.clear()`). If an escape sequence
      consumes fewer bytes than the buffer holds (e.g. `ESC ] a`), the trailing byte
      (`'a'`) is permanently destroyed. Furthermore, the 64-byte safety fallback in
      `should_discard_unrecognized_sequence` would prematurely truncate valid OSC
      sequences (such as OSC 52 clipboard delivery or OSC 8 hyperlinks) that exceed 64
      bytes.

- [x] **Phase 8.0: Reorganize and Harmonize Input Parser Rustdocs**:
    - In `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`:
        - Update module summary to state that it handles both user keystrokes and terminal
          emulator responses.
        - Insert `## Bidirectional Communication: User Input vs. Terminal Responses` at
          line 9 (before `## Primary Consumer`), explaining the shared `stdin` pipe,
          queries via `stdout`, and why terminal responses (OSC 10/11 color queries, OSC
          52 clipboard, shell/multiplexer queries) arrive on `stdin`.
        - Update the `## Primary Consumer` ASCII dataflow diagram to show the dedicated
          `mio` poller thread reading non-blocking `stdin` into `StatefulInputParser`
          (removing the outdated `tokio::io::stdin()`).
    - In `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs`:
        - Remove the `## Parser Dispatch Priority Pipeline` section (lines 251-294) which
          details dispatching across CSI, SS3, mouse, terminal events, and UTF-8.
        - Keep `keyboard.rs` strictly focused on human keyboard encoding (VT-100 history,
          7-bit ASCII, bitmask formulas, function key quirks, and ambiguous control
          characters).
    - In `tui/src/core/ansi/vt_100_terminal_input_parser/router.rs`:
        - Integrate the `## Parser Dispatch Priority Pipeline` into the doc comment of
          `try_parse_input_event()`, documenting the routing order across `keyboard`,
          `mouse`, `terminal_events`, and `utf8`.
    - **Verification**:
        - [x] Run `./check.fish --quick-doc` to verify that all intra-doc links resolve
              cleanly.
        - [x] Run `./check.fish --check`, `./check.fish --clippy`, and
              `./check.fish --fmt`.
    - **Phase 8.0 Mandatory Manual Review**:
        - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
        - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs`
        - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/router.rs`

- [x] **Phase 8.1: Define OSC Protocol Constants** in
      `tui/src/core/ansi/constants/input_sequences.rs`:
    - Define `ANSI_OSC_CLOSE_BRACKET: u8 = b']'`.
    - Define `OSC_PREFIX: &[u8] = b"\x1b]"` and `OSC_PREFIX_LEN: usize = 2`.
    - Define `ANSI_BEL: u8 = 7; // 0x07 hex` (decimal for non-printable constant).
    - Define `ANSI_ST_7BIT: &[u8] = b"\x1b\\"`.
    - Define
      `MAX_OSC_SEQUENCE_LENGTH: usize = 1_048_576; // 1 MiB (safely accommodates large payloads like OSC 52 clipboard transfers)`.
    - **Note on 8-bit ST**: Do NOT define `0x9C` as an ST terminator. In UTF-8
      environments, `0x9C` is a valid continuation byte. Omitting it preserves UTF-8
      integrity.

- [x] **Phase 8.2: Add `VT100InputEventIR::Ignored`** in
      `tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs`:
    - Add `Ignored` variant to represent protocol control sequences that are recognized
      and consumed, but do not generate application-level input events.
    - Update `convert_input_event` in
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/protocol_conversion.rs` to
      return `None` for `VT100InputEventIR::Ignored`.
    - Update `apply_paste_state_machine` in
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/paste_state_machine.rs` to
      return `PasteStateResult::Absorbed` for `VT100InputEventIR::Ignored`.
    - Update `generate_keyboard_sequence` in `tui/src/core/ansi/generator/ansi_input.rs`
      to return `None` for `VT100InputEventIR::Ignored`.

- [x] **Phase 8.3: Implement OSC State Machine Scanner & Disambiguation Helper** in
      `tui/src/core/ansi/vt_100_terminal_input_parser/terminal_events.rs`:
    - **State Machine Architecture**: Parsing incoming OSC sequences and disambiguating
      them from `Alt+]` requires a formal, type-safe state machine. The state machine
      operates in two layers:
        1. **Lexical Scanner State Machine (`OscScanState` & `scan_osc_sequence`)**:
           Enforces **Rule 2: Strict OSC Syntax Validation** by validating grammar
           transitions:
           `Prefix (ESC ]) -> CommandDigits -> (';' | '?') -> Payload -> (BEL | 7-bit ST)`.
        2. **Disambiguation Decision Helper (`try_disambiguate_osc_or_alt_bracket`)**:
           Evaluates the scanner state machine's output against **Rule 1: The
           [`MaybeMore`] Stream Availability Heuristic**, determining whether to absorb an
           OSC sequence, emit an `Alt+]` keystroke, or wait for more bytes across read
           boundaries.

    - **Enum 1: `OscScanState` (Internal State Machine States)**:

        ```rust
        /// Internal states of the OSC sequence lexical scanner state machine.
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        enum OscScanState {
            /// Scanning decimal command code digits immediately following `ESC ]` (e.g. `11`, `52`).
            CommandDigits,
            /// Scanning payload parameters and content following the delimiter `;` or `?`.
            Payload,
        }
        ```

    - **Enum 2: `OscScanResult` (Scanner Transition & Outcome States)**:

        ```rust
        /// Outcome of scanning a byte buffer with the OSC lexical state machine.
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        pub enum OscScanResult {
            /// Sequence was syntactically valid and cleanly terminated by `BEL` (`\x07`) or 7-bit `ST` (`\x1b\\`).
            /// Wraps total consumed bytes.
            Complete(ByteOffset),
            /// Sequence is scanning decimal command digits (no `;` or `?` delimiter seen yet).
            /// If stream is drained (`maybe_more == MaybeMore::KernelDrained`), this indicates human typing
            /// (e.g. `Alt+]` followed by digits), so the parser falls back to `Alt+]`.
            IncompleteDigits,
            /// Sequence is scanning payload parameters (delimiter was parsed; definitely an in-flight OSC sequence).
            /// Parser must wait across read boundaries for remaining payload bytes without falling back to `Alt+]`.
            IncompletePayload,
            /// Sequence violates OSC grammar (e.g. non-digit before delimiter `;`, embedded `\r` or `\n`, or unexpected `ESC`).
            /// Proves this is human input, not an OSC sequence.
            InvalidSyntax,
            /// Sequence exceeded [`MAX_OSC_SEQUENCE_LENGTH`] without a terminator.
            Runaway,
        }
        ```

    - **Function 1: `pub fn scan_osc_sequence(buffer: &[u8]) -> OscScanResult`**:
      Implements **Rule 2: Strict OSC Syntax Validation**:
        - If `!buffer.starts_with(OSC_PREFIX)`: return `OscScanResult::InvalidSyntax`.
        - State starts at `OscScanState::CommandDigits` at index 2.
        - Iterates through bytes:
            - In `CommandDigits`:
                - If `b.is_ascii_digit()`: stay in `CommandDigits`.
                - If `b == b';'` or `b == b'?'`: transition to `OscScanState::Payload`.
                - If `b == ANSI_BEL` (`7`): return `OscScanResult::Complete(consumed)`.
                - If `b == ANSI_ESC`:
                    - If next byte is `b'\\'`: return `OscScanResult::Complete(consumed)`.
                    - If `b` is the last byte in `buffer`: return
                      `OscScanResult::IncompleteDigits` (partial 7-bit `ST`).
                    - Otherwise: return `OscScanResult::InvalidSyntax`.
                - Otherwise (non-digit before delimiter, whitespace, newline, control
                  char): return `OscScanResult::InvalidSyntax`.
            - In `Payload`:
                - If `b == ANSI_BEL` (`7`): return `OscScanResult::Complete(consumed)`.
                - If `b == ANSI_ESC`:
                    - If next byte is `b'\\'`: return `OscScanResult::Complete(consumed)`.
                    - If `b` is the last byte in `buffer`: return
                      `OscScanResult::IncompletePayload` (partial 7-bit `ST`).
                    - Otherwise (unescaped `ESC` inside payload aborts control string):
                      return `OscScanResult::InvalidSyntax`.
                - If `b == b'\r'` or `b == b'\n'`: return `OscScanResult::InvalidSyntax`
                  (OSC payloads never contain unescaped raw newlines).
                - Otherwise: stay in `Payload`.
        - If buffer exhausted without terminator:
            - If `buffer.len() >= MAX_OSC_SEQUENCE_LENGTH`: return
              `OscScanResult::Runaway`.
            - If state is `OscScanState::CommandDigits`: return
              `OscScanResult::IncompleteDigits`.
            - If state is `OscScanState::Payload`: return
              `OscScanResult::IncompletePayload`.

    - **Function 2: `pub fn try_disambiguate_osc_or_alt_bracket`**: Implements **Rule 1: The
      [`MaybeMore`] Stream Availability Heuristic** and routes based on state machine
      outcome:

        ```rust
        pub fn try_disambiguate_osc_or_alt_bracket(
            buffer: &[u8],
            maybe_more: MaybeMore,
        ) -> Option<(VT100InputEventIR, ByteOffset)> {
            if !buffer.starts_with(OSC_PREFIX) {
                return None;
            }

            // Route based on lexical state machine outcome.
            // Note: When buffer is lone `ESC ]` (len == 2), `scan_osc_sequence` performs 0 iterations
            // and returns `IncompleteDigits`, seamlessly evaluating the `maybe_more` check below.
            match scan_osc_sequence(buffer) {
                OscScanResult::Complete(consumed) => {
                    Some((VT100InputEventIR::Ignored, consumed))
                }
                OscScanResult::InvalidSyntax => {
                    // Violated OSC syntax; cannot be OSC. Emit Alt+] (2 bytes)
                    // and leave trailing bytes in buffer for next cycle.
                    Some((alt_bracket_event(), byte_offset(OSC_PREFIX_LEN)))
                }
                OscScanResult::IncompleteDigits => {
                    match maybe_more {
                        MaybeMore::KernelMayHaveMore => None, // In-flight burst; wait for possible delimiter/payload
                        MaybeMore::KernelDrained => {
                            // Stream drained before delimiter arrived. Human typed Alt+] (alone or with digits).
                            // Emit Alt+] (2 bytes) and leave any trailing digits in buffer.
                            Some((alt_bracket_event(), byte_offset(OSC_PREFIX_LEN)))
                        }
                    }
                }
                OscScanResult::IncompletePayload => {
                    // Delimiter was already parsed. This is guaranteed to be an in-flight OSC sequence.
                    // Always wait for the rest of the payload across reads (bounded by MAX_OSC_SEQUENCE_LENGTH).
                    None
                }
                OscScanResult::Runaway => {
                    // Defer to should_discard_unrecognized_sequence to purge buffer.
                    None
                }
            }
        }

        /// Helper to construct an Alt+] key event.
        #[must_use]
        pub fn alt_bracket_event() -> VT100InputEventIR {
            VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(']'),
                modifiers: VT100KeyModifiersIR {
                    shift: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                    alt: KeyState::Pressed,
                },
            }
        }
        ```

    - **Documentation Architecture & Deliverables (Single Source of Truth)**:
        - **SSOT Mandate**: `maybe_more.rs` is the Single Source of Truth (SSOT) for the
          Disambiguation Matrix, Zero-Latency Heuristic vs. Fixed Timers, and Packet
          Fragmentation Scenarios. `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
          is the SSOT for the high-level architecture and mental model of bidirectional
          terminal communication. `terminal_events.rs` serves as the implementation module
          for scanning and framing, cross-referencing `mod.rs` and `maybe_more.rs`.
          `osc_codes.rs` updates its historical description and links to `mod.rs`.
        - **Deliverable 1: `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
          (SSOT)**:
            - Note: The section
              `## Bidirectional Communication: User Input vs. Terminal Responses` was
              established in Phase 8.0 before `## Primary Consumer`.
            - Add section
              `//! ## Terminal Input Capability Matrix: Legacy VT-100 vs. Kitty Keyboard Protocol`:
                ```rust
                //! ## Terminal Input Capability Matrix: Legacy VT-100 vs. Kitty Keyboard Protocol
                //!
                //! | Keystroke / Protocol Event | Legacy VT-100 / xterm (Default) | Kitty Keyboard Protocol (`CSI u`) | Technical Reason & Ambiguity |
                //! | :--- | :--- | :--- | :--- |
                //! | **Standard Characters (`a-z`, `0-9`, UTF-8)** | ✅ Supported (`UTF-8` bytes) | ✅ Supported (`UTF-8` bytes) | Unambiguous in both modes. |
                //! | **Basic Control Keys (`Ctrl+A` .. `Ctrl+Z`)** | ✅ Supported (`0x01` .. `0x1A`) | ✅ Supported | Standard ASCII control characters. |
                //! | **Enter / Return** | ✅ Supported (`\r`, `0x0D`) | ✅ Supported (`\r` or `CSI 13 u`) | Standard carriage return. |
                //! | **`Shift + Enter`** | ❌ **Collides with Enter** (`\r`) | ✅ **Supported** (`ESC [ 13 ; 2 u`) | Legacy terminals send identical `0x0D` for both. |
                //! | **Tab** | ✅ Supported (`\t`, `0x09`) | ✅ Supported (`\t` or `CSI 9 u`) | Standard horizontal tab. |
                //! | **`Shift + Tab` (BackTab)** | ✅ Supported (`ESC [ Z`) | ✅ Supported (`ESC [ 9 ; 2 u`) | Legacy terminals have standard `CSI Z`. |
                //! | **`Ctrl + Tab`** | ❌ **Collides with Tab** (`\t`) | ✅ **Supported** (`ESC [ 9 ; 5 u`) | Legacy terminals send identical `0x09` for both. |
                //! | **Distinct `Ctrl+I` vs. `Tab`** | ❌ **Indistinguishable** (`0x09`) | ✅ **Supported** (distinct codepoints) | ASCII `Ctrl+I` is literally `0x09` (`Tab`). |
                //! | **Distinct `Ctrl+M` vs. `Enter`** | ❌ **Indistinguishable** (`0x0D`) | ✅ **Supported** (distinct codepoints) | ASCII `Ctrl+M` is literally `0x0D` (`Enter`). |
                //! | **Standalone `Escape` Key** | ✅ Supported (0ms latency) | ✅ Supported (`ESC [ 27 u`) | Legacy uses `MaybeMore::KernelDrained`; Kitty is unambiguous. |
                //! | **Navigation Keys (Arrows, Home, End, PageUp/Dn)** | ✅ Supported (`CSI` / `SS3`) | ✅ Supported (`CSI` / `CSI u`) | Standard xterm / VT220 sequences. |
                //! | **Modified Navigation (`Shift+Home`, `Ctrl+Up`)** | ✅ Supported (`ESC [ 1 ; <m> <final>`) | ✅ Supported | Standard xterm parameter encoding. |
                //! | **Function Keys (`F1` .. `F12`)** | ✅ Supported (VT220 `~` / `SS3`) | ✅ Supported | Standard escape encodings. |
                //! | **`Alt + Key` (Letters & Digits)** | ✅ Supported (`ESC <char>`) | ✅ Supported (`ESC [ <codepoint> ; 3 u`) | Legacy prefixes with `0x1B`. |
                //! | **`Alt + ]` (OSC Prefix Collision)** | ✅ **Solved in Step 8** (`scan_osc_sequence`) | ✅ **Supported** (`ESC [ 93 ; 3 u`) | Step 8 rejects non-digits / uses `MaybeMore::KernelDrained`. |
                //! | **`Alt + [` (CSI Prefix Collision)** | ❌ **Unresolvable in Legacy** | ✅ **Solved in Step 9** (`ESC [ 91 ; 3 u`) | Legacy `Alt+[` is byte-for-byte identical to `CSI` (`\x1b[`). |
                //! | **Terminal OSC Query Replies (Theme, Clipboard)** | ✅ **Solved in Step 8** (Framed & Absorbed) | ✅ Supported (Framed & Absorbed) | Step 8 frames with `scan_osc_sequence`, prevents text leakage. |
                //! | **Key Release & Repeat Events** | ❌ Unsupported by VT-100 | ✅ Supported (via Kitty Flag 2) | Legacy terminals only report key press down events. |
                ```
            - Update `//! ### [`terminal_events`]` section in `mod.rs` with:
                ```rust
                //! ### [`terminal_events`]
                //! - Parse window resize events: `CSI 8 ; rows ; cols t`
                //! - Parse focus gained/lost: `CSI I` / `CSI O`
                //! - Parse bracketed paste markers: `ESC [ 200 ~` / `ESC [ 201 ~`
                //! - Detect, frame, and discard unhandled OSC responses: `ESC ] ... (BEL | ST)`
                //! - Disambiguate lone `Alt+]` from terminal-generated OSC responses
                //! - Provide [`scan_osc_sequence()`], [`try_disambiguate_osc_or_alt_bracket()`], and [`OscScanResult`]
                ```
            - Add reference-style links at bottom of `mod.rs`:
                ```rust
                //! [`OscScanResult`]: crate::vt_100_terminal_input_parser::terminal_events::OscScanResult
                //! [`try_disambiguate_osc_or_alt_bracket()`]: crate::vt_100_terminal_input_parser::terminal_events::try_disambiguate_osc_or_alt_bracket
                //! [`scan_osc_sequence()`]: crate::vt_100_terminal_input_parser::terminal_events::scan_osc_sequence
                ```
        - **Deliverable 2:
          `tui/src/core/ansi/vt_100_terminal_input_parser/terminal_events.rs`**:
            - Update `//! ## Where You Are in the Pipeline` ASCII diagram box to include:
              `│  • Scan and discard unhandled OSC responses │`
            - Update `//! ## Supported Events` list to include:
              `//! - **OSC Responses (Framed & Discarded)**: `ESC ] ... (BEL | ST)``
            - Add new section `//! ## Bidirectional Terminal Communication & OSC Handling`
              right before `use super::ir_event_types...`:
                ```rust
                //! ## Bidirectional Terminal Communication & OSC Handling
                //!
                //! Modern terminal emulators write responses to queries (e.g. background color,
                //! clipboard contents) directly into `stdin`. These responses begin with `ESC ]`
                //! (OSC prefix).
                //!
                //! This module houses [`scan_osc_sequence()`], [`try_disambiguate_osc_or_alt_bracket()`],
                //! and [`OscScanResult`], which inspect input buffers using a dedicated state
                //! machine to distinguish between human keystrokes (such as `Alt+]`) and incoming
                //! terminal responses, ensuring complete OSC payloads are consumed and ignored
                //! without leaking text to the screen.
                //!
                //! For the comprehensive architectural mental model and ASCII data flow diagram, see
                //! the [Bidirectional Communication section in the parent module].
                //!
                //! [Bidirectional Communication section in the parent module]: mod@super#bidirectional-communication-user-input-vs-terminal-responses
                ```
            - Add comprehensive doc comments on `pub enum OscScanResult`,
              `pub fn scan_osc_sequence`, and `pub fn try_disambiguate_osc_or_alt_bracket`:

                ```rust
                /// Result of scanning an input buffer for an Operating System Command ([`OSC`]) sequence.
                ///
                /// Returned by [`scan_osc_sequence()`] to guide the parser in distinguishing between
                /// human keystrokes (`Alt+]`) and terminal emulator query responses.
                ///
                /// [`OSC`]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC
                #[derive(Debug, PartialEq, Eq, Clone, Copy)]
                pub enum OscScanResult {
                    /// A complete OSC sequence was recognized and terminated by either `BEL` (`0x07`)
                    /// or 7-bit `ST` (`ESC \`). The wrapped [`ByteOffset`] indicates the total number
                    /// of bytes consumed from the buffer (prefix + payload + terminator).
                    Complete(ByteOffset),
                    /// The sequence begins with `ESC ]` and follows valid OSC command syntax, but is still
                    /// scanning decimal command digits (no `;` or `?` delimiter arrived yet). If more input
                    /// is anticipated (`maybe_more == MaybeMore::KernelMayHaveMore`), the parser waits.
                    /// If the stream has drained (`maybe_more == MaybeMore::KernelDrained`), this indicates human
                    /// typing (e.g., `Alt+]` followed by digits), and the parser falls back to `Alt+]`.
                    IncompleteDigits,
                    /// The sequence has seen the parameter delimiter (`;` or `?`) and is scanning payload
                    /// content. This is guaranteed to be an in-flight OSC sequence. The parser always waits
                    /// for the remaining payload across read boundaries (bounded by [`MAX_OSC_SEQUENCE_LENGTH`]).
                    IncompletePayload,
                    /// The sequence begins with `ESC ]`, but violates OSC command syntax (such as non-digit
                    /// characters before the parameter delimiter `;`, or embedded newline/carriage return
                    /// characters). This cannot be a valid OSC sequence; the parser immediately rejects it
                    /// and falls back to emitting `Alt+]`.
                    InvalidSyntax,
                    /// The candidate OSC sequence exceeded [`MAX_OSC_SEQUENCE_LENGTH`] without encountering
                    /// a terminator. The buffer is purged to prevent unbounded memory growth and input freezes.
                    Runaway,
                }

                /// State machine that scans a byte buffer for an Operating System Command ([`OSC`]) sequence.
                ///
                /// Terminal query responses (e.g., background color OSC 11, clipboard OSC 52) start with
                /// `ESC ]` (`0x1B 0x5D`), have a numeric command identifier, parameters, and terminate with
                /// either `BEL` (`\x07`) or 7-bit `ST` (`\x1b\\`).
                ///
                /// # State Machine Grammar & Validation Rules
                ///
                /// 1. **Rule 2: Strict OSC Syntax Validation**:
                ///    All standard OSC sequences follow the strict grammar:
                ///    `ESC ] <command_digits> ; <payload> (BEL | ST)`.
                ///    If non-digit characters appear before the parameter delimiter `;`, or if raw
                ///    carriage returns (`\r`) or newlines (`\n`) are encountered, the state machine
                ///    immediately halts and returns [`OscScanResult::InvalidSyntax`]. This prevents
                ///    human keystrokes like `Alt+] 5 a` from being falsely treated as candidate OSC.
                ///
                /// 2. **UTF-8 Safety Note**:
                ///    ECMA-48 specifies `0x9C` as an 8-bit String Terminator (`ST`). However, in [`UTF-8`],
                ///    `0x9C` is a common continuation byte (`1001_1100`, used in characters like `£`, `œ`,
                ///    and `✓`). Modern terminal emulators in [`UTF-8`] mode exclusively send 7-bit `ST` or
                ///    `BEL`. This scanner intentionally does not match `0x9C` to prevent truncating OSC
                ///    payloads containing valid [`UTF-8`] text.
                ///
                /// [`OSC`]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC
                /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
                pub fn scan_osc_sequence(buffer: &[u8]) -> OscScanResult { ... }

                /// Disambiguates an incoming `ESC ]` buffer between an `Alt+]` human keystroke and a
                /// terminal emulator [`OSC`] query response.
                ///
                /// # Disambiguation Rules
                ///
                /// 1. **Rule 1: The [`MaybeMore`] Heuristic**:
                ///    Terminal emulators emit OSC responses in high-speed single-burst writes, whereas
                ///    human keystrokes have tens of milliseconds between them. If all available
                ///    input was drained, the sequence is incomplete, and it is guaranteed to be
                ///    human input (e.g., `Alt+]` alone or `Alt+] 5`), so the function emits `Alt+]`
                ///    (2 bytes consumed) and preserves trailing bytes for the next parse cycle.
                ///    If `maybe_more == MaybeMore::KernelMayHaveMore`, the function defers parsing to allow the
                ///    rest of the burst to arrive.
                ///
                /// 2. **Rule 2: Strict OSC Syntax Validation**:
                ///    If [`scan_osc_sequence()`] returns [`OscScanResult::InvalidSyntax`], the sequence
                ///    cannot be an OSC response; `Alt+]` is emitted immediately.
                ///
                /// [`OSC`]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC
                /// [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
                pub fn try_disambiguate_osc_or_alt_bracket(
                    buffer: &[u8],
                    maybe_more: MaybeMore,
                ) -> Option<(VT100InputEventIR, ByteOffset)> { ... }
                ```

        - **Deliverable 3: `tui/src/core/osc/osc_codes.rs`**:
            - Replace lines 9-16 with updated bidirectional explanation and
              cross-reference:
                ```rust
                //! # Data Flow
                //!
                //! **Bidirectional (Child Process <-> [`PTY`] <-> Terminal Emulator)**:
                //!
                //! Historically, [`OSC`] sequences were considered unidirectional (the child process sends
                //! commands to the terminal to set titles, hyperlinks, or notifications). In modern terminals,
                //! however, [`OSC`] sequences are frequently **bidirectional**:
                //!
                //! - **Queries to stdout**: A TUI app writes queries to the terminal (e.g., color queries
                //!   `OSC 10`/`11`, clipboard queries `OSC 52`).
                //! - **Responses to stdin**: The terminal emulator synthesizes responses and writes them back
                //!   into `stdin`.
                //!
                //! For a comprehensive explanation of bidirectional terminal communication and how incoming
                //! OSC responses are safely parsed and framed on `stdin`, see the
                //! [`vt_100_terminal_input_parser`](crate::core::ansi::vt_100_terminal_input_parser) module.
                ```

        - **Deliverable 4: `tui/src/lib.rs` (Crate-Level Documentation)**:
            - Add section
              `//! ## Terminal Input Capabilities: Legacy VT-100 vs. Kitty Keyboard Protocol`
              right after `## direct_to_ansi backend (Linux-native)` and before
              `## Architecture`:
                ```rust
                //! ## Terminal Input Capabilities: Legacy VT-100 vs. Kitty Keyboard Protocol
                //!
                //! In terminal applications, input handling is inherently split between traditional
                //! legacy VT-100 conventions (which date back to 1978 and lack distinction for many key
                //! combinations) and modern enhanced protocols like the [Kitty Keyboard Protocol].
                //!
                //! ### Sans-IO Protocol Architecture
                //!
                //! Crucially, input parsing in `r3bl_tui` follows a **Sans-IO architecture**:
                //! - The protocol parser ([`vt_100_terminal_input_parser`]) is purely functional:
                //!   it consumes raw byte slices (`&[u8]`) and produces structured intermediate
                //!   representations ([`VT100InputEventIR`]) with zero knowledge of threads, file
                //!   descriptors, or operating system I/O syscalls.
                //! - The I/O layer ([`direct_to_ansi`]) uses an asynchronous [`mio`] poller thread to
                //!   read from non-blocking `stdin`, buffering bytes into `StatefulInputParser` before
                //!   passing them to the Sans-IO parser.
                //! - This decoupling means [`vt_100_terminal_input_parser`] is fully reusable across
                //!   multiple consumers, including [`pty_mux`] multiplexer sessions, headless test
                //!   harnesses, session replay tools, and future platform backends.
                //!
                //! ### Capability Matrix
                //!
                //! The following matrix establishes the ground truth for input handling capabilities
                //! across both modes in `r3bl_tui`:
                //!
                //! | Keystroke / Protocol Event | Legacy VT-100 / xterm (Default) | Kitty Keyboard Protocol (`CSI u`) | Technical Reason & Ambiguity |
                //! | :--- | :--- | :--- | :--- |
                //! | **Standard Characters (`a-z`, `0-9`, UTF-8)** | ✅ Supported (`UTF-8` bytes) | ✅ Supported (`UTF-8` bytes) | Unambiguous in both modes. |
                //! | **Basic Control Keys (`Ctrl+A` .. `Ctrl+Z`)** | ✅ Supported (`0x01` .. `0x1A`) | ✅ Supported | Standard ASCII control characters. |
                //! | **Enter / Return** | ✅ Supported (`\r`, `0x0D`) | ✅ Supported (`\r` or `CSI 13 u`) | Standard carriage return. |
                //! | **`Shift + Enter`** | ❌ **Collides with Enter** (`\r`) | ✅ **Supported** (`ESC [ 13 ; 2 u`) | Legacy terminals send identical `0x0D` for both. |
                //! | **Tab** | ✅ Supported (`\t`, `0x09`) | ✅ Supported (`\t` or `CSI 9 u`) | Standard horizontal tab. |
                //! | **`Shift + Tab` (BackTab)** | ✅ Supported (`ESC [ Z`) | ✅ Supported (`ESC [ 9 ; 2 u`) | Legacy terminals have standard `CSI Z`. |
                //! | **`Ctrl + Tab`** | ❌ **Collides with Tab** (`\t`) | ✅ **Supported** (`ESC [ 9 ; 5 u`) | Legacy terminals send identical `0x09` for both. |
                //! | **Distinct `Ctrl+I` vs. `Tab`** | ❌ **Indistinguishable** (`0x09`) | ✅ **Supported** (distinct codepoints) | ASCII `Ctrl+I` is literally `0x09` (`Tab`). |
                //! | **Distinct `Ctrl+M` vs. `Enter`** | ❌ **Indistinguishable** (`0x0D`) | ✅ **Supported** (distinct codepoints) | ASCII `Ctrl+M` is literally `0x0D` (`Enter`). |
                //! | **Standalone `Escape` Key** | ✅ Supported (0ms latency) | ✅ Supported (`ESC [ 27 u`) | Legacy uses `MaybeMore::KernelDrained`; Kitty is unambiguous. |
                //! | **Navigation Keys (Arrows, Home, End, PageUp/Dn)** | ✅ Supported (`CSI` / `SS3`) | ✅ Supported (`CSI` / `CSI u`) | Standard xterm / VT220 sequences. |
                //! | **Modified Navigation (`Shift+Home`, `Ctrl+Up`)** | ✅ Supported (`ESC [ 1 ; <m> <final>`) | ✅ Supported | Standard xterm parameter encoding. |
                //! | **Function Keys (`F1` .. `F12`)** | ✅ Supported (VT220 `~` / `SS3`) | ✅ Supported | Standard escape encodings. |
                //! | **`Alt + Key` (Letters & Digits)** | ✅ Supported (`ESC <char>`) | ✅ Supported (`ESC [ <codepoint> ; 3 u`) | Legacy prefixes with `0x1B`. |
                //! | **`Alt + ]` (OSC Prefix Collision)** | ✅ **Solved in Step 8** (`scan_osc_sequence`) | ✅ **Supported** (`ESC [ 93 ; 3 u`) | Step 8 rejects non-digits / uses `MaybeMore::KernelDrained`. |
                //! | **`Alt + [` (CSI Prefix Collision)** | ❌ **Unresolvable in Legacy** | ✅ **Solved in Step 9** (`ESC [ 91 ; 3 u`) | Legacy `Alt+[` is byte-for-byte identical to `CSI` (`\x1b[`). |
                //! | **Terminal OSC Query Replies (Theme, Clipboard)** | ✅ **Solved in Step 8** (Framed & Absorbed) | ✅ Supported (Framed & Absorbed) | Step 8 frames with `scan_osc_sequence`, prevents text leakage. |
                //! | **Key Release & Repeat Events** | ❌ Unsupported by VT-100 | ✅ Supported (via Kitty Flag 2) | Legacy terminals only report key press down events. |
                //!
                //! For deeper protocol architecture and parser implementation, see the
                //! [`vt_100_terminal_input_parser`](crate::core::ansi::vt_100_terminal_input_parser) module.
                //!
                //! [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
                //! [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
                //! [`direct_to_ansi`]: crate::direct_to_ansi
                //! [`mio`]: https://docs.rs/mio
                //! [`pty_mux`]: crate::core::pty_mux
                //! [`vt_100_terminal_input_parser`]: crate::vt_100_terminal_input_parser
                ```

- [x] **Phase 8.4: Route OSC in `router.rs`**:
    - In `try_parse_input_event`, delegate `[ANSI_ESC, ANSI_OSC_CLOSE_BRACKET, ..]` to the
      disambiguation helper function (placed immediately before the `[ANSI_ESC, _, ..]`
      catch-all arm so it is not shadowed):
        ```rust
        // OSC sequence or Alt+] keypress disambiguation.
        [ANSI_ESC, ANSI_OSC_CLOSE_BRACKET, ..] => {
            terminal_events::try_disambiguate_osc_or_alt_bracket(buffer, maybe_more)
        }
        ```
    - Update `router.rs` documentation to describe the routing table entry and reference
      [`terminal_events::try_disambiguate_osc_or_alt_bracket`]:
        ```rust
        //! ## Disambiguating `Alt+]` from Terminal OSC Responses
        //!
        //! In VT-100 terminals, both the `Alt+]` key combination and terminal-generated
        //! [`OSC`] responses begin with `ESC ]` (`\x1b]`).
        //!
        //! Routing for `ESC ]` delegates directly to
        //! [`terminal_events::try_disambiguate_osc_or_alt_bracket()`], which uses a dedicated
        //! state machine implementing Rule 1 (the [`MaybeMore`] stream availability heuristic)
        //! and Rule 2 (strict OSC syntax validation) to safely distinguish between user
        //! keystrokes and terminal query responses.
        ```

- [x] **Phase 8.5: Fix `StatefulInputParser` Buffer Draining & Multi-Event Dispatch**:
    - In `StatefulInputParser::advance`
      (`tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs`):
        - Refactor `advance` to batch-append incoming bytes and drain iteratively:
            ```rust
            self.accumulator.extend_from_slice(read_buffer);
            while !self.accumulator.is_empty() {
                match try_parse_input_event(&self.accumulator, maybe_more) {
                    Some((event, bytes_consumed)) => {
                        let consumed = bytes_consumed.as_usize();
                        debug_assert!(
                            consumed > 0,
                            "Parser must consume at least 1 byte to prevent infinite loops"
                        );
                        if consumed == 0 {
                            break;
                        }
                        if event != VT100InputEventIR::Ignored {
                            self.internal_events.push_back(event);
                        }
                        self.accumulator.drain(..consumed);
                    }
                    None => {
                        if self.should_discard_unrecognized_sequence() {
                            DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| {
                                tracing::warn! {
                                    message = "StatefulInputParser::advance - discarding unrecognized escape sequence",
                                    discarded_hex = %format!("{:02X?}", self.accumulator),
                                    discarded_str = %String::from_utf8_lossy(&self.accumulator),
                                    buffer_len = self.accumulator.len(),
                                };
                            });
                            self.accumulator.clear();
                        }
                        break;
                    }
                }
            }
            ```
    - In `StatefulInputParser::should_discard_unrecognized_sequence`:
        - Differentiate between OSC sequences and normal CSI/SS3 escape sequences:

            ```rust
            // OSC sequence runaway check (1 MiB).
            if self.accumulator.starts_with(OSC_PREFIX) {
                return scan_osc_sequence(&self.accumulator) == OscScanResult::Runaway;
            }

            // Completed CSI sequence that could not be parsed.
            if self.accumulator.starts_with(CSI_PREFIX)
                && self.accumulator.len() >= CSI_MIN_LEN
                && self.accumulator[CSI_PREFIX_LEN..]
                    .iter()
                    .any(|b| (CSI_FINAL_BYTE_MIN..=CSI_FINAL_BYTE_MAX).contains(b))
            {
                return true;
            }

            // Completed SS3 sequence that could not be parsed.
            if self.accumulator.starts_with(SS3_PREFIX) && self.accumulator.len() >= SS3_SEQ_LEN {
                return true;
            }

            // Safety fallback for non-OSC sequences (64 bytes).
            if self.accumulator.len() >= MAX_ESCAPE_SEQUENCE_LENGTH {
                return true;
            }

            false
            ```

        - This prevents valid, long OSC sequences (> 64 bytes) from being prematurely
          purged, while preserving strict 64-byte overflow protection for malformed
          CSI/SS3 streams.

- [x] **Phase 8.6: Comprehensive Unit & Integration Tests**:
    - **In `tui/src/core/ansi/vt_100_terminal_input_parser/terminal_events.rs`**:
        - [x] Test `scan_osc_sequence` with `BEL` (`\x07`) -> returns
              `OscScanResult::Complete`.
        - [x] Test `scan_osc_sequence` with 7-bit `ST` (`\x1b\\`) -> returns
              `OscScanResult::Complete`.
        - [x] Test `scan_osc_sequence` with incomplete digits (e.g. `ESC ] 1 2`) ->
              returns `OscScanResult::IncompleteDigits`.
        - [x] Test `scan_osc_sequence` with incomplete payload (e.g. `ESC ] 1 2 ; data`)
              -> returns `OscScanResult::IncompletePayload`.
        - [x] Test `scan_osc_sequence` with partial `ST` in payload (buffer ends in lone
              `\x1b`) -> returns `OscScanResult::IncompletePayload`.
        - [x] Test `scan_osc_sequence` with partial `ST` in digits (e.g. `ESC ] 0 \x1b`)
              -> returns `OscScanResult::IncompleteDigits`.
        - [x] Test `scan_osc_sequence` with invalid syntax (letters before semicolon, e.g.
              `ESC ] 1 2 a ;`) -> returns `OscScanResult::InvalidSyntax`.
        - [x] Test `scan_osc_sequence` with embedded newline before terminator -> returns
              `OscScanResult::InvalidSyntax`.
        - [x] Test `scan_osc_sequence` with unexpected `ESC` inside payload not followed
              by `\` -> returns `OscScanResult::InvalidSyntax`.
        - [x] Test `scan_osc_sequence` with UTF-8 continuation byte `0x9C` (e.g. `✓`
              `\xE2\x9C\x93` or `£` `\xC2\x9C`) -> does NOT terminate early; remains
              `IncompletePayload` until `BEL` or `ST`.
        - [x] Test `scan_osc_sequence` exceeding `MAX_OSC_SEQUENCE_LENGTH` -> returns
              `OscScanResult::Runaway`.
    - **In `tui/src/core/ansi/vt_100_terminal_input_parser/router.rs`**:
        - [x] Test lone `Alt+]` with `MaybeMore::KernelDrained` -> emits `Alt+]` (2
              bytes).
        - [x] Test lone `Alt+]` with `MaybeMore::KernelMayHaveMore` -> returns `None`.
        - [x] Test `Alt+]` followed by non-digit (`ESC ] a`) -> emits `Alt+]` (2 bytes).
        - [x] Test `Alt+]` followed by digit with `MaybeMore::KernelDrained` (`ESC ] 5`)
              -> emits `Alt+]` (2 bytes), leaves `5` in buffer.
        - [x] Test `Alt+]` followed by digit with `MaybeMore::KernelMayHaveMore`
              (`ESC ] 5`) -> returns `None`.
        - [x] Test candidate OSC (`ESC ] 0 ... BEL`) -> returns `Some((Ignored, len))`.
        - [x] Test candidate OSC with payload across chunk boundaries when
              `MaybeMore::KernelDrained` -> returns `None` (waits for payload).
        - [x] Test candidate OSC with invalid syntax -> emits `Alt+]` (2 bytes).
        - [x] Test runaway candidate OSC -> returns `None`.
    - **In `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs`**:
        - [x] Test lone `Alt+]` keypress (single chunk and split reads).
        - [x] Test `Alt+]` followed by multiple characters in same chunk (`ESC ] abc`
              emits `Alt+]`, `'a'`, `'b'`, `'c'`).
        - [x] Test `Alt+]` followed by digit in same chunk when `more == false` (`ESC ] 5`
              emits `Alt+]`, then `'5'`).
        - [x] Test OSC sequence terminated with `BEL` and 7-bit `ST` cleanly absorbed with
              0 text leakage.
        - [x] Test long OSC sequence (> 64 bytes, e.g. 120 bytes) cleanly absorbed without
              being purged by 64-byte limit.
        - [x] Test OSC sequence with UTF-8 continuation byte `0x9C` cleanly absorbed with
              0 text leakage.
        - [x] Test OSC sequence followed by typing in same chunk (`ESC ] 0 ; title BEL a`
              absorbs OSC, emits `'a'`).
        - [x] Test chunked OSC sequences split across multiple `advance()` calls where
              intermediate reads have `more == false`.
        - [x] Test chunked OSC sequence followed by typing across chunk boundaries (Chunk
              1: `ESC ] 0 ; ti`, Chunk 2: `tle BEL a` absorbs OSC, emits `'a'`).
        - [x] Test runaway unterminated OSC exceeding `MAX_OSC_SEQUENCE_LENGTH`: buffer
              purged, 0 text leakage.
        - [x] Test post-purge recovery: typing immediately after runaway purge succeeds
              without input freeze.
    - **In
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/protocol_conversion.rs`**:
        - [x] Test `convert_input_event` with `VT100InputEventIR::Ignored` returns `None`.
    - **In
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/paste_state_machine.rs`**:
        - [x] Test `apply_paste_state_machine` with `VT100InputEventIR::Ignored` returns
              `PasteStateResult::Absorbed` in both `Inactive` and `Accumulating` states.
    - **In `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`**:
        - [x] Add `Alt+]` test sequence to verify Crossterm and DirectToAnsi backend
              parity.

- [x] **Phase 8.7: Verification**:
    - [x] Run `./check.fish --check`.
    - [x] Run `./check.fish --clippy`.
    - [x] Run `./check.fish --test`.
    - [x] Run `./check.fish --fmt`.
    - [x] Run `cd tui && cargo readme > README.md` to regenerate `tui/README.md` from
          `tui/src/lib.rs`.

### [x] Step 9: Handle `Alt+[` via Kitty Keyboard Protocol (`CSI u`)

- **Problem Analysis**:
    - **The Legacy Limitation**: In legacy VT-100, `Alt+[` emits `ESC [` (`\x1b[`), which
      is byte-for-byte identical to the CSI escape sequence prefix. As documented in
      `maybe_more.rs`, it is mathematically impossible to disambiguate a lone `Alt+[` from
      an incomplete CSI prefix (e.g. arrow keys fragmented over SSH) using zero-latency
      heuristics without introducing a 25-100ms timer delay.
    - **The Modern Resolution**: Modern terminal emulators (Kitty, Ghostty, WezTerm, Foot,
      and terminals supporting `CSI u` / progressive keyboard enhancement) solve this
      through the **Kitty Keyboard Protocol**. Under this protocol, modified keys are
      encoded unambiguously as `CSI <codepoint> ; <modifier> u`.
    - **`Alt+[` Representation**: For `Alt+[`, the sequence is `ESC [ 91 ; 3 u`
      (`\x1b[91;3u`), where `91` is the Unicode/ASCII codepoint for `[` and `3` represents
      the `Alt` modifier (`1 + 2`).
    - **Current Defect in DirectToAnsi**: `protocol_conversion.rs` notes that the
      `direct_to_ansi` backend currently does not parse `CSI u` sequences. Any incoming
      `CSI u` sequences are treated as unrecognized escape sequences and dropped.

- [x] **Phase 9.1: Define `CSI u` Protocol Constants** in
      `tui/src/core/ansi/constants/input_sequences.rs`:
    - Define `ANSI_CSI_U: u8 = b'u'`.

- [x] **Phase 9.2: Implement `CSI u` Parser** in
      `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs`:
    - Implement
      `parse_csi_u_sequence(buffer: &[u8]) -> Option<(VT100InputEventIR, ByteOffset)>`.
    - Grammar: `ESC [ <codepoint> [; <modifiers> [: <event_type>]] u`.
    - Decode standard modifier masks:
        - `1`: No modifier
        - `2`: Shift
        - `3`: Alt
        - `4`: Shift+Alt
        - `5`: Ctrl
        - `6`: Shift+Ctrl
        - `7`: Alt+Ctrl
        - `8`: Shift+Alt+Ctrl
    - Decode codepoints:
        - Printable ASCII / Unicode (e.g. `91` -> `Char('[')`).
        - Control / Special keys (e.g. `13` -> `Enter`, `9` -> `Tab`, `27` -> `Escape`).
    - Integrate `parse_csi_u_sequence` into `parse_keyboard_sequence`.

- [x] **Phase 9.3: Update Protocol Conversion & Key Mapping** in
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/protocol_conversion.rs`:
    - Map parsed `VT100InputEventIR` into `InputEvent::Keyboard(KeyPress)` with proper
      `ModifierKeysMask`.
    - Specifically ensure `Alt+[` produces `Key::Character('[')` with
      `ModifierKeysMask::ALT`.

- [x] **Phase 9.4: Protocol Capability Negotiation (Progressive Enhancement)**:
    - In `tui/src/core/terminal_io/terminal_mode_controller.rs`:
        - Add `enable_keyboard_enhancement(&self) -> miette::Result<()>`:
            - For `TerminalLibBackend::Crossterm`: queue
              `crossterm::event::PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)`.
            - For `TerminalLibBackend::DirectToAnsi`: write `b"\x1b[>1u"` to `stdout` and
              flush.
        - Add `disable_keyboard_enhancement(&self) -> miette::Result<()>`:
            - For `TerminalLibBackend::Crossterm`: queue
              `crossterm::event::PopKeyboardEnhancementFlags`.
            - For `TerminalLibBackend::DirectToAnsi`: write `b"\x1b[<1u"` to `stdout` and
              flush (poison-safe via `lock_raw_poison_safe`).
    - Wire into `TerminalModeController::setup()` and
      `TerminalModeController::teardown()`.

- [x] **Phase 9.5: Unit & Integration Tests**:
    - [x] Test `ESC [ 91 ; 3 u` parses into `Alt+[` in `keyboard.rs`.
    - [x] Test `ESC [ 13 ; 2 u` parses into `Shift+Enter`.
    - [x] Test `ESC [ 9 ; 5 u` parses into `Ctrl+Tab`.
    - [x] Test `ESC [ 27 ; 3 u` parses into `Alt+Escape`.
    - [x] Test event type handling (e.g. key press `:1` accepted, key release `:3`
          ignored).
    - [x] Test round-trip generation and parsing in `generator_round_trip_tests.rs`.
    - [x] In `backend_compat_input_test.rs`: verify parity between Crossterm and
          DirectToAnsi for `Alt+[` and modified navigation keys.
    - [x] In `maybe_more.rs`: update Disambiguation Matrix status for `Alt+[` from
          `Documented Limitation` to `Solved (via Kitty Protocol)`.
    - [x] In `tui/src/lib.rs` and `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`:
          update Capability Matrix status for `Alt+[` and modified keys (`Shift+Enter`,
          `Ctrl+Tab`) from `Unresolvable in Legacy` to
          `Solved in Step 9 (via Kitty Protocol)`.

- [x] **Phase 9.6: Verification**:
    - [x] Run `./check.fish --check`.
    - [x] Run `./check.fish --clippy`.
    - [x] Run `./check.fish --test`.
    - [x] Run `./check.fish --fmt`.
    - [x] Run `cd tui && cargo readme > README.md` to regenerate `tui/README.md` from
          `tui/src/lib.rs`.

- [x] **Step 10: Remove bypass of `TERMINAL_LIB_BACKEND`**

- **Problem Analysis (Bypass of `TERMINAL_LIB_BACKEND` on Output Side)**:
    1. **Input is already pure `DirectToAnsi`**: On Linux, both `choose()` and
       `readline_async()` already construct
       `tui/src/core/terminal_io/input_device.rs:80-101`, which instantiates the input
       device using `mio_poller` (`epoll`/`eventfd`).
    2. **Terminal raw mode is already pure `rustix`**: Entering and exiting raw mode uses
       `tui/src/core/ansi/terminal_raw_mode/raw_mode_unix.rs:178`, which manipulates Linux
       `termios` directly via `rustix`.
    3. **The line editor is already pure ANSI**: `LineState` (prompt rendering, typing,
       cursor navigation, clearing line in
       `tui/src/readline_async/readline_async_impl/line_state/render.rs`) writes typed
       ANSI sequences (`tui/src/core/ansi/constants/csi.rs`, `CSI_ERASE_DISPLAY_TO_END`)
       directly to the writer with zero Crossterm overhead.
    4. **Where the inconsistency lives (The Output Side)**:
        - `readline_async()` startup/shutdown toggles: Uses hardcoded Crossterm commands
          (`cursor::Hide`, `terminal::EnableLineWrap`, `cursor::Show`,
          `terminal::Clear(ClearType::All)`) in
          `tui/src/readline_async/readline_async_impl/readline_struct.rs` rather than
          going through `terminal_mode_controller.rs`.
        - `choose()` rendering (`tui/src/readline_async/choose_impl/select_component.rs`):
          Uses `tui/src/readline_async/choose_impl/crossterm_macros.rs:11`
          (`queue_commands!`) which directly calls
          `crossterm::QueueableCommand::queue(...)` (`MoveToColumn`, `Clear(CurrentLine)`,
          `MoveToNextLine`, `MoveToPreviousLine`).
        - `spinner` rendering (`tui/src/readline_async/spinner_impl/spinner_print.rs`):
          Also uses `queue_commands_no_lock!` with direct Crossterm commands (`Hide`,
          `Show`, `Clear`, `MoveToNextLine`, `MoveToPreviousLine`, `MoveToColumn`).

    Because Crossterm happens to emit standard ANSI escape bytes to stdout on Linux, the
    terminal emulator understands it fine today. However, it violates our architectural
    goal of letting `TERMINAL_LIB_BACKEND` control the output pipeline.

- [x] **Phase 10.1: Extend `TerminalModeController` & `ansi_output::terminal_modes`**:
    - [x] In `tui/src/core/ansi/generator/ansi_output.rs` (`terminal_modes` module):
        - Add
          `pub fn enable_line_wrap() -> &'static str { const_format::formatcp!("{CSI_START}?7h") }`
        - Add
          `pub fn disable_line_wrap() -> &'static str { const_format::formatcp!("{CSI_START}?7l") }`
    - [x] In `tui/src/core/terminal_io/terminal_mode_controller.rs`
          (`TerminalModeController` trait):
        - Add `fn enable_line_wrap(&self) -> miette::Result<()>`
        - Add `fn disable_line_wrap(&self) -> miette::Result<()>`
        - Add `fn clear_screen(&self) -> miette::Result<()>`
    - [x] In `tui/src/core/terminal_io/terminal_mode_controller.rs`
          (`impl TerminalModeController for OutputDevice`):
        - Implement `enable_line_wrap(&self)`: match on `TERMINAL_LIB_BACKEND` (Crossterm:
          `writer.queue(terminal::EnableLineWrap)`, DirectToAnsi: write
          `ansi_output::terminal_modes::enable_line_wrap()`).
        - Implement `disable_line_wrap(&self)`: match on `TERMINAL_LIB_BACKEND`
          (Crossterm: `writer.queue(terminal::DisableLineWrap)`, DirectToAnsi: write
          `ansi_output::terminal_modes::disable_line_wrap()`).
        - Implement `clear_screen(&self)`: match on `TERMINAL_LIB_BACKEND` (Crossterm:
          `writer.queue(terminal::Clear(ClearType::All))`, DirectToAnsi: write
          `ansi_output::screen_clearing::clear_screen()`).

- [x] **Phase 10.2: Refactor `readline_async` to use `TerminalModeController`**:
    - [x] In `tui/src/readline_async/readline_async_impl/readline_struct.rs`:
        - Replace `execute_commands_no_lock!(writer, cursor::Hide);` and
          `execute_commands_no_lock!(writer, terminal::EnableLineWrap);` with
          `output_device.hide_cursor()?;` and `output_device.enable_line_wrap()?;`.
        - Replace `drop(term.execute(cursor::Show));` on shutdown/drop with
          `output_device.show_cursor()?;`.
        - Replace `term.queue(Clear(terminal::ClearType::All))?;` in
          `LineStateControlSignal::Clear` with `output_device.clear_screen()?;`.
        - Remove `use crossterm::{cursor, terminal};` imports from `readline_struct.rs`.

- [x] **Phase 10.3: Refactor `choose()` Lifecycle**:
    - [x] In `tui/src/readline_async/choose_impl/event_loop.rs`:
        - In `run_before_event_loop`: replace
          `execute_commands!(function_component.get_output_device(), Hide);` with
          `function_component.get_output_device().hide_cursor()?;`.
        - In `run_after_event_loop`: replace
          `execute_commands!(function_component.get_output_device(), Show);` with
          `function_component.get_output_device().show_cursor()?;`.

- [x] **Phase 10.4: Refactor `choose()` and `spinner` Inline Painting**:
    - [x] In `tui/src/readline_async/choose_impl/select_component.rs`:
        - Update rendering (`render_header`, `render_items`, `move_cursor_back_to_start`)
          to respect `TERMINAL_LIB_BACKEND`:
            - When `DirectToAnsi`: use `ansi_output::cursor_movement::cursor_to_column`,
              `ansi_output::screen_clearing::clear_current_line`,
              `ansi_output::cursor_movement::cursor_next_line`,
              `ansi_output::cursor_movement::cursor_previous_line`, and `SGR_RESET_STR`.
            - When `Crossterm`: use existing `queue_commands!` / Crossterm commands.
    - [x] In `tui/src/readline_async/spinner_impl/spinner_print.rs`:
        - Update `clear_lines_for_spinner` to respect `TERMINAL_LIB_BACKEND`:
            - When `DirectToAnsi`: use `ansi_output` cursor movement, clearing, and
              visibility sequences.
            - When `Crossterm`: use existing Crossterm commands.

- [x] **Phase 10.5: Verification**:
    - [x] Run `./check.fish --check`.
    - [x] Run `./check.fish --clippy`.
    - [x] Run `./check.fish --test`.
    - [x] Run `./check.fish --fmt`.
    - [x] Run `cd tui && cargo readme > README.md`.

- [x] **Phase 11: Consolidate `term` into `terminal_io::capabilities`**:
    - [x] Create directory `tui/src/core/terminal_io/capabilities/` and move/rename files:
        - `tui/src/core/term/constants.rs` ->
          `tui/src/core/terminal_io/capabilities/constants.rs`
        - `tui/src/core/term/term_api.rs` ->
          `tui/src/core/terminal_io/capabilities/capabilities_api.rs`
        - `tui/src/core/term/term_api_impl.rs` ->
          `tui/src/core/terminal_io/capabilities/capabilities_api_impl.rs`
        - `tui/src/core/term/term_integration_tests/` ->
          `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/`
        - Create `tui/src/core/terminal_io/capabilities/mod.rs` with re-exports
    - [x] Remove `tui/src/core/term/` directory
    - [x] Update `tui/src/core/terminal_io/mod.rs`:
        - Add `mod capabilities;`
        - Add `pub use capabilities::*;`
    - [x] Update `tui/src/core/mod.rs`:
        - Remove `pub mod term;` and `pub use term::*;`
    - [x] Update any intra-doc links or references to `core::term` (e.g.
          `channel_types.rs:75`)
    - [x] Run verification:
        - [x] `./check.fish --check`
        - [x] `./check.fish --test`
        - [x] `./check.fish --clippy`
        - [x] `./check.fish --quick-doc`

### [x] Step 10: Implement OSC 52 Clipboard Support (Copy Fallback & Comprehensive Documentation)

- **Problem Analysis & Architectural Motivation**:
    1. **SSH & Headless Clipboard Isolation (The Copy Problem)**: Currently,
       `SystemClipboard` in
       `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service.rs` uses the
       `copypasta` crate, which requires a local desktop display server connection (X11
       socket, Wayland compositor, macOS Cocoa `NSPasteboard`, or Win32 API). When running
       an R3BL TUI editor in a remote SSH session, Docker container, or headless
       environment, `copypasta` fails because `$DISPLAY` / `$WAYLAND_DISPLAY` are unset.
    2. **The In-Band Copy Solution (OSC 52)**: Modern terminal emulators (Alacritty,
       Kitty, iTerm2, WezTerm, Windows Terminal, Foot) support Operating System Command 52
       (`OSC 52`) to write the client host desktop clipboard in-band over `stdout` without
       requiring X11 forwarding or network daemons. Formatted as
       `ESC ] 52 ; c ; <base64_data> BEL`, writing this sequence to `stdout` instructs the
       client terminal emulator to place the text directly onto the client desktop
       clipboard.
    3. **Pasting is Handled by Bracketed Paste (Not OSC 52 Query)**: When a user pastes
       text into the terminal (Ctrl+V / Shift+Insert / Right-Click), the terminal emulator
       brackets the clipboard text using DEC Private Mode 2004
       (`CSI 200 ~ <content> CSI 201 ~`), which `r3bl_tui` already captures and converts
       into `InputEvent::BracketedPaste`. We intentionally do NOT query the terminal
       clipboard via `OSC 52 ; c ; ?` because querying is asynchronous, creates latency,
       and is security-restricted/disabled in many secure terminals (Kitty, tmux) to
       prevent clipboard snooping.
    4. **The Input Parser's Role (Quarantine & Absorption)**: Outgoing OSC 52 copy
       commands are intercepted and consumed by the terminal emulator directly from
       `stdout`. They never touch `stdin`. However, unsolicited OSC sequences (such as
       query responses generated by external scripts, terminal multiplexers like tmux, or
       reflected sequences) may arrive on `stdin`. In `terminal_events.rs`, the input
       parser's job is strictly **quarantine and absorption**: safely framing complete OSC
       sequences and mapping them to `VT100InputEventIR::Ignored` so that raw control
       characters, delimiters, and base64 payloads never leak into the active editor
       buffer as typed characters.

- [x] **Phase 10.1: Add `base64` Dependency & Define OSC 52 Protocol Constants**:
    - [x] In `tui/Cargo.toml`:
        - Add `base64 = "0.22.1"` (matching `cmdr/Cargo.toml`).
    - [x] In `tui/src/core/ansi/constants/input_sequences.rs`:
        - Define `pub const OSC_CODE_CLIPBOARD: &str = "52";`
        - Define `pub const CLIPBOARD_TARGET_CLIPBOARD: u8 = b'c';` (system clipboard)
        - Define `pub const CLIPBOARD_TARGET_PRIMARY: u8 = b'p';` (primary selection)

- [x] **Phase 10.2: Implement Outgoing `OscSequence::ClipboardSet`**:
    - [x] In `tui/src/core/osc/osc_codes.rs`:
        - Add `ClipboardTarget` enum:
            ```rust
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum ClipboardTarget {
                Clipboard,
                Primary,
            }
            ```
        - Add variant to `OscSequence`:
            ```rust
            pub enum OscSequence {
                // ...
                ClipboardSet {
                    target: ClipboardTarget,
                    data: String,
                },
            }
            ```
        - Implement zero-allocation formatting for `ClipboardSet` in
          `FastStringify for OscSequence`:
            - Base64-encode `data` using standard engine
              (`base64::prelude::BASE64_STANDARD.encode`).
            - Format `\x1b]52;{target};{base64}\x07`.
        - Add unit tests for `OscSequence::ClipboardSet` formatting (empty string, ASCII,
          UTF-8 text, and checkmark `✓`).

- [x] **Phase 10.3: Implement `Osc52Clipboard` & Hybrid Fallback in `ClipboardService`**:
    - [x] In `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service_impl.rs`:
        - Implement `Osc52Clipboard`:

            ```rust
            #[derive(Debug, Default)]
            pub struct Osc52Clipboard;

            impl ClipboardService for Osc52Clipboard {
                fn try_to_put_content_into_clipboard(&mut self, content: String) -> ClipboardResult<()>;
                fn try_to_get_content_from_clipboard(&mut self) -> ClipboardResult<String>;
            }
            ```

            - `try_to_put_content_into_clipboard`: Formats `OscSequence::ClipboardSet` and
              writes to stdout using `OutputDevice` / `std::io::stdout()`.
            - `try_to_get_content_from_clipboard`: Returns an informative error noting
              that inbound clipboard data arrives asynchronously via terminal bracketed
              paste.

        - Update `SystemClipboard`:
            - Implement automatic hybrid fallback: first attempt `copypasta`. If
              `copypasta` fails (e.g. headless / SSH without display server),
              automatically fall back to
              `Osc52Clipboard::try_to_put_content_into_clipboard()`.

- [x] **Phase 10.4: Comprehensive Documentation Across 4 Locations**:
    - [x] **Location 1**: `tui/src/tui/editor/editor_buffer/clipboard/mod.rs` &
          `clipboard_service.rs`:
        - Document the architectural overview of the hybrid clipboard model (local display
          server via `copypasta` with fallback to in-band terminal emulator
          `Osc52Clipboard`).
    - [x] **Location 2**: `tui/src/core/osc/mod.rs` & `osc_codes.rs`:
        - Document wire protocol for OSC 52, `OscSequence::ClipboardSet`, base64 encoding,
          and target selection (`c` for clipboard vs `p` for primary selection).
    - [x] **Location 3**:
          `tui/src/core/ansi/vt_100_terminal_input_parser/terminal_events.rs`:
        - Document the input parser's role: quarantine and absorption
          (`VT100InputEventIR::Ignored`) to prevent raw sequence leaks into user input.
        - Document why terminal applications do NOT rely on OSC 52 queries (`?`):
            - Standard terminals (Kitty, Alacritty, Foot) disable OSC 52 queries by
              default due to clipboard snooping security risks.
            - User paste is driven by Bracketed Paste (`CSI 200 ~`), while CLI/TUI tools
              use system helpers (`xclip`, `wl-copy`, `pbcopy`) or internal registers.
            - Programs probing OSC 52 use non-blocking timeouts and fall back cleanly
              without hanging.
        - Document the structured diagnostic warning gated by
          `DEBUG_TUI_SHOW_DIRECT_TO_ANSI` when complete OSC sequences are absorbed.
    - [x] **Location 4**: `tui/src/lib.rs` & `tui/README.md`:
        - Document OSC 52 clipboard copy support in the terminal capability matrix and
          feature guide. Run `cd tui && cargo readme > README.md`.

- [x] **Phase 10.5: Unit & Integration Tests & Tracing Warning**:
    - [x] In `tui/src/core/osc/osc_codes.rs`:
        - Test `OscSequence::ClipboardSet` formatting with empty string, ASCII, UTF-8
          text, and checkmark `✓`.
    - [x] In `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service_impl.rs`:
        - Test `Osc52Clipboard` putting content into clipboard.
        - Test `SystemClipboard` hybrid fallback behavior when display server is absent.
    - [x] In `tui/src/core/ansi/vt_100_terminal_input_parser/terminal_events.rs`:
        - Add `DEBUG_TUI_SHOW_DIRECT_TO_ANSI.then(|| { tracing::warn!(...); })` on
          `OscScanResult::Complete(consumed)`.
        - Verify `try_disambiguate_osc_or_alt_bracket()` safely absorbs OSC 52 sequences with
          BEL and ST terminators, as well as UTF-8 continuation byte `0x9C`.

- [x] **Phase 10.6: Verification**:
    - [x] Run `./check.fish --check`.
    - [x] Run `./check.fish --clippy`.
    - [x] Run `./check.fish --test`.
    - [x] Run `./check.fish --fmt`.
    - [x] Run `./check.fish --quick-doc`.

### [x] Step 11: Implement Runaway OSC Payload Draining State in StatefulInputParser

- **Problem Analysis & Architectural Motivation**:
    - **Framing Desynchronization & Text Leakage**: Currently, when an OSC sequence
      exceeds `MAX_OSC_SEQUENCE_LENGTH` (1 MiB), `StatefulInputParser::advance()` purges
      the accumulator via `self.accumulator.clear()`. Because the stream framing context
      (`ESC ]`) is lost, all remaining trailing Base64 payload bytes in the stream leak as
      printable keystrokes (`parse_utf8_text()`), corrupting application state with
      thousands of spurious `InputEvent::Keyboard` events.
    - **Circuit-Breaker Streaming Drain**: Removing the 1 MiB limit is dangerous because
      it risks unbounded memory allocation (OOM) and permanent input lockup. The proper
      architectural solution is an explicit streaming drain mode: once the 1 MiB threshold
      is crossed, the parser reclaims the accumulated 1 MiB and trips
      `OscCircuitBreaker::Open`, swallowing and discarding all incoming
      bytes on-the-fly without heap reallocations until a terminator (`BEL` or `ST`) or
      abort character is encountered.

- [x] **Phase 11.1: Define `OscCircuitBreaker` & Constants**:
    - In `tui/src/core/ansi/constants/input_sequences.rs`:
        - Define `pub const MAX_OSC_DRAIN_BYTES: usize = 16_777_216; // 16 MiB`: Safety
          upper bound for total bytes discarded during an active drain, preventing an
          infinite drain loop if input stream corruption never terminates.
        - Re-export `MAX_OSC_DRAIN_BYTES` in `tui/src/lib.rs` and
          `tui/src/core/ansi/constants/mod.rs`.
    - In `tui/src/core/ansi/vt_100_terminal_input_parser/input_byte_stream_to_ir/osc_circuit_breaker.rs`:
        - Define `pub enum OscCircuitBreaker`:
            ```rust
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub enum OscCircuitBreaker {
                #[default]
                Closed,
                /// Discarding bytes of an in-flight runaway OSC sequence until a terminator
                /// (`BEL` 0x07 or 7-bit `ST` `\x1b\\`) or abort condition is encountered.
                Open {
                    /// If the previous chunk ended in a lone `ESC` (0x1B), this tracks whether
                    /// the next byte completes a 7-bit `ST` (`\x1b\\`).
                    saw_partial_esc: bool,
                    /// Total bytes drained so far across chunks (bounded by `MAX_OSC_DRAIN_BYTES`).
                    drained_bytes: usize,
                },
            }
            ```
        - Add `osc_circuit_breaker: OscCircuitBreaker` field to `InputByteStreamToIrParser`
          (initialized to `Default::default()`).

- [x] **Phase 11.2: Implement Streaming Drain in `StatefulInputParser::advance()`**:
    - Implement helper method `drain_osc_payload(&mut self, chunk: &[u8]) -> usize`:
        - **Partial `ESC` resolution across chunk boundaries**: If `saw_partial_esc` was
          true from previous chunk:
            - If `chunk[0] == b'\\'`: completes 7-bit `ST`. Drain terminates cleanly;
              consumes 1 byte; resets `drain_state` to `Normal`.
            - If `chunk[0] != b'\\'`: previous `ESC` was not `ST`. Aborts the OSC control
              string; resets `drain_state` to `Normal`; returns 0 (leaves `chunk` for
              normal parsing).
        - **Byte scanner loop through `chunk`**:
            - `ANSI_BEL` (`7`): consumes up to and including `BEL`; resets `drain_state`
              to `Normal`; returns consumed count.
            - `ANSI_ESC` (`0x1B`):
                - If next byte is `b'\\'`: consumes up to and including `\\` (`ST`);
                  resets `drain_state` to `Normal`; returns consumed count.
                - If lone `ESC` at the very end of chunk: sets `saw_partial_esc = true`;
                  returns chunk length.
                - If next byte is NOT `\\`: new escape sequence aborts the OSC control
                  string; resets `drain_state` to `Normal`; returns index up to `ESC`
                  (leaves `ESC` for normal parsing).
            - Raw `\r` or `\n`: violates OSC syntax; aborts drain; resets `drain_state` to
              `Normal`; returns index up to newline (leaves newline for normal parsing).
            - `drained_bytes >= MAX_OSC_DRAIN_BYTES`: safety ceiling triggered; resets
              `drain_state` to `Normal`; returns consumed count.
        - If entire chunk consumed without terminator: updates cumulative `drained_bytes`
          and returns `chunk.len()`.
    - Update `advance(&mut self, read_buffer: &[u8], maybe_more: MaybeMore)`:
        - If `self.drain_state` is active:
            - Calls `self.drain_osc_payload()`.
            - If leftover bytes remain in `read_buffer` after drain completion, feeds
              leftover bytes into normal accumulation.
            - If drain is still active, returns immediately (0 allocations, 0 events
              emitted).
        - When `should_discard_unrecognized_sequence()` detects a runaway OSC:
            - Sets
              `self.osc_circuit_breaker.trip(self.accumulator.len())`.
            - Clears `self.accumulator` (immediately reclaims the 1 MiB).
            - Emits structured `tracing::warn!` diagnostic gated by
              `DEBUG_TUI_SHOW_DIRECT_TO_ANSI`.

- [x] **Phase 11.3: Structured Tracing & Rustdoc Updates**:
    - Update rustdoc comments in `core.rs` detailing the two-stage defense:
        1. Bounded accumulation (1 MiB `MAX_OSC_SEQUENCE_LENGTH`).
        2. Zero-allocation streaming drain (`OscCircuitBreaker::Open`).
    - Add structured tracing logs for drain entry and drain completion under
      `DEBUG_TUI_SHOW_DIRECT_TO_ANSI`.

- [x] **Phase 11.4: Unit Tests in `stateful_parser.rs`**:
    - Test runaway OSC terminated by `BEL` across separate chunks: verify 0 `InputEvent`
      emitted during drain, normal typing resumes after `BEL`.
    - Test runaway OSC terminated by `ST` (`\x1b\\`) split across chunk boundary (`ESC` at
      end of chunk 1, `\` at start of chunk 2): verify clean recovery and trailing text
      parsing.
    - Test runaway OSC aborted by embedded newline `\n`: verify drain aborts and
      subsequent keystrokes parse normally.
    - Test runaway OSC exceeding `MAX_OSC_DRAIN_BYTES`: verify safety ceiling terminates
      drain without panic.
    - Update `runaway_unterminated_osc_purged_and_recovers` to terminate the runaway
      sequence before typing `'z'`, verifying complete end-to-end recovery.
    - Verify all existing standard OSC tests (< 1 MiB) pass without regression.

- [x] **Phase 11.5: Reorganize & Relocate Streaming Parser to
      `core/ansi/vt_100_terminal_input_parser/input_byte_stream_to_ir/`**:
    - Create directory
      `tui/src/core/ansi/vt_100_terminal_input_parser/input_byte_stream_to_ir/`.
    - Move `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/runaway_osc_drain.rs`
      to
      `tui/src/core/ansi/vt_100_terminal_input_parser/input_byte_stream_to_ir/osc_circuit_breaker.rs`.
    - Move `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/stateful_parser.rs` to
      `tui/src/core/ansi/vt_100_terminal_input_parser/input_byte_stream_to_ir/core.rs`,
      renaming `StatefulInputParser` to `InputByteStreamToIrParser` and providing a
      compatibility type alias
      `pub type StatefulInputParser = InputByteStreamToIrParser;`.
    - Create
      `tui/src/core/ansi/vt_100_terminal_input_parser/input_byte_stream_to_ir/mod.rs` with
      barrel exports.
    - Update `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs` to declare and
      re-export `input_byte_stream_to_ir`.
    - Update `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mod.rs` to remove
      standalone `stateful_parser` and `runaway_osc_drain` declarations, and re-export
      from `crate::core::ansi::vt_100_terminal_input_parser`.
    - Update imports and intra-doc links in `mio_poller` (`handler_stdin.rs`,
      `mio_poll_worker.rs`, etc.).

- [x] **Phase 11.6: Verification**:
    - Run `./check.fish --check`.
    - Run `./check.fish --clippy`.
    - Run `./check.fish --test`.
    - Run `./check.fish --fmt`.
    - Run `./check.fish --quick-doc`.

- [ ] **Phase 11.7: Mandatory Manual Review**:
    - [x] `tui/src/core/ansi/generator/ansi_output.rs`
    - [x] `tui/src/core/ansi/constants/input_sequences.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/maybe_more.rs`
    - [x] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service_impl.rs`
    - [x] `tui/src/tui/editor/editor_buffer/clipboard/clipboard_service.rs`
    - [x] `tui/src/tui/editor/editor_buffer/clipboard/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/input_byte_stream_to_ir/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/input_byte_stream_to_ir/core.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/input_byte_stream_to_ir/osc_circuit_breaker.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/terminal_events.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/keyboard.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/router.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
    - [ ] `tui/src/core/ansi/generator/ansi_input.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/protocol_conversion.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/paste_state_machine.rs`
    - [ ] `tui/src/core/osc/osc_codes.rs`
    - [ ] `tui/src/core/terminal_io/terminal_mode_controller.rs`
    - [ ] `tui/src/core/terminal_io/output_device.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_paint_render_op_impl.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/readline_struct.rs`
    - [ ] `tui/src/readline_async/readline_async_impl/lock_manager.rs`
    - [ ] `tui/src/readline_async/spinner_impl/spinner_print.rs`
    - [ ] `tui/src/readline_async/choose_impl/event_loop.rs`
    - [ ] `tui/src/readline_async/choose_impl/function_component.rs`
    - [ ] `tui/src/readline_async/choose_impl/select_component.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_api.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_api_impl.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/constants.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/mod.rs`
    - [ ] `tui/src/core/terminal_io/mod.rs`
    - [ ] `tui/src/core/mod.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/channel_types.rs`
    - [ ] `tui/src/lib.rs`
    - [ ] `tui/Cargo.toml`
    - [ ] `tui/README.md`
    - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/unit_tests/generator_round_trip_tests.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/backend_compat_input_test.rs`
    - [ ] `tui/src/core/terminal_io/backend_compat_tests/pty_terminal_mode_test.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/tests.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_pty_is_interactive.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_disclaimer.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_piped_stdin.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/test_piped_stdout.rs`
    - [ ] `tui/src/core/terminal_io/capabilities/capabilities_integration_tests/mod.rs`
    - [ ] `tui/src/core/osc/mod.rs`
    - [ ] `tui/src/core/ansi/vt_100_pty_output_parser/modes.rs`

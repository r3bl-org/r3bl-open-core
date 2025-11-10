// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Keyboard input event parsing from ANSI/CSI sequences.
//!
//! This module handles conversion of raw ANSI escape sequences into keyboard events.
//! It provides comprehensive support for VT-100 compatible terminal input while
//! maintaining clarity about protocol limitations and design decisions.
//!
//! ## Parser Dispatch Priority Pipeline
//!
//! This module provides multiple parser functions that are invoked in a **predefined
//! priority order** by the [`try_parse`] backend input handler.
//!
//! ## Comprehensive List of Supported Keyboard Shortcuts
//!
//! ### Basic Keys
//! | Key             | Sequence        | Notes                            |
//! | --------------- | --------------- | -------------------------------- |
//! | **Tab**         | `0x09`          | Fixed: was returning None        |
//! | **Enter**       | `0x0D`/`0x0A`   | CR or LF depending on terminal   |
//! | **Backspace**   | `0x08`/`0x7F`   | BS or DEL encoding               |
//! | **Escape**      | `0x1B`          | Modal UI support                 |
//! | **Space**       | `0x20`          | Regular space character          |
//!
//! ### Control Key Combinations (Ctrl+Letter)
//! | Key                             | Byte            | Notes                            |
//! | ------------------------------- | --------------- | -------------------------------- |
//! | **Ctrl+Space**                  | `0x00`          | Ctrl+@, treated as Ctrl+Space    |
//! | **Ctrl+A** through **Ctrl+Z**   | `0x01`-`0x1A`   | Standard control chars           |
//! | **Ctrl+\\**                     | `0x1C`          | FS (File Separator)              |
//! | **Ctrl+]**                      | `0x1D`          | GS (Group Separator)             |
//! | **Ctrl+^**                      | `0x1E`          | RS (Record Separator)            |
//! | **Ctrl+_**                      | `0x1F`          | US (Unit Separator)              |
//!
//! ### Alt Key Combinations (Alt+Letter)
//! | Key                         | Sequence          | Format                |
//! | --------------------------- | ----------------- | --------------------- |
//! | **Alt+\[a-z\]**             | `ESC` + letter    | Lowercase letters     |
//! | **Alt+\[A-Z\]**             | `ESC` + letter    | Uppercase letters     |
//! | **Alt+\[0-9\]**             | `ESC` + digit     | Digits                |
//! | **Alt+Space**               | `ESC` + space     | Space key             |
//! | **Alt+Backspace**           | `ESC` + `0x7F`    | Delete word           |
//! | **Alt+\[punctuation\]**     | `ESC` + char      | Any printable ASCII   |
//!
//! ### Arrow Keys
//! | Key         | CSI Sequence   | SS3 Sequence   | Application Mode   |
//! | ----------- | -------------- | -------------- | ------------------ |
//! | **Up**      | `ESC[A`        | `ESC O A`      | vim/less/emacs     |
//! | **Down**    | `ESC[B`        | `ESC O B`      | vim/less/emacs     |
//! | **Right**   | `ESC[C`        | `ESC O C`      | vim/less/emacs     |
//! | **Left**    | `ESC[D`        | `ESC O D`      | vim/less/emacs     |
//!
//! ### Arrow Keys with Modifiers
//! | Key                            | Sequence            | Format               |
//! | ------------------------------ | ------------------- | -------------------- |
//! | **Ctrl+Up/Down/Left/Right**    | `ESC[1;5A/B/D/C`    | CSI with modifier    |
//! | **Alt+Up/Down/Left/Right**     | `ESC[1;3A/B/D/C`    | CSI with modifier    |
//! | **Shift+Up/Down/Left/Right**   | `ESC[1;2A/B/D/C`    | CSI with modifier    |
//! | **Ctrl+Alt+arrows**            | `ESC[1;7A/B/D/C`    | Combined modifiers   |
//!
//! ### Special Navigation Keys
//! | Key             | Primary     | Alt 1       | Alt 2      | SS3         |
//! | --------------- | ----------- | ----------- | ---------- | ----------- |
//! | **Home**        | `ESC[H`     | `ESC[1~`    | `ESC[7~`   | `ESC O H`   |
//! | **End**         | `ESC[F`     | `ESC[4~`    | `ESC[8~`   | `ESC O F`   |
//! | **Insert**      | `ESC[2~`    | -           | -          | -           |
//! | **Delete**      | `ESC[3~`    | -           | -          | -           |
//! | **Page Up**     | `ESC[5~`    | -           | -          | -           |
//! | **Page Down**   | `ESC[6~`    | -           | -          | -           |
//!
//! ### Tab Navigation
//! | Key                          | Sequence   | Notes                 |
//! | ---------------------------- | ---------- | --------------------- |
//! | **Tab**                      | `0x09`     | Forward navigation    |
//! | **Shift+Tab (`BackTab`)**    | `ESC[Z`    | Backward navigation   |
//!
//! ### Function Keys F1-F12
//! | Key       | CSI Code     | SS3 Sequence   | Notes             |
//! | --------- | ------------ | -------------- | ----------------- |
//! | **F1**    | `ESC[11~`    | `ESC O P`      | SS3 in app mode   |
//! | **F2**    | `ESC[12~`    | `ESC O Q`      | SS3 in app mode   |
//! | **F3**    | `ESC[13~`    | `ESC O R`      | SS3 in app mode   |
//! | **F4**    | `ESC[14~`    | `ESC O S`      | SS3 in app mode   |
//! | **F5**    | `ESC[15~`    | -              | CSI only          |
//! | **F6**    | `ESC[17~`    | -              | Note: gap at 16   |
//! | **F7**    | `ESC[18~`    | -              | CSI only          |
//! | **F8**    | `ESC[19~`    | -              | CSI only          |
//! | **F9**    | `ESC[20~`    | -              | CSI only          |
//! | **F10**   | `ESC[21~`    | -              | CSI only          |
//! | **F11**   | `ESC[23~`    | -              | Note: gap at 22   |
//! | **F12**   | `ESC[24~`    | -              | CSI only          |
//!
//! ### Function Keys with Modifiers
//! Function keys support all modifier combinations using CSI format:
//! - **Shift+F5**: `ESC[15;2~` (modifier = 2)
//! - **Alt+F5**: `ESC[15;3~` (modifier = 3)
//! - **Ctrl+F5**: `ESC[15;5~` (modifier = 5)
//! - **Ctrl+Alt+F10**: `ESC[21;7~` (modifier = 7)
//!
//! ## Intentionally Unsupported Features
//!
//! ### Extended Function Keys (F13-F24)
//! **Decision**: F13-F24 are intentionally NOT supported.
//!
//! **Rationale**:
//! - Rarely available on modern keyboards
//! - No standardized escape sequences across terminals
//! - Different terminals use different codes (xterm vs linux console vs rxvt)
//! - Minimal real-world usage in applications
//! - Would add complexity without practical benefit
//!
//! ### Numpad Application Mode
//! **Status**: ✅ Fully implemented.
//!
//! **What it is**: In application mode (DECPAM), numpad keys send SS3 sequences instead
//! of their literal digits. This allows applications to distinguish numpad from regular
//! number keys.
//!
//! **Numpad Key Mappings**:
//! | Numpad Key   | Normal Mode   | Application Mode   | SS3 Char   |
//! | ------------ | ------------- | ------------------ | ---------- |
//! | **0**        | `'0'`         | `ESC O p`          | p          |
//! | **1**        | `'1'`         | `ESC O q`          | q          |
//! | **2**        | `'2'`         | `ESC O r`          | r          |
//! | **3**        | `'3'`         | `ESC O s`          | s          |
//! | **4**        | `'4'`         | `ESC O t`          | t          |
//! | **5**        | `'5'`         | `ESC O u`          | u          |
//! | **6**        | `'6'`         | `ESC O v`          | v          |
//! | **7**        | `'7'`         | `ESC O w`          | w          |
//! | **8**        | `'8'`         | `ESC O x`          | x          |
//! | **9**        | `'9'`         | `ESC O y`          | y          |
//! | **Enter**    | `CR`          | `ESC O M`          | M          |
//! | **+**        | `'+'`         | `ESC O k`          | k          |
//! | **-**        | `'-'`         | `ESC O m`          | m          |
//! | **\***       | `'*'`         | `ESC O j`          | j          |
//! | **/**        | `'/'`         | `ESC O o`          | o          |
//! | **.**        | `'.'`         | `ESC O n`          | n          |
//! | **,**        | `','`         | `ESC O l`          | l          |
//!
//! **Use cases**:
//! - Calculator applications (distinguish numpad for calculations)
//! - Games (numpad for movement, regular numbers for item selection)
//! - Vim (numpad for navigation, regular numbers for counts)
//!
//! ## Why Alt Uses ESC Prefix (Not CSI)
//!
//! You might wonder: why does Alt+B send `ESC b` (2 bytes) instead of a CSI sequence
//! like `ESC[1;3b`? This design goes back to the 1970s and remains standard today.
//!
//! ### The Three-Tier Encoding Hierarchy
//!
//! Terminal input uses the **simplest encoding that works**:
//!
//! ```text
//! 1. Single byte (0x00-0x7F)
//!    ├─ Printable: 'a', 'B', '3', etc
//!    └─ Control codes: Ctrl+A (0x01), Ctrl+D (0x04)
//!
//! 2. ESC prefix (2 bytes)
//!    └─ Alt+letter: ESC+'a', ESC+'b'  ← Simple & efficient!
//!
//! 3. CSI sequences (6+ bytes)
//!    └─ Complex modifiers: ESC[1;5A (Ctrl+Up)
//! ```
//!
//! ### Why Each Modifier Uses Different Encoding
//!
//! | Modifier     | Encoding                 | Reason                           |
//! | ------------ | ------------------------ | -------------------------------- |
//! | **Ctrl**     | Single byte (0x00-0x1F)  | Fits in ASCII control codes      |
//! | **Alt**      | ESC prefix (2 bytes)     | No room in ASCII, prepend ESC    |
//! | **Shift**    | Implicit in case         | 'a' vs 'A' already encodes it    |
//! | **Combos**   | CSI parameters           | Need bitmask encoding            |
//!
//! **Why ESC prefix for Alt?**
//! - **Historical**: Used since VT52 (1975), proven for 50+ years
//! - **Efficient**: 2 bytes vs 6+ for CSI
//! - **Simple**: Just prepend ESC, no parameter encoding needed
//! - **Universal**: Works on every terminal emulator ever made
//!
//! ### Real-World Examples
//!
//! What terminals actually send (confirmed via `cat -v`):
//!
//! ```text
//! Key Press       Sequence       Bytes  Format
//! ─────────────────────────────────────────────────
//! Alt+B          ESC b          2      ESC prefix ✓
//! Alt+F          ESC f          2      ESC prefix ✓
//! Alt+Shift+B    ESC B          2      ESC + uppercase ✓
//! Ctrl+Alt+Up    ESC[1;7A       6      CSI (complex)
//! ```
//!
//! ### Historical Timeline
//!
//! ```text
//! 1975: VT52  → Introduced ESC + letter commands
//! 1978: VT100 → Added CSI, kept ESC+letter for compatibility
//! 1983: VT220 → Extended CSI, still kept ESC+letter
//! 2025: Modern → Still using ESC+letter for Alt!
//! ```
//!
//! **Why this design survived 50 years:**
//! - ✅ Works everywhere (bash, vim, emacs, tmux, etc.)
//! - ✅ Simpler to parse than CSI
//! - ✅ More efficient (fewer bytes)
//! - ✅ Unambiguous (ESC always means "next char is modified")
//!
//! ## CSI vs ESC Prefix: When to Use Each
//!
//! **ESC prefix** (this module's `parse_alt_letter()`):
//! - ✅ Alt+printable-character (Alt+B, Alt+F, Alt+3, Alt+.)
//! - Simple 2-byte sequences: `ESC char`
//!
//! **CSI sequences** (this module's `parse_keyboard_sequence()`):
//! - ✅ Special keys with modifiers (Ctrl+Up, Shift+F5)
//! - ✅ Complex modifier combinations (Ctrl+Alt+Up)
//! - Parametric sequences: `ESC [ params finalchar`
//!
//! This dual approach gives us the best of both worlds: efficiency for simple
//! cases (Alt+letter) and expressiveness for complex cases (Ctrl+Alt+Shift+Up).
//!
//! ### CSI Sequences (ESC[...)
//!
//! When buffer starts with `ESC[`:
//! 1. **`parse_keyboard_sequence()`** - Arrow keys, function keys, modified keys with CSI
//!    format
//!    - Examples: `ESC[A` (Up), `ESC[1;5A` (Ctrl+Up), `ESC[15~` (F5)
//! 2. **`parse_mouse_sequence()`** - SGR mouse protocol for clicks, drags, scrolling
//!    - Examples: `ESC[<0;10;20M` (left click), `ESC[<64;10;20M` (scroll up)
//! 3. **`parse_terminal_event()`** - Window resize, focus gained/lost, paste markers
//!    - Examples: `ESC[8;24;80t` (resize to 24x80), `ESC[I` (focus gained)
//!
//! ### SS3 Sequences (ESC O...)
//!
//! When buffer starts with `ESC O`:
//! - **`parse_ss3_sequence()`** - Application mode keys (F1-F4, Home, End, arrows)
//!   - Examples: `ESOP` (F1), `ESOA` (Up in app mode)
//!
//! ### ESC + Unknown Byte
//!
//! When buffer starts with `ESC +` (something other than `[` or `O`):
//! - **`parse_alt_letter()`** - Alt+printable character combinations
//!   - Examples: `ESCb` (Alt+B), `ESC3` (Alt+3), `ESC ` (Alt+Space)
//!
//! ### Non-ESC Sequences (Regular Input)
//!
//! When first byte is not ESC:
//! 1. **`parse_terminal_event()`** - (Re-attempted for non-ESC input)
//! 2. **`parse_mouse_sequence()`** - X10/RXVT mouse protocols (legacy)
//! 3. **`parse_control_character()`** - Ctrl+A through Ctrl+Z (0x00-0x1F)
//!    - Examples: `0x01` (Ctrl+A), `0x04` (Ctrl+D), `0x17` (Ctrl+W)
//!    - **Must be tried before UTF-8** because control bytes are valid UTF-8
//! 4. **`parse_utf8_text()`** - Regular text input and printable characters
//!    - Examples: `a`, `ñ`, `日`, multi-byte UTF-8 sequences
//!
//! **Critical**: Control characters must be parsed before UTF-8 because bytes 0x00-0x1F
//! are technically valid UTF-8 but represent Ctrl+letter combinations. Without this
//! priority, Ctrl+A would be misinterpreted as incomplete UTF-8.
//!
//! ## Ambiguous Control Character Handling
//!
//! **Design Decision**: Some control characters are ambiguous at the protocol level
//! because terminals send identical byte sequences for different key combinations. This
//! parser **prioritizes the common key** over the Ctrl+letter combination.
//!
//! ### Ambiguous Mappings (Identical Bytes)
//!
//! | Bytes     | Key Combination            | Parser Interpretation   | Rationale                           |
//! | --------- | -------------------------- | ----------------------- | ----------------------------------- |
//! | `0x09`    | Tab **OR** Ctrl+I          | **Tab**                 | Tab key is far more commonly used   |
//! | `0x0A`    | Enter (LF) **OR** Ctrl+J   | **Enter**               | Enter key is essential for apps     |
//! | `0x0D`    | Enter (CR) **OR** Ctrl+M   | **Enter**               | Enter key is essential for apps     |
//! | `0x08`    | Backspace **OR** Ctrl+H    | **Backspace**           | Backspace is critical for editing   |
//! | `0x1B`    | ESC **OR** Ctrl+[          | **ESC**                 | Standard for vi-mode, modals        |
//!
//! ### Why This Matters
//!
//! **Problem**: In VT-100 terminals, Ctrl modifies keys by masking with `0x1F`:
//! - `Ctrl+I` = `'I'` (0x49) & 0x1F = 0x09 (same as Tab)
//! - `Ctrl+M` = `'M'` (0x4D) & 0x1F = 0x0D (same as Enter/CR)
//! - `Ctrl+H` = `'H'` (0x48) & 0x1F = 0x08 (same as Backspace)
//!
//! **Solution**: Prioritize the dedicated key's interpretation. Applications that need
//! Ctrl+I/Ctrl+M/Ctrl+H can use alternative key bindings (e.g., Ctrl+Space for custom
//! actions).
//!
//! ### Unambiguous Cases (Different Sequences)
//!
//! These DO work correctly because terminals send distinct sequences:
//! - **Shift+Tab**: Sends `ESC[Z` (parsed as `BackTab`)
//! - **Ctrl+Arrow**: Sends `ESC[1;5A/B/C/D` (parsed with Ctrl modifier)
//! - **Alt+Letter**: Sends `ESC + letter` (parsed with Alt modifier)
//! - **Function Keys**: Send `ESC[n~` or `ESC O P/Q/R/S`
//!
//! This is a fundamental VT-100 protocol limitation, not a parser bug. Modern protocols
//! like Kitty keyboard protocol solve this, but we maintain VT-100 compatibility.
//!
//! [`try_parse`]: crate::tui::terminal_lib_backends::direct_to_ansi::DirectToAnsiInputDevice::try_parse

use super::types::{VT100InputEvent, VT100KeyCode, VT100KeyModifiers};
use crate::{ASCII_DEL, KeyState,
            core::ansi::constants::{ANSI_CSI_BRACKET, ANSI_ESC,
                                    ANSI_FUNCTION_KEY_TERMINATOR, ANSI_PARAM_SEPARATOR,
                                    ANSI_SS3_O, ARROW_DOWN_FINAL, ARROW_LEFT_FINAL,
                                    ARROW_RIGHT_FINAL, ARROW_UP_FINAL, ASCII_DIGIT_0,
                                    ASCII_DIGIT_9, ASCII_LOWER_A, ASCII_LOWER_Z,
                                    ASCII_UPPER_A, ASCII_UPPER_Z, BACKTAB_FINAL,
                                    CONTROL_BACKSPACE, CONTROL_ENTER, CONTROL_ESC,
                                    CONTROL_LF, CONTROL_NUL, CONTROL_TAB,
                                    CTRL_CHAR_RANGE_MAX, CTRL_TO_LOWERCASE_MASK,
                                    FUNCTION_F1_CODE, FUNCTION_F2_CODE,
                                    FUNCTION_F3_CODE, FUNCTION_F4_CODE,
                                    FUNCTION_F5_CODE, FUNCTION_F6_CODE,
                                    FUNCTION_F7_CODE, FUNCTION_F8_CODE,
                                    FUNCTION_F9_CODE, FUNCTION_F10_CODE,
                                    FUNCTION_F11_CODE, FUNCTION_F12_CODE, MODIFIER_ALT,
                                    MODIFIER_CTRL, MODIFIER_NONE,
                                    MODIFIER_PARAMETER_OFFSET, MODIFIER_SHIFT,
                                    PRINTABLE_ASCII_MAX, PRINTABLE_ASCII_MIN,
                                    SPECIAL_DELETE_CODE, SPECIAL_END_ALT1_CODE,
                                    SPECIAL_END_ALT2_CODE, SPECIAL_END_FINAL,
                                    SPECIAL_HOME_ALT1_CODE, SPECIAL_HOME_ALT2_CODE,
                                    SPECIAL_HOME_FINAL, SPECIAL_INSERT_CODE,
                                    SPECIAL_PAGE_DOWN_CODE, SPECIAL_PAGE_UP_CODE,
                                    SS3_F1_FINAL, SS3_F2_FINAL, SS3_F3_FINAL,
                                    SS3_F4_FINAL, SS3_NUMPAD_0, SS3_NUMPAD_1,
                                    SS3_NUMPAD_2, SS3_NUMPAD_3, SS3_NUMPAD_4,
                                    SS3_NUMPAD_5, SS3_NUMPAD_6, SS3_NUMPAD_7,
                                    SS3_NUMPAD_8, SS3_NUMPAD_9, SS3_NUMPAD_COMMA,
                                    SS3_NUMPAD_DECIMAL, SS3_NUMPAD_DIVIDE,
                                    SS3_NUMPAD_ENTER, SS3_NUMPAD_MINUS,
                                    SS3_NUMPAD_MULTIPLY, SS3_NUMPAD_PLUS}};

/// Parse a control character (bytes 0x00-0x1F) and convert to Ctrl+key event.
///
/// **Dispatch position**: 3rd parser in non-ESC priority. Must be tried before UTF-8 text
/// because control bytes are valid UTF-8 but represent Ctrl+letter combinations.
///
/// See module docs [`Parser Dispatch Priority Pipeline`] for dispatch order and
/// [`Control Key Combinations`] for complete byte mappings. Note: some bytes are treated
/// as dedicated keys (Tab, Enter, Backspace, Escape) - see
/// [`Ambiguous Control Character Handling`] for details.
///
/// ## Returns
///
/// `Some((event, 1))` if successful, `None` otherwise.
///
/// [`Parser Dispatch Priority Pipeline`]
/// [`Control Key Combinations`]
/// [`Ambiguous Control Character Handling`]
///
/// [`Parser Dispatch Priority Pipeline`]: mod@self#parser-dispatch-priority-pipeline
/// [`Control Key Combinations`]: mod@self#control-key-combinations-ctrlletter
/// [`Ambiguous Control Character Handling`]: mod@self#ambiguous-control-character-handling
#[must_use]
pub fn parse_control_character(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Check minimum length
    if buffer.is_empty() {
        return None;
    }

    let byte = buffer[0];

    // Handle ASCII DEL (0x7F) - common Backspace encoding
    if byte == ASCII_DEL {
        return Some((
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Backspace,
                modifiers: VT100KeyModifiers::default(),
            },
            1,
        ));
    }

    // Only handle control character range (0x00-0x1F)
    if byte > CTRL_CHAR_RANGE_MAX {
        return None;
    }

    // Handle special control characters as dedicated keys (not Ctrl+letter)
    match byte {
        CONTROL_NUL => {
            // Ctrl+Space (or Ctrl+@) generates NUL
            // Treat as Ctrl+Space for better usability
            return Some((
                VT100InputEvent::Keyboard {
                    code: VT100KeyCode::Char(' '),
                    modifiers: VT100KeyModifiers {
                        shift: KeyState::NotPressed,
                        ctrl: KeyState::Pressed,
                        alt: KeyState::NotPressed,
                    },
                },
                1,
            ));
        }
        CONTROL_TAB => {
            // Tab key (0x09) - treated as Tab, not Ctrl+I
            return Some((
                VT100InputEvent::Keyboard {
                    code: VT100KeyCode::Tab,
                    modifiers: VT100KeyModifiers::default(),
                },
                1,
            ));
        }
        CONTROL_LF | CONTROL_ENTER => {
            // Enter key sends CR (0x0D) or LF (0x0A) depending on terminal
            return Some((
                VT100InputEvent::Keyboard {
                    code: VT100KeyCode::Enter,
                    modifiers: VT100KeyModifiers::default(),
                },
                1,
            ));
        }
        CONTROL_BACKSPACE => {
            // Backspace can send BS (0x08) or DEL (0x7F)
            return Some((
                VT100InputEvent::Keyboard {
                    code: VT100KeyCode::Backspace,
                    modifiers: VT100KeyModifiers::default(),
                },
                1,
            ));
        }
        CONTROL_ESC => return None, // Escape - handled in try_parse() routing
        _ => {}
    }

    // Convert control character to Ctrl+letter
    // Control characters are generated as: letter & 0x1F
    // Reverse: (byte | 0x40) gives uppercase letter, (byte | 0x60) gives lowercase
    // Example: 0x01 | 0x60 = 0x61 = 'a'
    let letter = char::from(byte | CTRL_TO_LOWERCASE_MASK);

    Some((
        VT100InputEvent::Keyboard {
            code: VT100KeyCode::Char(letter),
            modifiers: VT100KeyModifiers {
                shift: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
                alt: KeyState::NotPressed,
            },
        },
        1,
    ))
}

/// Parse Alt+key combination (ESC followed by printable ASCII or DEL).
///
/// **Dispatch position**: Only parser for ESC + unknown byte. See module docs
/// [`Parser Dispatch Priority Pipeline`] for dispatch order.
///
/// Terminals send Alt+key as ESC (0x1B) + key byte. This parses two-byte sequences like
/// Alt+B → (0x1B, 0x62) or Alt+Backspace → (0x1B, 0x7F).
///
/// For design rationale on why Alt uses ESC prefix vs CSI sequences, see module docs
/// [`Why Alt Uses ESC Prefix`].
///
/// ## Returns
///
/// `Some((event, 2))` if buffer starts with ESC + (printable ASCII or DEL),
/// `None` otherwise.
///
/// [`Parser Dispatch Priority Pipeline`]
/// [`Why Alt Uses ESC Prefix`]
///
/// [`Parser Dispatch Priority Pipeline`]: mod@self#parser-dispatch-priority-pipeline
/// [`Why Alt Uses ESC Prefix`]: mod@self#why-alt-uses-esc-prefix-not-csi
#[must_use]
pub fn parse_alt_letter(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Need at least 2 bytes: ESC + key
    if buffer.len() < 2 {
        return None;
    }

    // First byte must be ESC
    if buffer[0] != ANSI_ESC {
        return None;
    }

    let second = buffer[1];

    // Handle Alt+Backspace (ESC + DEL)
    if second == ASCII_DEL {
        return Some((
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Backspace,
                modifiers: VT100KeyModifiers {
                    shift: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                    alt: KeyState::Pressed,
                },
            },
            2, // Consume both ESC and DEL
        ));
    }

    // Second byte must be printable ASCII (space through ~)
    // Range: 0x20 (space) to 0x7E (~)
    if !(PRINTABLE_ASCII_MIN..=PRINTABLE_ASCII_MAX).contains(&second) {
        return None;
    }

    // Convert to character
    let ch = second as char;

    Some((
        VT100InputEvent::Keyboard {
            code: VT100KeyCode::Char(ch),
            modifiers: VT100KeyModifiers {
                shift: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
                alt: KeyState::Pressed,
            },
        },
        2, // Consume both ESC and letter
    ))
}

/// Parse a CSI keyboard sequence and return the parsed event with bytes consumed.
///
/// **Dispatch position**: 1st parser for CSI sequences (ESC[). See module docs
/// [`Parser Dispatch Priority Pipeline`] for dispatch order. Keyboard sequences are tried
/// first because they're more common than mouse or terminal events.
///
/// Handles arrow keys, function keys, and modified keys like Alt+Right, Ctrl+Up, etc.
///
/// ## Returns
///
/// `Some((event, bytes_consumed))` if a complete sequence was parsed,
/// `None` if the sequence is incomplete or invalid.
///
/// [`Parser Dispatch Priority Pipeline`]
/// [`CSI Sequences`]
///
/// [`Parser Dispatch Priority Pipeline`]: mod@self#parser-dispatch-priority-pipeline
/// [`CSI Sequences`]: mod@self#csi-sequences-esc
#[must_use]
pub fn parse_keyboard_sequence(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // Check minimum length: ESC [ + final byte
    if buffer.len() < 3 {
        return None;
    }

    // Check for ESC [ sequence start
    if buffer[0] != ANSI_ESC || buffer[1] != ANSI_CSI_BRACKET {
        return None;
    }

    // Handle simple control keys first (single character after ESC[)
    if buffer.len() == 3 {
        return helpers::parse_csi_single_char(buffer[2]).map(|event| (event, 3));
    }

    // Parse parameters and final byte for multi-character sequences
    helpers::parse_csi_parameters(buffer)
}

/// Parse an SS3 keyboard sequence and return the parsed event with bytes consumed.
///
/// **Dispatch position**: Only parser for SS3 sequences (ESC O). See module docs
/// [`Parser Dispatch Priority Pipeline`] for dispatch order.
///
/// SS3 sequences (ESC O + single char) are used in terminal application mode (vim, less,
/// emacs) for arrow keys, function keys (F1-F4), Home, End, and numpad keys. Always 3
/// bytes.
///
/// **Note**: SS3 sequences do NOT support modifiers. Modified arrow keys use CSI format.
///
/// ## Returns
///
/// `Some((event, 3))` if a valid SS3 sequence was parsed,
/// `None` if the sequence is incomplete or invalid.
///
/// [`Parser Dispatch Priority Pipeline`]
/// [`SS3 Sequences`]
///
/// [`Parser Dispatch Priority Pipeline`]: mod@self#parser-dispatch-priority-pipeline
/// [`SS3 Sequences`]: mod@self#ss3-sequences-esc-o
#[must_use]
pub fn parse_ss3_sequence(buffer: &[u8]) -> Option<(VT100InputEvent, usize)> {
    // SS3 sequences must be exactly 3 bytes: ESC O + command_char
    if buffer.len() < 3 {
        return None;
    }

    // Check for ESC O sequence start
    if buffer[0] != ANSI_ESC || buffer[1] != ANSI_SS3_O {
        return None;
    }

    // Parse the command character
    let code = helpers::parse_ss3_command(buffer[2])?;

    Some((
        VT100InputEvent::Keyboard {
            code,
            modifiers: VT100KeyModifiers::default(),
        },
        3,
    ))
}

/// Private helper functions for keyboard sequence parsing.
///
/// This module contains internal parsing utilities that support the public API functions.
/// Functions here handle lower-level sequence parsing and decoding tasks.
mod helpers {
    use super::*;

    /// Parse SS3 command character and return the corresponding [`VT100KeyCode`].
    pub(super) fn parse_ss3_command(byte: u8) -> Option<VT100KeyCode> {
        match byte {
            // Arrow keys
            ARROW_UP_FINAL => Some(VT100KeyCode::Up),
            ARROW_DOWN_FINAL => Some(VT100KeyCode::Down),
            ARROW_RIGHT_FINAL => Some(VT100KeyCode::Right),
            ARROW_LEFT_FINAL => Some(VT100KeyCode::Left),
            // Home and End keys
            SPECIAL_HOME_FINAL => Some(VT100KeyCode::Home),
            SPECIAL_END_FINAL => Some(VT100KeyCode::End),
            // Function keys F1-F4 (SS3 mode)
            SS3_F1_FINAL => Some(VT100KeyCode::Function(1)),
            SS3_F2_FINAL => Some(VT100KeyCode::Function(2)),
            SS3_F3_FINAL => Some(VT100KeyCode::Function(3)),
            SS3_F4_FINAL => Some(VT100KeyCode::Function(4)),
            // Numpad keys in application mode
            // Note: These send SS3 sequences instead of literal digits to allow
            // applications to distinguish numpad from regular number keys
            SS3_NUMPAD_0 => Some(VT100KeyCode::Char('0')),
            SS3_NUMPAD_1 => Some(VT100KeyCode::Char('1')),
            SS3_NUMPAD_2 => Some(VT100KeyCode::Char('2')),
            SS3_NUMPAD_3 => Some(VT100KeyCode::Char('3')),
            SS3_NUMPAD_4 => Some(VT100KeyCode::Char('4')),
            SS3_NUMPAD_5 => Some(VT100KeyCode::Char('5')),
            SS3_NUMPAD_6 => Some(VT100KeyCode::Char('6')),
            SS3_NUMPAD_7 => Some(VT100KeyCode::Char('7')),
            SS3_NUMPAD_8 => Some(VT100KeyCode::Char('8')),
            SS3_NUMPAD_9 => Some(VT100KeyCode::Char('9')),
            // Numpad operators and special keys
            SS3_NUMPAD_ENTER => Some(VT100KeyCode::Enter),
            SS3_NUMPAD_PLUS => Some(VT100KeyCode::Char('+')),
            SS3_NUMPAD_MINUS => Some(VT100KeyCode::Char('-')),
            SS3_NUMPAD_MULTIPLY => Some(VT100KeyCode::Char('*')),
            SS3_NUMPAD_DIVIDE => Some(VT100KeyCode::Char('/')),
            SS3_NUMPAD_DECIMAL => Some(VT100KeyCode::Char('.')),
            SS3_NUMPAD_COMMA => Some(VT100KeyCode::Char(',')),
            _ => None,
        }
    }

    /// Parse single-character CSI sequences like `CSI A` (up arrow)
    pub(super) fn parse_csi_single_char(final_byte: u8) -> Option<VT100InputEvent> {
        let code = match final_byte {
            ARROW_UP_FINAL => VT100KeyCode::Up,
            ARROW_DOWN_FINAL => VT100KeyCode::Down,
            ARROW_RIGHT_FINAL => VT100KeyCode::Right,
            ARROW_LEFT_FINAL => VT100KeyCode::Left,
            SPECIAL_HOME_FINAL => VT100KeyCode::Home,
            SPECIAL_END_FINAL => VT100KeyCode::End,
            BACKTAB_FINAL => VT100KeyCode::BackTab,
            _ => return None,
        };

        Some(VT100InputEvent::Keyboard {
            code,
            modifiers: VT100KeyModifiers::default(),
        })
    }

    /// Parse CSI sequences with numeric parameters (e.g., `CSI 5 ~ `, `CSI 1 ; 3 C`)
    /// Returns (`InputEvent`, `bytes_consumed`) on success.
    pub(super) fn parse_csi_parameters(
        buffer: &[u8],
    ) -> Option<(VT100InputEvent, usize)> {
        // Extract the parameters and final byte
        // Format: ESC [ [param;param;...] final_byte
        let mut params = Vec::new();
        let mut current_num = String::new();
        let mut final_byte = 0u8;
        let mut bytes_scanned = 0;

        for (idx, &byte) in buffer[2..].iter().enumerate() {
            bytes_scanned = idx + 1; // Track position relative to buffer[2..]

            // IMPORTANT: We use if/else chains instead of match arms because Rust treats
            // constants in match patterns as variable bindings, not value comparisons.
            // This is a Rust language limitation documented in RFC 1445.
            //
            // Using named constants in match arms like:
            //   ASCII_DIGIT_0..=ASCII_DIGIT_9 => { ... }
            // would create new bindings named ASCII_DIGIT_0 and ASCII_DIGIT_9 instead of
            // matching against the constant values. The if/else chain correctly compares
            // against the constant values.

            if (ASCII_DIGIT_0..=ASCII_DIGIT_9).contains(&byte) {
                // Digit: accumulate in current_num
                current_num.push(byte as char);
            } else if byte == ANSI_PARAM_SEPARATOR {
                // Semicolon: parameter separator
                if !current_num.is_empty() {
                    params.push(current_num.parse::<u16>().unwrap_or(0));
                    current_num.clear();
                }
            } else if byte == ANSI_FUNCTION_KEY_TERMINATOR
                || (ASCII_UPPER_A..=ASCII_UPPER_Z).contains(&byte)
                || (ASCII_LOWER_A..=ASCII_LOWER_Z).contains(&byte)
            {
                // Terminal character: end of sequence
                if !current_num.is_empty() {
                    params.push(current_num.parse::<u16>().unwrap_or(0));
                }
                final_byte = byte;
                break;
            } else {
                return None; // Invalid byte in sequence
            }
        }

        if final_byte == 0 {
            return None; // No final byte found
        }

        // Total bytes consumed: ESC [ (2 bytes) + scanned bytes (includes final)
        let total_consumed = 2 + bytes_scanned;

        // Parse based on parameters and final byte
        let event = match (params.len(), final_byte) {
            // BackTab (Shift+Tab): CSI Z
            (0, BACKTAB_FINAL) => Some(VT100InputEvent::Keyboard {
                code: VT100KeyCode::BackTab,
                modifiers: VT100KeyModifiers::default(),
            }),
            // Arrow keys with modifiers: CSI 1 ; m A/B/C/D
            (2, ARROW_UP_FINAL) if params[0] == 1 => {
                let modifiers = decode_modifiers(extract_modifier_parameter(params[1]));
                Some(VT100InputEvent::Keyboard {
                    code: VT100KeyCode::Up,
                    modifiers,
                })
            }
            (2, ARROW_DOWN_FINAL) if params[0] == 1 => {
                let modifiers = decode_modifiers(extract_modifier_parameter(params[1]));
                Some(VT100InputEvent::Keyboard {
                    code: VT100KeyCode::Down,
                    modifiers,
                })
            }
            (2, ARROW_RIGHT_FINAL) if params[0] == 1 => {
                let modifiers = decode_modifiers(extract_modifier_parameter(params[1]));
                Some(VT100InputEvent::Keyboard {
                    code: VT100KeyCode::Right,
                    modifiers,
                })
            }
            (2, ARROW_LEFT_FINAL) if params[0] == 1 => {
                let modifiers = decode_modifiers(extract_modifier_parameter(params[1]));
                Some(VT100InputEvent::Keyboard {
                    code: VT100KeyCode::Left,
                    modifiers,
                })
            }
            // Function keys and special keys: CSI n ~ or CSI n ; m ~
            (1, ANSI_FUNCTION_KEY_TERMINATOR) => {
                parse_function_or_special_key(params[0], VT100KeyModifiers::default())
            }
            (2, ANSI_FUNCTION_KEY_TERMINATOR) => {
                let modifiers = decode_modifiers(extract_modifier_parameter(params[1]));
                parse_function_or_special_key(params[0], modifiers)
            }
            // Other CSI sequences
            _ => None,
        }?;

        Some((event, total_consumed))
    }

    /// Parse function keys (F1-F12) and special keys (Insert, Delete, Home, End, PageUp,
    /// PageDown).
    ///
    /// Maps ANSI codes to VT100KeyCode. Called by CSI parameter parser.
    fn parse_function_or_special_key(
        code: u16,
        modifiers: VT100KeyModifiers,
    ) -> Option<VT100InputEvent> {
        let key_code = match code {
            // Function keys: map ANSI codes to F1-F12
            FUNCTION_F1_CODE => VT100KeyCode::Function(1),
            FUNCTION_F2_CODE => VT100KeyCode::Function(2),
            FUNCTION_F3_CODE => VT100KeyCode::Function(3),
            FUNCTION_F4_CODE => VT100KeyCode::Function(4),
            FUNCTION_F5_CODE => VT100KeyCode::Function(5),
            FUNCTION_F6_CODE => VT100KeyCode::Function(6),
            FUNCTION_F7_CODE => VT100KeyCode::Function(7),
            FUNCTION_F8_CODE => VT100KeyCode::Function(8),
            FUNCTION_F9_CODE => VT100KeyCode::Function(9),
            FUNCTION_F10_CODE => VT100KeyCode::Function(10),
            FUNCTION_F11_CODE => VT100KeyCode::Function(11),
            FUNCTION_F12_CODE => VT100KeyCode::Function(12),
            // Special keys
            // Home: Multiple alternative codes for different terminal implementations
            SPECIAL_HOME_ALT1_CODE | SPECIAL_HOME_ALT2_CODE => VT100KeyCode::Home,
            SPECIAL_INSERT_CODE => VT100KeyCode::Insert,
            SPECIAL_DELETE_CODE => VT100KeyCode::Delete,
            // End: Multiple alternative codes for different terminal implementations
            SPECIAL_END_ALT1_CODE | SPECIAL_END_ALT2_CODE => VT100KeyCode::End,
            SPECIAL_PAGE_UP_CODE => VT100KeyCode::PageUp,
            SPECIAL_PAGE_DOWN_CODE => VT100KeyCode::PageDown,
            _ => return None,
        };

        Some(VT100InputEvent::Keyboard {
            code: key_code,
            modifiers,
        })
    }

    /// Extract modifier parameter from CSI with type safety.
    ///
    /// Safe to cast u16→u8 because VT-100 modifiers are always 1-8.
    #[allow(clippy::cast_possible_truncation)]
    fn extract_modifier_parameter(param: u16) -> u8 {
        debug_assert!(param <= 255, "Modifier parameter out of range: {}", param);
        param as u8
    }

    /// Decode CSI modifier parameter (1-8) to VT100KeyModifiers.
    ///
    /// CSI encoding: param = 1 + bitfield, where bitfield = Shift(1)|Alt(2)|Ctrl(4).
    /// See module docs [`Modifier Encoding`] for full table.
    ///
    /// [`Modifier Encoding`]
    ///
    /// [`Modifier Encoding`]: mod@super#why-each-modifier-uses-different-encoding
    fn decode_modifiers(modifier_mask: u8) -> VT100KeyModifiers {
        // Subtract offset to get the bitfield (CSI parameter = 1 + bitfield)
        let bits = modifier_mask.saturating_sub(MODIFIER_PARAMETER_OFFSET);

        // Fast path: if no modifiers, return default (all NotPressed)
        if bits == MODIFIER_NONE {
            return VT100KeyModifiers::default();
        }

        VT100KeyModifiers {
            shift: if (bits & MODIFIER_SHIFT) != MODIFIER_NONE {
                KeyState::Pressed
            } else {
                KeyState::NotPressed
            },
            alt: if (bits & MODIFIER_ALT) != MODIFIER_NONE {
                KeyState::Pressed
            } else {
                KeyState::NotPressed
            },
            ctrl: if (bits & MODIFIER_CTRL) != MODIFIER_NONE {
                KeyState::Pressed
            } else {
                KeyState::NotPressed
            },
        }
    }
}

/// Unit tests for keyboard input parsing.
///
/// Uses generator functions for round-trip testing consistency between
/// sequence generation and parsing. See module docs [`Testing Strategy`] for details.
///
/// [`Testing Strategy`]
///
/// [`Testing Strategy`]: mod@super#testing-strategy
#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Test Helpers ====================
    // These helpers use the input event generator to build test sequences,
    // ensuring consistency between parsing and generation (round-trip testing).

    /// Build an arrow key sequence using the generator.
    fn arrow_key_sequence(code: VT100KeyCode, modifiers: VT100KeyModifiers) -> Vec<u8> {
        use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_keyboard_sequence;
        let event = VT100InputEvent::Keyboard { code, modifiers };
        generate_keyboard_sequence(&event).expect("Failed to generate arrow key sequence")
    }

    /// Build a function key sequence using the generator.
    fn function_key_sequence(n: u8, modifiers: VT100KeyModifiers) -> Vec<u8> {
        use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_keyboard_sequence;
        let event = VT100InputEvent::Keyboard {
            code: VT100KeyCode::Function(n),
            modifiers,
        };
        generate_keyboard_sequence(&event)
            .expect("Failed to generate function key sequence")
    }

    /// Build a special key sequence (Home, End, Insert, Delete, `PageUp`, `PageDown`)
    /// using the generator.
    fn special_key_sequence(code: VT100KeyCode, modifiers: VT100KeyModifiers) -> Vec<u8> {
        use crate::core::ansi::vt_100_terminal_input_parser::test_fixtures::generate_keyboard_sequence;
        let event = VT100InputEvent::Keyboard { code, modifiers };
        generate_keyboard_sequence(&event)
            .expect("Failed to generate special key sequence")
    }

    // ==================== SS3 Sequences ====================
    // SS3 sequences (ESC O) are used in vim, less, emacs and other terminal apps
    // when they're in application mode. Simple 3-byte format: ESC O + command_char

    #[test]
    fn test_ss3_arrow_up() {
        let input = b"\x1bOA"; // ESC O A
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 up");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_arrow_down() {
        let input = b"\x1bOB"; // ESC O B
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 down");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Down,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_arrow_right() {
        let input = b"\x1bOC"; // ESC O C
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 right");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Right,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_arrow_left() {
        let input = b"\x1bOD"; // ESC O D
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 left");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_home() {
        let input = b"\x1bOH"; // ESC O H
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 home");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Home,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_end() {
        let input = b"\x1bOF"; // ESC O F
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 end");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::End,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_f1() {
        let input = b"\x1bOP"; // ESC O P
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 F1");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(1),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_f2() {
        let input = b"\x1bOQ"; // ESC O Q
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 F2");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(2),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_f3() {
        let input = b"\x1bOR"; // ESC O R
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 F3");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(3),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_f4() {
        let input = b"\x1bOS"; // ESC O S
        let (event, bytes_consumed) =
            parse_ss3_sequence(input).expect("Should parse SS3 F4");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(4),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, 3);
    }

    #[test]
    fn test_ss3_incomplete_sequence() {
        let input = b"\x1bO"; // Only ESC O, missing command char
        assert!(
            parse_ss3_sequence(input).is_none(),
            "Incomplete SS3 sequence should return None"
        );
    }

    #[test]
    fn test_ss3_invalid_command_char() {
        let input = b"\x1bOX"; // ESC O X (X is not a valid command)
        assert!(
            parse_ss3_sequence(input).is_none(),
            "Invalid SS3 command should return None"
        );
    }

    #[test]
    fn test_ss3_rejects_csi_sequence() {
        // Make sure SS3 parser correctly rejects CSI sequences
        let input = b"\x1b[A"; // CSI sequence, not SS3
        assert!(
            parse_ss3_sequence(input).is_none(),
            "SS3 parser should reject CSI sequences"
        );
    }

    // ==================== Arrow Keys ====================

    #[test]
    fn test_arrow_up() {
        // Use generator to build the sequence (self-documenting)
        let input = arrow_key_sequence(VT100KeyCode::Up, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_arrow_down() {
        let input = arrow_key_sequence(VT100KeyCode::Down, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Down,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_arrow_right() {
        let input = arrow_key_sequence(VT100KeyCode::Right, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Right,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_arrow_left() {
        let input = arrow_key_sequence(VT100KeyCode::Left, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    // ==================== Arrow Keys with Modifiers ====================

    #[test]
    fn test_shift_up() {
        // Build sequence with Shift modifier using generator
        let input = arrow_key_sequence(
            VT100KeyCode::Up,
            VT100KeyModifiers {
                shift: KeyState::Pressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers: VT100KeyModifiers {
                    shift: KeyState::Pressed,
                    alt: KeyState::NotPressed,
                    ctrl: KeyState::NotPressed,
                }
            }
        );
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_alt_right() {
        let input = arrow_key_sequence(
            VT100KeyCode::Right,
            VT100KeyModifiers {
                shift: KeyState::NotPressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::NotPressed,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Right,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
            }
            _ => panic!("Expected Alt+Right"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_ctrl_up() {
        // ESC[1;5A = Ctrl+Up (verified with real terminal output)
        let input = arrow_key_sequence(
            VT100KeyCode::Up,
            VT100KeyModifiers {
                shift: KeyState::NotPressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Up,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
                assert_eq!(
                    modifiers.ctrl,
                    KeyState::Pressed,
                    "Ctrl+Up should have ctrl modifier set"
                );
            }
            _ => panic!("Expected Ctrl+Up"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_ctrl_down() {
        let input = arrow_key_sequence(
            VT100KeyCode::Down,
            VT100KeyModifiers {
                shift: KeyState::NotPressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Down,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
            }
            _ => panic!("Expected Ctrl+Down"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_alt_ctrl_left() {
        let input = arrow_key_sequence(
            VT100KeyCode::Left,
            VT100KeyModifiers {
                shift: KeyState::NotPressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::Pressed,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
            }
            _ => panic!("Expected Alt+Ctrl+Left"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_shift_alt_ctrl_left() {
        let input = arrow_key_sequence(
            VT100KeyCode::Left,
            VT100KeyModifiers {
                shift: KeyState::Pressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::Pressed,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Left,
                modifiers,
            } => {
                assert_eq!(modifiers.shift, KeyState::Pressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
            }
            _ => panic!("Expected Shift+Alt+Ctrl+Left"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    // ==================== Special Keys ====================

    #[test]
    fn test_home_key() {
        let input =
            special_key_sequence(VT100KeyCode::Home, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Home,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_end_key() {
        let input = special_key_sequence(VT100KeyCode::End, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::End,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_insert_key() {
        let input =
            special_key_sequence(VT100KeyCode::Insert, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Insert,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_delete_key() {
        let input =
            special_key_sequence(VT100KeyCode::Delete, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Delete,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_page_up() {
        let input =
            special_key_sequence(VT100KeyCode::PageUp, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::PageUp,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_page_down() {
        let input =
            special_key_sequence(VT100KeyCode::PageDown, VT100KeyModifiers::default());
        let (event, bytes_consumed) =
            parse_keyboard_sequence(&input).expect("Should parse");
        assert!(matches!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::PageDown,
                modifiers: _
            }
        ));
        assert_eq!(bytes_consumed, input.len());
    }

    // ==================== Function Keys ====================

    #[test]
    fn test_f1_key() {
        let input = function_key_sequence(1, VT100KeyModifiers::default());
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(n),
                modifiers: _,
            } => {
                assert_eq!(n, 1);
            }
            _ => panic!("Expected F1"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_f6_key() {
        let input = function_key_sequence(6, VT100KeyModifiers::default());
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(n),
                modifiers: _,
            } => {
                assert_eq!(n, 6);
            }
            _ => panic!("Expected F6"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_f12_key() {
        // Build F12 sequence (ANSI code 24) using generator
        let input = function_key_sequence(12, VT100KeyModifiers::default());
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        assert_eq!(
            event,
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(12),
                modifiers: VT100KeyModifiers::default()
            }
        );
        assert_eq!(bytes_consumed, input.len());
    }

    // ==================== Function Keys with Modifiers ====================

    #[test]
    fn test_shift_f5() {
        let input = function_key_sequence(
            5,
            VT100KeyModifiers {
                shift: KeyState::Pressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(n),
                modifiers,
            } => {
                assert_eq!(n, 5);
                assert_eq!(modifiers.shift, KeyState::Pressed);
                assert_eq!(modifiers.alt, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
            }
            _ => panic!("Expected Shift+F5"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    #[test]
    fn test_ctrl_alt_f10() {
        let input = function_key_sequence(
            10,
            VT100KeyModifiers {
                shift: KeyState::NotPressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::Pressed,
            },
        );
        let (event, bytes_consumed) = parse_keyboard_sequence(&input).unwrap();
        match event {
            VT100InputEvent::Keyboard {
                code: VT100KeyCode::Function(n),
                modifiers,
            } => {
                assert_eq!(n, 10);
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
                assert_eq!(modifiers.ctrl, KeyState::Pressed);
            }
            _ => panic!("Expected Ctrl+Alt+F10"),
        }
        assert_eq!(bytes_consumed, input.len());
    }

    // ==================== Alt+Key Tests (parse_alt_letter) ====================
    // These tests validate ESC prefix Alt+key combinations (ESC + printable ASCII or DEL)

    #[test]
    fn test_alt_letter_b() {
        let input = &[ANSI_ESC, b'b']; // ESC b → Alt+b
        let (event, bytes_consumed) =
            parse_alt_letter(input).expect("Should parse Alt+b");
        match event {
            VT100InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCode::Char('b'));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, 2);
    }

    #[test]
    fn test_alt_letter_f() {
        let input = &[ANSI_ESC, b'f']; // ESC f → Alt+f
        let (event, bytes_consumed) =
            parse_alt_letter(input).expect("Should parse Alt+f");
        match event {
            VT100InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCode::Char('f'));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, 2);
    }

    #[test]
    fn test_alt_letter_uppercase() {
        let input = &[ANSI_ESC, b'B']; // ESC B → Alt+B (uppercase)
        let (event, bytes_consumed) =
            parse_alt_letter(input).expect("Should parse Alt+B");
        match event {
            VT100InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCode::Char('B'));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, 2);
    }

    #[test]
    fn test_alt_digit() {
        let input = &[ANSI_ESC, b'3']; // ESC 3 → Alt+3
        let (event, bytes_consumed) =
            parse_alt_letter(input).expect("Should parse Alt+3");
        match event {
            VT100InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCode::Char('3'));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, 2);
    }

    #[test]
    fn test_alt_space() {
        let input = &[ANSI_ESC, b' ']; // ESC space → Alt+space
        let (event, bytes_consumed) =
            parse_alt_letter(input).expect("Should parse Alt+space");
        match event {
            VT100InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCode::Char(' '));
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, 2);
    }

    #[test]
    fn test_alt_backspace() {
        let input = &[ANSI_ESC, ASCII_DEL]; // ESC DEL → Alt+Backspace
        let (event, bytes_consumed) =
            parse_alt_letter(input).expect("Should parse Alt+Backspace");
        match event {
            VT100InputEvent::Keyboard { code, modifiers } => {
                assert_eq!(code, VT100KeyCode::Backspace);
                assert_eq!(modifiers.shift, KeyState::NotPressed);
                assert_eq!(modifiers.ctrl, KeyState::NotPressed);
                assert_eq!(modifiers.alt, KeyState::Pressed);
            }
            _ => panic!("Expected Keyboard event"),
        }
        assert_eq!(bytes_consumed, 2);
    }

    #[test]
    fn test_alt_letter_incomplete() {
        let input = &[ANSI_ESC]; // Just ESC, no second byte
        let event = parse_alt_letter(input);
        assert_eq!(event, None, "Should return None for incomplete sequence");
    }

    #[test]
    fn test_alt_letter_not_esc() {
        let input = b"Ab"; // 'A' 'b' (not ESC prefix)
        let event = parse_alt_letter(input);
        assert_eq!(event, None, "Should return None when first byte is not ESC");
    }

    #[test]
    fn test_alt_letter_control_char() {
        let input = &[ANSI_ESC, 0x01]; // ESC Ctrl+A (0x01 is control char)
        let event = parse_alt_letter(input);
        assert_eq!(
            event, None,
            "Should return None for control characters (below 0x20)"
        );
    }

    #[test]
    fn test_alt_letter_above_del() {
        let input = &[ANSI_ESC, 0x80]; // ESC + 0x80 (above DEL)
        let event = parse_alt_letter(input);
        assert_eq!(event, None, "Should return None for bytes above DEL (0x7F)");
    }

    // ==================== Invalid/Incomplete Sequences ====================

    #[test]
    fn test_incomplete_sequence_short() {
        let input = b"\x1b["; // Just ESC [
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None);
    }

    #[test]
    fn test_incomplete_sequence_no_escape() {
        let input = b"[A"; // No ESC
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None);
    }

    #[test]
    fn test_invalid_final_byte() {
        let input = b"\x1b[@"; // ESC [ @ (invalid final byte)
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None);
    }

    #[test]
    fn test_unknown_function_key() {
        let input = b"\x1b[99~"; // ESC [ 99 ~ (unknown key code)
        let event = parse_keyboard_sequence(input);
        assert_eq!(event, None);
    }
}

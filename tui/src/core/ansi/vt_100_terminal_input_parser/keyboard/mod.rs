// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Keyboard input event parsing from [`ANSI`]/[`CSI`] sequences.
//!
//! This module handles conversion of raw [`ANSI`] escape sequences into keyboard events.
//! It provides comprehensive support for [`VT-100`] compatible terminal input while
//! maintaining clarity about protocol limitations and design decisions.
//!
//! ## Where You Are in the Pipeline
//!
//! For the full data flow, see the [parent module documentation]. This diagram shows
//! where [`keyboard`] fits:
//!
//! ```text
//! DirectToAnsiInputDevice (async I/O layer)
//!    │
//!    ▼
//! router.rs (routing & ESC detection)
//!    │ (routes CSI/SS3 keyboard sequences here)
//! ┌──▼───────────────────────────────────────┐  ┌──────────────────┐
//! │  keyboard.rs                             ◄──┤ **YOU ARE HERE** │
//! │  • Parse CSI u (Kitty protocol)          │  └──────────────────┘
//! │  • Parse CSI sequences (ESC [)           │
//! │  • Parse SS3 sequences (ESC O)           │
//! │  • Handle modifiers (Shift/Ctrl/Alt)     │
//! │  • Control characters (Ctrl+A, etc)      │
//! │  • Alt+letter combinations               │
//! └──────────────────────────────────────────┘
//!    │
//!    ▼
//! VT100InputEventIR::Keyboard { code, modifiers }
//!    │
//!    ▼
//! convert_input_event() → InputEvent (returned to application)
//! ```
//!
//! **Navigate**:
//! - ⬆️ **Up**: [`router`] - Main routing entry point
//! - ➡️ **Peer**: [`mouse`], [`terminal_events`], [`utf8`] - Other specialized parsers
//! - 📚 **Types**: [`VT100InputEventIR`], [`VT100KeyCodeIR`], [`VT100KeyModifiersIR`]
//! - 🔧 **Functions**: [`parse_csi_u_sequence`], [`parse_keyboard_sequence`],
//!   [`parse_ss3_sequence`], [`parse_control_character`], [`parse_alt_letter`]
//! - 📤 **Converted by**: [`convert_input_event()`] in `protocol_conversion.rs` (not this
//!   module)
//!
//! ## Keyboard Encoding Explained
//!
//! You might wonder:
//! - Why does Alt+A send `ESC a` (2 bytes) instead of a [`CSI`] sequence like `ESC [ 1 ;
//!   3 a`?
//! - Why can't I distinguish Ctrl+Shift+A from Ctrl+A?
//! - What does Ctrl+Alt+A send?
//! - Why does F6 send `ESC [ 1 7 ~` instead of `ESC [ 1 6 ~`?
//! - Can I detect when a key is released?
//!
//! These behaviors stem from [`VT-100`] design decisions made in the 1970s that remain
//! standard today. The core principle: use the **simplest encoding that works**. This
//! minimized bytes sent over slow serial lines (a 1970s constraint that became a lasting
//! design principle) and keeps parsing simple.
//!
//! ### [`ASCII`] (1963)
//!
//! Uses only 7 bits (0-127). The 8th bit was used for [parity checking] during serial
//! transmission—a transport-layer mechanism for error detection on noisy lines, not part
//! of stored character values. A committee including [Bob Bemer] developed [`ASCII`]; he
//! championed the [`ESC`] character that made escape sequences possible.
//!
//! ### [`ANSI`] escape codes] (1979)
//!
//! Built on the [`ASCII`] [`ESC`] character. Standardized as [`ANSI`] X3.64 based on
//! the [`DEC`] [`VT-100`] terminal, these are the `ESC [ ...` sequences we still use today
//! (e.g., `ESC [ 1 5 ~` for F5, `ESC [ < 0 ; 10 ; 20 M` for mouse click). Regular keys
//! use single [`ASCII`] bytes, Alt adds one [`ESC`] byte, and only complex modifier
//! combinations require multi-byte [`CSI`] sequences.
//!
//! ### [`UTF-8`] (1992)
//!
//! Created by [Ken Thompson] and [Rob Pike] at Bell Labs. [`UTF-8`] repurposed the high
//! bits as structural markers for multi-byte sequences (not parity), while remaining
//! backwards-compatible with [`ASCII`] and [`ANSI`] escape codes.
//!
//! ### Timeline
//!
//! ```text
//! 1963: ASCII → 7-bit character set with ESC (27)
//! 1975: VT52  → Introduced ESC + letter commands
//! 1978: `VT-100` → Added CSI (ESC [), kept ESC+letter for compatibility
//! 1983: VT220 → Extended CSI, still kept ESC+letter
//! 1992: UTF-8 → Replaced ASCII, but ASCII-compatible
//! 2025: Today → Still using ESC+letter for Alt!
//! ```
//!
//! ### The Three-Tier Encoding Hierarchy
//!
//! ```text
//! 7-bit ASCII stored in 8-bit bytes
//! ──────────────────────────────────
//! 0_000_0000 → 0x00 (0)
//! 0_111_1111 → 0x7F (127)
//! ▲
//! └─ MSB (most significant bit) always 0 for ASCII (values 0-127 fit in 7 bits)
//! ```
//!
//! Note: The 8th MSB bit was historically used for parity during serial transmission (a
//! transport-layer concern, not stored data). [`UTF-8`] repurposed these high bits for
//! multi-byte markers. See [`utf8` encoding] module for encoding details.
//!
//! ```text
//! 1. Single byte (0-127) containing the 7-bit ASCII character set. Any character
//!    that fits in a single byte within this range can be transmitted as-is without any
//!    escape sequence prefix
//!    ┌─ Dec ───────┬ Hex ──┬ Byte expr ──┬ Symbolic ────────────────────────────┐
//!    ├─ 0-31       │ 00-1F │             │ Control character range              │
//!    │  ├─ 0       │ 00    │             │ Ctrl+@ or Ctrl+Space (NUL)           │
//!    │  ├─ 1-26    │ 01-1A │             │ Ctrl+A through Ctrl+Z                │
//!    │  ├─ 27      │ 1B    │             │ ESC (handled separately)             │
//!    │  └─ 28-31   │ 1C-1F │             │ Ctrl+\, Ctrl+], Ctrl+^, Ctrl+_       │
//!    ├─ 32-126     │ 20-7E │             │ Printable ASCII range                │
//!    │  ├─ 32      │ 20    │ b' '        │ Space                                │
//!    │  ├─ 33-47   │ 21-2F │ b'!' - b'/' │ Punct: ! " # $ % & ' ( ) * + , - . / │
//!    │  ├─ 48-57   │ 30-39 │ b'0' - b'9' │ Digits: '0'-'9'                      │
//!    │  ├─ 58-64   │ 3A-40 │ b':' - b'@' │ Punct: : ; < = > ? @                 │
//!    │  ├─ 65-90   │ 41-5A │ b'A' - b'Z' │ Uppercase: 'A'-'Z'                   │
//!    │  ├─ 91-96   │ 5B-60 │ b'[' - b'`' │ Punct: [ \ ] ^ _ `                   │
//!    │  ├─ 97-122  │ 61-7A │ b'a' - b'z' │ Lowercase: 'a'-'z'                   │
//!    │  └─ 123-126 │ 7B-7E │ b'{' - b'~' │ Punct: { | } ~                       │
//!    ├─ 127        │ 7F    │             │ DEL character (used for Backspace)   │
//!    └─────────────┴───────┴─────────────┴──────────────────────────────────────┘
//!
//! 2. ESC prefix (2 bytes). Alt+printable character uses this simple encoding since
//!    there's no room in ASCII for Alt. Just prepend ESC (1B hex) to the character
//!    ┌─ Sequence ─────┬ Dec ───┬ Hex ──┬ Symbolic ─┐
//!    ├─ Alt+a         │ 27 97  │ 1B 61 │ (ESC a)   │
//!    ├─ Alt+B         │ 27 66  │ 1B 42 │ (ESC B)   │
//!    ├─ Alt+3         │ 27 51  │ 1B 33 │ (ESC 3)   │
//!    ├─ Alt+Space     │ 27 32  │ 1B 20 │ (ESC ░)   │
//!    ├─ Alt+Backspace │ 27 127 │ 1B 7F │ (ESC DEL) │
//!    └────────────────┴────────┴───────┴───────────┘
//!
//! 3. CSI sequences (3-7 bytes). Complex modifier combinations and special keys that
//!    can't be represented in simpler encodings use parametric escape sequences
//!    ┌─ Sequence ─┬─ Dec ─────────────────┬─ Hex ────────────────┬─ Symbolic ────────┬ Size ┐
//!    ├─ Home      │ 27 91 72              │ 1B 5B 48             │ (ESC [ H)         │ 3    │
//!    ├─ Delete    │ 27 91 51 126          │ 1B 5B 33 7E          │ (ESC [ 3 ~)       │ 4    │
//!    ├─ F5        │ 27 91 49 53 126       │ 1B 5B 31 35 7E       │ (ESC [ 1 5 ~)     │ 5    │
//!    ├─ Ctrl+Up   │ 27 91 49 59 53 65     │ 1B 5B 31 3B 35 41    │ (ESC [ 1 ; 5 A)   │ 6    │
//!    ├─ Alt+Down  │ 27 91 49 59 51 66     │ 1B 5B 31 3B 33 42    │ (ESC [ 1 ; 3 B)   │ 6    │
//!    ├─ Ctrl+F5   │ 27 91 49 53 59 53 126 │ 1B 5B 31 35 3B 35 7E │ (ESC [ 1 5 ; 5 ~) │ 7    │
//!    └────────────┴───────────────────────┴──────────────────────┴───────────────────┴──────┘
//! ```
//!
//! ### How Bitmask Encoding for Modifiers Works
//!
//! *(See also: [`mouse` module docs] for a contrasting example of where pure bitwise OR
//! is required).*
//!
//! [`CSI`] sequences encode modifiers as a number after the semicolon: `ESC [ 1 ; <n> A`.
//! The number `n` is calculated by adding the values of pressed modifiers to 1:
//!
//! ```text
//! Modifier values: Shift = 1, Alt = 2, Ctrl = 4
//!
//! Formula: n = 1 + (pressed modifiers)
//!
//! Examples:
//!                           n          offset (1-indexed, not 0-indexed)
//!                           │          │
//!                           ▼          ▼
//! Ctrl+Up       : ESC [ 1 ; 5 A    5 = 1 + Ctrl(4)
//! Alt+Down      : ESC [ 1 ; 3 B    3 = 1 + Alt(2)
//! Ctrl+Shift+Up : ESC [ 1 ; 6 A    6 = 1 + Shift(1) + Ctrl(4)
//! ```
//!
//! Here's how each modifier is encoded (sorted by byte count):
//!
//! | Bytes   | Encoding                    | Modifier       | Reason                                |
//! | :------ | :-------------------------- | :------------- | :------------------------------------ |
//! | 0       | Implicit in case            | **Shift**      | `'a'` vs `'A'` already encodes it     |
//! | 1       | Single byte (`0x00-0x1F`)   | **Ctrl**       | Fits in [`ASCII`] control codes       |
//! | 2       | [`ESC`] prefix              | **Alt**        | No room in [`ASCII`], prepend [`ESC`] |
//! | 4-8     | [`CSI`] parameters          | **Combos**     | Need bitmask encoding                 |
//!
//! - Why does Ctrl+Shift+A = Ctrl+Shift+a = Ctrl+A?
//!
//!   Shift is lost because both produce the same control code (`0x01`). Ctrl works by
//!   AND-ing with `0x1F` (the "Ctrl mask" that keeps only the lower 5 bits), and Shift
//!   only changes case—but both cases mask to the same result:
//!
//!   ```text
//!                       │         Ctrl mask (keeps lower 5 bits)
//!                       ▼         ────┴────
//!   Ctrl+A:       'A' 0100_0001 & 0001_1111  = 0000_0001
//!   Ctrl+Shift+A: 'A' 0100_0001 & 0001_1111  = 0000_0001 ← same!
//!   Ctrl+a:       'a' 0110_0001 & 0001_1111  = 0000_0001 ← also same!
//!                       ▲
//!                       └─ only this bit differs, and it gets masked away
//!   ```
//!
//! - What does Ctrl+Alt+A send?
//!
//!   `ESC 0x01` (0x1B 0x01). The terminal applies Ctrl first (masking 'A' → 0x01), then
//!   Alt prepends [`ESC`].
//!
//! ### Function Key Quirks
//!
//! **Why does F6 send `ESC [17~` instead of `ESC [16~`?** Historical [`VT-220`] quirk.
//! The original [`VT-220`] terminal reserved codes 16 and 22 for other purposes, creating
//! gaps: F5 = 15, F6 = 17 (skips 16); F10 = 21, F11 = 23 (skips 22).
//!
//! ### Protocol Limitations
//!
//! **Can I detect when a key is released?** No. [`VT-100`] protocol only sends sequences
//! on key **press**. Key release events are not part of the protocol. Modern protocols
//! like [`Kitty`] keyboard protocol support press/release/repeat events, but we maintain
//! [`VT-100`] compatibility.
//!
//! ### Real-World Examples
//!
//! What terminals actually send (confirmed via `showkey -a` on Linux, or `sed -n l` for
//! POSIX compliant OSes):
//!
//! ```text
//! Key Press      Sequence     Bytes   Format
//! ─────────────────────────────────────────────────────
//! Alt+A          ESC a        2       ESC prefix ✓
//! Alt+Shift+A    ESC A        2       ESC + uppercase ✓
//! Ctrl+Alt+Up    ESC [ 1 ; 7 A    6       CSI (complex)
//! ```
//!
//! **Why this design survived 50 years:**
//! - Works everywhere (bash, vim, emacs, tmux, etc.)
//! - Simpler to parse than [`CSI`]
//! - More efficient (fewer bytes)
//! - Unambiguous ([`ESC`] always means "next char is modified")
//!
//! ## [`CSI`] vs [`ESC`] Prefix: When to Use Each
//!
//! **[`ESC`] prefix** (this module's `parse_alt_letter()`):
//! - Alt+printable-character (Alt+B, Alt+F, Alt+3, Alt+.)
//! - Simple 2-byte sequences: `ESC char`
//!
//! **[`CSI`] sequences** (this module's `parse_keyboard_sequence()`):
//! - Special keys with modifiers (Ctrl+Up, Shift+F5)
//! - Complex modifier combinations (Ctrl+Alt+Up)
//! - Parametric sequences: `ESC [ params finalchar`
//!
//! This dual approach gives us the best of both worlds: efficiency for simple cases
//! (Alt+letter) and expressiveness for complex cases (Ctrl+Alt+Shift+Up).
//!
//! ## Ambiguous Control Character Handling
//!
//! **Design Decision**: Some control characters are ambiguous at the protocol level
//! because terminals send identical byte sequences for different key combinations. This
//! parser **prioritizes the common key** over the Ctrl+letter combination.
//!
//! ### Ambiguous Mappings (Identical Bytes)
//!
//! | Bytes    | Key Combination            | Parser Interpretation   | Rationale                           |
//! | :------- | :------------------------- | :---------------------- | :---------------------------------- |
//! | `0x09`   | Tab **OR** Ctrl+I          | **Tab**                 | Tab key is far more commonly used   |
//! | `0x0A`   | Enter (LF) **OR** Ctrl+J   | **Enter**               | Enter key is essential for apps     |
//! | `0x0D`   | Enter (CR) **OR** Ctrl+M   | **Enter**               | Enter key is essential for apps     |
//! | `0x08`   | Backspace **OR** Ctrl+H    | **Backspace**           | Backspace is critical for editing   |
//! | `0x1B`   | [`ESC`] **OR** Ctrl+\[     | **[`ESC`]**             | Standard for vi-mode, modals        |
//!
//! ### Why This Matters
//!
//! **Problem**: In [`VT-100`] terminals, Ctrl modifies keys by masking with `0x1F`:
//! - `Ctrl+I` = `'I'` (`0x49`) & `0x1F` = `0x09` (same as Tab)
//! - `Ctrl+M` = `'M'` (`0x4D`) & `0x1F` = `0x0D` (same as Enter/CR)
//! - `Ctrl+H` = `'H'` (`0x48`) & `0x1F` = `0x08` (same as Backspace)
//!
//! **Solution**: Prioritize the dedicated key's interpretation. Applications that need
//! Ctrl+I/Ctrl+M/Ctrl+H can use alternative key bindings (e.g., Ctrl+Space for custom
//! actions).
//!
//! ### Unambiguous Cases (Different Sequences)
//!
//! These DO work correctly because terminals send distinct sequences:
//! - **Shift+Tab**: Sends `ESC [Z` (parsed as `BackTab`)
//! - **Ctrl+Arrow**: Sends `ESC [1;5A/B/C/D` (parsed with Ctrl modifier)
//! - **Alt+Letter**: Sends `ESC + letter` (parsed with Alt modifier)
//! - **Function Keys**: Send `ESC [n~` or `ESC O P/Q/R/S`
//!
//! This is a fundamental [`VT-100`] protocol limitation, not a parser bug. Modern
//! protocols like [`Kitty`] keyboard protocol solve this, but we maintain [`VT-100`]
//! compatibility.
//!
//! ## Comprehensive List of Supported Keyboard Shortcuts
//!
//! ### Basic Keys
//! | Key               | Sequence          | Notes                              |
//! | :---------------- | :---------------- | :--------------------------------- |
//! | **Tab**           | `0x09`            | Fixed: was returning None          |
//! | **Enter**         | `0x0D`/`0x0A`     | CR or LF depending on terminal     |
//! | **Backspace**     | `0x08`/`0x7F`     | BS or DEL encoding                 |
//! | **Escape**        | `0x1B`            | Modal UI support                   |
//! | **Space**         | `0x20`            | Regular space character            |
//!
//! ### Control Key Combinations (Ctrl+Letter)
//! | Key                               | Byte              | Notes                            |
//! | :-------------------------------- | :---------------- | :------------------------------- |
//! | **Ctrl+Space**                    | `0x00`            | Ctrl+@, treated as Ctrl+Space    |
//! | **Ctrl+A** through **Ctrl+Z**     | `0x01`-`0x1A`     | Standard control chars           |
//! | **Ctrl+\\**                       | `0x1C`            | FS (File Separator)              |
//! | **Ctrl+]**                        | `0x1D`            | GS (Group Separator)             |
//! | **Ctrl+^**                        | `0x1E`            | RS (Record Separator)            |
//! | **Ctrl+_**                        | `0x1F`            | US (Unit Separator)              |
//!
//! ### Alt Key Combinations (Alt+Letter)
//! | Key                           | Sequence            | Format                  |
//! | :---------------------------- | :------------------ | :---------------------- |
//! | **Alt+\[a-z\]**               | [`ESC`] + letter    | Lowercase letters       |
//! | **Alt+\[A-Z\]**               | [`ESC`] + letter    | Uppercase letters       |
//! | **Alt+\[0-9\]**               | [`ESC`] + digit     | Digits                  |
//! | **Alt+Space**                 | [`ESC`] + space     | Space key               |
//! | **Alt+Backspace**             | [`ESC`] + `0x7F`    | Delete word             |
//! | **Alt+\[punctuation\]**       | [`ESC`] + char      | Any printable [`ASCII`] |
//!
//! ### Arrow Keys
//! | Key           | [`CSI`] Sequence | SS3 Sequence     | Application Mode     |
//! | :------------ | :--------------- | :--------------- | :------------------- |
//! | **Up**        | `ESC [A`         | `ESC O A`        | vim/less/emacs       |
//! | **Down**      | `ESC [B`         | `ESC O B`        | vim/less/emacs       |
//! | **Right**     | `ESC [C`         | `ESC O C`        | vim/less/emacs       |
//! | **Left**      | `ESC [D`         | `ESC O D`        | vim/less/emacs       |
//!
//! ### Arrow Keys with Modifiers
//! | Key                              | Sequence               | Format                |
//! | :------------------------------- | :--------------------- | :-------------------- |
//! | **Ctrl+Up/Down/Left/Right**      | `ESC [1;5A/B/D/C`      | [`CSI`] with modifier |
//! | **Alt+Up/Down/Left/Right**       | `ESC [1;3A/B/D/C`      | [`CSI`] with modifier |
//! | **Shift+Up/Down/Left/Right**     | `ESC [1;2A/B/D/C`      | [`CSI`] with modifier |
//! | **Ctrl+Alt+arrows**              | `ESC [1;7A/B/D/C`      | Combined modifiers    |
//!
//! ### Special Navigation Keys
//! | Key               | Primary      | Alt 1        | Alt 2        | SS3          |
//! | :---------------- | :----------- | :----------- | :----------- | :----------- |
//! | **Home**          | `ESC [H`     | `ESC [1~`    | `ESC [7~`    | `ESC O H`    |
//! | **End**           | `ESC [F`     | `ESC [4~`    | `ESC [8~`    | `ESC O F`    |
//! | **Insert**        | `ESC [2~`    | -            | -            | -            |
//! | **Delete**        | `ESC [3~`    | -            | -            | -            |
//! | **Page Up**       | `ESC [5~`    | -            | -            | -            |
//! | **Page Down**     | `ESC [6~`    | -            | -            | -            |
//!
//! ### Tab Navigation
//! | Key                            | Sequence      | Notes                 |
//! | :----------------------------- | :------------ | :-------------------- |
//! | **Tab**                        | `0x09`        | Forward navigation    |
//! | **Shift+Tab (`BackTab`)**      | `ESC [Z`      | Backward navigation   |
//!
//! ### Function Keys F1-F12
//! | Key         | [`CSI`] Code    | SS3 Sequence     | Notes              |
//! | :---------- | :-------------- | :--------------- | :----------------- |
//! | **F1**      | `ESC [11~`      | `ESC O P`        | SS3 in app mode    |
//! | **F2**      | `ESC [12~`      | `ESC O Q`        | SS3 in app mode    |
//! | **F3**      | `ESC [13~`      | `ESC O R`        | SS3 in app mode    |
//! | **F4**      | `ESC [14~`      | `ESC O S`        | SS3 in app mode    |
//! | **F5**      | `ESC [15~`      | -                | [`CSI`] only       |
//! | **F6**      | `ESC [17~`      | -                | Note: gap at 16    |
//! | **F7**      | `ESC [18~`      | -                | [`CSI`] only       |
//! | **F8**      | `ESC [19~`      | -                | [`CSI`] only       |
//! | **F9**      | `ESC [20~`      | -                | [`CSI`] only       |
//! | **F10**     | `ESC [21~`      | -                | [`CSI`] only       |
//! | **F11**     | `ESC [23~`      | -                | Note: gap at 22    |
//! | **F12**     | `ESC [24~`      | -                | [`CSI`] only       |
//!
//! ### Function Keys with Modifiers
//! Function keys support all modifier combinations using [`CSI`] format:
//! - **Shift+F5**: `ESC [15;2~` (modifier = 2)
//! - **Alt+F5**: `ESC [15;3~` (modifier = 3)
//! - **Ctrl+F5**: `ESC [15;5~` (modifier = 5)
//! - **Ctrl+Alt+F10**: `ESC [21;7~` (modifier = 7)
//!
//! ### Numpad Application Mode (SS3 Sequences)
//!
//! In application mode (DECPAM), numpad keys send SS3 sequences instead of their literal
//! digits. This allows applications to distinguish numpad from regular number keys.
//!
//! | Numpad Key     | Normal Mode     | Application Mode     | SS3 Char     |
//! | :------------- | :-------------- | :------------------- | :----------- |
//! | **0**          | `'0'`           | `ESC O p`            | `p`          |
//! | **1**          | `'1'`           | `ESC O q`            | `q`          |
//! | **2**          | `'2'`           | `ESC O r`            | `r`          |
//! | **3**          | `'3'`           | `ESC O s`            | `s`          |
//! | **4**          | `'4'`           | `ESC O t`            | `t`          |
//! | **5**          | `'5'`           | `ESC O u`            | `u`          |
//! | **6**          | `'6'`           | `ESC O v`            | `v`          |
//! | **7**          | `'7'`           | `ESC O w`            | `w`          |
//! | **8**          | `'8'`           | `ESC O x`            | `x`          |
//! | **9**          | `'9'`           | `ESC O y`            | `y`          |
//! | **Enter**      | `CR`            | `ESC O M`            | `M`          |
//! | **+**          | `'+'`           | `ESC O k`            | `k`          |
//! | **-**          | `'-'`           | `ESC O m`            | `m`          |
//! | **\***         | `'*'`           | `ESC O j`            | `j`          |
//! | **/**          | `'/'`           | `ESC O o`            | `o`          |
//! | **.**          | `'.'`           | `ESC O n`            | `n`          |
//! | **,**          | `','`           | `ESC O l`            | `l`          |
//!
//! **Use cases**: Calculator apps (distinguish numpad), games (numpad for movement), vim
//! (numpad for navigation).
//!
//! ## Intentionally Unsupported Features
//!
//! ### Extended Function Keys (F13-F24)
//!
//! F13-F24 are intentionally NOT supported:
//! - Rarely available on modern keyboards
//! - No standardized escape sequences across terminals
//! - Different terminals use different codes ([`xterm`] vs linux console vs [`rxvt`])
//! - Minimal real-world usage in applications
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`convert_input_event()`]:
//!     crate::direct_to_ansi::input::protocol_conversion::convert_input_event
//! [`CSI`]: crate::CsiSequence
//! [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
//! [`ESC`]: crate::EscSequence
//! [`keyboard`]: mod@self
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mouse` module docs]: mod@crate::core::ansi::constants::mouse#bitmask-arithmetic-operations
//! [`mouse`]: mod@super::mouse
//! [`router`]: mod@super::router
//! [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
//! [`rxvt`]: https://en.wikipedia.org/wiki/Rxvt
//! [`SGR`]: crate::SgrCode
//! [`terminal_events`]: mod@super::terminal_events
//! [`try_parse_input_event`]: super::try_parse_input_event
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`utf8` encoding]: mod@crate::vt_100_terminal_input_parser::utf8#utf-8-encoding-explained
//! [`utf8`]: mod@super::utf8
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT-220`]: https://en.wikipedia.org/wiki/VT220
//! [`VT100InputEventIR`]: super::VT100InputEventIR
//! [`VT100KeyCodeIR`]: super::VT100KeyCodeIR
//! [`VT100KeyModifiersIR`]: super::VT100KeyModifiersIR
//! [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
//! [`xterm`]: https://en.wikipedia.org/wiki/Xterm
//! [ANSI escape codes]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [Bob Bemer]: https://en.wikipedia.org/wiki/Bob_Bemer
//! [Ken Thompson]: https://en.wikipedia.org/wiki/Ken_Thompson
//! [parent module documentation]: mod@super#primary-consumer
//! [parity checking]: https://en.wikipedia.org/wiki/Parity_bit
//! [Rob Pike]: https://en.wikipedia.org/wiki/Rob_Pike

// Skip rustfmt for rest of file.
#![rustfmt::skip]

// Submodules with conditional visibility for documentation and testing.
#[cfg(any(test, doc))]
pub mod alt_keys;
#[cfg(not(any(test, doc)))]
mod alt_keys;

#[cfg(any(test, doc))]
pub mod ctrl_and_dedicated_keys;
#[cfg(not(any(test, doc)))]
mod ctrl_and_dedicated_keys;

#[cfg(any(test, doc))]
pub mod csi_decoder;
#[cfg(not(any(test, doc)))]
mod csi_decoder;

#[cfg(any(test, doc))]
pub mod csi_u;
#[cfg(not(any(test, doc)))]
mod csi_u;

#[cfg(any(test, doc))]
pub mod modifiers;
#[cfg(not(any(test, doc)))]
mod modifiers;

#[cfg(any(test, doc))]
pub mod ss3;
#[cfg(not(any(test, doc)))]
mod ss3;

// Public re-exports (barrel export pattern).
pub use alt_keys::*;
pub use csi_decoder::*;
pub use csi_u::*;
pub use ctrl_and_dedicated_keys::*;
pub use modifiers::*;
pub use ss3::*;

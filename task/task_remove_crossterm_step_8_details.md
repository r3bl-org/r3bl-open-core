# DirectToAnsi Input Handling Migration

> **Date**: 2025-10-28
> **Status**: Architecture & Planning
> **Based On**: Comprehensive analysis of crossterm 0.29.0 actual parsing implementation

## 1. Executive Summary

### 1.1 Migration Goal

Replace crossterm's EventStream-based input handling with a pure Rust, tokio-native DirectToAnsi implementation that:
- Mirrors the output backend's philosophy: direct ANSI protocol handling without external dependencies (on Linux)
- Achieves feature parity with crossterm's proven parser
- Maintains compatibility with existing InputEvent types
- Enables future removal of crossterm dependency entirely

### 1.2 Why This Migration?

**Current State**: The output path uses DirectToAnsi (pure Rust ANSI generation), but input still depends on crossterm + mio + futures-util for EventStream.

**Desired State**: Complete control over the terminal I/O stack on Linux:
```
Output: RenderOp → AnsiSequenceGenerator → stdout (✓ Already done)
Input:  stdin → ANSI Parser → InputEvent (← This migration)
```

**Benefits**:
- Reduced dependencies (remove mio, futures-util, crossterm on Linux)
- Consistent architecture philosophy across I/O paths
- Full control over parsing behavior and optimizations
- Foundation for future enhancements (custom protocols, performance tuning)

## 2. Complete ANSI Sequence Reference

Based on analysis of crossterm 0.29.0 (`src/event/sys/unix/parse.rs`), here are **all** sequences that must be parsed:

### 2.1 Keyboard Events

#### Raw Byte Mappings
```
0x08 or 0x7F        → Backspace
0x09                → Tab
0x0A or 0x0D        → Enter (0x0A only when NOT in raw mode)
0x1B (alone)        → Esc (after timeout to distinguish from escape sequences)
0x00                → Ctrl+Space

Control Characters (Ctrl+Letter):
0x01..=0x1A         → Ctrl+A through Ctrl+Z
                      Formula: (byte - 0x01 + b'a') as char
0x1C..=0x1F         → Ctrl+4 through Ctrl+7
                      0x1C = Ctrl+\
                      0x1D = Ctrl+]
                      0x1E = Ctrl+^
                      0x1F = Ctrl+_
```

#### Alt+Key Mechanism
```
ESC <any char>      → Alt + <that char>
ESC ESC             → Esc key (double escape = literal Esc)

Examples:
  ESC c             → Alt+C
  ESC H             → Alt+Shift+H (capital H = Shift modifier detected)
  ESC 0x14          → Alt+Ctrl+T (control char after ESC)
```

#### SS3 Sequences (ESC O) - ⚠️ CRITICAL FOR APPLICATION MODE
```
ESC O A             → Up
ESC O B             → Down
ESC O C             → Right
ESC O D             → Left
ESC O H             → Home
ESC O F             → End
ESC O P             → F1
ESC O Q             → F2
ESC O R             → F3
ESC O S             → F4
```

**Why Critical**: Many terminals in "application mode" (like vim) send SS3 instead of CSI for arrows and F1-F4. Missing this = parser fails in vim/less/etc.

#### CSI Sequences (ESC [) - Basic Keys
```
CSI A               → Up
CSI B               → Down
CSI C               → Right
CSI D               → Left
CSI H               → Home
CSI F               → End
CSI Z               → BackTab (Shift+Tab)

Kitty Compatibility (no modifiers):
CSI P               → F1
CSI Q               → F2
CSI S               → F4 (note: F3 not in this format)

Rare:
CSI [ A..E          → F1-F5 (legacy format, low priority)
```

#### CSI Sequences with Modifiers
```
Format: CSI 1 ; <modifier> <letter>

Modifier encoding (subtract 1 from value for bit flags):
  1 = no modifier (base value)
  2 = Shift          (bit 0)
  3 = Alt            (bit 1)
  4 = Shift+Alt      (bits 0+1)
  5 = Ctrl           (bit 2)
  6 = Shift+Ctrl     (bits 0+2)
  7 = Alt+Ctrl       (bits 1+2)
  8 = Shift+Alt+Ctrl (bits 0+1+2)
  9 = Super          (bit 3)
  ...continues for Super+Shift, Super+Alt, etc.

Examples:
  CSI 1;5A          → Ctrl+Up
  CSI 1;3D          → Alt+Left
  CSI 1;2C          → Shift+Right
  CSI 1;6H          → Ctrl+Shift+Home

Letters: A=Up, B=Down, C=Right, D=Left, H=Home, F=End, P=F1, Q=F2, R=F3, S=F4
```

#### CSI ~ Special Keys
```
CSI <n> ~           → Special key
CSI <n> ; <mod> ~   → Special key with modifiers

Key codes:
  1 or 7            → Home
  2                 → Insert
  3                 → Delete
  4 or 8            → End
  5                 → PageUp
  6                 → PageDown

Function keys:
  11, 12, 13, 14, 15    → F1-F5
  17, 18, 19, 20, 21    → F6-F10
  23, 24, 25, 26        → F11-F14
  28, 29                → F15-F16
  31, 32, 33, 34        → F17-F20

Examples:
  CSI 3~            → Delete
  CSI 5;5~          → Ctrl+PageUp
  CSI 11;2~         → Shift+F1
```

#### CSI u (Kitty Keyboard Protocol) - Advanced
```
Format: CSI <codepoint> ; <modifier>:<kind> u

<codepoint>: Unicode value or special functional key code (57358-57454)
<modifier>: Same encoding as above
<kind>: 1=Press, 2=Repeat, 3=Release

This protocol enables:
  - Press/Release/Repeat event types
  - Caps Lock / Num Lock state in modifier mask (bits 6-7)
  - Keypad key detection (KeyEventState::KEYPAD)
  - Media keys (57428-57440)
  - Individual modifier keys (57441-57454: LeftShift, RightCtrl, etc.)
  - Alternate key codes for shifted characters

Examples:
  CSI 97u           → Letter 'a'
  CSI 97;5u         → Ctrl+A
  CSI 97;5:2u       → Ctrl+A (repeat event)
  CSI 57441u        → Left Shift key press
  CSI 57428u        → Play media key
```

### 2.2 Mouse Events

Crossterm supports **3 different mouse protocols**. All must be handled for compatibility.

#### Protocol 1: SGR Mode (Modern) - ⭐ PRIMARY
```
Press:   CSI < Cb ; Cx ; Cy M
Release: CSI < Cb ; Cx ; Cy m

Cb: Button code with modifiers
Cx, Cy: Column and row (1-based, subtract 1 for 0-based)

Button number extraction:
  button_num = (Cb & 0x03) | ((Cb & 0xC0) >> 4)

  0 = Left button
  1 = Middle button
  2 = Right button
  3 = Release (button unknown in non-SGR protocols)
  4 = ScrollUp
  5 = ScrollDown
  6 = ScrollLeft
  7 = ScrollRight

Drag detection:
  dragging = (Cb & 0x20) != 0

Modifiers in Cb:
  Shift = Cb & 0x04
  Alt   = Cb & 0x08
  Ctrl  = Cb & 0x10

Special handling:
  - 'M' at end = press or drag
  - 'm' at end = release (can determine which button was released)

Examples:
  CSI < 0;10;5M     → Left button press at (10,5)
  CSI < 32;15;8M    → Left button drag at (15,8)
  CSI < 0;10;5m     → Left button release at (10,5)
  CSI < 64;20;10M   → Scroll up at (20,10)
```

#### Protocol 2: Normal/X10 Mode (Legacy)
```
Format: CSI M <cb> <cx> <cy>

Three raw bytes follow 'M':
  cb = button code (same encoding as SGR, but add 32)
  cx = column + 32 (raw byte)
  cy = row + 32 (raw byte)

Fixed length: exactly 6 bytes total

Limitation: Can't represent positions > 223 (255-32)

Example:
  ESC[M 0x20 0x3F 0x2A  → Left button at column=31, row=10
```

#### Protocol 3: RXVT Mode
```
Format: CSI Cb ; Cx ; Cy M

Semicolon-separated ASCII decimal numbers:
  Cb = button code (subtract 32)
  Cx, Cy = column, row (1-based)

Example:
  CSI 32;30;40M     → Left button at (29,39) after decoding
```

#### Mouse Button/Event Decoding Table
```
(button_num, dragging) → MouseEventKind:

(0, false) → Down(Left)
(1, false) → Down(Middle)
(2, false) → Down(Right)
(0, true)  → Drag(Left)
(1, true)  → Drag(Middle)
(2, true)  → Drag(Right)
(3, false) → Up(Left)     # button unknown in Normal mode
(3, true)  → Moved        # motion with no button
(4, true)  → Moved
(5, true)  → Moved
(4, false) → ScrollUp
(5, false) → ScrollDown
(6, false) → ScrollLeft
(7, false) → ScrollRight
```

### 2.3 Terminal Events

#### Focus Events
```
CSI I               → Focus gained
CSI O               → Focus lost
```

#### Bracketed Paste
```
Start:  ESC[200~
End:    ESC[201~

Content: Everything between markers, including escape sequences

⚠️ Critical: Pasted text can contain ANSI sequences that should NOT be parsed!

Example:
  ESC[200~hello ESC[2D worldESC[201~

Should emit:
  InputEvent::Paste("hello ESC[2D world")

NOT parse the ESC[2D as a Left arrow!

Implementation: Match ESC[200~, then buffer until ESC[201~ without parsing.
```

#### Resize Events
```
⚠️ CORRECTION: There is NO ANSI sequence for resize in crossterm!

Terminal resize is handled via SIGWINCH signals on Unix, not input parsing.

Original architecture doc incorrectly listed:
  CSI 8 ; rows ; cols t  ← This is a QUERY command, not an event!
```

### 2.4 Internal Query Responses (Not Exposed as InputEvents)

Crossterm parses these but doesn't expose them as public events:
```
CSI Cy ; Cx R           → Cursor position report
CSI ? <flags> u         → Keyboard enhancement flags
CSI ? <attrs> c         → Primary device attributes
```

These are responses to queries sent by the application, not user input.

### 2.5 UTF-8 Text Handling

```
Valid UTF-8 byte patterns:
  0x00..=0x7F           → 1 byte  (ASCII)
  0xC0..=0xDF           → 2 bytes (110xxxxx 10xxxxxx)
  0xE0..=0xEF           → 3 bytes (1110xxxx 10xxxxxx 10xxxxxx)
  0xF0..=0xF7           → 4 bytes (11110xxx 10xxxxxx 10xxxxxx 10xxxxxx)

Invalid start bytes:
  0x80..=0xBF           → Continuation byte, not start
  0xF8..=0xFF           → Invalid UTF-8

Parsing strategy:
  1. Detect first byte to determine required length
  2. If buffer.len() < required_bytes, return Ok(None) to wait
  3. Validate continuation bytes match 10xxxxxx pattern
  4. Parse complete sequence to char
  5. Emit InputEvent::Keyboard(KeyPress::Plain(Key::Character(...)))

Edge case: Uppercase letters
  - Uppercase char → add SHIFT modifier to KeyEvent
  - But NOT for non-Latin scripts (only ASCII A-Z)
```

## 3. Two-Layer Architecture: Protocol + Backend

### 3.1 Final Module Structure (APPROVED NAMING)

**Layer 1: Protocol Parsing** (`core/ansi/` - reusable, backend-agnostic):
```
tui/src/core/ansi/
├── vt_100_pty_output_parser/       ← Parses ANSI from PTY child process stdout
│   ├── mod.rs
│   ├── ansi_parser.rs
│   ├── performer.rs
│   └── operations/                 ← Commands to apply to offscreen buffer
│
├── vt_100_terminal_input_parser/   ← NEW: Parses ANSI from terminal stdin
│   ├── mod.rs                      # Public API exports
│   ├── keyboard.rs                 # parse_keyboard_sequence()
│   ├── mouse.rs                    # parse_mouse_sequence() - all 3 protocols
│   ├── terminal_events.rs          # parse_focus_event(), parse_bracketed_paste()
│   ├── utf8.rs                     # UTF-8 text handling
│   └── tests.rs                    # Pure parsing unit tests
│
├── generator/                      ← Generates ANSI for rendering
├── color/
└── constants/
```

**Layer 2: Backend I/O** (`terminal_lib_backends/` - backend-specific):
```
tui/src/tui/terminal_lib_backends/direct_to_ansi/
├── output/                         ← Uses generator (output path)
│   ├── paint_render_op_impl.rs
│   ├── pixel_char_renderer.rs
│   ├── render_to_ansi.rs
│   └── tests.rs
│
└── input/                          ← Uses vt_100_terminal_input_parser (input path)
    ├── mod.rs                      # Public API exports
    ├── input_device_impl.rs        # DirectToAnsiInputDevice: tokio I/O + buffering
    └── tests.rs
```

**Why This Structure:**
- **Protocol layer** (`core/ansi/vt_100_terminal_input_parser/`) handles VT-100 sequence parsing
- **Backend layer** (`terminal_lib_backends/direct_to_ansi/input/`) handles async I/O and buffering
- Parallel to output architecture: both have protocol layer + backend layer
- Protocol parsers are pure functions, reusable by other backends
- Semantically clear: input parsing lives where other ANSI parsing lives

### 3.2 Data Flow: Protocol + Backend Layers

```
User types/clicks on terminal
    ↓
Terminal generates bytes on stdin (keyboard/mouse/focus/paste)
    ↓
DirectToAnsiInputDevice (backend layer)
  ├─ tokio::io::stdin() reads bytes asynchronously
  ├─ Manages RingBuffer for partial sequences
  ├─ Detects complete sequences
    ↓
vt_100_terminal_input_parser (protocol layer) ← Pure functions
  ├─ parse_keyboard_sequence(bytes) → Option<KeyPress>
  ├─ parse_mouse_sequence(bytes) → Option<MouseInput>
  ├─ parse_focus_event(bytes) → Option<FocusEvent>
  ├─ parse_bracketed_paste(bytes) → Option<String>
    ↓
DirectToAnsiInputDevice converts to InputEvent
    ↓
InputDeviceExt::next_input_event() → App
```

### 3.3 DirectToAnsiInputDevice Struct Design

```rust
// Located in: tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device_impl.rs

pub struct DirectToAnsiInputDevice {
    /// Tokio async stdin handle
    stdin: tokio::io::Stdin,

    /// Ring buffer for efficient byte management (prevents expensive memmove)
    buffer: RingBuffer<4096>,

    /// Timeout for incomplete sequences (150ms before giving up)
    sequence_timeout: Duration,
}

impl DirectToAnsiInputDevice {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            stdin: tokio::io::stdin(),
            buffer: RingBuffer::new(),
            sequence_timeout: Duration::from_millis(150),
        })
    }

    /// Main entry point: read and parse next event
    /// Returns None on EOF
    pub async fn read_event(&mut self) -> io::Result<Option<InputEvent>> {
        loop {
            // 1. Try to parse from existing buffer using protocol parsers
            if let Some(event) = self.try_parse_from_buffer()? {
                return Ok(Some(event));
            }

            // 2. Read more bytes with timeout
            match timeout(self.sequence_timeout, self.read_bytes()).await {
                Ok(Ok(0)) => return Ok(None),  // EOF
                Ok(Ok(_n)) => continue,         // Got bytes, loop to parse
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    // Timeout: try lone Esc or skip malformed byte
                    self.handle_timeout()?;
                }
            }
        }
    }

    /// Try to parse complete sequence from buffer using protocol parsers
    fn try_parse_from_buffer(&mut self) -> io::Result<Option<InputEvent>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        let buffer_slice = self.buffer.as_slice();

        // Dispatch to protocol layer parsers
        match buffer_slice[0] {
            // Escape sequences
            b'\x1B' => self.try_parse_escape_sequence(buffer_slice),

            // Control characters
            0x00..=0x1F => self.try_parse_control_char(buffer_slice),

            // UTF-8 text
            _ => self.try_parse_utf8_text(buffer_slice),
        }
    }

    // These methods call the pure parsers from core::ansi::vt_100_terminal_input_parser::*
    fn try_parse_escape_sequence(&mut self, buffer: &[u8]) -> io::Result<Option<InputEvent>> {
        use crate::vt_100_terminal_input_parser;

        // Calls: parse_keyboard_sequence, parse_mouse_sequence, etc.
        // Returns parsed event + bytes consumed, or None if incomplete
    }

    fn try_parse_control_char(&mut self, buffer: &[u8]) -> io::Result<Option<InputEvent>> {
        use crate::vt_100_terminal_input_parser;

        // Handles Ctrl+X, Tab, Enter, etc.
    }

    fn try_parse_utf8_text(&mut self, buffer: &[u8]) -> io::Result<Option<InputEvent>> {
        use crate::vt_100_terminal_input_parser;

        // Calls UTF-8 parser for regular text input
    }
}
```

### 3.4 Important Design Points

**Why Two Layers?**
1. **Reusability**: Protocol parsers are pure functions with no I/O dependency
2. **Testability**: Parse functions tested independently without async/buffering complexity
3. **Symmetry**: Mirrors output architecture (generator = reusable, paint_impl = backend)
4. **Separation of Concerns**: Buffer/timeout logic separate from protocol parsing logic

**Why Backend Calls Protocol Parsers?**
- DirectToAnsiInputDevice (backend) manages I/O and buffering
- Calls pure parsers from vt_100_terminal_input_parser (protocol) for actual parsing
- Converts parser results to InputEvent
- Handles timeout/recovery gracefully

### 3.3 State Machine (4 Entry Points)

```
┌─────────────────────────────────────────────────────────────┐
│                    Buffer First Byte                         │
└───────┬─────────────────────────────────────────────────────┘
        │
        ├─ 0x1B (ESC) ─────────┬─ [  → parse_csi()
        │                      ├─ O  → parse_ss3()
        │                      ├─ 1B → Esc key
        │                      └─ *  → parse_alt_key()
        │
        ├─ 0x00..=0x1F ────────→ parse_control_char()
        │
        └─ Other ──────────────→ parse_utf8_char()

Each parser returns:
  Ok(Some((event, bytes_consumed))) → Success, advance buffer
  Ok(None)                          → Need more bytes, wait
  Err(_)                            → Malformed, skip 1 byte and retry
```

### 3.4 Parser Function Signatures

```rust
// Modular parser functions (one per protocol)

fn parse_csi(buffer: &[u8]) -> ParseResult {
    // Handles CSI sequences (ESC [ ...)
    match buffer.get(2) {
        Some(b'<') => parse_csi_sgr_mouse(buffer),
        Some(b'M') => parse_csi_normal_mouse(buffer),
        Some(b'A'..=b'D') => parse_csi_arrow_key(buffer),
        Some(b'0'..=b'9') => {
            // Could be special key or mouse
            match buffer.last() {
                Some(b'~') => parse_csi_special_key(buffer),
                Some(b'u') => parse_csi_u_encoded(buffer),
                Some(b'M') => parse_csi_rxvt_mouse(buffer),
                Some(b'R') => parse_csi_cursor_position(buffer),
                _ => parse_csi_modifier_key(buffer),
            }
        }
        Some(b'I') => Ok(Some((Event::FocusGained, 3))),
        Some(b'O') => Ok(Some((Event::FocusLost, 3))),
        Some(b'Z') => Ok(Some((Event::Key(BackTab), 3))),
        _ => Err(ParseError::Malformed),
    }
}

fn parse_ss3(buffer: &[u8]) -> ParseResult {
    // Handles SS3 sequences (ESC O ...)
    if buffer.len() < 3 { return Ok(None); }
    match buffer[2] {
        b'A' => Ok(Some((Event::Key(Up), 3))),
        b'B' => Ok(Some((Event::Key(Down), 3))),
        b'C' => Ok(Some((Event::Key(Right), 3))),
        b'D' => Ok(Some((Event::Key(Left), 3))),
        b'H' => Ok(Some((Event::Key(Home), 3))),
        b'F' => Ok(Some((Event::Key(End), 3))),
        b'P'..=b'S' => {
            let f_num = 1 + (buffer[2] - b'P');
            Ok(Some((Event::Key(F(f_num)), 3)))
        }
        _ => Err(ParseError::Malformed),
    }
}

fn parse_control_char(buffer: &[u8]) -> ParseResult {
    // 0x00-0x1F control character mappings
    match buffer[0] {
        b'\r' => Ok(Some((Event::Key(Enter), 1))),
        b'\n' if !is_raw_mode() => Ok(Some((Event::Key(Enter), 1))),
        b'\t' => Ok(Some((Event::Key(Tab), 1))),
        0x7F => Ok(Some((Event::Key(Backspace), 1))),
        0x00 => Ok(Some((Event::Key(Ctrl+Space), 1))),
        c @ 0x01..=0x1A => {
            let ch = (c - 0x01 + b'a') as char;
            Ok(Some((Event::Key(Ctrl+ch), 1)))
        }
        c @ 0x1C..=0x1F => {
            let ch = (c - 0x1C + b'4') as char;
            Ok(Some((Event::Key(Ctrl+ch), 1)))
        }
        _ => Err(ParseError::Malformed),
    }
}

fn parse_utf8_char(buffer: &[u8]) -> ParseResult {
    let required = match buffer[0] {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => return Err(ParseError::InvalidUtf8),
    };

    if buffer.len() < required {
        return Ok(None); // Wait for more bytes
    }

    let ch = std::str::from_utf8(&buffer[..required])
        .ok()
        .and_then(|s| s.chars().next())
        .ok_or(ParseError::InvalidUtf8)?;

    let modifiers = if ch.is_ascii_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };

    Ok(Some((Event::Key(KeyEvent::new(ch, modifiers)), required)))
}
```

### 3.5 Ring Buffer Strategy

```rust
pub struct RingBuffer<const N: usize> {
    buffer: [u8; N],
    read_pos: usize,
    write_pos: usize,
}

impl<const N: usize> RingBuffer<N> {
    pub fn new() -> Self { ... }

    /// Available bytes to read
    pub fn len(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            N - self.read_pos + self.write_pos
        }
    }

    /// Consume n bytes from front
    pub fn consume(&mut self, n: usize) {
        self.read_pos = (self.read_pos + n) % N;
    }

    /// Get slice view (may need 2 slices if wrapping)
    pub fn as_slices(&self) -> (&[u8], &[u8]) { ... }
}

Benefits:
  - O(1) byte consumption (just advance read_pos)
  - No memmove() on partial sequences
  - Fixed memory allocation (no Vec reallocation)
```

## 4. Implementation Plan (Phased)

### Phase 1: Core Keyboard Events (Week 1) 🎯

**Scope**:
- Control characters (0x00-0x1F, 0x7F)
- SS3 sequences (ESC O)
- Basic CSI sequences (arrows, Home, End, etc.)
- CSI with modifiers
- CSI ~ special keys (Insert, Delete, PageUp/Down, F1-F20)
- Alt+Key mechanism
- UTF-8 text input

**Files to Create**:
- `direct_to_ansi/mod.rs` - Main struct
- `direct_to_ansi/state.rs` - State machine
- `direct_to_ansi/buffer.rs` - Ring buffer
- `direct_to_ansi/parser_keyboard.rs` - All keyboard parsing
- `direct_to_ansi/parser_utf8.rs` - UTF-8 handling

**Success Criteria**:
- All keyboard unit tests pass (arrow keys, function keys, modifiers)
- Can navigate in vim (tests SS3 sequences)
- Ctrl+C/Ctrl+D work correctly
- Alt+letter combinations work
- UTF-8 characters (including emoji) parse correctly

**Testing**:
```rust
#[test]
fn test_arrow_keys() {
    assert_parses(b"\x1B[A", KeyCode::Up);
    assert_parses(b"\x1BOA", KeyCode::Up); // SS3 format
}

#[test]
fn test_modifiers() {
    assert_parses(b"\x1B[1;5A", Ctrl+Up);
    assert_parses(b"\x1B[1;3A", Alt+Up);
}

#[test]
fn test_function_keys() {
    assert_parses(b"\x1B[11~", F1);
    assert_parses(b"\x1BOP", F1); // SS3 format
}

#[test]
fn test_alt_keys() {
    assert_parses(b"\x1Bc", Alt+C);
    assert_parses(b"\x1BH", Alt+Shift+H);
}
```

### Phase 2: Mouse Support (Week 2) 🖱️

**Scope**:
- SGR mouse protocol (primary)
- Normal/X10 mouse protocol
- RXVT mouse protocol
- Click, drag, scroll detection
- Mouse modifiers

**Files to Create**:
- `direct_to_ansi/parser_mouse.rs` - All 3 mouse protocols

**Success Criteria**:
- Left/middle/right click detection
- Drag detection (all buttons)
- Scroll events (up/down/left/right)
- Mouse modifiers (Shift/Ctrl/Alt + click)
- Works in all 3 protocol modes

**Testing**:
```rust
#[test]
fn test_sgr_mouse() {
    assert_parses(b"\x1B[<0;10;5M", MouseEvent {
        kind: Down(Left),
        column: 9,
        row: 4,
    });
}

#[test]
fn test_normal_mouse() {
    assert_parses(b"\x1B[M \x30\x25", MouseEvent {
        kind: Down(Left),
        column: 15,
        row: 4,
    });
}

#[test]
fn test_scroll() {
    assert_parses(b"\x1B[<64;20;10M", MouseEvent {
        kind: ScrollUp,
        column: 19,
        row: 9,
    });
}
```

### Phase 3: Terminal Events (Week 3) 🪟

**Scope**:
- Focus events (CSI I/O)
- Bracketed paste (ESC[200~/201~)
- Query responses (cursor position, device attributes)

**Files to Create**:
- `direct_to_ansi/parser_terminal.rs` - Focus, paste, queries

**Success Criteria**:
- Focus gained/lost events work
- Bracketed paste captures multi-line text correctly
- Pasted text containing escape sequences doesn't break parser
- Query responses parsed but not exposed as events

**Testing**:
```rust
#[test]
fn test_focus_events() {
    assert_parses(b"\x1B[I", Event::FocusGained);
    assert_parses(b"\x1B[O", Event::FocusLost);
}

#[test]
fn test_bracketed_paste() {
    assert_parses(
        b"\x1B[200~hello\nworld\x1B[201~",
        Event::Paste("hello\nworld".to_string())
    );
}

#[test]
fn test_paste_with_escapes() {
    // Paste containing ANSI sequence that should NOT be parsed
    assert_parses(
        b"\x1B[200~text\x1B[2Dmore\x1B[201~",
        Event::Paste("text\x1B[2Dmore".to_string())
    );
}
```

### Phase 4: Advanced Features (Week 4) 🚀

**Scope**:
- CSI u (Kitty keyboard protocol)
- Press/Repeat/Release event types
- Caps Lock / Num Lock state
- Media keys
- Individual modifier keys (LeftShift vs RightShift)

**Files to Modify**:
- `direct_to_ansi/parser_keyboard.rs` - Add CSI u parsing

**Success Criteria**:
- Kitty protocol sequences parse correctly
- Press/Release events distinguished
- Caps Lock state detected
- Media keys recognized
- Individual modifiers detected

**Testing**:
```rust
#[test]
fn test_kitty_protocol() {
    assert_parses(b"\x1B[97;5u", Ctrl+A);
    assert_parses(b"\x1B[97;5:2u", Ctrl+A with Repeat);
    assert_parses(b"\x1B[97;5:3u", Ctrl+A with Release);
}

#[test]
fn test_media_keys() {
    assert_parses(b"\x1B[57428u", MediaKey::Play);
}

#[test]
fn test_individual_modifiers() {
    assert_parses(b"\x1B[57441u", ModifierKey::LeftShift);
}
```

### Phase 5: Integration & Platform Support (Week 5) 🔧

**Scope**:
- Conditional compilation for Linux vs macOS/Windows
- InputDevice wrapper with platform-specific backends
- Integration with existing tui event loop
- Crossterm backend marked deprecated

**Files to Create/Modify**:
- `input/input_device.rs` - Platform wrapper
- `input/mod.rs` - Public API

**Success Criteria**:
- Linux builds use DirectToAnsi
- macOS/Windows builds use crossterm (deprecated)
- Existing TUI apps work without changes
- Performance benchmarks show no regression

**Platform Strategy**:
```rust
#[cfg(target_os = "linux")]
pub struct InputDevice {
    inner: DirectToAnsiInputDevice,
}

#[cfg(not(target_os = "linux"))]
pub struct InputDevice {
    inner: CrosstermInputDevice, // Deprecated, marked for removal
}

impl InputDevice {
    pub async fn read_event(&mut self) -> io::Result<Option<InputEvent>> {
        self.inner.read_event().await
    }
}
```

## 5. Testing Strategy

### 5.1 Unit Tests (Per Parser Function)

Each parser function gets comprehensive unit tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csi_arrow_keys() {
        assert_eq!(parse_csi(b"\x1B[A"), Ok(Some((KeyCode::Up, 3))));
        assert_eq!(parse_csi(b"\x1B[B"), Ok(Some((KeyCode::Down, 3))));
        // ... all arrow variations
    }

    #[test]
    fn test_parse_csi_incomplete() {
        assert_eq!(parse_csi(b"\x1B["), Ok(None)); // Need more bytes
        assert_eq!(parse_csi(b"\x1B[1"), Ok(None));
    }

    #[test]
    fn test_parse_csi_malformed() {
        assert!(parse_csi(b"\x1B[X").is_err()); // Invalid
    }
}
```

### 5.2 Integration Tests (Real Terminal)

Test with actual terminal emulators:
```rust
#[tokio::test]
async fn test_real_terminal_arrow_keys() {
    // Requires terminal in raw mode
    let mut device = DirectToAnsiInputDevice::new().unwrap();

    // Simulate key press (in CI: use tmux/expect scripts)
    let event = device.read_event().await.unwrap();

    assert!(matches!(event, Some(InputEvent::Keyboard(_))));
}
```

### 5.3 Crossterm Parity Tests

Ensure identical behavior:
```rust
#[test]
fn test_crossterm_parity() {
    let sequences = [
        (b"\x1B[A", "Up arrow"),
        (b"\x1B[1;5A", "Ctrl+Up"),
        (b"\x1B[<0;10;5M", "Mouse click"),
        // ... all sequences
    ];

    for (bytes, desc) in sequences {
        let our_result = parse_event(bytes);
        let crossterm_result = crossterm::event::parse(bytes);

        assert_eq!(our_result, crossterm_result, "Mismatch for: {}", desc);
    }
}
```

### 5.4 Terminal Emulator Compatibility Matrix

Test on multiple terminals:
- ✅ xterm (reference implementation)
- ✅ GNOME Terminal (common default)
- ✅ Alacritty (GPU-accelerated)
- ✅ Kitty (modern features)
- ✅ WezTerm (Rust-based)
- ✅ foot (Wayland-native)
- ✅ tmux/screen (terminal multiplexers)

## 6. Migration Path & Risk Mitigation

### 6.1 Backward Compatibility

```rust
// Phase 1-4: DirectToAnsi implementation (Linux only, feature-flagged)
#[cfg(all(target_os = "linux", feature = "direct-to-ansi-input"))]
use direct_to_ansi::DirectToAnsiInputDevice as InputDevice;

#[cfg(not(all(target_os = "linux", feature = "direct-to-ansi-input")))]
use crossterm_wrapper::CrosstermInputDevice as InputDevice;

// Phase 5: Make DirectToAnsi default on Linux
#[cfg(target_os = "linux")]
use direct_to_ansi::DirectToAnsiInputDevice as InputDevice;

#[cfg(not(target_os = "linux"))]
use crossterm_wrapper::CrosstermInputDevice as InputDevice;

// Future: Remove crossterm entirely (Step 9)
```

### 6.2 Rollback Plan

If critical issues are discovered:
1. Disable `direct-to-ansi-input` feature by default
2. Document known issues in CHANGELOG
3. Fix issues in patch release
4. Re-enable feature

### 6.3 Performance Benchmarks

Ensure no regression:
```rust
#[bench]
fn bench_parse_throughput(b: &mut Bencher) {
    let input = b"\x1B[A\x1B[B\x1B[C\x1B[D".repeat(1000);
    b.iter(|| {
        for chunk in input.chunks(4) {
            parse_event(chunk);
        }
    });
}
```

Target: ≥ crossterm performance (should be faster due to no FFI overhead)

## 7. Key Differences from Original Architecture Document

The original `ARCHITECTURE_STEP_8_INPUT.md` had these issues (now corrected):

1. **Missing SS3 Sequences** ❌→✅
   - Original: Didn't mention `ESC O` at all
   - Corrected: Full SS3 support for application mode

2. **Incomplete Mouse Support** ❌→✅
   - Original: Only SGR protocol
   - Corrected: All 3 protocols (SGR, Normal, RXVT)

3. **Wrong Mouse Decode Formula** ❌→✅
   - Original: `Cb & 0xC0` for modifiers
   - Corrected: `(Cb & 0x03) | ((Cb & 0xC0) >> 4)` for button number

4. **Invalid Resize Sequence** ❌→✅
   - Original: `CSI 8 ; rows ; cols t`
   - Corrected: Resize via SIGWINCH, not ANSI parsing

5. **Incomplete Control Chars** ❌→✅
   - Original: Mentioned but no mappings
   - Corrected: Full 0x00-0x1F mapping table

6. **Missing Alt+Key Mechanism** ❌→✅
   - Original: Modifier encoding only
   - Corrected: `ESC <char>` = Alt+<char>

7. **No Kitty Protocol** ❌→✅
   - Original: Not mentioned
   - Corrected: Full CSI u support documented

## 8. Success Criteria (Overall)

- ✅ All Phase 1-4 tests passing
- ✅ Works in at least 5 different terminal emulators
- ✅ Crossterm parity tests pass (100% compatibility)
- ✅ vim/emacs/less work correctly (tests SS3 sequences)
- ✅ Mouse events work in all 3 protocols
- ✅ No performance regression vs crossterm
- ✅ Zero panics on malformed input (fuzzing tests pass)
- ✅ choose() and readline_async() functions work unchanged

---

**Next Steps**:
1. User review and approval of this architecture
2. Begin Phase 1 implementation (Core Keyboard Events)
3. Set up testing infrastructure (unit + integration)

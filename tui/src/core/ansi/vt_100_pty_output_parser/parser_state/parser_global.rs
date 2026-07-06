// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::modes::{AutoWrapMode, CursorVisibilityMode};
use crate::{Pos, PtyResponseEvent, TermRow, TuiStyle, osc::OscEvent};

/// Encapsulated runtime state tracking active graphic renditions, terminal attributes,
/// and protocol requests for [`ANSI`] sequence parsing.
///
/// This struct maintains the global terminal states that persist across individual parser
/// sequences and must be inherited when transitioning between different screen buffers
/// (e.g., from the primary grid to the alternate grid).
///
/// It acts as the central holding area for:
/// - Graphic renditions ([`SGR`]) such as active foreground/background colors, text
///   styling (bold, italic, etc.), ensuring [`BCE`] (Background Color Erase) compliance.
/// - The active character set mapping (e.g., standard [`ASCII`] vs. [`DEC`] Line
///   Drawing).
/// - Cursor visibility states toggled via escape codes.
/// - Pending Device Status Report ([`DSR`]) requests that need to be flushed back to the
///   [`PTY`].
///
/// ## Primary and Alt Screen State Management
///
/// This struct manages global terminal settings. Why is the active cursor position
/// intentionally omitted here?
///
/// Under the [`VT-100`]/[`xterm`] spec, when switching to the alternate screen:
/// - **Global state shared between primary and alternate screens**: Graphic renditions
///   (colors, bold, etc.), active character sets (`ESC ( B`), and cursor visibility
///   settings persist across screen switches. They belong to the terminal's global state
///   machine, not to a specific text grid. That's why they are stored here in
///   [`ParserGlobalState`].
/// - **Screen (primary or alternate) specific state**: The active cursor position (`row`,
///   `col`) is not shared. The primary screen and alternate screen maintain entirely
///   independent cursors.
///   - To keep primary and alternate screen drawing logic simple, we use
///     [`std::mem::swap()`] in [`set_alt_screen_mode()`] to physically swap the screen's
///     underlying [`OfsBuf`] data when switching to another screen.
///   - Because of this, the [`OfsBuf`] and its [`OfsBuf::get_cursor_pos()`]
///     **always** represent the currently visible screen (regardless of whether it is
///     primary or alternate).
///   - The hidden screen's data and its specific cursor position are parked in
///     [`HiddenScreenState`] (in its [`hidden_buffer`] and [`hidden_cursor_pos`]) until
///     they are swapped back when the screen switches.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`BCE`]:
///     https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
/// [`DECRC`]: https://vt100.net/docs/vt510-rm/DECRC.html
/// [`DECSC`]: https://vt100.net/docs/vt510-rm/DECSC.html
/// [`DSR`]: crate::PtyResponseEvent
/// [`hidden_buffer`]: crate::HiddenScreenState::hidden_buffer
/// [`hidden_cursor_pos`]: crate::HiddenScreenState::hidden_cursor_pos
/// [`HiddenScreenState`]: crate::HiddenScreenState
/// [`OfsBuf::get_cursor_pos()`]: crate::OfsBuf::get_cursor_pos
/// [`OfsBuf`]: crate::OfsBuf
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`set_alt_screen_mode()`]: crate::OfsBufVT100::set_alt_screen_mode
/// [`SGR`]: crate::SgrCode
/// [`std::mem::swap()`]: std::mem::swap
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParserGlobalState {
    /// Temporary cursor position storage for [`DECSC`]/[`DECRC`] escape sequences only.
    ///
    /// This field is ONLY used for `ESC 7` ([`DECSC`]) save and `ESC 8` ([`DECRC`])
    /// restore operations, as well as their [`CSI`] equivalents (`CSI s` and `CSI
    /// u`). It does NOT track the current cursor position - that's stored in
    /// [`OfsBuf::get_cursor_pos()`].
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to implement the [`DECSC`] (`ESC 7`) and
    /// [`DECRC`] (`ESC 8`) escape sequences for saving and restoring cursor
    /// position.
    ///
    /// ## Data Flow:
    /// ```text
    /// 1. Child process (e.g., vim) sends ESC 7 to save cursor
    ///                             ↓
    /// 2. AnsiToOfsBufPerformer::esc_dispatch() handles ESC 7
    ///                             ↓
    /// 3. Saves current cursor_pos to buffer.parser_global_state.
    ///     `cursor_pos_for_esc_save_and_restore`.
    ///                             ↓
    /// 4. Later, child sends ESC 8 to restore cursor
    ///                             ↓
    /// 5. AnsiToOfsBufPerformer::esc_dispatch() handles ESC 8
    ///                             ↓
    /// 6. Restores cursor_pos from buffer.parser_global_state.cursor_pos_for_esc_save_and_restore
    /// ```
    ///
    /// [`AnsiToOfsBufPerformer`]: crate::core::ansi::vt_100_pty_output_parser::AnsiToOfsBufPerformer
    /// [`CSI`]: crate::CsiSequence
    /// [`DECRC`]: https://vt100.net/docs/vt510-rm/DECRC.html
    /// [`DECSC`]: https://vt100.net/docs/vt510-rm/DECSC.html
    /// [`OfsBuf::get_cursor_pos()`]: crate::OfsBuf::get_cursor_pos
    pub cursor_pos_for_esc_save_and_restore: Option<Pos>,

    /// Active character set for [`ANSI`] escape sequence support.
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to implement character set switching via the
    /// sequences:
    /// - `ESC ( B` for [`ASCII`].
    /// - `ESC ( 0` for [`DEC`] graphics.
    ///
    /// When in Graphics mode, characters like 'q' are translated to box-drawing
    /// characters like '─' during the [`print()`] operation.
    ///
    /// ## Character Set Usage:
    /// ```text
    /// ASCII Mode    : ESC ( B  : 'q' → 'q' (literal)
    /// Graphics Mode : ESC ( 0  : 'q' → '─' (horizontal line)
    /// ```
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    /// [`ESC`]: crate::EscSequence
    /// [`print()`]: vte::Perform::print
    pub character_set: CharacterSet,

    /// Auto-wrap mode ([`DECAWM`]) for [`ANSI`] escape sequence support.
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to control line wrapping behavior when printing
    /// characters. This implements the [`VT-100`] [`DECAWM`] (Auto Wrap Mode)
    /// specification.
    ///
    /// ## [`DECAWM`] Control:
    ///
    /// ```text
    /// ESC[ ? 7h : Enable auto-wrap (default)  - Characters wrap to next line
    /// ESC[ ? 7l : Disable auto-wrap           - Characters overwrite at right margin
    /// ```
    ///
    /// When enabled (default), characters that would exceed the right margin
    /// automatically wrap to the beginning of the next line. When disabled, the cursor
    /// stays at the right margin and subsequent characters overwrite.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    /// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    pub auto_wrap_mode: AutoWrapMode,

    /// Pending wrap state for deferred wrapping.
    ///
    /// Tracks if the cursor is waiting to wrap to the next line upon receiving the next
    /// printable character.
    pub pending_wrap: PendingWrap,

    /// Currently active [`SGR`] (Select Graphic Rendition) text formatting.
    ///
    /// Accumulates text attributes (bold, italic, etc.) and colors from `ESC [ ... m`
    /// sequences. These styles are stamped onto all subsequent characters printed to the
    /// buffer.
    ///
    /// [`SGR`]: crate::SgrCode
    pub current_style: TuiStyle,

    /// [`OSC`] events (hyperlinks, titles, etc.) accumulated during processing.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub pending_osc_events: Vec<OscEvent>,

    /// [`DSR`] response events accumulated during processing - need to be sent back to
    /// [`PTY`].
    ///
    /// [`DSR`]: crate::DsrSequence
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub pending_pty_response_events: Vec<PtyResponseEvent>,

    /// Top margin for the **scrollable region** ([`DECSTBM`]) - 1-based row number.
    ///
    /// This variable defines the **upper boundary** of the area where scrolling occurs.
    /// Rows above this boundary are part of the **static top region** and do not scroll.
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to implement [`DECSTBM`] (Set Top and Bottom
    /// Margins) functionality via the sequence: `ESC [ <top> ; <bottom> r`.
    ///
    /// When [`None`], the default top margin is row 1 (first row), making the entire
    /// terminal screen the scrollable region. When [`Some(n)`], scrolling operations
    /// only affect rows from n to [`scroll_region_bottom`].
    ///
    /// ## [`DECSTBM`] Usage:
    /// ```text
    /// ESC [ 5 ; 20 r   - Set scrolling region from row 5 to row 20
    /// ESC [ r          - Reset to full screen (clears both margins)
    /// ```
    ///
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`ESC`]: crate::EscSequence
    /// [`scroll_region_bottom`]: Self::scroll_region_bottom
    /// [`Some(n)`]: Some
    pub scroll_region_top: Option<TermRow>,

    /// Bottom margin for the **scrollable region** ([`DECSTBM`]) - 1-based row number.
    ///
    /// This variable defines the **lower boundary** of the area where scrolling occurs.
    /// Rows below this boundary are part of the **static bottom region** and do not
    /// scroll.
    ///
    /// Used by [`AnsiToOfsBufPerformer`] to implement [`DECSTBM`] (Set Top and Bottom
    /// Margins) functionality via the sequence: `ESC [ <top> ; <bottom> r`.
    ///
    /// When [`None`], the default bottom margin is the last row of the terminal, making
    /// the entire terminal screen the scrollable region. When [`Some(n)`], scrolling
    /// operations only affect rows from [`scroll_region_top`] to n.
    ///
    /// ## [`DECSTBM`] Behavior:
    /// - Scrolling commands ([`ESC`] D, [`ESC`] M, [`CSI`] S, [`CSI`] T) only affect the
    ///   region
    /// - Cursor movement is constrained to the region boundaries
    /// - Content outside the region remains unchanged during scrolling
    ///
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    /// [`CSI`]: crate::CsiSequence
    /// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
    /// [`ESC`]: crate::EscSequence
    /// [`scroll_region_top`]: Self::scroll_region_top
    /// [`Some(n)`]: Some
    pub scroll_region_bottom: Option<TermRow>,

    /// Controls whether the terminal cursor is visible.
    ///
    /// Corresponds to the [`DECTCEM`] (`?25`) private mode.
    ///
    /// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
    pub cursor_visibility: CursorVisibilityMode,
}

impl ParserGlobalState {
    /// Puts the terminal into a pending wrap state.
    pub fn set_pending_wrap(&mut self) { self.pending_wrap = PendingWrap::Yes; }

    /// Clears the pending wrap state.
    pub fn clear_pending_wrap(&mut self) { self.pending_wrap = PendingWrap::No; }

    /// Returns the current pending wrap state.
    #[must_use]
    pub fn get_pending_wrap(&self) -> PendingWrap { self.pending_wrap }
}

/// Character set modes for terminal emulation.
///
/// Used by [`AnsiToOfsBufPerformer`] to handle `ESC ( <char>` sequences that switch
/// between [`ASCII`] and [`DEC`] line-drawing graphics.
///
/// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
/// [`ESC`]: crate::EscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterSet {
    /// Normal [`ASCII`] character set:
    /// - `ESC ( B`
    /// - or `[27, 40, 66]` in decimal
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`ESC`]: crate::EscSequence
    #[default]
    Ascii,

    /// [`DEC`] Special Graphics character set for line drawing:
    /// - `ESC ( 0`
    /// - or `[27, 40, 48]` in decimal
    ///
    /// Maps [`ASCII`] characters to box-drawing Unicode characters.
    ///
    /// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
    /// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
    /// [`ESC`]: crate::EscSequence
    DECGraphics,
}

/// Pending wrap state for deferred wrapping.
///
/// Controls whether a wrap to the next line is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PendingWrap {
    Yes,

    #[default]
    No,
}

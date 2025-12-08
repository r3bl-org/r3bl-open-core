// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Paste state machine for collecting bracketed paste text. See [`PasteCollectionState`]
//! docs.

use super::{protocol_conversion::convert_input_event, types::PasteStateResult};
use crate::{InputEvent,
            core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                       VT100KeyCodeIR, VT100PasteModeIR}};

/// State machine for collecting bracketed paste text.
///
/// When the terminal sends a bracketed paste sequence, it arrives as:
/// - `Paste(Start)` marker
/// - Multiple `Keyboard` events (the actual pasted text)
/// - `Paste(End)` marker
///
/// See the data flow diagram in [`try_read_event()`] for how this state machine
/// integrates with the input pipeline.
///
/// This state tracks whether we're currently collecting text between markers.
///
/// # Line Ending Handling
///
/// Both CR (`\r`) and LF (`\n`) are parsed by the keyboard parser as
/// [`VT100KeyCodeIR::Enter`], which is then accumulated as `'\n'`. This means:
/// - LF (`\n`) → `'\n'` ✓
/// - CR (`\r`) → `'\n'` ✓
/// - CRLF (`\r\n`) → `'\n\n'` (double newline)
///
/// Most Unix terminals normalize line endings before sending bracketed paste,
/// so CRLF sequences are uncommon in practice.
///
/// # TODO(windows)
///
/// Windows uses CRLF line endings natively. When adding Windows support for
/// [`DirectToAnsi`], consider normalizing CRLF → LF in the paste accumulator.
/// This would require either tracking the previous byte in the keyboard parser
/// or post-processing the accumulated text.
///
/// [`DirectToAnsi`]: mod@super::super
/// [`VT100KeyCodeIR::Enter`]: crate::core::ansi::vt_100_terminal_input_parser::VT100KeyCodeIR::Enter
/// [`try_read_event()`]: super::input_device::DirectToAnsiInputDevice::try_read_event
#[derive(Debug)]
pub enum PasteCollectionState {
    /// Not currently in a paste operation.
    Inactive,
    /// Currently collecting text for a paste operation.
    Accumulating(String),
}

/// Applies the paste collection state machine to a parsed VT100 event.
///
/// Returns [`PasteStateResult::Emit`] if the event should be emitted to
/// the caller, or [`PasteStateResult::Absorbed`] if the event was absorbed
/// (paste in progress).
pub fn apply_paste_state_machine(
    paste_state: &mut PasteCollectionState,
    vt100_event: &VT100InputEventIR,
) -> PasteStateResult {
    match (paste_state, vt100_event) {
        // Start marker: enter collecting state, don't emit event.
        (
            state @ PasteCollectionState::Inactive,
            VT100InputEventIR::Paste(VT100PasteModeIR::Start),
        ) => {
            *state = PasteCollectionState::Accumulating(String::new());
            PasteStateResult::Absorbed
        }

        // End marker: emit complete paste and exit collecting state.
        (
            state @ PasteCollectionState::Accumulating(_),
            VT100InputEventIR::Paste(VT100PasteModeIR::End),
        ) => {
            // Swap out `&mut state` to `Inactive` to get ownership of what is
            // currently there, then extract accumulated text.
            let state = std::mem::replace(state, PasteCollectionState::Inactive);
            let PasteCollectionState::Accumulating(text) = state else {
                unreachable!(
                    "state was matched as Accumulating(String), so this can't happen"
                );
            };
            PasteStateResult::Emit(InputEvent::BracketedPaste(text))
        }

        // While collecting: accumulate keyboard characters and whitespace.
        // Tab/Enter/Backspace are parsed as dedicated keys (not Char variants),
        // so we must handle them explicitly to preserve whitespace in pastes.
        (PasteCollectionState::Accumulating(buffer), vt100_event) => {
            match vt100_event {
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Char(ch),
                    ..
                } => buffer.push(*ch),
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Enter,
                    ..
                } => buffer.push('\n'),
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Tab,
                    ..
                } => buffer.push('\t'),
                // Other events (mouse, resize, focus, arrow keys, etc.) are
                // ignored during paste - they're unlikely to be intentional.
                _ => {}
            }
            PasteStateResult::Absorbed
        }

        // Orphaned end marker (End without Start): emit empty paste.
        (
            PasteCollectionState::Inactive,
            VT100InputEventIR::Paste(VT100PasteModeIR::End),
        ) => PasteStateResult::Emit(InputEvent::BracketedPaste(String::new())),

        // Normal event processing when not pasting.
        (PasteCollectionState::Inactive, _) => {
            match convert_input_event(vt100_event.clone()) {
                Some(event) => PasteStateResult::Emit(event),
                None => PasteStateResult::Absorbed,
            }
        }
    }
}

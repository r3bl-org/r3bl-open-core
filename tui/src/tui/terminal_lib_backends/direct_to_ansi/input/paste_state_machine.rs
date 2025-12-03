// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Paste state machine for collecting bracketed paste text.
//!
//! This module handles the state transitions for bracketed paste mode, accumulating
//! text between `Paste(Start)` and `Paste(End)` markers.

use super::{protocol_conversion::convert_input_event, types::{PasteAction, PasteCollectionState}};
use crate::{InputEvent,
            core::ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                       VT100KeyCodeIR,
                                                       VT100PasteModeIR}};

/// Applies the paste collection state machine to a parsed VT100 event.
///
/// Returns [`PasteAction::Emit`] if the event should be emitted to the caller,
/// or [`PasteAction::Continue`] if the event was absorbed (paste in progress).
pub fn apply_paste_state_machine(
    paste_state: &mut PasteCollectionState,
    vt100_event: &VT100InputEventIR,
) -> PasteAction {
    match (paste_state, vt100_event) {
        // Start marker: enter collecting state, don't emit event.
        (
            state @ PasteCollectionState::Inactive,
            VT100InputEventIR::Paste(VT100PasteModeIR::Start),
        ) => {
            *state = PasteCollectionState::Accumulating(String::new());
            PasteAction::Continue
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
            PasteAction::Emit(InputEvent::BracketedPaste(text))
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
            PasteAction::Continue
        }

        // Orphaned end marker (End without Start): emit empty paste.
        (
            PasteCollectionState::Inactive,
            VT100InputEventIR::Paste(VT100PasteModeIR::End),
        ) => PasteAction::Emit(InputEvent::BracketedPaste(String::new())),

        // Normal event processing when not pasting.
        (PasteCollectionState::Inactive, _) => {
            match convert_input_event(vt100_event.clone()) {
                Some(event) => PasteAction::Emit(event),
                None => PasteAction::Continue,
            }
        }
    }
}

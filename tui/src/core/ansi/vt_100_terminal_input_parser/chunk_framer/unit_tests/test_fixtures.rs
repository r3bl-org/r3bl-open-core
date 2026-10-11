// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared test helpers, constants, and imports for [`ChunkFramer`] tests.
//!
//! [`ChunkFramer`]: super::super::ChunkFramer

pub use super::super::*;
pub use crate::{ANSI_BEL, ANSI_ESC, ANSI_ST_7BIT_TRANSPORT_ENCODING, ANSI_ST_FINAL,
                ASCII_DEL, CONTROL_ENTER, CONTROL_TAB, CSI_PREFIX, KeyState, LINE_FEED,
                MAX_OSC_DRAIN_BYTES, MAX_OSC_SEQUENCE_LENGTH, MODIFIER_CTRL,
                MODIFIER_SHIFT, OSC_PREFIX, RgbValue, SPECIAL_DELETE_CODE,
                SPECIAL_END_FINAL, SPECIAL_HOME_FINAL, SPECIAL_INSERT_CODE,
                SPECIAL_PAGE_DOWN_CODE, SPECIAL_PAGE_UP_CODE, byte_offset,
                core::ansi::{generator::{SEQ_ARROW_DOWN, SEQ_ARROW_LEFT,
                                         SEQ_ARROW_RIGHT, SEQ_ARROW_UP, SEQ_END,
                                         SEQ_HOME, csi_modified, csi_tilde, ss3},
                             vt_100_terminal_input_parser::{MaybeMore,
                                                            TerminalColorReport,
                                                            TerminalColorRole,
                                                            VT100InputEventIR,
                                                            VT100KeyCodeIR,
                                                            VT100KeyModifiersIR,
                                                            chunk_framer::OscCircuitBreaker}}};

/// Helper to create a keyboard event for assertions.
#[must_use]
pub fn keyboard_event(code: VT100KeyCodeIR) -> VT100InputEventIR {
    VT100InputEventIR::Keyboard {
        code,
        modifiers: VT100KeyModifiersIR::default(),
    }
}

/// Helper to create a keyboard event with modifiers for assertions.
#[must_use]
pub fn keyboard_event_with_modifiers(
    code: VT100KeyCodeIR,
    modifiers: VT100KeyModifiersIR,
) -> VT100InputEventIR {
    VT100InputEventIR::Keyboard { code, modifiers }
}

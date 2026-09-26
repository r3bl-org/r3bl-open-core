// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Modifier parameter extraction and decoding for [`CSI`] sequences.
//!
//! [`CSI`]: crate::CsiSequence

use super::super::ir_event_types::VT100KeyModifiersIR;
use crate::{KeyState, NarrowingCastToU8, WideningCastToU16,
            core::ansi::constants::{MODIFIER_ALT, MODIFIER_CTRL, MODIFIER_NONE,
                                    MODIFIER_PARAMETER_OFFSET, MODIFIER_SHIFT}};

/// Extracts modifier parameter from [`CSI`] with type safety.
///
/// Safe to cast u16→u8 because [`VT-100`] modifiers are always 1-8.
///
/// [`CSI`]: crate::CsiSequence
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[must_use]
pub fn extract_modifier_parameter(param: u16) -> u8 {
    debug_assert!(
        param <= u8::MAX.as_u16_widening(),
        "Modifier parameter out of range: {param}"
    );
    param.as_u8_narrowing()
}

/// Decode [`CSI`] modifier parameter (1-8) to [`VT100KeyModifiersIR`].
///
/// [`CSI`] encoding: param = 1 + bitfield, where bitfield = Shift(1)|Alt(2)|Ctrl(4).
/// See module docs [`Modifier Encoding`] for full table.
///
/// [`CSI`]: crate::CsiSequence
/// [`Modifier Encoding`]: mod@super#how-bitmask-encoding-for-modifiers-works
#[must_use]
pub fn decode_modifiers(modifier_mask: u8) -> VT100KeyModifiersIR {
    // Subtract offset to get the bitfield (CSI parameter = 1 + bitfield).
    let bits = modifier_mask.saturating_sub(MODIFIER_PARAMETER_OFFSET);

    // Fast path: if no modifiers, return default (all NotPressed).
    if bits == MODIFIER_NONE {
        return VT100KeyModifiersIR::default();
    }

    VT100KeyModifiersIR {
        shift: if (bits & MODIFIER_SHIFT) == MODIFIER_NONE {
            KeyState::NotPressed
        } else {
            KeyState::Pressed
        },
        alt: if (bits & MODIFIER_ALT) == MODIFIER_NONE {
            KeyState::NotPressed
        } else {
            KeyState::Pressed
        },
        ctrl: if (bits & MODIFIER_CTRL) == MODIFIER_NONE {
            KeyState::NotPressed
        } else {
            KeyState::Pressed
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_modifier_parameter() {
        assert_eq!(extract_modifier_parameter(1), 1);
        assert_eq!(extract_modifier_parameter(2), 2);
        assert_eq!(extract_modifier_parameter(8), 8);
    }

    #[test]
    fn test_decode_modifiers() {
        assert_eq!(decode_modifiers(1), VT100KeyModifiersIR::default());
        assert_eq!(
            decode_modifiers(2),
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::NotPressed,
            }
        );
        assert_eq!(
            decode_modifiers(3),
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::NotPressed,
            }
        );
        assert_eq!(
            decode_modifiers(5),
            VT100KeyModifiersIR {
                shift: KeyState::NotPressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
            }
        );
        assert_eq!(
            decode_modifiers(6),
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                alt: KeyState::NotPressed,
                ctrl: KeyState::Pressed,
            }
        );
        assert_eq!(
            decode_modifiers(8),
            VT100KeyModifiersIR {
                shift: KeyState::Pressed,
                alt: KeyState::Pressed,
                ctrl: KeyState::Pressed,
            }
        );
    }
}

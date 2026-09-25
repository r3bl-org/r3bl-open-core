// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Modifier parameter extraction and decoding for [`CSI`] sequences.
//!
//! [`CSI`]: crate::CsiSequence

use super::super::super::ir_event_types::VT100KeyModifiersIR;
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
    use crate::core::ansi::constants::{MODIFIER_ALT, MODIFIER_ALT_SHIFT, MODIFIER_CTRL,
                                       MODIFIER_CTRL_ALT, MODIFIER_CTRL_ALT_SHIFT,
                                       MODIFIER_CTRL_SHIFT, MODIFIER_NONE,
                                       MODIFIER_PARAMETER_OFFSET, MODIFIER_SHIFT};

    // ==================== CSI Modifier Wire Parameters ====================
    //
    // Wire parameter values in `CSI 1 ; <parameter> <final>`: `parameter = 1 + bitfield`.

    /// Modifier Wire Parameter: No Modifiers ([`ANSI`]).
    ///
    /// Value: `1` dec.
    ///
    /// Encoding: `CSI 1 ; 1 X` (or omitted).
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    const MODIFIER_PARAMETER_NONE: u8 = MODIFIER_NONE + MODIFIER_PARAMETER_OFFSET;

    /// Modifier Wire Parameter: Shift ([`ANSI`]).
    ///
    /// Value: `2` dec.
    ///
    /// Encoding: `CSI 1 ; 2 X`.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    const MODIFIER_PARAMETER_SHIFT: u8 = MODIFIER_SHIFT + MODIFIER_PARAMETER_OFFSET;

    /// Modifier Wire Parameter: Alt ([`ANSI`]).
    ///
    /// Value: `3` dec.
    ///
    /// Encoding: `CSI 1 ; 3 X`.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    const MODIFIER_PARAMETER_ALT: u8 = MODIFIER_ALT + MODIFIER_PARAMETER_OFFSET;

    /// Modifier Wire Parameter: Alt+Shift ([`ANSI`]).
    ///
    /// Value: `4` dec.
    ///
    /// Encoding: `CSI 1 ; 4 X`.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    const MODIFIER_PARAMETER_ALT_SHIFT: u8 =
        MODIFIER_ALT_SHIFT + MODIFIER_PARAMETER_OFFSET;

    /// Modifier Wire Parameter: Ctrl ([`ANSI`]).
    ///
    /// Value: `5` dec.
    ///
    /// Encoding: `CSI 1 ; 5 X`.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    const MODIFIER_PARAMETER_CTRL: u8 = MODIFIER_CTRL + MODIFIER_PARAMETER_OFFSET;

    /// Modifier Wire Parameter: Ctrl+Shift ([`ANSI`]).
    ///
    /// Value: `6` dec.
    ///
    /// Encoding: `CSI 1 ; 6 X`.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    const MODIFIER_PARAMETER_CTRL_SHIFT: u8 =
        MODIFIER_CTRL_SHIFT + MODIFIER_PARAMETER_OFFSET;

    /// Modifier Wire Parameter: Ctrl+Alt ([`ANSI`]).
    ///
    /// Value: `7` dec.
    ///
    /// Encoding: `CSI 1 ; 7 X`.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    const MODIFIER_PARAMETER_CTRL_ALT: u8 = MODIFIER_CTRL_ALT + MODIFIER_PARAMETER_OFFSET;

    /// Modifier Wire Parameter: Ctrl+Alt+Shift ([`ANSI`]).
    ///
    /// Value: `8` dec.
    ///
    /// Encoding: `CSI 1 ; 8 X`.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    const MODIFIER_PARAMETER_CTRL_ALT_SHIFT: u8 =
        MODIFIER_CTRL_ALT_SHIFT + MODIFIER_PARAMETER_OFFSET;

    /// Minimum valid modifier parameter value (`1`).
    const MODIFIER_PARAMETER_MIN: u8 = MODIFIER_PARAMETER_NONE;

    /// Maximum valid modifier parameter value (`8`).
    const MODIFIER_PARAMETER_MAX: u8 = MODIFIER_PARAMETER_CTRL_ALT_SHIFT;

    #[test]
    fn test_modifier_parameter_constants() {
        assert_eq!(MODIFIER_PARAMETER_NONE, 1);
        assert_eq!(MODIFIER_PARAMETER_SHIFT, 2);
        assert_eq!(MODIFIER_PARAMETER_ALT, 3);
        assert_eq!(MODIFIER_PARAMETER_ALT_SHIFT, 4);
        assert_eq!(MODIFIER_PARAMETER_CTRL, 5);
        assert_eq!(MODIFIER_PARAMETER_CTRL_SHIFT, 6);
        assert_eq!(MODIFIER_PARAMETER_CTRL_ALT, 7);
        assert_eq!(MODIFIER_PARAMETER_CTRL_ALT_SHIFT, 8);
        assert_eq!(MODIFIER_PARAMETER_MIN, 1);
        assert_eq!(MODIFIER_PARAMETER_MAX, 8);
    }

    #[test]
    fn test_extract_modifier_parameter() {
        assert_eq!(
            extract_modifier_parameter(MODIFIER_PARAMETER_MIN.as_u16_widening()),
            MODIFIER_PARAMETER_MIN
        );
        assert_eq!(
            extract_modifier_parameter(MODIFIER_PARAMETER_SHIFT.as_u16_widening()),
            MODIFIER_PARAMETER_SHIFT
        );
        assert_eq!(
            extract_modifier_parameter(MODIFIER_PARAMETER_MAX.as_u16_widening()),
            MODIFIER_PARAMETER_MAX
        );
    }

    #[test]
    fn test_decode_modifiers() {
        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_NONE),
            VT100KeyModifiersIR::NONE
        );
        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_NONE).to_csi_parameter(),
            MODIFIER_PARAMETER_NONE
        );

        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_SHIFT),
            VT100KeyModifiersIR::SHIFT
        );
        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_SHIFT).to_csi_parameter(),
            MODIFIER_PARAMETER_SHIFT
        );

        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_ALT),
            VT100KeyModifiersIR::ALT
        );
        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_ALT).to_csi_parameter(),
            MODIFIER_PARAMETER_ALT
        );

        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_CTRL),
            VT100KeyModifiersIR::CTRL
        );
        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_CTRL).to_csi_parameter(),
            MODIFIER_PARAMETER_CTRL
        );

        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_CTRL_SHIFT),
            VT100KeyModifiersIR::CTRL.with_shift()
        );
        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_CTRL_SHIFT).to_csi_parameter(),
            MODIFIER_PARAMETER_CTRL_SHIFT
        );

        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_CTRL_ALT_SHIFT),
            VT100KeyModifiersIR::CTRL.with_shift().with_alt()
        );
        assert_eq!(
            decode_modifiers(MODIFIER_PARAMETER_CTRL_ALT_SHIFT).to_csi_parameter(),
            MODIFIER_PARAMETER_CTRL_ALT_SHIFT
        );
    }
}

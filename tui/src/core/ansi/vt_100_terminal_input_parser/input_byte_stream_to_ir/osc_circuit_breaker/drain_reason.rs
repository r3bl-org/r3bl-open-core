// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::Display;

/// Reason specifying the exact condition, termination, or transition that occurred
/// during an [`OSC`] drain operation in [`OscCircuitBreaker`].
///
/// Each variant represents a distinct parsing outcome (clean termination by `BEL`/`ST`,
/// syntax abort by raw newline or new escape sequence, safety ceiling overflow,
/// or ongoing runaway payload consumption).
///
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`OscCircuitBreaker`]: super::OscCircuitBreaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscDrainReason {
    /// Terminated cleanly by [`ANSI_BEL`] (`0x07`).
    ///
    /// [`ANSI_BEL`]: crate::ANSI_BEL
    TerminatedByBel,

    /// Terminated cleanly by 7-bit [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] (`ESC \`,
    /// `0x1B 0x5C`).
    ///
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
    TerminatedBySt,

    /// Terminated cleanly by the final byte (`\`) of a 7-bit
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`] sequence (`ESC \`) whose initial
    /// [`ANSI_ESC`] arrived at the end of the previous chunk.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    /// [`ANSI_ST_7BIT_TRANSPORT_ENCODING`]: crate::ANSI_ST_7BIT_TRANSPORT_ENCODING
    TerminatedAcrossBoundary,

    /// Aborted by an [`ANSI_ESC`] followed by a non-backslash character (starting a
    /// new escape sequence). Draining stopped before the [`ANSI_ESC`].
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    AbortedByNewEsc,

    /// Aborted by a raw newline ([`CARRIAGE_RETURN`] or [`LINE_FEED`]). Draining
    /// stopped before the newline character.
    ///
    /// [`CARRIAGE_RETURN`]: crate::CARRIAGE_RETURN
    /// [`LINE_FEED`]: crate::LINE_FEED
    AbortedByNewline,

    /// Aborted because total drained bytes reached [`MAX_OSC_DRAIN_BYTES`] safety
    /// ceiling.
    ///
    /// [`MAX_OSC_DRAIN_BYTES`]: crate::MAX_OSC_DRAIN_BYTES
    ExceededSafetyCeiling,

    /// The entire chunk was consumed as runaway [`OSC`] payload. The circuit breaker
    /// remains in [`OscCircuitBreaker::Open`].
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`OscCircuitBreaker::Open`]: super::OscCircuitBreaker::Open
    RunawayPayloadOngoing,

    /// The chunk ended with a lone [`ANSI_ESC`] (`0x1B`). The byte was consumed, and
    /// the circuit breaker transitions to [`OscCircuitBreaker::OpenAwaitingSt`]
    /// waiting for the next chunk.
    ///
    /// [`ANSI_ESC`]: crate::ANSI_ESC
    /// [`OscCircuitBreaker::OpenAwaitingSt`]: super::OscCircuitBreaker::OpenAwaitingSt
    LoneEscAtBoundary,
}

impl Display for OscDrainReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::TerminatedByBel => "runaway OSC terminated by BEL",
            Self::TerminatedBySt => "runaway OSC terminated by ST",
            Self::TerminatedAcrossBoundary => {
                "runaway OSC terminated by ST across boundary"
            }
            Self::AbortedByNewEsc => "runaway OSC aborted by new ESC sequence",
            Self::AbortedByNewline => "runaway OSC aborted by raw newline",
            Self::ExceededSafetyCeiling => {
                "runaway OSC exceeded MAX_OSC_DRAIN_BYTES safety ceiling"
            }
            Self::RunawayPayloadOngoing => "runaway OSC chunk fully consumed",
            Self::LoneEscAtBoundary => "runaway OSC chunk ended in lone ESC",
        };
        f.write_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_drain_reason_display() {
        assert_eq!(
            format!("{}", OscDrainReason::TerminatedByBel),
            "runaway OSC terminated by BEL"
        );
        assert_eq!(
            format!("{}", OscDrainReason::TerminatedBySt),
            "runaway OSC terminated by ST"
        );
        assert_eq!(
            format!("{}", OscDrainReason::TerminatedAcrossBoundary),
            "runaway OSC terminated by ST across boundary"
        );
        assert_eq!(
            format!("{}", OscDrainReason::AbortedByNewEsc),
            "runaway OSC aborted by new ESC sequence"
        );
        assert_eq!(
            format!("{}", OscDrainReason::AbortedByNewline),
            "runaway OSC aborted by raw newline"
        );
        assert_eq!(
            format!("{}", OscDrainReason::ExceededSafetyCeiling),
            "runaway OSC exceeded MAX_OSC_DRAIN_BYTES safety ceiling"
        );
        assert_eq!(
            format!("{}", OscDrainReason::RunawayPayloadOngoing),
            "runaway OSC chunk fully consumed"
        );
        assert_eq!(
            format!("{}", OscDrainReason::LoneEscAtBoundary),
            "runaway OSC chunk ended in lone ESC"
        );
    }
}

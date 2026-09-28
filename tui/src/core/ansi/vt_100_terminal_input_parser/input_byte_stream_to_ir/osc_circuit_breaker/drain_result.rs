// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::OscDrainReason;
use crate::{ByteOffset, byte_offset};

/// Result of an [`OSC`] drain operation, categorizing whether the chunk was
/// fully or partially consumed, or passed through intact.
///
/// # Coordinate Semantics
///
/// In [`OscDrainResult::PartiallyDrained`], `bytes_consumed` is typed as [`ByteOffset`]
/// because it designates **scanner cursor displacement** into the current chunk slice:
/// - In [`OscDrainResult::NotDrained`], zero bytes were consumed because the circuit
///   breaker was closed.
/// - In [`OscDrainResult::FullyDrained`], the entire chunk slice was consumed by the
///   circuit breaker, leaving zero remaining bytes for normal input parsing.
/// - In [`OscDrainResult::PartiallyDrained`], the displacement serves as the slicing
///   offset where the stream cursor stopped draining and where normal input parsing must
///   resume (`&chunk[*bytes_consumed..]`).
///
/// [`ByteOffset`]: crate::ByteOffset
/// [`OSC`]: crate::osc_codes::OscSequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscDrainResult<'a> {
    /// Zero bytes were consumed by the circuit breaker (circuit breaker was closed). The
    /// entire chunk must be processed as normal input.
    NotDrained(&'a [u8]),

    /// The entire chunk was consumed by the circuit breaker. There are zero remaining
    /// bytes for normal input parsing.
    FullyDrained { reason: OscDrainReason },

    /// The chunk was partially consumed. Remaining bytes starting at `bytes_consumed`
    /// must be processed as normal input.
    PartiallyDrained {
        undrained_bytes: &'a [u8],
        bytes_consumed: ByteOffset,
        reason: OscDrainReason,
    },
}

impl<'a> OscDrainResult<'a> {
    /// Creates an [`OscDrainResult`] classifying a drain operation as
    /// [`OscDrainResult::FullyDrained`] if `bytes_consumed == chunk.len()`, or
    /// [`OscDrainResult::PartiallyDrained`] otherwise.
    ///
    /// # Arguments
    ///
    /// - `chunk`: Incoming byte slice to classify as fully or partially drained.
    /// - `reason`: Specific [`OscDrainReason`] identifying the drain, termination, or
    ///   abort outcome.
    /// - `bytes_consumed`: Scanner cursor displacement ([`ByteOffset`]) into `chunk`
    ///   consumed by the circuit breaker.
    ///
    /// [`ByteOffset`]: crate::ByteOffset
    /// [`OscDrainReason`]: OscDrainReason
    /// [`OscDrainResult::FullyDrained`]: OscDrainResult::FullyDrained
    /// [`OscDrainResult::PartiallyDrained`]: OscDrainResult::PartiallyDrained
    /// [`OscDrainResult`]: OscDrainResult
    #[must_use]
    pub fn classify_drain(
        chunk: &'a [u8],
        reason: OscDrainReason,
        bytes_consumed: ByteOffset,
    ) -> Self {
        let chunk_len = byte_offset(chunk.len());
        if bytes_consumed == chunk_len {
            Self::FullyDrained { reason }
        } else {
            Self::PartiallyDrained {
                undrained_bytes: &chunk[bytes_consumed.as_usize()..],
                bytes_consumed,
                reason,
            }
        }
    }

    /// Returns the sub-slice of `chunk` that was not consumed by the circuit breaker.
    ///
    /// - [`Self::NotDrained`]: Returns `chunk` intact.
    /// - [`Self::FullyDrained`]: Returns an empty slice `&[]`.
    /// - [`Self::PartiallyDrained`]: Returns `undrained_bytes`.
    #[must_use]
    pub fn undrained_bytes(&self) -> &'a [u8] {
        match *self {
            Self::NotDrained(chunk) => chunk,
            Self::FullyDrained { .. } => &[],
            Self::PartiallyDrained {
                undrained_bytes, ..
            } => undrained_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_drain_result_classify_drain() {
        let chunk = b"12345";
        let full = OscDrainResult::classify_drain(
            chunk,
            OscDrainReason::RunawayPayloadOngoing,
            byte_offset(5),
        );
        assert_eq!(
            full,
            OscDrainResult::FullyDrained {
                reason: OscDrainReason::RunawayPayloadOngoing,
            }
        );
        assert_eq!(full.undrained_bytes(), b"");

        let partial = OscDrainResult::classify_drain(
            chunk,
            OscDrainReason::TerminatedByBel,
            byte_offset(3),
        );
        assert_eq!(
            partial,
            OscDrainResult::PartiallyDrained {
                undrained_bytes: b"45",
                bytes_consumed: byte_offset(3),
                reason: OscDrainReason::TerminatedByBel,
            }
        );
        assert_eq!(partial.undrained_bytes(), b"45");
    }

    #[test]
    fn test_osc_drain_result_undrained_bytes() {
        let chunk = b"abcdef";

        // NotDrained: returns entire chunk.
        assert_eq!(
            OscDrainResult::NotDrained(chunk).undrained_bytes(),
            b"abcdef"
        );

        // FullyDrained: returns empty slice.
        let full = OscDrainResult::FullyDrained {
            reason: OscDrainReason::RunawayPayloadOngoing,
        };
        assert_eq!(full.undrained_bytes(), b"");

        // PartiallyDrained: returns slice starting at bytes_consumed.
        let partial = OscDrainResult::PartiallyDrained {
            undrained_bytes: b"cdef",
            bytes_consumed: byte_offset(2),
            reason: OscDrainReason::TerminatedByBel,
        };
        assert_eq!(partial.undrained_bytes(), b"cdef");
    }
}

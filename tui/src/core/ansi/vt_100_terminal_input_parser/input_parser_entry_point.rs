// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{chunk_decoder, ir_event_types::ParsedInputEventIR, maybe_more::MaybeMore};

/// Pure, zero-allocation sequence decoder that parses a single input event from a byte
/// slice.
///
/// 1. Delegates directly to [`chunk_decoder::try_decode_input_event()`]. See
///    [`chunk_decoder`] for the full dispatch pipeline and grammar specifications.
///
/// 2. For the parser architecture, mental model, and framing layer, see the
///    [`vt_100_terminal_input_parser`] parent module.
///
/// [`chunk_decoder::try_decode_input_event()`]: super::chunk_decoder::try_decode_input_event
/// [`chunk_decoder`]: super::chunk_decoder
/// [`vt_100_terminal_input_parser`]: mod@super
#[inline]
#[must_use]
pub fn try_parse_input_event(
    buffer: &[u8],
    maybe_more: MaybeMore,
) -> Option<ParsedInputEventIR> {
    chunk_decoder::try_decode_input_event(buffer, maybe_more)
}

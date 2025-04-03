/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use miette::IntoDiagnostic;
use r3bl_core::{AnsiStyledText,
                InlineVec,
                InputDevice,
                LineStateControlSignal,
                OutputDevice,
                SharedWriter,
                Size};

use super::StyleSheet;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum HowToChoose {
    #[default]
    Single,
    Multiple,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Chosen<'a> {
    /// Use selected nothing.
    None,
    /// User selected 1 item. The index of the item in the original list and the item itself.
    One(usize, &'a str),
    /// User selected multiple items. The indices of the items in the original list and the items themselves.
    Many(InlineVec<usize>, &'a [&'a str]),
}

#[derive(Debug, Clone, PartialEq, Default)]
#[allow(clippy::large_enum_variant)]
pub enum Header<'a> {
    /// No header.
    #[default]
    None,
    /// Single line header.
    SingleLine(AnsiStyledText<'a>),
    /// Multi line header.
    MultiLine(InlineVec<InlineVec<AnsiStyledText<'a>>>),
}

// 00: add State (migrate tuify::State)

/// Choose an item from a list of items.
///
/// # Arguments
///
/// - `header`: The header to display above the list of items.
/// - `from`: A slice of strings to choose from.
/// - `how`: How to choose the item(s).
/// - `io`: A tuple of the output and input devices to use.
///     - `0`: The output device to use.
///     - `1`: The input device to use.
/// - `maybe_shared_writer`: An optional shared writer, if [super::ReadlineAsync] is in
///   use when this function is called. This is provided for compatibility with
///   [super::ReadlineAsync]. If passed:
///     - It will be paused while the user is choosing and item, and will be resumed after
///       the user has made their choice.
///     - This is useful for preventing the shared writer from printing while the user is
///       choosing an item, when [super::ReadlineAsync] is in use.
/// - `styling`: A tuple of the header, max size, and style sheet to use.
///     - `0`: The maximum display height and width the list of items can take up.
///       If the height or width is `0` then defaults will be used. If `None` then the
///       defaults will be used.
///     - `1`: The style sheet to use for the list of items.
pub async fn choose<'a>(
    header: Header<'a>,
    from: &'a [&'a str],
    how: HowToChoose,
    io: (&'a mut OutputDevice, &'a mut InputDevice),
    maybe_shared_writer: Option<SharedWriter>,
    styling: (Option<Size>, StyleSheet),
) -> miette::Result<Chosen<'a>> {
    if from.is_empty() {
        return Ok(Chosen::None);
    }

    // For compatibility with ReadlineAsync (if it is in use).
    if let Some(ref shared_writer) = maybe_shared_writer {
        // Pause the shared writer while the user is choosing an item.
        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Pause)
            .await
            .into_diagnostic()?;
    }

    // 00: impl this

    // For compatibility with ReadlineAsync (if it is in use).
    if let Some(ref shared_writer) = maybe_shared_writer {
        // Resume the shared writer after the user has made their choice.
        shared_writer
            .line_state_control_channel_sender
            .send(LineStateControlSignal::Resume)
            .await
            .into_diagnostic()?;
    }

    // 00: remove this after impl above (just for testing)
    Ok(Chosen::Many(smallvec::smallvec![1], &from[0..1]))
}

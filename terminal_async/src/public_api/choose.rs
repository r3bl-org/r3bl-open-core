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

// 00: define the top level function here (public API)

use miette::IntoDiagnostic;
use r3bl_core::{InlineVec,
                InputDevice,
                LineStateControlSignal,
                OutputDevice,
                SharedWriter};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HowToChoose {
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

// 00: add StyleSheet (migrate tuify::StyleSheet)
// 00: add State (migrate tuify::State)

/// Choose an item from a list of items.
///
/// # Arguments
///
/// - `from`: A slice of strings to choose from.
/// - `how`: How to choose the item(s).
/// - `output_device`: The output device to use.
/// - `input_device`: The input device to use.
/// - `maybe_shared_writer`: An optional shared writer, if [super::ReadlineAsync] is in
///   use when this function is called. This is provided for compatibility with
///   [super::ReadlineAsync]. If passed:
///     - It will be paused while the user is choosing and item, and will be resumed after
///       the user has made their choice.
///     - This is useful for preventing the shared writer from printing while the user is
///       choosing an item, when [super::ReadlineAsync] is in use.
pub async fn choose<'a>(
    from: &'a [&'a str],
    how: HowToChoose,
    output_device: &mut OutputDevice,
    input_device: &mut InputDevice,
    maybe_shared_writer: Option<SharedWriter>,
    // 00: add header: String
    // 00: add max_size: Size
    // 00: stylesheet: StyleSheet
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

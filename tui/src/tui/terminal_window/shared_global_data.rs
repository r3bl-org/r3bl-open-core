/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use std::fmt::{Debug, Formatter};

use tokio::sync::mpsc::Sender;

use super::TerminalWindowMainThreadSignal;
use crate::{ok, spinner_impl, telemetry::telemetry_sizing::TelemetryReportLineStorage,
            ChUnit, CommonResult, InlineString, OffscreenBuffer, OffscreenBufferPool,
            OutputDevice, Size, SpinnerStyle, TelemetryHudReport, DEBUG_TUI_COMPOSITOR,
            DEBUG_TUI_MOD};

/// This is a global data structure that holds state for the entire application
/// [`crate::App`] and the terminal window [`crate::TerminalWindow`] itself.
///
/// # Fields
/// - The `window_size` holds the [Size] of the terminal window.
/// - The `maybe_saved_offscreen_buffer` holds the last rendered [`OffscreenBuffer`].
/// - The `main_thread_channel_sender` is used to send [`TerminalWindowMainThreadSignal`]s
/// - The `state` holds the application's state.
/// - The `output_device` is the terminal's output device (anything that implements
///   [`crate::SafeRawTerminal`] which can be [`std::io::stdout`] or
///   [`crate::SharedWriter`], etc.).
pub struct GlobalData<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    pub window_size: Size,
    pub maybe_saved_offscreen_buffer: Option<OffscreenBuffer>,
    pub main_thread_channel_sender: Sender<TerminalWindowMainThreadSignal<AS>>,
    pub state: S,
    pub output_device: OutputDevice,
    pub offscreen_buffer_pool: OffscreenBufferPool,
    /// Stack allocated string buffer for the HUD report. This is re-used and
    /// pre-allocated to avoid heap allocations.
    pub hud_report: TelemetryReportLineStorage,
    pub spinner_helper: SpinnerHelper,
}

#[derive(Debug, Default)]
pub struct SpinnerHelper {
    pub spinner_style: SpinnerStyle,
    pub count: ChUnit,
    pub empty_message: InlineString,
}

impl<S, AS> Debug for GlobalData<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GlobalData")?;
        write!(f, "\n  - window_size: {:?}", self.window_size)?;
        write!(f, "\n  - ")?;
        match &self.maybe_saved_offscreen_buffer {
            None => write!(f, "no saved offscreen_buffer")?,
            Some(ref offscreen_buffer) => {
                if DEBUG_TUI_COMPOSITOR {
                    write!(f, "{offscreen_buffer:?}")?;
                } else {
                    write!(f, "offscreen_buffer saved from previous render")?;
                }
            }
        }
        ok!()
    }
}

impl<S, AS> GlobalData<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    /// Create a new instance of [`GlobalData`] with the given parameters.
    pub fn try_to_create_instance(
        main_thread_channel_sender: Sender<TerminalWindowMainThreadSignal<AS>>,
        state: S,
        initial_size: Size,
        output_device: OutputDevice,
        offscreen_buffer_pool: OffscreenBufferPool,
    ) -> CommonResult<GlobalData<S, AS>>
    where
        AS: Debug + Default + Clone + Sync + Send,
    {
        let mut it = GlobalData {
            window_size: Size::default(),
            maybe_saved_offscreen_buffer: Option::default(),
            state,
            main_thread_channel_sender,
            output_device,
            offscreen_buffer_pool,
            hud_report: TelemetryReportLineStorage::new(),
            spinner_helper: SpinnerHelper::default(),
        };

        it.set_size(initial_size);

        Ok(it)
    }

    pub fn set_size(&mut self, new_size: Size) {
        self.window_size = new_size;
        DEBUG_TUI_MOD.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = "main_event_loop -> Resize ⇲",
                new_size = ?new_size
            );
        });
    }

    pub fn get_size(&self) -> Size { self.window_size }

    /// Generate display output for the HUD report by writing it to [`Self::hud_report`],
    /// a pre-allocated (and re-used) string buffer [`TelemetryReportLineStorage`],
    /// which is stack allocated.
    ///
    /// Look at the [`std::fmt::Display`] implementation of [`TelemetryHudReport`] for
    /// details on how the report is formatted.
    pub fn set_hud_report(&mut self, new: TelemetryHudReport) {
        use std::fmt::Write as _;
        self.hud_report.clear();
        // We don't care about the result of this operation.
        write!(self.hud_report, "{new}").ok();
    }

    const EMPTY_HUD_REPORT_STATIC: &str = "⮺ Collecting data ⠎";
    const EMPTY_HUD_REPORT_PREFIX_SPINNER: &str = "⮺ Collecting data ";

    /// If [`Self::set_hud_report()`] has not been called, this will return an empty
    /// string with a static message.
    pub fn get_hud_report_no_spinner(&self) -> &str {
        if self.hud_report.is_empty() {
            Self::EMPTY_HUD_REPORT_STATIC
        } else {
            &self.hud_report
        }
    }

    /// If [`Self::set_hud_report()`] has not been called, this will return an empty
    /// string with a "dynamic" message where a spinner glyph changes every time this
    /// method is called.
    pub fn get_hud_report_with_spinner(&mut self) -> &str {
        use std::fmt::Write as _;
        if self.hud_report.is_empty() {
            let count = self.spinner_helper.count;
            let style = &mut self.spinner_helper.spinner_style;
            let spinner_glyph = spinner_impl::spinner_render::get_next_tick_glyph(
                style,
                count.as_usize(),
            );

            self.spinner_helper.empty_message.clear();
            // We don't care about the result of this operation.
            write!(
                self.spinner_helper.empty_message,
                "{a}{b}",
                a = Self::EMPTY_HUD_REPORT_PREFIX_SPINNER,
                b = spinner_glyph,
            ).ok();

            self.spinner_helper.count += 1;

            &self.spinner_helper.empty_message
        } else {
            &self.hud_report
        }
    }
}

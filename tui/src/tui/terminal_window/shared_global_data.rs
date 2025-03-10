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

use r3bl_core::{call_if_true,
                ok,
                sizing::TelemetryReportLineStorage,
                CommonResult,
                OutputDevice,
                Size,
                TelemetryHudReport};
use tokio::sync::mpsc::Sender;

use super::TerminalWindowMainThreadSignal;
use crate::{OffscreenBuffer, OffscreenBufferPool, DEBUG_TUI_COMPOSITOR, DEBUG_TUI_MOD};

/// This is a global data structure that holds state for the entire application
/// [crate::App] and the terminal window [crate::TerminalWindow] itself.
///
/// # Fields
/// - The `window_size` holds the [Size] of the terminal window.
/// - The `maybe_saved_offscreen_buffer` holds the last rendered [OffscreenBuffer].
/// - The `main_thread_channel_sender` is used to send [TerminalWindowMainThreadSignal]s
/// - The `state` holds the application's state.
/// - The `output_device` is the terminal's output device (anything that implements
///   [r3bl_core::SafeRawTerminal] which can be [std::io::stdout] or
///   [r3bl_core::SharedWriter], etc.`).
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
            Some(ref offscreen_buffer) => match DEBUG_TUI_COMPOSITOR {
                false => write!(f, "offscreen_buffer saved from previous render")?,
                true => write!(f, "{:?}", offscreen_buffer)?,
            },
        }
        ok!()
    }
}

impl<S, AS> GlobalData<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    /// Create a new instance of [GlobalData] with the given parameters.
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
            window_size: Default::default(),
            maybe_saved_offscreen_buffer: Default::default(),
            state,
            main_thread_channel_sender,
            output_device,
            offscreen_buffer_pool,
            hud_report: TelemetryReportLineStorage::new(),
        };

        it.set_size(initial_size);

        Ok(it)
    }

    pub fn set_size(&mut self, new_size: Size) {
        self.window_size = new_size;
        call_if_true!(
            DEBUG_TUI_MOD,
            tracing::info!("main_event_loop -> Resize ⇲"=?new_size)
        );
    }

    pub fn get_size(&self) -> Size { self.window_size }

    /// Generate display output for the HUD report by writing it to [Self::hud_report], a
    /// pre-allocated (and re-used) string buffer [TelemetryReportLineStorage], which is
    /// stack allocated.
    ///
    /// Look at the [std::fmt::Display] implementation of [TelemetryHudReport] for details
    /// on how the report is formatted.
    pub fn set_hud_report(&mut self, new: TelemetryHudReport) {
        self.hud_report.clear();
        use std::fmt::Write as _;
        _ = write!(self.hud_report, "{}", new);
    }

    /// If [Self::set_hud_report()] has not been called, this will return an empty string.
    pub fn get_hud_report_as_str(&self) -> &str {
        if self.hud_report.is_empty() {
            "⮺ Collecting data ⠎"
        } else {
            &self.hud_report
        }
    }
}

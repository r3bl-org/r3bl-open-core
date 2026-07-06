// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::TerminalWindowMainThreadSignal;
use crate::{CommonResult, DEBUG_TUI_COMPOSITOR, DEBUG_TUI_MOD, MemoizedLenMap,
            OfsBuf, OfsBufPool, OutputDevice, RenderPipeline, Size,
            SpinnerStyle, TelemetryHudReport, core::glyphs, ok, spinner_impl,
            telemetry::telemetry_sizing::TelemetryReportLineStorage};
use std::{collections::HashMap,
          fmt::{Debug, Formatter}};
use tokio::sync::mpsc::Sender;

/// This is a global data structure that holds state for the entire application
/// [`crate::App`] and the terminal window [`crate::TerminalWindow`] itself.
pub struct GlobalData<S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    /// The [Size] of the terminal window.
    pub window_size: Size,

    /// The last rendered [`OfsBuf`].
    pub maybe_saved_ofs_buf: Option<OfsBuf>,

    /// Channel used to send [`TerminalWindowMainThreadSignal`]s to the main thread.
    pub main_thread_channel_sender: Sender<TerminalWindowMainThreadSignal<AS>>,

    /// Application's state.
    pub state: S,

    /// The terminal's output device (anything that implements [`SafeRawTerminal`] which
    /// can be [`stdout`] or [`SharedWriter`], etc.).
    ///
    /// [`SafeRawTerminal`]: crate::SafeRawTerminal
    /// [`SharedWriter`]: crate::SharedWriter
    /// [`stdout`]: std::io::stdout
    pub output_device: OutputDevice,

    /// Pool for reusing offscreen buffers across frames to avoid allocations.
    pub ofs_buf_pool: OfsBufPool,

    /// Data and animation state for the Heads Up Display (HUD) performance report.
    pub hud_data: HudData,

    /// Memoized text width calculations for styled text (70x speedup for repeated text).
    /// Persists across frames to enable caching of repeated text patterns.
    pub memoized_text_widths: MemoizedLenMap,

    /// Persistent render pipeline. Reused across frames via `.clear()` to retain heap
    /// capacity.
    pub pipeline: RenderPipeline,
}

#[derive(Debug)]
pub struct HudData {
    /// Pre-allocated string buffer for the full report (Zero-allocation)
    text_buffer: TelemetryReportLineStorage,
    /// Style template for the animated spinner
    spinner_style: SpinnerStyle,
    /// Animation frame counter
    tick_count: usize,
}

impl Default for HudData {
    fn default() -> Self {
        Self {
            text_buffer: TelemetryReportLineStorage::new(),
            spinner_style: SpinnerStyle::default(),
            tick_count: 0,
        }
    }
}

mod hud_constants {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub const EMPTY_REPORT_MSG: &str = const_format::formatcp!(
        "⮺ Collecting data - Waiting for your input {ch}",
        ch = glyphs::SMILING_GLYPH
    );
}

impl HudData {
    /// Called by the event loop at the end of every frame to inject metrics
    pub fn set_report(&mut self, new: TelemetryHudReport) {
        use std::fmt::Write as _;

        // Tick the spinner
        self.tick_count = self.tick_count.wrapping_add(1);
        let spinner_glyph = spinner_impl::spinner_render::get_next_tick_glyph_str(
            &self.spinner_style,
            self.tick_count,
        );

        // Zero-allocation write directly to the buffer
        self.text_buffer.clear();
        if !new.is_empty() {
            write!(self.text_buffer, "{spinner_glyph} {new}").ok();
        }
    }

    /// Called by the app to get the formatted text for rendering
    #[must_use]
    pub fn get_report(&self) -> &str {
        if self.text_buffer.is_empty() {
            hud_constants::EMPTY_REPORT_MSG
        } else {
            &self.text_buffer
        }
    }
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
        match &self.maybe_saved_ofs_buf {
            None => write!(f, "no saved ofs_buf")?,
            Some(ofs_buf) => {
                if DEBUG_TUI_COMPOSITOR {
                    write!(f, "{ofs_buf:?}")?;
                } else {
                    write!(f, "ofs_buf saved from previous render")?;
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
    /// Creates a new instance of [`GlobalData`] with the given parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the initial window size update fails.
    pub fn try_to_create_instance(
        main_thread_channel_sender: Sender<TerminalWindowMainThreadSignal<AS>>,
        state: S,
        initial_size: Size,
        output_device: OutputDevice,
        ofs_buf_pool: OfsBufPool,
    ) -> CommonResult<GlobalData<S, AS>>
    where
        AS: Debug + Default + Clone + Sync + Send,
    {
        let mut it = GlobalData {
            window_size: Size::default(),
            maybe_saved_ofs_buf: Option::default(),
            state,
            main_thread_channel_sender,
            output_device,
            ofs_buf_pool,
            hud_data: HudData::default(),
            memoized_text_widths: HashMap::new(),
            pipeline: RenderPipeline::default(),
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
}

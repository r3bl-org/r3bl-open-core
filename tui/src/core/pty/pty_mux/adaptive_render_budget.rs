// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Implements an adaptive render budget to prevent terminal emulator overload.
//!
//! If a terminal emulator is overwhelmed by receiving too many [`ANSI`] escape sequences
//! too quickly, it can cause severe UI lag and flickering. This module implements
//! an asymmetric backoff algorithm (throttling) to dynamically adjust the frame rate
//! (target FPS) based on how quickly the terminal is able to process and flush the
//! output buffer.
//!
//! - **Backpressure Detection**: If flushing the buffer takes too long (exceeds a
//!   threshold), the system assumes the terminal is struggling and applies a heavy
//!   penalty to the render loop, dropping the frame rate.
//! - **Asymmetric Recovery**: Once the terminal catches up, the system slowly restores
//!   the frame rate to its target 60 FPS, ensuring it doesn't immediately overwhelm the
//!   terminal again.
//!
//! The main entry points to this module are the methods on the [`Budget`] struct:
//! - [`should_render()`]: Determines if a frame should be drawn based on the current
//!   budget.
//! - [`mark_start()`]: Records the timestamp before sending data to the terminal.
//! - [`mark_end()`]: Measures elapsed time and executes the asymmetric backoff algorithm.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`mark_end()`]: Budget::mark_end
//! [`mark_start()`]: Budget::mark_start
//! [`should_render()`]: Budget::should_render

use super::ProcessManager;
use crate::DEBUG_TUI_PTY_MUX;
use std::time::{Duration, Instant};

/// Default render speed is ~60 FPS.
pub const DEFAULT_FRAME_DELAY_MS: Duration = Duration::from_millis(16);

/// Slowest render speed is ~10 FPS.
pub const MAX_FRAME_DELAY_MS: Duration = Duration::from_millis(100);

/// Min FPS is uncapped. If the terminal is fast enough, there is no throttling and it
/// will render as fast as possible.
pub const MIN_FRAME_DELAY_MS: Duration = Duration::from_millis(0);

/// Flush taking >5ms indicates pressure.
pub const RENDER_TIME_BACKPRESSURE_THRESHOLD_MS: Duration =
    Duration::from_millis(RENDER_TIME_BASE * 5);

// Asymmetric backoff - penalty is 5 x higher than reward. If we detect backpressure,
// we backoff and hit the brakes hard.
pub const THROTTLE_PENALTY_MS: Duration = Duration::from_millis(RENDER_TIME_BASE * 10);

// Asymmetric backoff - reward is 5 x lower than reward. If we detect flushing is
// getting faster, we are easy in the recovery, by getting back on the gas
// gradually.
pub const RECOVERY_REWARD_MS: Duration = Duration::from_millis(RENDER_TIME_BASE);

const RENDER_TIME_BASE: u64 = 1;

#[derive(Debug)]
pub enum AdaptiveRenderResult {
    Skip,
    Render,
}

#[derive(Debug)]
pub struct Budget {
    pub last_render_time: Instant,
    pub render_cooldown_delay: Duration,
    pub maybe_render_start: Option<Instant>,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            last_render_time: Instant::now(),
            render_cooldown_delay: DEFAULT_FRAME_DELAY_MS,
            maybe_render_start: None,
        }
    }
}

impl Budget {
    /// Decides if we should render this frame based on output and budget.
    pub fn should_render(
        &self,
        process_manager: &mut ProcessManager,
    ) -> AdaptiveRenderResult {
        let process_had_output = process_manager.poll_all_processes();
        if !process_had_output {
            return AdaptiveRenderResult::Skip;
        }
        let time_since_last_render = self.last_render_time.elapsed();
        if time_since_last_render >= self.render_cooldown_delay {
            AdaptiveRenderResult::Render
        } else {
            AdaptiveRenderResult::Skip
        }
    }

    /// Marks the start of a rendering pass. This timestamp is used to measure how
    /// long the rendering operation takes, which informs the adaptive budget
    /// calculation.
    ///
    /// # Panics
    ///
    /// Panics if called twice without an intervening [`mark_end()`] call, enforcing
    /// the strict [`mark_start()`] -> render -> [`mark_end()`] state machine.
    ///
    /// [`mark_end()`]: Self::mark_end
    /// [`mark_start()`]: Self::mark_start
    pub fn mark_start(&mut self) {
        assert!(
            self.maybe_render_start.is_none(),
            "Can't call mark_start() more than once"
        );
        self.maybe_render_start = Some(Instant::now());
    }

    /// Updates the budget based on how long the render actually took.
    ///
    /// # Panics
    ///
    /// Panics if called without a preceding [`mark_start()`] call, enforcing the
    /// strict [`mark_start()`] -> render -> [`mark_end()`] state machine.
    ///
    /// [`mark_end()`]: Self::mark_end
    /// [`mark_start()`]: Self::mark_start
    pub fn mark_end(&mut self) {
        // Mark the end of the render pass. This is how long a render pass took.
        let render_duration = self
            .maybe_render_start
            .take()
            .expect("Can't call mark_end() without calling mark_start() first")
            .elapsed();

        // Mark the current time as the last render time. This will be used in the
        // should_render() method.
        self.last_render_time = Instant::now();

        // Adjust budget dynamically based on detected back pressure. We are
        // implementing asymmetric backoff.
        let backpressure_detected =
            render_duration > RENDER_TIME_BACKPRESSURE_THRESHOLD_MS;
        if backpressure_detected {
            // Penalize render budget for backpressure.
            self.render_cooldown_delay = self
                .render_cooldown_delay
                .saturating_add(THROTTLE_PENALTY_MS)
                .min(MAX_FRAME_DELAY_MS);

            DEBUG_TUI_PTY_MUX.then(|| {
                // % is Display, ? is Debug.
                tracing::info! {
                    message = "Budget::mark_end",
                    info = %format!(
                        "Render took {:?} (> {:?}). Throttling frame delay to {:?}",
                        render_duration,
                        RENDER_TIME_BACKPRESSURE_THRESHOLD_MS,
                        self.render_cooldown_delay
                    )
                };
            });
        } else {
            // Used for logging.
            let old_delay = self.render_cooldown_delay;

            // Reward render budget for smooth rendering.
            self.render_cooldown_delay = self
                .render_cooldown_delay
                .saturating_sub(RECOVERY_REWARD_MS)
                .max(MIN_FRAME_DELAY_MS);

            // Only log recovery if the delay actually changed to avoid spamming
            // the logs
            if old_delay != self.render_cooldown_delay {
                DEBUG_TUI_PTY_MUX.then(|| {
                    // % is Display, ? is Debug.
                    tracing::info! {
                        message = "Budget::mark_end",
                        info = %format!(
                            "Render took {:?}. Recovering frame delay to {:?}",
                            render_duration,
                            self.render_cooldown_delay
                        )
                    };
                });
            }
        }
    }
}

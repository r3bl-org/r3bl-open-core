/*
 *   Copyright (c) 2023 R3BL LLC
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

use std::sync::atomic::{AtomicI64, Ordering};

use chrono::Utc;

use r3bl_ansi_color::ColorSupport;
const NOT_SET_VALUE: i64 = -1;

/// This module contains static global data that is meant to be used by the entire application. It
/// also provides functions to manipulate this data.
///
/// ### Color support
/// The app can override the color support detection heuristics by providing a [ColorSupport] value.
/// It is not always possible to accurately detect the color support of the terminal. So this gives
/// the app a way to set it to whatever the user wants (for example).
pub mod telemetry_global_static {
    use super::*;

    // Time related.

    /// Time unit is microseconds.
    pub static mut START_TS_MICROS: AtomicI64 = AtomicI64::new(NOT_SET_VALUE);
    /// Time unit is microseconds.
    pub static mut END_TS_MICROS: AtomicI64 = AtomicI64::new(NOT_SET_VALUE);
    /// Time unit is microseconds.
    pub static mut AVG_RESPONSE_TIME_MICROS: AtomicI64 = AtomicI64::new(NOT_SET_VALUE);

    /// Save the current time to the static mutable variable [START_TS_MICROS].
    pub fn set_start_ts() {
        let current_ts_ms = Utc::now().timestamp_micros();
        unsafe {
            START_TS_MICROS.store(current_ts_ms, Ordering::SeqCst);
        };
    }

    /// Get the saved time from the static mutable variable [START_TS_MICROS]. In order for this to
    /// return [Some] value, you must have already called [set_start_ts].
    fn get_start_ts() -> Option<i64> {
        let start_ts_ms = unsafe { START_TS_MICROS.load(Ordering::SeqCst) };
        if start_ts_ms == NOT_SET_VALUE {
            None
        } else {
            Some(start_ts_ms)
        }
    }

    /// Save the current time to the static mutable variable [END_TS_MICROS]. And update the average
    /// response time.
    pub fn set_end_ts() {
        let current_ts_ms = Utc::now().timestamp_micros();
        unsafe {
            END_TS_MICROS.store(current_ts_ms, Ordering::SeqCst);
        };

        if let Some(start_ts) = get_start_ts() {
            let elapsed_ms = current_ts_ms - start_ts;
            let saved_avg_response_time =
                unsafe { AVG_RESPONSE_TIME_MICROS.load(Ordering::SeqCst) };
            if saved_avg_response_time == NOT_SET_VALUE {
                unsafe {
                    AVG_RESPONSE_TIME_MICROS.store(elapsed_ms, Ordering::SeqCst);
                };
            } else {
                let new_avg_response_time = (saved_avg_response_time + elapsed_ms) / 2;
                unsafe {
                    AVG_RESPONSE_TIME_MICROS
                        .store(new_avg_response_time, Ordering::SeqCst);
                };
            }
        }
    }

    /// Get the saved average response time from the static mutable variable
    /// [AVG_RESPONSE_TIME_MICROS]. In order for this to return [Some] value, you must have already
    /// called [set_end_ts].
    pub fn get_avg_response_time_micros() -> String {
        let avg_response_time_micros =
            unsafe { AVG_RESPONSE_TIME_MICROS.load(Ordering::SeqCst) };
        if avg_response_time_micros == NOT_SET_VALUE {
            "Not set.".to_string()
        } else {
            let fps = 1_000_000 / avg_response_time_micros;
            format!("{avg_response_time_micros} μs, {fps} fps")
        }
    }
}

pub mod is_vscode_term_global_static {
    use super::*;

    pub static mut IS_VSCODE_TERM: AtomicI64 = AtomicI64::new(NOT_SET_VALUE);

    fn detect_whether_is_vscode_term_from_env() -> VSCodeTerm {
        let env_key = "TERM_PROGRAM";
        let env_value = match std::env::var(env_key) {
            Ok(value) => value == "vscode",
            _ => false,
        };
        match env_value {
            true => VSCodeTerm::Yes,
            false => VSCodeTerm::No,
        }
    }

    #[derive(Debug, Copy, Clone)]
    pub enum VSCodeTerm {
        Yes,
        No,
    }

    impl From<i64> for VSCodeTerm {
        fn from(value: i64) -> Self {
            match value {
                0 => VSCodeTerm::No,
                1 => VSCodeTerm::Yes,
                _ => VSCodeTerm::No,
            }
        }
    }

    impl From<VSCodeTerm> for i64 {
        fn from(value: VSCodeTerm) -> Self {
            match value {
                VSCodeTerm::No => 0,
                VSCodeTerm::Yes => 1,
            }
        }
    }

    pub fn get_is_vscode_term() -> VSCodeTerm {
        let existing_value = unsafe { IS_VSCODE_TERM.load(Ordering::SeqCst) };

        match existing_value == NOT_SET_VALUE {
            // If not set, then calculate new value, save it, return it.
            true => {
                let vscode_term = detect_whether_is_vscode_term_from_env();
                unsafe {
                    IS_VSCODE_TERM.store(i64::from(vscode_term), Ordering::SeqCst);
                }
                vscode_term
            }

            // Return saved value.
            false => VSCodeTerm::from(existing_value),
        }
    }
}

/// This module contains static global data that is meant to be used by the entire application. It
/// also provides functions to manipulate this data.
///
/// ### Runtime analytics
/// This is a global data structure that is meant to handle (fast) runtime analytics for the entire
/// application.
/// - It is a static unsafe global.
/// - It is also atomic.
/// - This is **not** wrapped in an [Arc](std::sync::Arc) and [Mutex](std::sync::Mutex).
pub mod color_support_global_static {
    use super::*;

    /// Global [ColorSupport] override.
    pub static mut COLOR_SUPPORT_OVERRIDE: AtomicI64 = AtomicI64::new(NOT_SET_VALUE);

    /// Get the saved [ColorSupport] from the static mutable variable [COLOR_SUPPORT_OVERRIDE].
    pub fn get_color_support_override() -> Option<ColorSupport> {
        let color_support_override =
            unsafe { COLOR_SUPPORT_OVERRIDE.load(Ordering::SeqCst) };
        if color_support_override == NOT_SET_VALUE {
            None
        } else {
            match color_support_override {
                0 => Some(ColorSupport::Grayscale),
                1 => Some(ColorSupport::Ansi256),
                2 => Some(ColorSupport::Truecolor),
                _ => None,
            }
        }
    }

    pub fn clear_color_support_override() {
        unsafe {
            COLOR_SUPPORT_OVERRIDE.store(NOT_SET_VALUE, Ordering::SeqCst);
        };
    }

    /// Save the [ColorSupport] to the static mutable variable [COLOR_SUPPORT_OVERRIDE].
    pub fn set_color_support_override(color_support: ColorSupport) {
        let color_support_override = match color_support {
            ColorSupport::Grayscale => 0,
            ColorSupport::Ansi256 => 1,
            ColorSupport::Truecolor => 2,
            ColorSupport::NoColor => todo!(),
            ColorSupport::NotSet => todo!(),
        };
        unsafe {
            COLOR_SUPPORT_OVERRIDE.store(color_support_override, Ordering::SeqCst);
        };
    }
}

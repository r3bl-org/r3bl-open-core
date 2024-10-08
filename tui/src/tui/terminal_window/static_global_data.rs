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

const NOT_SET_VALUE: i64 = -1;

/// This module contains static global data that is meant to be used by the entire
/// application. It also provides functions to manipulate this data.
///
/// # Color support
///
/// The app can override the color support detection heuristics by providing a
/// [r3bl_ansi_color::global_color_support::detect] value. It is not always possible to
/// accurately detect the color support of the terminal. So this gives the app a way to
/// set it to whatever the user wants (for example).
///
/// # Changing atomic ordering
///
/// <https://emschwartz.me/understanding-memory-ordering-in-rust/>
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
    #[allow(static_mut_refs)]
    pub fn set_start_ts() {
        let current_ts_ms = Utc::now().timestamp_micros();
        unsafe {
            START_TS_MICROS.store(current_ts_ms, Ordering::Release);
        };
    }

    /// Get the saved time from the static mutable variable [START_TS_MICROS]. In order for this to
    /// return [Some] value, you must have already called [set_start_ts].
    #[allow(static_mut_refs)]
    fn get_start_ts() -> Option<i64> {
        let start_ts_ms = unsafe { START_TS_MICROS.load(Ordering::Acquire) };
        if start_ts_ms == NOT_SET_VALUE {
            None
        } else {
            Some(start_ts_ms)
        }
    }

    /// Save the current time to the static mutable variable [END_TS_MICROS]. And update the average
    /// response time.
    #[allow(static_mut_refs)]
    pub fn set_end_ts() {
        let current_ts_ms = Utc::now().timestamp_micros();
        unsafe {
            END_TS_MICROS.store(current_ts_ms, Ordering::Release);
        };

        if let Some(start_ts) = get_start_ts() {
            let elapsed_ms = current_ts_ms - start_ts;
            let saved_avg_response_time =
                unsafe { AVG_RESPONSE_TIME_MICROS.load(Ordering::Acquire) };
            if saved_avg_response_time == NOT_SET_VALUE {
                unsafe {
                    AVG_RESPONSE_TIME_MICROS.store(elapsed_ms, Ordering::Release);
                };
            } else {
                let new_avg_response_time = (saved_avg_response_time + elapsed_ms) / 2;
                unsafe {
                    AVG_RESPONSE_TIME_MICROS
                        .store(new_avg_response_time, Ordering::Release);
                };
            }
        }
    }

    /// Get the saved average response time from the static mutable variable
    /// [AVG_RESPONSE_TIME_MICROS]. In order for this to return [Some] value, you must have already
    /// called [set_end_ts].
    #[allow(static_mut_refs)]
    pub fn get_avg_response_time_micros() -> String {
        let avg_response_time_micros =
            unsafe { AVG_RESPONSE_TIME_MICROS.load(Ordering::Acquire) };
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

    #[allow(static_mut_refs)]
    pub fn get_is_vscode_term() -> VSCodeTerm {
        let existing_value = unsafe { IS_VSCODE_TERM.load(Ordering::Acquire) };

        match existing_value == NOT_SET_VALUE {
            // If not set, then calculate new value, save it, return it.
            true => {
                let vscode_term = detect_whether_is_vscode_term_from_env();
                unsafe {
                    IS_VSCODE_TERM.store(i64::from(vscode_term), Ordering::Release);
                }
                vscode_term
            }

            // Return saved value.
            false => VSCodeTerm::from(existing_value),
        }
    }
}

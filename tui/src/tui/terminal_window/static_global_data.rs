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

/// This is a global data structure that is meant to handle (fast) runtime analytics for the entire
/// application.
/// - It is a static unsafe global.
/// - It is also atomic.
/// - This is **not** wrapped in an [Arc](std::sync::Arc) and [Mutex](std::sync::Mutex).
pub mod terminal_window_static_global_data {
    use std::sync::atomic::{AtomicI64, Ordering};

    use chrono::Utc;

    pub const NOT_SET_VALUE: i64 = -1;

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
                    AVG_RESPONSE_TIME_MICROS.store(new_avg_response_time, Ordering::SeqCst);
                };
            }
        }
    }

    /// Get the saved average response time from the static mutable variable
    /// [AVG_RESPONSE_TIME_MICROS]. In order for this to return [Some] value, you must have already
    /// called [set_end_ts].
    pub fn get_avg_response_time_micros() -> String {
        let avg_response_time_micros = unsafe { AVG_RESPONSE_TIME_MICROS.load(Ordering::SeqCst) };
        if avg_response_time_micros == NOT_SET_VALUE {
            "Not set.".to_string()
        } else {
            let fps = 1_000_000 / avg_response_time_micros;
            format!("{avg_response_time_micros} μs, {fps} fps")
        }
    }
}

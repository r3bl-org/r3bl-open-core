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

/// Helper macros just for this module.
/// Check to see if buffer is empty and return early if it is.
#[macro_export]
macro_rules! empty_check_early_return {
    ($arg_buffer: expr, @None) => {
        if $arg_buffer.is_empty() {
            return None;
        }
    };

    ($arg_buffer: expr, @Nothing) => {
        if $arg_buffer.is_empty() {
            return;
        }
    };
}

/// Check to see if multiline mode is disabled and return early if it is.
#[macro_export]
macro_rules! multiline_disabled_check_early_return {
    ($arg_engine: expr, @None) => {
        if let $crate::LineMode::SingleLine = $arg_engine.config_options.multiline_mode {
            return None;
        }
    };

    ($arg_engine: expr, @Nothing) => {
        if let $crate::LineMode::SingleLine = $arg_engine.config_options.multiline_mode {
            return;
        }
    };
}

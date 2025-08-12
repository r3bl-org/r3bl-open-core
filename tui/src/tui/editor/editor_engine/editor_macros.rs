// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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

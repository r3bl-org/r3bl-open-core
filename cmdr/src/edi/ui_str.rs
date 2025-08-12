// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{InlineString, inline_string};

use crate::get_self_bin_name;

#[must_use]
pub fn multiple_files_not_supported_yet() -> InlineString {
    inline_string!(
        "{} currently only allows you to edit one file at a time. Select one:",
        get_self_bin_name()
    )
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::upgrade_bin_names;
use r3bl_tui::{InlineString, inline_string};
use std::env::current_exe;

/// Returns the filename of the currently running executable (without directory path).
#[must_use]
pub fn get_current_bin_name() -> InlineString {
    let try_resolve_name = || -> Option<InlineString> {
        let path_buf = current_exe().ok()?;
        let f_name = path_buf.file_name()?.to_str()?;
        Some(inline_string!("{f_name}"))
    };

    try_resolve_name().unwrap_or_else(|| {
        inline_string!("{}", upgrade_bin_names::DEFAULT_FALLBACK_BIN_NAME)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current_bin_name_is_non_empty() {
        let bin_name = get_current_bin_name();
        assert!(!bin_name.is_empty());
    }
}

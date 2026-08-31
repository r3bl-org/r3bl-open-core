// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

mod test_golden_formatters;
#[cfg(unix)]
mod test_subshell_unix;
#[cfg(windows)]
mod test_subshell_windows;

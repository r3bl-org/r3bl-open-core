// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::ChUnitPrimitiveType;

/// The number of lines to scroll back or forward when a mouse wheel event is received.
/// This is typically 3 lines to match standard operating system defaults.
pub const MOUSE_SCROLL_BY_AMOUNT: usize = 3;

/// The fraction of the viewport height that the page up or down scroll back or forwards
/// action scrolls. If you use `1` that will just use the full viewport height, `2` is
/// half, `3` is one third.
pub const PAGE_UP_OR_DOWN_SCROLL_BY_FACTOR: ChUnitPrimitiveType = 3;

/// The interval in milliseconds at which the status bar at the bottom of the terminal is
/// forced to re-render to update the visual state.
pub const STATUS_BAR_UPDATE_INTERVAL_MS: u64 = 500;

/// The interval in milliseconds at which the multiplexer wakes up to poll for new output
/// from the active pseudoterminal process.
pub const OUTPUT_POLL_INTERVAL_MS: u64 = 10;

/// The height in rows reserved at the bottom of the terminal for the multiplexer status
/// bar. This height is subtracted from the usable space for child processes.
pub const STATUS_BAR_HEIGHT: u16 = 1;

/// The maximum number of child processes the multiplexer can handle simultaneously. This
/// is set to 9 to cleanly map to the F1 through F9 keys for switching tabs.
pub const MAX_PROCESSES: usize = 9;

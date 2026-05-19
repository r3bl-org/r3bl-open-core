// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::tui::DEBUG_TUI_SHOW_PTY_MUX_NOTIFICATIONS;
use notify_rust::{Notification, Timeout};

/// Default notification display duration.
pub const NOTIFICATION_TIMEOUT_MS: u32 = 1_000;

/// Shows a desktop notification with error handling.
///
/// This does not block the calling thread, which makes it safe to call this function from
/// the thread running the main event loop.
///
/// To show the notification we make a synchronous blocking OS API calls via [`show()`].
///
/// [`show()`]: notify_rust::Notification::show
pub fn show_notification_non_blocking(title: &str, message: &str) {
    if !DEBUG_TUI_SHOW_PTY_MUX_NOTIFICATIONS {
        return;
    }

    // Don't block calling thread since this is synchronous blocking OS API call.
    std::thread::spawn({
        let title_owned = title.to_string();
        let message_owned = message.to_string();
        move || {
            if let Err(e) = Notification::new()
                .summary(&title_owned)
                .body(&message_owned)
                .timeout(Timeout::Milliseconds(NOTIFICATION_TIMEOUT_MS))
                .show()
            {
                tracing::warn!("Failed to show notification '{}': {}", title_owned, e);
            }
        }
    });
}

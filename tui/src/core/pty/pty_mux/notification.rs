// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use notify_rust::Notification;

/// Default notification display duration.
pub const NOTIFICATION_TIMEOUT_MS: u32 = 1_000;

/// Shows a desktop notification with error handling.
pub fn show_notification(title: &str, message: &str) {
    if let Err(e) = Notification::new()
        .summary(title)
        .body(message)
        .timeout(notify_rust::Timeout::Milliseconds(NOTIFICATION_TIMEOUT_MS))
        .show()
    {
        tracing::warn!("Failed to show notification '{}': {}", title, e);
    }
}

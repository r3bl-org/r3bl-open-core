// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{AnalyticsAction, report_analytics};
use crate::{DEBUG_ANALYTICS_CLIENT, try_get_proxy_machine_id_file_path};
use r3bl_tui::{InlineString, friendly_random_id,
               into_existing::{read_from_file::try_read_file_path_into_inline_string,
                               write_to_file}};
use std::fs;

/// Reads the proxy machine ID from [`try_get_proxy_machine_id_file_path`]
/// ([`<cache_dir>/r3bl/proxy_machine_id`][1]) if it exists, or generates and persists a
/// new friendly random ID.
///
/// [1]: crate::r3bl_file_paths#platform-base-directories
pub fn load_id_from_file_or_generate_and_save_it() -> InlineString {
    let Some(id_file_path) = try_get_proxy_machine_id_file_path() else {
        return friendly_random_id::generate_friendly_random_id();
    };

    if let Some(parent_dir) = id_file_path.parent()
        && let Err(error) = fs::create_dir_all(parent_dir)
    {
        // % is Display, ? is Debug.
        tracing::error!(
            message = "Could not create r3bl cache folder for proxy machine ID.",
            cache_folder = ?parent_dir,
            error = ?error
        );
        return friendly_random_id::generate_friendly_random_id();
    }

    let Some(id_file_path_str) = id_file_path.to_str() else {
        return friendly_random_id::generate_friendly_random_id();
    };

    // Try to read the file directly into InlineString.
    let mut content = InlineString::new();
    if try_read_file_path_into_inline_string(&mut content, id_file_path_str).is_ok() {
        DEBUG_ANALYTICS_CLIENT.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Successfully read proxy machine ID from file.",
                contents = %content
            );
        });
        content
    } else {
        let new_id = friendly_random_id::generate_friendly_random_id();
        match write_to_file::try_write_str_to_file(&id_file_path, &new_id) {
            Ok(()) => {
                report_analytics::start_task_to_generate_event(
                    String::new(),
                    AnalyticsAction::MachineIdProxyCreate,
                );
                DEBUG_ANALYTICS_CLIENT.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = "Successfully wrote proxy machine ID to file.",
                        new_id = %new_id
                    );
                });
            }
            Err(error) => {
                // % is Display, ? is Debug.
                tracing::error!(
                    message = "Could not write proxy machine ID to file.",
                    error = ?error
                );
            }
        }

        new_id
    }
}

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

use r3bl_tui::{InlineString,
               friendly_random_id,
               into_existing::{read_from_file::try_read_file_path_into_inline_string,
                               write_to_file}};

use super::{AnalyticsAction, config_folder, report_analytics};
use crate::DEBUG_ANALYTICS_CLIENT_MOD;

/// Read the file contents from [`config_folder::get_id_file_path`] and return it as a
/// string if it exists and can be read.
///
/// # Panics
///
/// This will panic if the lock is poisoned, which can happen if a thread
/// panics while holding the lock. To avoid panics, ensure that the code that
/// locks the mutex does not panic while holding the lock.
pub fn load_id_from_file_or_generate_and_save_it() -> InlineString {
    match config_folder::create() {
        Ok(config_folder_path) => {
            let id_file_path =
                config_folder::get_id_file_path(config_folder_path.clone());

            // Create a new InlineString to store the contents
            let mut content = InlineString::new();
            let res_read_from_file = try_read_file_path_into_inline_string(
                &mut content,
                id_file_path.to_str().expect("Invalid path"),
            );

            // Try to read the file directly into InlineString
            match res_read_from_file {
                Ok(()) => {
                    DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug!(
                            message = "Successfully read proxy machine ID from file.",
                            contents = %content
                        );
                    });
                    content
                }
                Err(_) => {
                    let new_id = friendly_random_id::generate_friendly_random_id();
                    let res_write_to_file =
                        write_to_file::try_write_str_to_file(&id_file_path, &new_id);
                    match res_write_to_file {
                        Ok(()) => {
                            report_analytics::start_task_to_generate_event(
                                "".to_string(),
                                AnalyticsAction::MachineIdProxyCreate,
                            );
                            DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
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
        }
        Err(_) => friendly_random_id::generate_friendly_random_id(),
    }
}

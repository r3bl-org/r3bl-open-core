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

use r3bl_tui::friendly_random_id;

use super::*;
use crate::DEBUG_ANALYTICS_CLIENT_MOD;

/// Read the file contents from [config_folder::get_id_file_path] and return it as a
/// string if it exists and can be read.
pub fn load_id_from_file_or_generate_and_save_it() -> String {
    match config_folder::create() {
        Ok(config_folder_path) => {
            let id_file_path =
                config_folder::get_id_file_path(config_folder_path.clone());
            let result = file_io::try_read_file_contents(&id_file_path);
            match result {
                Ok(contents) => {
                    DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug!(
                            message = "Successfully read proxy machine ID from file.",
                            contents = %contents
                        );
                    });

                    contents
                }
                Err(_) => {
                    let new_id = friendly_random_id::generate_friendly_random_id();
                    let result_write_file_contents =
                        file_io::try_write_file_contents(&id_file_path, &new_id);
                    match result_write_file_contents {
                        Ok(_) => {
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

                    new_id.to_string()
                }
            }
        }
        Err(_) => friendly_random_id::generate_friendly_random_id().to_string(),
    }
}

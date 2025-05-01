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

use std::{fmt::{Display, Formatter, Result},
          fs::{self},
          path::PathBuf};

use dirs::config_dir;
use r3bl_tui::{CommonError, CommonErrorType, CommonResult};

use crate::DEBUG_ANALYTICS_CLIENT_MOD;

pub enum ConfigPaths {
    R3BLTopLevelFolderName,
    ProxyMachineIdFile,
}

impl Display for ConfigPaths {
    /// This generates a `to_string()` method used by [get_id_file_path].
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let path = match self {
            ConfigPaths::R3BLTopLevelFolderName => "r3bl-cmdr",
            ConfigPaths::ProxyMachineIdFile => "id",
        };
        write!(f, "{path}")
    }
}

/// This is where the config file is stored.
pub fn get_id_file_path(path: PathBuf) -> PathBuf {
    path.join(format!("{}", ConfigPaths::ProxyMachineIdFile))
}

/// This is where the config folder is.
pub fn try_get_config_folder_path() -> Option<PathBuf> {
    let home_config_folder_path = config_dir()?;
    let config_file_path =
        home_config_folder_path.join(ConfigPaths::R3BLTopLevelFolderName.to_string());
    Some(config_file_path)
}

pub fn exists() -> bool {
    match try_get_config_folder_path() {
        Some(config_file_path) => config_file_path.exists(),
        None => false,
    }
}

pub fn create() -> CommonResult<PathBuf> {
    match try_get_config_folder_path() {
        Some(config_folder_path) => {
            let result_create_dir_all = fs::create_dir_all(&config_folder_path);
            match result_create_dir_all {
                Ok(_) => {
                    DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                        // % is Display, ? is Debug.
                        tracing::debug!(
                            message = "Successfully created config folder.",
                            config_folder = ?config_folder_path
                        );
                    });
                    Ok(config_folder_path)
                }
                Err(error) => {
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = "Could not create config folder.",
                        error = ?error
                    );
                    CommonError::new_error_result_with_only_type(
                        CommonErrorType::ConfigFolderCountNotBeCreated,
                    )
                }
            }
        }
        None => {
            // % is Display, ? is Debug.
            tracing::error!(message = "Could not access config folder.", error = "None");
            CommonError::new_error_result_with_only_type(
                CommonErrorType::ConfigFolderPathCouldNotBeAccessed,
            )
        }
    }
}

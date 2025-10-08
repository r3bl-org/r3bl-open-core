// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::DEBUG_ANALYTICS_CLIENT_MOD;
use dirs::config_dir;
use r3bl_tui::{CommonError, CommonErrorType, CommonResult};
use std::{fmt::{Display, Formatter, Result},
          fs::{self},
          path::PathBuf};

#[derive(Debug)]
pub enum ConfigPaths {
    R3BLTopLevelFolderName,
    ProxyMachineIdFile,
}

impl Display for ConfigPaths {
    /// This generates a `to_string()` method used by [`get_id_file_path`].
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let path = match self {
            ConfigPaths::R3BLTopLevelFolderName => "r3bl-cmdr",
            ConfigPaths::ProxyMachineIdFile => "id",
        };
        write!(f, "{path}")
    }
}

/// This is where the config file is stored.
#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn get_id_file_path(path: PathBuf) -> PathBuf {
    path.join(format!("{}", ConfigPaths::ProxyMachineIdFile))
}

/// This is where the config folder is.
#[must_use]
pub fn try_get_config_folder_path() -> Option<PathBuf> {
    let home_config_folder_path = config_dir()?;
    let config_file_path =
        home_config_folder_path.join(ConfigPaths::R3BLTopLevelFolderName.to_string());
    Some(config_file_path)
}

#[must_use]
pub fn exists() -> bool {
    match try_get_config_folder_path() {
        Some(config_file_path) => config_file_path.exists(),
        None => false,
    }
}

/// Creates the configuration folder for the application.
///
/// # Errors
///
/// Returns an error if:
/// - The config folder path cannot be determined
/// - Directory creation fails due to permissions or I/O issues
pub fn create() -> CommonResult<PathBuf> {
    if let Some(config_folder_path) = try_get_config_folder_path() {
        let result_create_dir_all = fs::create_dir_all(&config_folder_path);
        match result_create_dir_all {
            Ok(()) => {
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
    } else {
        // % is Display, ? is Debug.
        tracing::error!(message = "Could not access config folder.", error = "None");
        CommonError::new_error_result_with_only_type(
            CommonErrorType::ConfigFolderPathCouldNotBeAccessed,
        )
    }
}

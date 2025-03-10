/*
 *   Copyright (c) 2023-2025 R3BL LLC
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
          fs::{self, File},
          io::{BufReader, Read, Write},
          path::PathBuf,
          sync::atomic::AtomicBool};

use dirs::config_dir;
use miette::IntoDiagnostic as _;
use r3bl_analytics_schema::AnalyticsEvent;
use r3bl_ansi_color::{green, magenta, red};
use r3bl_core::{CommonError,
                CommonErrorType,
                CommonResult,
                call_if_true,
                friendly_random_id};
use reqwest::{Client, Response};

use crate::DEBUG_ANALYTICS_CLIENT_MOD;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnalyticsAction {
    GitiBranchDelete,
    GitiFailedToRun,
    GitiAppStart,
    EdiAppStart,
    EdiFileNew,
    EdiFileOpenSingle,
    EdiFileOpenMultiple,
    EdiFileSave,
    MachineIdProxyCreate,
}

impl std::fmt::Display for AnalyticsAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #[rustfmt::skip]
        let action = match self {
            AnalyticsAction::GitiAppStart =>          "giti app start",
            AnalyticsAction::GitiBranchDelete =>      "giti branch delete",
            AnalyticsAction::GitiFailedToRun =>       "giti failed to run",
            AnalyticsAction::EdiAppStart =>           "edi app start",
            AnalyticsAction::EdiFileNew =>            "edi file new",
            AnalyticsAction::EdiFileOpenSingle =>     "edi file open one file",
            AnalyticsAction::EdiFileOpenMultiple =>   "edi file open many files",
            AnalyticsAction::EdiFileSave =>           "edi file save",
            AnalyticsAction::MachineIdProxyCreate =>  "proxy machine id create",
        };
        write!(f, "{}", action)
    }
}

pub mod config_folder {
    use super::*;

    pub enum ConfigPaths {
        R3BLTopLevelFolderName,
        ProxyMachineIdFile,
    }

    impl Display for ConfigPaths {
        /// This generates a `to_string()` method used by [config_folder::get_id_file_path].
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            let path = match self {
                ConfigPaths::R3BLTopLevelFolderName => "r3bl-cmdr",
                ConfigPaths::ProxyMachineIdFile => "id",
            };
            write!(f, "{}", path)
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
                        call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                            tracing::debug!(
                                "Successfully created config folder: {}",
                                green(&format!("{config_folder_path:?}"))
                            );
                        });
                        Ok(config_folder_path)
                    }
                    Err(error) => {
                        tracing::error!(
                            "Could not create config folder.\n{}",
                            red(&format!("{error:?}"))
                        );
                        CommonError::new_error_result_with_only_type(
                            CommonErrorType::ConfigFolderCountNotBeCreated,
                        )
                    }
                }
            }
            None => {
                tracing::error!(
                    "Could not get config folder.\n{}",
                    red(&format!("{:?}", try_get_config_folder_path()))
                );
                CommonError::new_error_result_with_only_type(
                    CommonErrorType::ConfigFolderPathCouldNotBeGenerated,
                )
            }
        }
    }
}

pub mod file_io {
    use super::*;

    pub fn try_read_file_contents(path: &PathBuf) -> CommonResult<String> {
        let file = File::open(path).into_diagnostic()?;
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        let _ = reader.read_to_string(&mut contents).into_diagnostic()?;
        Ok(contents)
    }

    pub fn try_write_file_contents(path: &PathBuf, contents: &str) -> CommonResult<()> {
        let mut file = File::create(path).into_diagnostic()?;
        file.write_all(contents.as_bytes()).into_diagnostic()?;
        Ok(())
    }
}

pub mod proxy_machine_id {
    use super::*;

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
                        call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                            tracing::debug!(
                                "Successfully read proxy machine ID from file: {}",
                                green(&format!("{contents:?}"))
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

                                call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                                    tracing::debug!(
                                        "Successfully wrote proxy machine ID to file: {}",
                                        green(&format!("{new_id:?}"))
                                    );
                                });
                            }
                            Err(error) => {
                                tracing::error!(
                                    "Could not write proxy machine ID to file.\n{}",
                                    red(&format!("{error:?}"))
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
}

pub mod report_analytics {
    use super::*;

    static mut ANALYTICS_REPORTING_ENABLED: bool = true;

    const ANALYTICS_REPORTING_ENDPOINT: &str =
        "https://r3bl-base.shuttleapp.rs/add_analytics_event"; // "http://localhost:8000/add_analytics_event"

    pub fn disable() {
        unsafe {
            ANALYTICS_REPORTING_ENABLED = false;
        }
    }

    pub fn start_task_to_generate_event(proxy_user_id: String, action: AnalyticsAction) {
        unsafe {
            if !ANALYTICS_REPORTING_ENABLED {
                return;
            };
        }

        tokio::spawn(async move {
            let proxy_machine_id =
                proxy_machine_id::load_id_from_file_or_generate_and_save_it();

            let event =
                AnalyticsEvent::new(proxy_user_id, proxy_machine_id, action.to_string());
            let result_event_json = serde_json::to_value(&event);
            match result_event_json {
                Ok(json) => {
                    let result = http_client::make_post_request(
                        ANALYTICS_REPORTING_ENDPOINT,
                        &json,
                    )
                    .await;
                    match result {
                        Ok(_) => {
                            tracing::debug!(
                                "Successfully reported analytics event to r3bl-base.\n{}",
                                green(&format!("{json:#?}"))
                            );
                        }
                        Err(error) => {
                            tracing::error!(
                                "Could not report analytics event to r3bl-base.\n{}",
                                red(&format!("{error:#?}"))
                            );
                        }
                    }
                }
                Err(error) => {
                    tracing::error!(
                        "Could not report analytics event to r3bl-base.\n{}",
                        red(&format!("{error:#?}"))
                    );
                }
            }
        });
    }
}

/// To trigger an update message to be displayed to the user here are the details:
///
/// - The value of `GET_LATEST_VERSION_ENDPOINT` needs to be different,
/// - from the value of `UPDATE_IF_NOT_THIS_VERSION` in the `r3bl_base` repo.
pub mod upgrade_check {
    use super::*;

    static UPDATE_REQUIRED: AtomicBool = AtomicBool::new(false);

    const UPDATE_IF_NOT_THIS_VERSION: &str = "0.0.15";

    const GET_LATEST_VERSION_ENDPOINT: &str =
        "https://r3bl-base.shuttleapp.rs/get_latest_version"; // "http://localhost:8000/get_latest_version"

    pub fn is_update_required() -> bool {
        UPDATE_REQUIRED.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn start_task_to_check_for_updates() {
        tokio::spawn(async move {
            let result = http_client::make_get_request(GET_LATEST_VERSION_ENDPOINT).await;
            if let Ok(response) = result {
                if let Ok(body_text) = response.text().await {
                    let latest_version = body_text.trim().to_string();
                    tracing::info!(
                        "\n📦📦📦\nLatest version of cmdr is: {}",
                        magenta(&latest_version)
                    );
                    let current_version = UPDATE_IF_NOT_THIS_VERSION.to_string();
                    if latest_version != current_version {
                        UPDATE_REQUIRED.store(true, std::sync::atomic::Ordering::Relaxed);
                        tracing::info!(
                            "\n💿💿💿\nThere is a new version of cmdr available: {}",
                            magenta(&latest_version)
                        );
                    }
                }
            }
        });
    }
}

pub mod http_client {
    use super::*;

    pub async fn make_get_request(
        url: &str,
    ) -> core::result::Result<Response, reqwest::Error> {
        let client = Client::new();
        let response = client.get(url).send().await?;
        if response.status().is_success() {
            // Handle successful response.
            call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                tracing::debug!(
                    "GET request succeeded: {}",
                    green(&format!("{response:#?}"))
                );
            });
            Ok(response)
        } else {
            // Handle error response.
            tracing::error!("GET request failed: {}", red(&format!("{response:#?}")));
            response.error_for_status()
        }
    }

    pub async fn make_post_request(
        url: &str,
        data: &serde_json::Value,
    ) -> core::result::Result<Response, reqwest::Error> {
        let client = Client::new();
        let response = client.post(url).json(data).send().await?;
        if response.status().is_success() {
            // Handle successful response.
            call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                tracing::debug!(
                    "POST request succeeded: {}",
                    green(&format!("{response:#?}"))
                );
            });
            Ok(response)
        } else {
            // Handle error response.
            tracing::error!("POST request failed: {}", red(&format!("{response:#?}")));
            response.error_for_status()
        }
    }
}

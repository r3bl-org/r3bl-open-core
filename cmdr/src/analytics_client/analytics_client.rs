/*
 *   Copyright (c) 2023 R3BL LLC
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

use std::{fs::File,
          io::{BufReader, Read},
          path::PathBuf};

use crossterm::style::Stylize;
use dirs::*;
use r3bl_analytics_schema::AnalyticsEvent;
use r3bl_rs_utils_core::{call_if_true, log_debug, log_error, CommonResult};
use reqwest::Client;

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

impl AnalyticsAction {
    pub fn to_string(&self) -> String {
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
        action.to_string()
    }
}

pub mod config_file_proxy_machine_id {
    use std::{fs, io::Write};

    use super::*;

    const CONFIG_FOLDER_NAME: &str = "r3bl-cmdr";
    const PROXY_MACHINE_ID_FILE_NAME: &str = "id";

    /// This is where the config file is stored.
    fn try_get_config_file_path() -> Option<PathBuf> {
        let config_dir = config_dir()?;
        let config_file_path = config_dir
            .join(CONFIG_FOLDER_NAME)
            .join(PROXY_MACHINE_ID_FILE_NAME);
        Some(config_file_path)
    }

    /// This is where the config folder is.
    fn try_get_config_folder_path() -> Option<PathBuf> {
        let config_dir = config_dir()?;
        let config_file_path = config_dir.join(CONFIG_FOLDER_NAME);
        Some(config_file_path)
    }

    fn try_read_file_contents(path: &PathBuf) -> CommonResult<String> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        let _ = reader.read_to_string(&mut contents)?;
        Ok(contents)
    }

    fn try_write_file_contents(path: &PathBuf, contents: &str) -> CommonResult<()> {
        let mut file = File::create(path)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }

    /// Read the file contents from [try_get_config_file_path] and return it as a string
    /// if it exists and can be read.
    pub fn load_id_from_file_or_generate_and_save_it() -> String {
        match try_get_config_folder_path() {
            Some(config_folder_path) => {
                let result_create_dir_all = fs::create_dir_all(&config_folder_path);
                match result_create_dir_all {
                    Ok(_) => {
                        call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                            log_debug(
                                format!(
                                    "Successfully created config folder: {config_folder_path:?}"
                                )
                                .green()
                                .to_string(),
                            );
                        });
                    }
                    Err(error) => {
                        log_error(
                            format!("Could not create config folder.\n{error:?}",)
                                .red()
                                .to_string(),
                        );
                    }
                }
            }
            None => {
                log_error(
                    format!(
                        "Could not get config folder for proxy machine ID.\n{:?}",
                        try_get_config_folder_path(),
                    )
                    .red()
                    .to_string(),
                );
                return friendly_random_id::generate();
            }
        }

        match try_get_config_file_path() {
            Some(config_file_path) => {
                let result = try_read_file_contents(&config_file_path);
                match result {
                    Ok(contents) => {
                        call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                            log_debug(
                            format!("Successfully read proxy machine ID from file: {contents:?}")
                            .green()
                            .to_string(),
                        );
                        });
                        return contents;
                    }
                    Err(_) => {
                        let new_id = friendly_random_id::generate();
                        let result_write_file_contents =
                            try_write_file_contents(&config_file_path, &new_id);
                        match result_write_file_contents {
                            Ok(_) => {
                                report_analytics::generate_event(
                                    "".to_string(),
                                    AnalyticsAction::MachineIdProxyCreate,
                                );

                                call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                                    log_debug(
                                    format!(
                                        "Successfully wrote proxy machine ID to file: {new_id:?}"
                                    )
                                    .green()
                                    .to_string(),
                                );
                                });
                            }
                            Err(error) => {
                                log_error(
                                    format!(
                                        "Could not write proxy machine ID to file.\n{error:?}",
                                    )
                                    .red()
                                    .to_string(),
                                );
                            }
                        }
                        return new_id;
                    }
                }
            }
            None => {
                log_error(
                    format!(
                        "Could not get config file path for proxy machine ID.\n{:?}",
                        try_get_config_file_path(),
                    )
                    .red()
                    .to_string(),
                );
                return friendly_random_id::generate();
            }
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

    pub fn generate_event(proxy_user_id: String, action: AnalyticsAction) {
        unsafe {
            if !ANALYTICS_REPORTING_ENABLED {
                return;
            };
        }

        tokio::spawn(async move {
            let proxy_machine_id =
                config_file_proxy_machine_id::load_id_from_file_or_generate_and_save_it();

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
                            log_debug(
                                 format!(
                                     "Successfully reported analytics event to r3bl-base.\n{:#?}",
                                     json
                                 )
                                 .green()
                                 .to_string(),
                             );
                        }
                        Err(error) => {
                            log_error(
                                 format!(
                                     "Could not report analytics event to r3bl-base.\n{:#?}",
                                     error
                                 )
                                 .red()
                                 .to_string(),
                             );
                        }
                    }
                }
                Err(error) => {
                    log_error(
                        format!(
                            "Could not report analytics event to r3bl-base.\n{:#?}",
                            error
                        )
                        .red()
                        .to_string(),
                    );
                }
            }
        });
    }
}

pub mod http_client {
    use reqwest::Response;

    use super::*;

    pub async fn make_get_request(url: &str) -> Result<Response, reqwest::Error> {
        let client = Client::new();
        let response = client.get(url).send().await?;
        if response.status().is_success() {
            // Handle successful response.
            call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                log_debug(
                    format!("GET request succeeded: {response:#?}",)
                        .green()
                        .to_string(),
                );
            });
            return Ok(response);
        } else {
            // Handle error response.
            log_error(
                format!("GET request failed: {response:#?}",)
                    .red()
                    .to_string(),
            );
            return response.error_for_status();
        }
    }

    pub async fn make_post_request(
        url: &str,
        data: &serde_json::Value,
    ) -> Result<Response, reqwest::Error> {
        let client = Client::new();
        let response = client.post(url).json(data).send().await?;
        if response.status().is_success() {
            // Handle successful response.
            call_if_true!(DEBUG_ANALYTICS_CLIENT_MOD, {
                log_debug(
                    format!("POST request succeeded: {response:#?}",)
                        .green()
                        .to_string(),
                );
            });
            return Ok(response);
        } else {
            // Handle error response.
            log_error(
                format!("POST request failed: {response:#?}",)
                    .red()
                    .to_string(),
            );
            return response.error_for_status();
        }
    }
}

pub mod friendly_random_id {
    const PET_NAMES: [&str; 20] = [
        "Buddy", "Max", "Bella", "Charlie", "Lucy", "Daisy", "Molly", "Lola", "Sadie",
        "Maggie", "Bailey", "Sophie", "Chloe", "Duke", "Lily", "Rocky", "Jack", "Cooper",
        "Riley", "Zoey",
    ];

    const FRUIT_NAMES: [&str; 20] = [
        "Apple",
        "Banana",
        "Orange",
        "Pear",
        "Peach",
        "Strawberry",
        "Grape",
        "Kiwi",
        "Mango",
        "Pineapple",
        "Watermelon",
        "Cherry",
        "Blueberry",
        "Raspberry",
        "Lemon",
        "Lime",
        "Grapefruit",
        "Plum",
        "Apricot",
        "Pomegranate",
    ];

    pub fn generate() -> String {
        use rand::Rng;

        // Generate friendly pet and fruit name combination.
        let pet = {
            let mut rng = rand::thread_rng();
            let pet = PET_NAMES[rng.gen_range(0..PET_NAMES.len())];
            pet.to_lowercase()
        };

        let fruit = {
            let mut rng = rand::thread_rng();
            let fruit = FRUIT_NAMES[rng.gen_range(0..FRUIT_NAMES.len())];
            fruit.to_lowercase()
        };

        let random_number = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..1000)
        };

        format!("{pet}-{fruit}-{random_number}")
    }
}

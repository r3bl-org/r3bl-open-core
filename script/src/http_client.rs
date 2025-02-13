/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use miette::IntoDiagnostic;

mod constants {
    pub const USER_AGENT: &str = "scripting.rs/1.0";
}

pub fn create_client_with_user_agent(
    user_agent: Option<&str>,
) -> miette::Result<reqwest::Client> {
    let it = reqwest::Client::builder()
        .user_agent(user_agent.map_or_else(
            || constants::USER_AGENT.to_owned(),
            |user_agent| user_agent.to_owned(),
        ))
        .build();
    it.into_diagnostic()
}

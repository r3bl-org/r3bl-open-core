// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use miette::IntoDiagnostic;

mod constants {
    pub const USER_AGENT: &str = "scripting.rs/1.0";
}

/// # Errors
///
/// Returns an error if:
/// - The HTTP client builder fails to build
/// - TLS backend initialization fails
pub fn create_client_with_user_agent(
    user_agent: Option<&str>,
) -> miette::Result<reqwest::Client> {
    let it = reqwest::Client::builder()
        .user_agent(user_agent.map_or_else(
            /* none */ || constants::USER_AGENT.to_owned(),
            /* some */ ToOwned::to_owned,
        ))
        .build();
    it.into_diagnostic()
}

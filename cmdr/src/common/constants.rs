// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

pub mod bin_names {
    pub const GITI: &str = "giti";
    pub const EDI: &str = "edi";
    pub const RC: &str = "rc";
    pub const ENV_SOURCE: &str = "env-source";
}

pub mod bin_emojis {
    pub const GITI: &str = "😺";
    pub const EDI: &str = "🦜";
    pub const RC: &str = "🐒";
    pub const ENV_SOURCE: &str = "📜";
    pub const FALLBACK: &str = "👾";
}

/// Gets the emoji representing the given binary name.
#[must_use]
pub fn get_bin_emoji(bin_name: &str) -> &'static str {
    match bin_name {
        bin_names::GITI => bin_emojis::GITI,
        bin_names::EDI => bin_emojis::EDI,
        bin_names::RC => bin_emojis::RC,
        bin_names::ENV_SOURCE => bin_emojis::ENV_SOURCE,
        _ => bin_emojis::FALLBACK,
    }
}

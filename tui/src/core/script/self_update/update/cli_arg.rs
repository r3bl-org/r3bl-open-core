// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Behavior governing background update checks and application exit handling.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    clap::ValueEnum,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateBehavior {
    /// Interactive prompt asking the user (`"Yes, upgrade now"` / `"No, thanks"`) at
    /// exit if a newer version is available.
    #[value(name = "prompt-to-upgrade")]
    PromptToUpgrade,

    /// Automatically downloads and installs the upgrade at exit without prompting if a
    /// newer version is available. Default behavior for all workspace binaries.
    #[default]
    #[value(name = "auto")]
    Auto,

    /// Prints a one-line notification that a newer version is available without running
    /// the upgrade. Selected explicitly via `--update-behavior notify-only`.
    #[value(name = "notify-only")]
    NotifyOnly,

    /// Suppresses background version checks and exit upgrade handling entirely.
    #[value(name = "disable")]
    Disable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_update_behavior_clap_value_enum() {
        use clap::ValueEnum;

        let prompt = UpdateBehavior::from_str("prompt-to-upgrade", true).unwrap();
        assert_eq2!(prompt, UpdateBehavior::PromptToUpgrade);

        let auto = UpdateBehavior::from_str("auto", true).unwrap();
        assert_eq2!(auto, UpdateBehavior::Auto);
        assert_eq2!(UpdateBehavior::default(), UpdateBehavior::Auto);

        let notify = UpdateBehavior::from_str("notify-only", true).unwrap();
        assert_eq2!(notify, UpdateBehavior::NotifyOnly);

        let disable = UpdateBehavior::from_str("disable", true).unwrap();
        assert_eq2!(disable, UpdateBehavior::Disable);

        // Verify that aliases and obsolete names are rejected to prevent ambiguity.
        assert!(UpdateBehavior::from_str("prompt", true).is_err());
        assert!(UpdateBehavior::from_str("interactive", true).is_err());
        assert!(UpdateBehavior::from_str("interactive-prompt", true).is_err());
        assert!(UpdateBehavior::from_str("auto-upgrade", true).is_err());
        assert!(UpdateBehavior::from_str("notify", true).is_err());
        assert!(UpdateBehavior::from_str("disabled", true).is_err());
        assert!(UpdateBehavior::from_str("none", true).is_err());
        assert!(UpdateBehavior::from_str("never", true).is_err());
        assert!(UpdateBehavior::from_str("off", true).is_err());
    }
}

// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Single-start guard ([`StartOutcome::AlreadyStarted`]) and disabled mode
//! ([`UpgradePolicy::Disabled`]) integration tests for [`UpgradeJourney`].
//!
//! [`StartOutcome::AlreadyStarted`]: crate::self_upgrade::StartOutcome::AlreadyStarted
//! [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
//! [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled

#[cfg(test)]
mod tests {
    use crate::{CooldownPolicy, CrateVersion, EndpointUrl, JsoncScalar,
                UpgradeActionOnExit, UpgradeConfig, UpgradePolicy, UpgradeStrategy,
                UserFeedbackChannel, self_upgrade::*};
    use r3bl_tui::assert_eq2;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_single_start_guard_returns_already_started_without_mutating_singleton() {
        SingletonState::reset_for_testing();

        let make_config = |policy: UpgradePolicy| {
            UpgradeConfig::new(
                "r3bl-cmdr",
                CrateVersion::try_parse("0.0.25").unwrap(),
                UserFeedbackChannel::Quiet,
                policy,
                EndpointUrl::default_manifest_url(),
                CooldownPolicy::Disabled,
            )
        };

        let disabled_config = make_config(UpgradePolicy::Disabled);
        let first = UpgradeJourney::new(disabled_config.clone());
        assert_eq2!(
            first.start(),
            StartOutcome::Disabled(UpgradeJourney::new(disabled_config).into_disabled())
        );

        // Duplicate Immediate call must return AlreadyStarted and leave SINGLETON
        // untouched.
        let dup_immediate = UpgradeJourney::new(make_config(UpgradePolicy::Enabled(
            UpgradeStrategy::Immediate,
        )));
        assert_eq2!(dup_immediate.start(), StartOutcome::AlreadyStarted);

        // Duplicate Deferred call must also return AlreadyStarted with 0 threads.
        let dup_thread = UpgradeJourney::new(make_config(UpgradePolicy::Enabled(
            UpgradeStrategy::Deferred(UpgradeActionOnExit::Auto),
        )));
        let dup_outcome = dup_thread.start();
        assert_eq2!(dup_outcome, StartOutcome::AlreadyStarted);
        assert_eq2!(
            dup_outcome.handle_upgrade_on_exit(),
            ExitOutcome::NoDeferredCheckRunning
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_disabled_async_and_sync() {
        SingletonState::reset_for_testing();
        
        let config = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Disabled,
            EndpointUrl::default_manifest_url(),
            CooldownPolicy::Disabled,
        );
        let outcome = UpgradeJourney::new(config.clone()).start();
        assert_eq2!(
            outcome,
            StartOutcome::Disabled(UpgradeJourney::new(config).into_disabled())
        );
        assert_eq2!(
            outcome.handle_upgrade_on_exit(),
            ExitOutcome::NoDeferredCheckRunning
        );
    }
}

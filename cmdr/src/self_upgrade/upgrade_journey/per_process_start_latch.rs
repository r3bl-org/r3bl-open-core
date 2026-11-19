// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::AtomicBoolExt;
use std::sync::atomic::AtomicBool;

/// Monotonic per-process latch backing [`PerProcessStartLatch`].
///
/// Transitions `false -> true` at most once per OS process via
/// [`AtomicBoolExt::try_acquire`]. Never reset in production (even on early abort or
/// after exit completion) so a detached check thread can never overlap with a second
/// spawned check thread and shutdown hooks cannot restart upgrade checks.
static PER_PROCESS_START_LATCH: AtomicBool = AtomicBool::new(false);

/// Witness returned by [`PerProcessStartLatch::claim`] indicating whether
/// [`UpgradeJourney::start`] acquired the single-start latch or observed a prior start.
///
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartClaimStatus {
    /// Process-wide latch transitioned from `false` to `true`.
    Claimed,

    /// Process-wide latch was already `true`; caller must return
    /// [`StartOutcome::AlreadyStarted`][1].
    ///
    /// [1]: crate::self_upgrade::StartOutcome::AlreadyStarted
    AlreadyStarted,
}

/// Lock-free process-wide single-start latch for [`UpgradeJourney::start`].
///
/// Part of the [Control Plane] (see [Thread Model]). Unlike the former mutex-backed
/// singleton, `PerProcessStartLatch` stores zero thread handles; any spawned
/// [`DeferredCheckHandle`] is owned linearly inside [`DeferredUpgradeOutcome`].
///
/// [`DeferredCheckHandle`]: crate::self_upgrade::DeferredCheckHandle
/// [`DeferredUpgradeOutcome`]: crate::self_upgrade::DeferredUpgradeOutcome
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
#[derive(Debug)]
pub struct PerProcessStartLatch;

impl PerProcessStartLatch {
    /// Atomically claims the process-wide start latch on first call, returning
    /// [`StartClaimStatus::Claimed`] once and [`StartClaimStatus::AlreadyStarted`] on all
    /// subsequent calls in this process.
    #[must_use]
    pub fn claim() -> StartClaimStatus {
        match PER_PROCESS_START_LATCH.try_acquire() {
            Some(()) => StartClaimStatus::Claimed,
            None => StartClaimStatus::AlreadyStarted,
        }
    }

    /// Resets [`PER_PROCESS_START_LATCH`] back to `false` between `#[serial]` tests.
    ///
    /// In production, [`PER_PROCESS_START_LATCH`] is a per-process latch: once
    /// [`PerProcessStartLatch::claim`] flips it to `true`, it is never reset.
    ///
    /// However, `cargo test` runs multiple `#[test]` functions inside the same OS
    /// process, so they all share that single static. Without this reset escape hatch,
    /// the first test to call [`UpgradeJourney::start`] (or
    /// [`PerProcessStartLatch::claim`]) would permanently latch
    /// [`PER_PROCESS_START_LATCH`], causing all subsequent tests in the binary to
    /// observe [`StartClaimStatus::AlreadyStarted`] instead of running their own upgrade
    /// flow.
    ///
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    #[cfg(any(test, doc))]
    pub fn reset_for_testing() { PER_PROCESS_START_LATCH.release(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3bl_tui::assert_eq2;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_per_process_start_latch_single_claim_and_reset() {
        PerProcessStartLatch::reset_for_testing();

        assert_eq2!(PerProcessStartLatch::claim(), StartClaimStatus::Claimed);
        assert_eq2!(
            PerProcessStartLatch::claim(),
            StartClaimStatus::AlreadyStarted
        );

        PerProcessStartLatch::reset_for_testing();

        assert_eq2!(PerProcessStartLatch::claim(), StartClaimStatus::Claimed);
    }
}

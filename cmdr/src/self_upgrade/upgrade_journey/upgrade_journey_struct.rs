// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::UpgradeConfig;

/// State machine for the self-upgrade lifecycle using the [Typestate Pattern].
///
/// This struct holds an immutable [`UpgradeConfig`] alongside the current `State`,
/// which can be one of [`NotStarted`], [`Disabled`], [`CooldownSkipped`], [`Checking`],
/// [`UpToDate`], [`UpgradeAvailable`], [`CheckFailed`], [`Upgrading`], or [`Completed`].
///
/// - External callers (code in other modules) can only construct
///   [`UpgradeJourney<NotStarted>`], via [`UpgradeJourney::new`] or [`upgrade_journey!`],
///   and call [`.start()`]. Because `config`, `state`, and all `into_*` transition
///   methods are module-private (`pub(in crate::self_upgrade)`), code in other modules
///   cannot forge state transitions.
/// - The [Control Plane] drives each transition across the caller/current/main thread and
///   the dedicated check thread (see [Thread Model]) by consuming
///   `UpgradeJourney<PrevState>` by value to produce `UpgradeJourney<NextState>`.
///
/// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// [`self_upgrade`] module documentation for how the CLI-to-domain boundary, typestate
/// machine, and process-wide singleton fit together.
///
/// # State Machine & Lifecycle Flow
///
/// See [`upgrade_journey!`] macro for a [code example][ex].
///
/// ```text
/// 1. At startup: upgrade_journey!(channel, cli_args) ──► <NotStarted>.start()
/// ┌─────────────────────────────────────────────────────────────────────────┘
/// ├─ 1st .start() (claims SINGLETON; transitions from <NotStarted> based on UpgradePolicy):
/// │  │
/// │  ├─ UpgradePolicy::Disabled (--upgrade-behavior disable)
/// │  │  └─► <Disabled> ───────────────────────────────────────► StartOutcome::ContinueApplication
/// │  │
/// │  ├─ UpgradePolicy::Enabled(Immediate) (--upgrade)
/// │  │  │ ( Runs entire lifecycle inside .start(); main() exits with result.exit_code() )
/// │  │  └─► <Checking> (fetches versions.jsonc with 5s timeout; bypasses 1h cooldown)
/// │  │      ├─► <CheckFailed> (network/parse error) ──────────► StartOutcome::UpgradedAndExit(Failed)
/// │  │      ├─► <UpToDate> (remote <= current) ─► touch file ─► StartOutcome::UpgradedAndExit(Succeeded)
/// │  │      └─► <Upgrading> (remote > current) ─► perform_upgrade()
/// │  │          ├─► <Completed(Upgraded)> ─► touch file ──────► StartOutcome::UpgradedAndExit(Succeeded)
/// │  │          └─► <Completed(UpgradeFailed)> ───────────────► StartOutcome::UpgradedAndExit(Failed)
/// │  │
/// │  └─ UpgradePolicy::Enabled(DedicatedThread(action))
/// │     │ ( Returns ContinueApplication immediately: main() discards the return value and
/// │     │   continues running; handle_upgrade_on_exit() later reads the active state and
/// │     │   JoinHandle directly from SINGLETON )
/// │     ├─ Active 1h cooldown ──► <CooldownSkipped> ──────────► StartOutcome::ContinueApplication
/// │     └─ Expired/missing timestamp file ──► <Checking> ─────► StartOutcome::ContinueApplication
/// │        │
/// │        └─ Dedicated check thread (fetches versions.jsonc with 2s timeout):
/// │           ├─► <UpToDate> (remote <= current) ──► touch file
/// │           ├─► <CheckFailed> (network/parse error)
/// │           └─► <UpgradeAvailable> (remote > current)
/// │
/// └─ 2nd+ .start() (SINGLETON already claimed) ───────────────► StartOutcome::AlreadyStarted (no-op)
///
/// 2. At exit: UpgradeCheckStatus::handle_upgrade_on_exit()
/// ┌──────────────────────────────────────────────────────┘
/// ├─ Step 1: Takes JoinHandle from SINGLETON, drops lock, and joins dedicated check thread (waits <= 2s)
/// └─ Step 2: Inspects SINGLETON state:
///    ├─ <Disabled> | <CooldownSkipped> | <UpToDate> | <CheckFailed> | <Completed> ──► silent no-op
///    └─ <UpgradeAvailable> ──► <Upgrading> ──► executes UpgradeActionOnExit:
///                              │               ├─ Auto ────────────► perform_upgrade()
///                              │               ├─ PromptToUpgrade ─► prompt user ──► perform_upgrade()
///                              │               └─ NotifyOnly ──────► print notification
///                              └──► <Completed> (touches timestamp file if Upgraded)
///
/// Legend: <State> denotes the UpgradeJourney<State> typestate stored in SINGLETON.
///         So, <NotStarted> is UpgradeJourney<NotStarted>.
/// ```
///
/// [`.start()`]: crate::self_upgrade::UpgradeJourney::start
/// [`CheckFailed`]: crate::self_upgrade::CheckFailed
/// [`Checking`]: crate::self_upgrade::Checking
/// [`Completed`]: crate::self_upgrade::Completed
/// [`CooldownSkipped`]: crate::self_upgrade::CooldownSkipped
/// [`Disabled`]: crate::self_upgrade::Disabled
/// [`NotStarted`]: crate::self_upgrade::NotStarted
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`upgrade_journey!`]: crate::upgrade_journey
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
/// [`UpgradeJourney::new`]: crate::self_upgrade::UpgradeJourney::new
/// [`UpgradeJourney<NotStarted>`]: crate::self_upgrade::UpgradeJourney
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [`UpToDate`]: crate::self_upgrade::UpToDate
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
/// [ex]: crate::upgrade_journey!#examples
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
/// [Typestate Pattern]: https://stanford-cs242.github.io/f19/lectures/08-2-typestate.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeJourney<State: UpgradeJourneyStateMarker> {
    pub(in crate::self_upgrade) config: UpgradeConfig,
    pub(in crate::self_upgrade) state: State,
}

impl<State: UpgradeJourneyStateMarker> UpgradeJourney<State> {
    /// Returns a reference to the immutable [`UpgradeConfig`] for this journey.
    ///
    /// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
    #[must_use]
    pub fn get_config(&self) -> &UpgradeConfig { &self.config }

    /// Returns a reference to the current typestate payload `State`.
    #[must_use]
    pub fn get_state(&self) -> &State { &self.state }
}

/// Marker trait implemented by all valid typestates of [`UpgradeJourney`].
///
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
pub trait UpgradeJourneyStateMarker {}

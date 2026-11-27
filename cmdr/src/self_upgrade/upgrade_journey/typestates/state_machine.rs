// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::UpgradeConfig;

/// Core state machine of the [Control Plane] that orchestrates the self-upgrade
/// lifecycle, using the [Typestate Pattern].
///
/// This struct holds an immutable [`UpgradeConfig`] alongside the current `State`, which
/// can be one of [`NotStarted`], [`Disabled`], [`Skipped`], [`Checking`],
/// [`UpToDate`], [`UpgradeAvailable`], [`CheckFailed`], [`Upgrading`], or [`Completed`].
///
/// # How it is used
///
/// External callers (code in other modules) can only construct
/// [`UpgradeJourney<NotStarted>`], via [`UpgradeJourney::new`] or [`upgrade_journey!`],
/// and call [`.start()`]. And nothing else, by design. Because `config` and `state` are
/// restricted to `pub(super)` (`typestates/`) and all `into_*` transition methods are
/// module-private (`pub(in crate::self_upgrade)`), code in other modules cannot forge
/// state transitions. This gives us compile-time (witness) proof that the state
/// transitions are legit - so we don't need to perform runtime validation.
///
/// As the [Control Plane] state machine, it drives each transition across the
/// caller/current/main thread and the dedicated thread (see [Thread Model]) by consuming
/// `UpgradeJourney<PrevState>` by value to produce `UpgradeJourney<NextState>`, while
/// delegating network, disk, and build I/O to the [Execution Plane].
///
/// > See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// > [`self_upgrade`] module documentation for how the CLI-to-domain boundary, typestate
/// > machine, and lock-free [`PerProcessStartLatch`] fit together.
///
/// # State Machine & Lifecycle Flow
///
/// See [`upgrade_journey!`] macro for a [code example][ex].
///
/// ```text
/// 1. At startup: upgrade_journey!(channel, cli_args) ──► <NotStarted>.start()
/// ┌─────────────────────────────────────────────────────────────────────────┘
/// ├─ 1st .start() (claims PerProcessStartLatch false -> true; branches on UpgradePolicy):
/// │  │
/// │  ├─ UpgradePolicy::Disabled (--upgrade-behavior disable)
/// │  │  └─► <Disabled> ──► StartOutcome::Disabled(<Disabled>)
/// │  │
/// │  ├─ UpgradePolicy::Enabled(Immediate) (--upgrade)
/// │  │  │ (Runs entire lifecycle inside .start(); main() exits immediately)
/// │  │  └─► <Checking> (fetches versions.jsonc, 5s timeout; bypasses 1h cooldown)
/// │  │      ├─► <CheckFailed> (network/parse/init/panic error)
/// │  │      │   └─► ImmediateUpgrade(CheckFailed(<CheckFailed>))
/// │  │      ├─► <UpToDate> (remote <= current) ──► touch timestamp file
/// │  │      │   └─► ImmediateUpgrade(UpToDate(<UpToDate>))
/// │  │      └─► <Upgrading> (remote > current) ──► try_perform_upgrade()
/// │  │          ├─► <Completed(Upgraded)> ──► touch timestamp file
/// │  │          │   └─► ImmediateUpgrade(Completed(<Completed>))
/// │  │          └─► <Completed(UpgradeFailed)>
/// │  │              └─► ImmediateUpgrade(Completed(<Completed>))
/// │  │
/// │  └─ UpgradePolicy::Enabled(Deferred(action))
/// │     │ (Returns DeferredUpgrade immediately; any spawned
/// │     │  DeferredCheckHandle is owned linearly inside DeferredUpgradeOutcome)
/// │     ├─ Active 1h cooldown ──► <Skipped>
/// │     │  └─► DeferredUpgrade(Skipped(<Skipped>))
/// │     └─ Expired/missing timestamp file ──► <Checking>
/// │        ├─► Main thread: DeferredUpgrade(DeferredCheckSpawned(DeferredCheckHandle))
/// │        └─► Dedicated thread (2s timeout; returns DeferredCheckOutcome):
/// │            ├─► DeferredCheckOutcome::UpToDate(<UpToDate>) ──► touch file
/// │            ├─► DeferredCheckOutcome::CheckFailed(<CheckFailed>)
/// │            └─► DeferredCheckOutcome::UpgradeAvailable(<UpgradeAvailable>)
/// │
/// └─ 2nd+ .start() (PerProcessStartLatch already claimed)
///    └─► StartOutcome::AlreadyStarted (no-op, owns zero handles)
///
/// 2. At exit (only for StartOutcome::DeferredUpgrade(deferred)):
///    deferred.handle_upgrade_on_exit() [consumes self] ──► ExitOutcome
/// ┌───────────────────────────────────────────────────────────────────┘
/// ├─ Skipped(<Skipped>) ──────────────► ExitOutcome::Skipped(<Skipped>)
/// ├─ DeferredCheckSpawnFailed(<CheckFailed>)
/// │  └─► ExitOutcome::CheckFailed(<CheckFailed>)
/// └─ DeferredCheckSpawned(DeferredCheckHandle) ──► joins dedicated thread (<= 2s):
///    ├─ FailedToStart | Panicked ─► ExitOutcome::CheckFailed(<CheckFailed>)
///    ├─ UpToDate(<UpToDate>) ─────► ExitOutcome::UpToDate(<UpToDate>)
///    ├─ CheckFailed(<CheckFailed>) ► ExitOutcome::CheckFailed(<CheckFailed>)
///    └─ UpgradeAvailable(<UpgradeAvailable>) ──► <Upgrading>
///       ├─ Executes UpgradeActionOnExit:
///       │  ├─ Auto ────────────► try_perform_upgrade()
///       │  ├─ PromptToUpgrade ─► prompt user ──► try_perform_upgrade()
///       │  └─ NotifyOnly ──────► print notification
///       └─► ExitOutcome::Completed(<Completed>) (touches file if Upgraded)
///
/// Legend:
/// - <State>              = UpgradeJourney<State> (e.g. <NotStarted>)
/// - DeferredUpgrade(..)  = StartOutcome::DeferredUpgrade(DeferredUpgradeOutcome::..)
/// - ImmediateUpgrade(..) = StartOutcome::ImmediateUpgrade(ImmediateUpgradeOutcome::..)
/// ```
///
/// # Why Not to Use `impl From`
///
/// In the [Typestate Pattern], an [`UpgradeJourney<State>`] value is a witness, or
/// compile-time proof, that the [Control Plane] verified the preconditions and
/// orchestrated the required [Execution Plane] work to reach [`State`] (e.g., claiming
/// [`PerProcessStartLatch`], checking [`UpgradePolicy`], and verifying cooldown or
/// manifest versions).
///
/// Because Rust makes every `impl From<A> for B` unconditionally `pub` whenever `A` and
/// `B` are `pub`, implementing [`From`] for state transitions would expose unguarded
/// public constructors: allowing external callers to forge states (such as `let forged:
/// UpgradeJourney<Disabled> = journey.into();`) without calling [`.start()`]. This would
/// defeat the typesafety afforded by the typestate machine. Using `pub(in
/// crate::self_upgrade) fn into_*` seals transitions inside [`self_upgrade`].
///
/// [`.start()`]: crate::self_upgrade::UpgradeJourney::start
/// [`CheckFailed`]: crate::self_upgrade::CheckFailed
/// [`Checking`]: crate::self_upgrade::Checking
/// [`Completed`]: crate::self_upgrade::Completed
/// [`Disabled`]: crate::self_upgrade::Disabled
/// [`NotStarted`]: crate::self_upgrade::NotStarted
/// [`PerProcessStartLatch`]: crate::self_upgrade::upgrade_journey::per_process_start_latch::PerProcessStartLatch
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`Skipped`]: crate::self_upgrade::Skipped
/// [`State`]: super::StateMarker
/// [`upgrade_journey!`]: crate::upgrade_journey
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
/// [`UpgradeJourney::new`]: crate::self_upgrade::UpgradeJourney::new
/// [`UpgradeJourney<NotStarted>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradeJourney<State>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [`UpToDate`]: crate::self_upgrade::UpToDate
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
/// [ex]: crate::upgrade_journey!#examples
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Thread Model]: crate::self_upgrade::NotStarted#thread-model
/// [Typestate Pattern]: https://stanford-cs242.github.io/f19/lectures/08-2-typestate.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeJourney<State: StateMarker> {
    pub(super) config: UpgradeConfig,
    pub(super) state: State,
}

impl<State: StateMarker> UpgradeJourney<State> {
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
pub trait StateMarker {}

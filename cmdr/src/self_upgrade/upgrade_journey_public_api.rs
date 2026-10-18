// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CooldownStatus, CrateMetadata, ImmediateCheckOutcome, PromptUpgradeDecision,
            UpgradeActionOnExit, UpgradeCheckStatus, UpgradeConfig, UpgradePolicy,
            UpgradeStrategy, check_cooldown_status, check_for_immediate_upgrade,
            check_for_upgrade, check_thread, format_already_up_to_date_msg,
            format_upgrade_available_notice, format_upgrade_failed_retry_msg,
            get_current_bin_name, perform_upgrade, prompt_user_to_upgrade,
            record_successful_check};
use crate::DEBUG_SELF_UPGRADE;
use r3bl_tui::{TokioRuntimeOutcome, run_in_tokio};
use std::{any::Any,
          panic::{AssertUnwindSafe, catch_unwind},
          process::{ExitCode, Termination}};

/// Convenience macro to construct an [`UpgradeJourney<NotStarted>`]. It creates an
/// [`UpdateConfig`]:
/// 1. Automatically captures the calling crate's compile-time identity via
///    [`env!("CARGO_PKG_NAME")`][env] and `env!("CARGO_PKG_VERSION_*")`.
/// 2. Converts [`UpgradeCLIArgs`] into [`UpgradePolicy`].
/// 3. Passes `None` for the URL overrides (so [`UpgradeConfig::new()`] defaults to
///    [`R3BL_VERSIONS_JSON_URL`] and [`R3BL_BINARIES_RELEASE_BASE_URL`]).
/// 4. Passes [`CooldownPolicy::default()`].
///
/// For full explicit control (such as custom URL overrides or
/// [`CooldownPolicy::Disabled`] / [`CooldownPolicy::CustomTimestampFilePath`] in unit
/// tests), construct [`UpgradeConfig::new()`] (with an explicit [`UpgradePolicy`]) and
/// pass it to [`UpgradeJourney::new()`] directly.
///
/// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// [`self_upgrade`] module documentation.
///
/// # Why provide a declarative macro instead of a constructor?
///
/// 1. **Library vs. binary crate boundary**: Because [`self_upgrade`] lives in the
///    `r3bl-cmdr` library crate and is linked by other binary crates (`r3bl-build-infra`,
///    `r3bl-rust-analyzer-mcp-server`), calling [`env!("CARGO_PKG_NAME")`][env] inside a
///    regular function in [`self_upgrade`], like [`UpgradeConfig::new()`], would expand
///    when compiling `r3bl-cmdr`, and permanently bake `"r3bl-cmdr"`'s name and version
///    into the library!
/// 2. **Call-site expansion**: A `macro_rules!` macro expands at the **invocation site**
///    inside the downstream binary crate, resolving [`env!("CARGO_PKG_NAME")`][env] and
///    `CARGO_PKG_VERSION_*` against that binary's own `Cargo.toml` without forcing
///    callers to pass noisy identity boilerplate.
/// 3. **Not available at runtime**: `CARGO_PKG_NAME` and `CARGO_PKG_VERSION_*` are only
///    set by Cargo during `cargo build`; they do not exist in [`std::env::var`] when a
///    user runs the compiled binary on their machine.
/// 4. **Compile-time version construction (infallible at runtime)**: Cargo's pre-split
///    `CARGO_PKG_VERSION_{MAJOR,MINOR,PATCH,PRE}` variables are converted inside a `const
///    { ... }` block via [`CrateVersion::from_cargo_parts`], so any pre-release or
///    out-of-range `Cargo.toml` version fails at compile time (`cargo check` / `cargo
///    build`) and [`upgrade_journey!`] can never panic or fail at runtime.
///
/// # Examples
///
/// ```
/// use clap::Parser;
/// use r3bl_cmdr::{UpgradeActionOnExit, UpgradeCLIArgs, UpgradePolicy, UpgradeStrategy,
///                 UserFeedbackChannel, upgrade_journey};
///
/// // In production, a binary flattens `UpgradeCLIArgs` into its `CLIArg` struct,
/// // by calling `let cli_arg = CLIArg::parse()`, then passes `cli_arg.upgrade_args`
/// // into `upgrade_journey!`.
/// // In this doctest (where there are no OS CLI args), we call `parse_from`
/// // directly on `UpgradeCLIArgs` to simulate what clap produces:
/// let upgrade_args =
///     UpgradeCLIArgs::parse_from(["my-binary", "--upgrade-behavior", "prompt-to-upgrade"]);
/// let journey = upgrade_journey!(UserFeedbackChannel::Stderr, upgrade_args);
/// assert_eq!(
///     journey.config().get_upgrade_policy(),
///     UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(
///         UpgradeActionOnExit::PromptToUpgrade
///     ))
/// );
/// ```
///
/// [`CooldownPolicy::CustomTimestampFilePath`]: crate::self_upgrade::CooldownPolicy::CustomTimestampFilePath
/// [`CooldownPolicy::default()`]: crate::self_upgrade::CooldownPolicy::default
/// [`CooldownPolicy::Disabled`]: crate::self_upgrade::CooldownPolicy::Disabled
/// [`CrateVersion::from_cargo_parts`]: crate::self_upgrade::CrateVersion::from_cargo_parts
/// [`NotStarted`]: crate::self_upgrade::NotStarted
/// [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::self_upgrade::R3BL_BINARIES_RELEASE_BASE_URL
/// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`UpdateConfig`]: crate::self_upgrade::UpgradeConfig
/// [`upgrade_journey!`]: crate::upgrade_journey
/// [`UpgradeCLIArgs`]: crate::self_upgrade::UpgradeCLIArgs
/// [`UpgradeConfig::new()`]: crate::self_upgrade::UpgradeConfig::new
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
/// [`UpgradeJourney::new()`]: crate::self_upgrade::UpgradeJourney::new
/// [`UpgradeJourney<NotStarted>`]: crate::self_upgrade::UpgradeJourney
/// [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
/// [env]: std::env!
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
#[macro_export]
macro_rules! upgrade_journey {
    ($user_feedback_channel:expr, $upgrade_cli_args:expr) => {
        $crate::UpgradeJourney::<$crate::NotStarted>::new($crate::UpgradeConfig::new(
            $crate::CrateIdentity::new(
                ::core::env!("CARGO_PKG_NAME"),
                // This const block ensures that from_cargo_parts is evaluated at compile
                // time; without it, this expression would evaluate at runtime.
                const {
                    $crate::CrateVersion::from_cargo_parts(
                        ::core::env!("CARGO_PKG_VERSION_MAJOR"),
                        ::core::env!("CARGO_PKG_VERSION_MINOR"),
                        ::core::env!("CARGO_PKG_VERSION_PATCH"),
                        ::core::env!("CARGO_PKG_VERSION_PRE"),
                    )
                },
            ),
            $user_feedback_channel,
            $crate::UpgradePolicy::from($upgrade_cli_args),
            None,
            None,
            $crate::CooldownPolicy::default(),
        ))
    };
}

/// Marker trait implemented by all valid typestates of [`UpgradeJourney`].
///
/// [`UpgradeJourney`]: crate::self_upgrade::UpgradeJourney
pub trait UpgradeJourneyStateMarker {}

/// State machine for the self-upgrade lifecycle using the [Typestate Pattern].
///
/// Holds [`UpgradeConfig`] and the current `State` ([`NotStarted`], [`Disabled`],
/// [`CooldownSkipped`], [`Checking`], [`UpToDate`], [`UpgradeAvailable`],
/// [`CheckFailed`], [`Upgrading`], or [`Completed`]). Because `config`, `state`, and all
/// `into_*` transition methods are module-private (`pub(super)`), external callers cannot
/// forge state transitions; every transition consumes `UpgradeJourney<PrevState>` by
/// value and produces `UpgradeJourney<NextState>`.
///
/// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
/// [`self_upgrade`] module documentation.
///
/// [`CheckFailed`]: crate::self_upgrade::CheckFailed
/// [`Checking`]: crate::self_upgrade::Checking
/// [`Completed`]: crate::self_upgrade::Completed
/// [`CooldownSkipped`]: crate::self_upgrade::CooldownSkipped
/// [`Disabled`]: crate::self_upgrade::Disabled
/// [`NotStarted`]: crate::self_upgrade::NotStarted
/// [`R3BL_BINARIES_RELEASE_BASE_URL`]: crate::self_upgrade::R3BL_BINARIES_RELEASE_BASE_URL
/// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
/// [`self_upgrade`]: mod@crate::self_upgrade
/// [`UpgradeAvailable`]: crate::self_upgrade::UpgradeAvailable
/// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
/// [`Upgrading`]: crate::self_upgrade::Upgrading
/// [`UpToDate`]: crate::self_upgrade::UpToDate
/// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
/// [Typestate Pattern]: https://stanford-cs242.github.io/f19/lectures/08-2-typestate.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeJourney<State: UpgradeJourneyStateMarker> {
    config: UpgradeConfig,
    state: State,
}

impl<State: UpgradeJourneyStateMarker> UpgradeJourney<State> {
    /// Returns a reference to the immutable [`UpgradeConfig`] for this journey.
    ///
    /// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
    #[must_use]
    pub fn config(&self) -> &UpgradeConfig { &self.config }

    /// Returns a reference to the current typestate payload `State`.
    #[must_use]
    pub fn state(&self) -> &State { &self.state }
}

/// Initial typestate before [`UpgradeJourney::start`] is called.
///
/// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NotStarted;
impl UpgradeJourneyStateMarker for NotStarted {}

/// Typestate when upgrade checks are disabled via [`UpgradePolicy::Disabled`].
///
/// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disabled;
impl UpgradeJourneyStateMarker for Disabled {}

/// Typestate when an [`UpgradeStrategy::DedicatedThread`] check is bypassed because the
/// `1h` `mtime` cooldown timestamp file is still active.
///
/// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CooldownSkipped;
impl UpgradeJourneyStateMarker for CooldownSkipped {}

/// Typestate while querying [`R3BL_VERSIONS_JSON_URL`] (either synchronously via
/// [`UpgradeStrategy::Immediate`] or on the dedicated check thread via
/// [`UpgradeStrategy::DedicatedThread`]).
///
/// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
/// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checking {
    /// Active [`UpgradeStrategy`] (`Immediate` or
    /// `DedicatedThread(UpgradeActionOnExit)`).
    ///
    /// [`UpgradeStrategy`]: crate::self_upgrade::UpgradeStrategy
    pub strategy: UpgradeStrategy,
}
impl UpgradeJourneyStateMarker for Checking {}

/// Typestate when the remote manifest was checked and `remote_version <= local_version`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpToDate {
    /// Remote [`CrateMetadata`] retrieved from [`R3BL_VERSIONS_JSON_URL`].
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    /// [`R3BL_VERSIONS_JSON_URL`]: crate::self_upgrade::R3BL_VERSIONS_JSON_URL
    pub metadata: CrateMetadata,
}
impl UpgradeJourneyStateMarker for UpToDate {}

/// Typestate when an [`UpgradeStrategy::DedicatedThread`] check found
/// `remote_version > local_version` and awaits
/// [`UpgradeCheckStatus::handle_upgrade_on_exit`].
///
/// [`UpgradeStrategy::Immediate`] transitions directly from [`Checking`] to
/// [`Upgrading`], so `Immediate` is unrepresentable at application exit.
///
/// [`Checking`]: crate::self_upgrade::Checking
/// [`UpgradeCheckStatus::handle_upgrade_on_exit`]: crate::self_upgrade::UpgradeCheckStatus::handle_upgrade_on_exit
/// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`Upgrading`]: crate::self_upgrade::Upgrading
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeAvailable {
    /// Active upgrade action to execute at application exit.
    pub action_on_exit: UpgradeActionOnExit,

    /// Verified remote [`CrateMetadata`] where
    /// `metadata.version > config.get_current_version()`.
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    pub metadata: CrateMetadata,
}
impl UpgradeJourneyStateMarker for UpgradeAvailable {}

/// Typestate when the manifest check encountered a network, parse, or thread/runtime
/// failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFailed {
    /// Full diagnostic chain formatted via `format!("{err:#}")`.
    pub error: String,
}
impl UpgradeJourneyStateMarker for CheckFailed {}

/// Typestate while an upgrade or interactive exit prompt is actively executing after the
/// `SINGLETON` leaf lock has been released (`Checking -> Upgrading` for
/// [`UpgradeStrategy::Immediate`]; `UpgradeAvailable -> Upgrading` for
/// [`UpgradeStrategy::DedicatedThread`]).
///
/// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgrading {
    /// Verified remote [`CrateMetadata`] being installed or presented to the user.
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    pub metadata: CrateMetadata,
}
impl UpgradeJourneyStateMarker for Upgrading {}

/// Terminal outcome recorded inside [`Completed`] after an immediate upgrade or exit
/// upgrade action finishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeResolution {
    /// Binary replacement or source installation succeeded (and the `1h` cooldown
    /// timestamp file was refreshed).
    Upgraded,

    /// Download, checksum verification, swap, or source compilation failed; carries the
    /// full `format!("{err:#}")` diagnostic string.
    UpgradeFailed(String),

    /// User selected `"No, thanks"` or dismissed the interactive prompt.
    DeclinedByUser,

    /// [`UpgradeActionOnExit::PromptToUpgrade`] was configured, and the session was
    /// non-interactive so the modal prompt was skipped without blocking.
    ///
    /// [`UpgradeActionOnExit::PromptToUpgrade`]: crate::self_upgrade::UpgradeActionOnExit::PromptToUpgrade
    SkippedNonInteractive,

    /// [`UpgradeActionOnExit::NotifyOnly`] printed the one-line availability notice.
    ///
    /// [`UpgradeActionOnExit::NotifyOnly`]: crate::self_upgrade::UpgradeActionOnExit::NotifyOnly
    NotifiedOnly,
}

/// Terminal typestate after an immediate upgrade or exit upgrade action finishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed {
    /// Remote [`CrateMetadata`] targeted by the upgrade flow.
    ///
    /// [`CrateMetadata`]: crate::self_upgrade::CrateMetadata
    pub metadata: CrateMetadata,

    /// Terminal [`UpgradeResolution`] describing how the upgrade flow concluded.
    pub resolution: UpgradeResolution,
}
impl UpgradeJourneyStateMarker for Completed {}

/// Outcome of an explicit `--upgrade` ([`UpgradeStrategy::Immediate`]) run inside
/// [`StartupOutcome::UpgradedAndExit`].
///
/// Uses a `Copy` domain enum instead of `miette::Result<()>` so [`UserFeedbackChannel`]
/// remains the single output emitter (preventing `miette`'s `Termination` handler from
/// printing duplicate `Error:` diagnostics on [`stderr`] or leaking output when
/// [`UserFeedbackChannel::Quiet`] is configured).
///
/// [`stderr`]: std::io::stderr
/// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
/// [`UserFeedbackChannel::Quiet`]: crate::self_upgrade::UserFeedbackChannel::Quiet
/// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImmediateUpgradeResult {
    /// Binary was already up to date or upgraded cleanly (`exit 0`).
    Succeeded,

    /// Manifest check or binary installation failed (`exit 1`); user-facing retry
    /// feedback has already been emitted via [`UserFeedbackChannel`].
    ///
    /// [`UserFeedbackChannel`]: crate::self_upgrade::UserFeedbackChannel
    Failed,
}

impl ImmediateUpgradeResult {
    /// Maps this result to the corresponding process [`ExitCode`] (`0` for
    /// [`Self::Succeeded`], `1` for [`Self::Failed`]).
    pub const fn exit_code(self) -> ExitCode {
        match self {
            Self::Succeeded => ExitCode::SUCCESS,
            Self::Failed => ExitCode::FAILURE,
        }
    }
}

impl Termination for ImmediateUpgradeResult {
    fn report(self) -> ExitCode { self.exit_code() }
}

/// Outcome returned by [`UpgradeJourney::<NotStarted>::start`][start], telling the
/// binary's `main()` whether `--upgrade` executed immediately, normal application
/// execution should continue, or `start()` was already called earlier in this process.
///
/// [start]: crate::self_upgrade::UpgradeJourney::start
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupOutcome {
    /// `--upgrade` ([`UpgradeStrategy::Immediate`]) executed; `main()` should exit with
    /// [`ImmediateUpgradeResult::exit_code`] (`0` on [`Succeeded`], `1` on [`Failed`]).
    ///
    /// [`Failed`]: ImmediateUpgradeResult::Failed
    /// [`Succeeded`]: ImmediateUpgradeResult::Succeeded
    /// [`UpgradeStrategy::Immediate`]: crate::self_upgrade::UpgradeStrategy::Immediate
    UpgradedAndExit(ImmediateUpgradeResult),

    /// Normal startup ([`UpgradeStrategy::DedicatedThread`] or
    /// [`UpgradePolicy::Disabled`]); `main()` should continue running the application.
    ///
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    /// [`UpgradeStrategy::DedicatedThread`]: crate::self_upgrade::UpgradeStrategy::DedicatedThread
    ContinueApplication,

    /// [`UpgradeJourney::start`] was already called earlier in this process; the
    /// duplicate call spawned zero threads and left [`UpgradeCheckStatus`] unchanged.
    ///
    /// [`UpgradeCheckStatus`]: crate::self_upgrade::UpgradeCheckStatus
    /// [`UpgradeJourney::start`]: crate::self_upgrade::UpgradeJourney::start
    AlreadyStarted,
}

impl UpgradeJourney<NotStarted> {
    /// Constructs a new [`UpgradeJourney<NotStarted>`] from immutable [`UpgradeConfig`].
    ///
    /// In CLI binaries, prefer [`upgrade_journey!`], which accepts [`UpgradeCLIArgs`] and
    /// converts it into [`UpgradePolicy`] inside [`UpgradeConfig`] at the CLI boundary.
    ///
    /// [`upgrade_journey!`]: crate::upgrade_journey
    /// [`UpgradeCLIArgs`]: crate::self_upgrade::UpgradeCLIArgs
    /// [`UpgradeConfig`]: crate::self_upgrade::UpgradeConfig
    /// [`UpgradePolicy`]: crate::self_upgrade::UpgradePolicy
    #[must_use]
    pub fn new(config: UpgradeConfig) -> Self {
        Self {
            config,
            state: NotStarted,
        }
    }

    /// Single entry point for the self-upgrade engine. Atomically claims
    /// [`UpgradeCheckStatus::NotStarted`] and dispatches based on
    /// [`UpgradeConfig::get_upgrade_policy()`]:
    ///
    /// - **Duplicate `.start()` call**: Leaves [`UpgradeCheckStatus`] untouched, spawns
    ///   zero threads, logs via [`DEBUG_SELF_UPGRADE`], and returns
    ///   [`StartupOutcome::AlreadyStarted`].
    /// - **`UpgradePolicy::Enabled(UpgradeStrategy::Immediate)`** (`--upgrade`): Queries
    ///   the manifest with a `5s` timeout (bypassing the `1h` cooldown), runs
    ///   [`perform_upgrade`] if a newer version exists, updates [`UpgradeCheckStatus`]
    ///   through [`Checking`] -> [`UpToDate`] / [`CheckFailed`] / [`Upgrading`] ->
    ///   [`Completed`], touches the `1h` cooldown timestamp file on [`UpToDate`] or
    ///   [`UpgradeResolution::Upgraded`], and returns
    ///   [`StartupOutcome::UpgradedAndExit`].
    /// - **`UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(action_on_exit))`**:
    ///   Checks the `1h` `mtime` cooldown timestamp file via [`check_cooldown_status`].
    ///   If active, records [`UpgradeCheckStatus::CooldownSkipped`] with zero threads and
    ///   returns [`StartupOutcome::ContinueApplication`]. Otherwise records
    ///   [`UpgradeCheckStatus::Checking`], spawns a single `"self-upgrade-check"` OS
    ///   thread (storing its `JoinHandle` for
    ///   [`UpgradeCheckStatus::handle_upgrade_on_exit`]), and returns
    ///   [`StartupOutcome::ContinueApplication`].
    /// - **[`UpgradePolicy::Disabled`]** (`--upgrade-behavior disable`): Records
    ///   [`UpgradeCheckStatus::Disabled`] with zero threads and zero I/O, returning
    ///   [`StartupOutcome::ContinueApplication`].
    ///
    /// See [Main Entry Point: `UpgradeJourney` & Typestate Lifecycle][ep] in the
    /// [`self_upgrade`] module documentation.
    ///
    /// [`check_cooldown_status`]: crate::self_upgrade::check_cooldown_status
    /// [`CheckFailed`]: crate::self_upgrade::CheckFailed
    /// [`Checking`]: crate::self_upgrade::Checking
    /// [`Completed`]: crate::self_upgrade::Completed
    /// [`DEBUG_SELF_UPGRADE`]: crate::DEBUG_SELF_UPGRADE
    /// [`perform_upgrade`]: crate::self_upgrade::perform_upgrade
    /// [`self_upgrade`]: mod@crate::self_upgrade
    /// [`UpgradeCheckStatus::Checking`]: crate::self_upgrade::UpgradeCheckStatus::Checking
    /// [`UpgradeCheckStatus::CooldownSkipped`]: crate::self_upgrade::UpgradeCheckStatus::CooldownSkipped
    /// [`UpgradeCheckStatus::Disabled`]: crate::self_upgrade::UpgradeCheckStatus::Disabled
    /// [`UpgradeCheckStatus::handle_upgrade_on_exit`]: crate::self_upgrade::UpgradeCheckStatus::handle_upgrade_on_exit
    /// [`UpgradeCheckStatus::NotStarted`]: crate::self_upgrade::UpgradeCheckStatus::NotStarted
    /// [`UpgradeCheckStatus`]: crate::self_upgrade::UpgradeCheckStatus
    /// [`UpgradeConfig::get_upgrade_policy()`]: crate::self_upgrade::UpgradeConfig::get_upgrade_policy
    /// [`UpgradePolicy::Disabled`]: crate::self_upgrade::UpgradePolicy::Disabled
    /// [`UpgradeResolution::Upgraded`]: crate::self_upgrade::UpgradeResolution::Upgraded
    /// [`Upgrading`]: crate::self_upgrade::Upgrading
    /// [`UpToDate`]: crate::self_upgrade::UpToDate
    /// [ep]: mod@crate::self_upgrade#main-entry-point-upgradejourney--typestate-lifecycle
    #[must_use]
    pub fn start(self) -> StartupOutcome {
        match self.config.get_upgrade_policy() {
            UpgradePolicy::Disabled => {
                let config_for_log = self.config.clone();
                let disabled = self.into_disabled();
                if !UpgradeCheckStatus::try_claim_not_started(
                    UpgradeCheckStatus::Disabled(disabled),
                ) {
                    return log_duplicate_start(&config_for_log);
                }
                StartupOutcome::ContinueApplication
            }
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate) => {
                let config_for_log = self.config.clone();
                let checking = self.into_checking(UpgradeStrategy::Immediate);
                if !UpgradeCheckStatus::try_claim_not_started(
                    UpgradeCheckStatus::Checking(checking.clone()),
                ) {
                    return log_duplicate_start(&config_for_log);
                }
                StartupOutcome::UpgradedAndExit(Self::run_immediate_upgrade(checking))
            }
            UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(action_on_exit)) => {
                if !UpgradeCheckStatus::is_not_started() {
                    return log_duplicate_start(&self.config);
                }

                let config_for_log = self.config.clone();
                match check_cooldown_status(&self.config) {
                    CooldownStatus::Active => {
                        let skipped = self.into_cooldown_skipped();
                        if !UpgradeCheckStatus::try_claim_not_started(
                            UpgradeCheckStatus::CooldownSkipped(skipped),
                        ) {
                            return log_duplicate_start(&config_for_log);
                        }
                        StartupOutcome::ContinueApplication
                    }
                    CooldownStatus::ExpiredOrMissing => {
                        let checking = self.into_checking(
                            UpgradeStrategy::DedicatedThread(action_on_exit),
                        );
                        if !UpgradeCheckStatus::try_claim_not_started(
                            UpgradeCheckStatus::Checking(checking.clone()),
                        ) {
                            return log_duplicate_start(&config_for_log);
                        }

                        let spawn_checking = checking.clone();
                        match std::thread::Builder::new()
                            .name(check_thread::CHECK_THREAD_NAME.to_string())
                            .spawn(move || {
                                run_on_dedicated_check_thread(
                                    spawn_checking,
                                    check_for_upgrade,
                                );
                            }) {
                            Ok(handle) => {
                                UpgradeCheckStatus::set_thread_check_handle(handle);
                            }
                            Err(io_err) => {
                                let error = format!(
                                    "Failed to spawn dedicated self-upgrade check thread: {io_err:#}"
                                );
                                DEBUG_SELF_UPGRADE.then(|| {
                                    // % is Display, ? is Debug.
                                    tracing::debug!(
                                        message = "Could not spawn dedicated self-upgrade check thread",
                                        crate_name = %checking.config().get_crate_name(),
                                        error = %error
                                    );
                                });
                                UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                                    checking.into_check_failed(error),
                                ));
                            }
                        }

                        StartupOutcome::ContinueApplication
                    }
                }
            }
        }
    }

    #[must_use]
    pub(super) fn into_disabled(self) -> UpgradeJourney<Disabled> {
        UpgradeJourney {
            config: self.config,
            state: Disabled,
        }
    }

    #[must_use]
    pub(super) fn into_cooldown_skipped(self) -> UpgradeJourney<CooldownSkipped> {
        UpgradeJourney {
            config: self.config,
            state: CooldownSkipped,
        }
    }

    #[must_use]
    pub(super) fn into_checking(
        self,
        strategy: UpgradeStrategy,
    ) -> UpgradeJourney<Checking> {
        UpgradeJourney {
            config: self.config,
            state: Checking { strategy },
        }
    }

    fn run_immediate_upgrade(
        checking: UpgradeJourney<Checking>,
    ) -> ImmediateUpgradeResult {
        let config = checking.config().clone();
        let fallback_checking = checking.clone();

        match run_in_tokio_logged(&config, async move || -> ImmediateUpgradeResult {
            match check_for_immediate_upgrade(checking).await {
                ImmediateCheckOutcome::CheckFailed(failed) => {
                    let bin_name = get_current_bin_name();
                    let msg =
                        format_upgrade_failed_retry_msg(&bin_name, &failed.state().error);
                    failed.config().get_user_feedback_channel().eprintln(msg);
                    ImmediateUpgradeResult::Failed
                }
                ImmediateCheckOutcome::UpToDate(up_to_date) => {
                    up_to_date.config().get_user_feedback_channel().println(
                        format_already_up_to_date_msg(
                            up_to_date.config().get_crate_name(),
                            &up_to_date.config().get_current_version().to_string(),
                        ),
                    );
                    ImmediateUpgradeResult::Succeeded
                }
                ImmediateCheckOutcome::Upgrading(upgrading) => {
                    match perform_upgrade(upgrading.config(), &upgrading.state().metadata)
                        .await
                    {
                        Ok(()) => {
                            record_successful_check(upgrading.config());
                            let completed =
                                upgrading.into_completed(UpgradeResolution::Upgraded);
                            UpgradeCheckStatus::set(UpgradeCheckStatus::Completed(
                                completed,
                            ));
                            ImmediateUpgradeResult::Succeeded
                        }
                        Err(err) => {
                            let completed = upgrading.into_completed(
                                UpgradeResolution::UpgradeFailed(format!("{err:#}")),
                            );
                            UpgradeCheckStatus::set(UpgradeCheckStatus::Completed(
                                completed,
                            ));
                            ImmediateUpgradeResult::Failed
                        }
                    }
                }
            }
        }) {
            TokioRuntimeOutcome::Completed(result) => result,
            TokioRuntimeOutcome::FailedToStart(err)
            | TokioRuntimeOutcome::Panicked(err) => {
                let error = format!("{err:#}");
                let bin_name = get_current_bin_name();
                config
                    .get_user_feedback_channel()
                    .eprintln(format_upgrade_failed_retry_msg(&bin_name, &error));

                if let UpgradeCheckStatus::Upgrading(upgrading) =
                    UpgradeCheckStatus::get()
                {
                    UpgradeCheckStatus::set(UpgradeCheckStatus::Completed(
                        upgrading.into_completed(UpgradeResolution::UpgradeFailed(error)),
                    ));
                } else {
                    UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                        fallback_checking.into_check_failed(error),
                    ));
                }
                ImmediateUpgradeResult::Failed
            }
        }
    }
}

impl UpgradeJourney<Checking> {
    #[must_use]
    pub(super) fn into_up_to_date(
        self,
        metadata: CrateMetadata,
    ) -> UpgradeJourney<UpToDate> {
        UpgradeJourney {
            config: self.config,
            state: UpToDate { metadata },
        }
    }

    #[must_use]
    pub(super) fn into_upgrade_available(
        self,
        action_on_exit: UpgradeActionOnExit,
        metadata: CrateMetadata,
    ) -> UpgradeJourney<UpgradeAvailable> {
        UpgradeJourney {
            config: self.config,
            state: UpgradeAvailable {
                action_on_exit,
                metadata,
            },
        }
    }

    #[must_use]
    pub(super) fn into_upgrading(
        self,
        metadata: CrateMetadata,
    ) -> UpgradeJourney<Upgrading> {
        UpgradeJourney {
            config: self.config,
            state: Upgrading { metadata },
        }
    }

    #[must_use]
    pub(super) fn into_check_failed(self, error: String) -> UpgradeJourney<CheckFailed> {
        UpgradeJourney {
            config: self.config,
            state: CheckFailed { error },
        }
    }
}

impl UpgradeJourney<UpgradeAvailable> {
    #[must_use]
    pub(super) fn into_upgrading(self) -> UpgradeJourney<Upgrading> {
        UpgradeJourney {
            config: self.config,
            state: Upgrading {
                metadata: self.state.metadata,
            },
        }
    }
}

impl UpgradeJourney<Upgrading> {
    #[must_use]
    pub(super) fn into_completed(
        self,
        resolution: UpgradeResolution,
    ) -> UpgradeJourney<Completed> {
        UpgradeJourney {
            config: self.config,
            state: Completed {
                metadata: self.state.metadata,
                resolution,
            },
        }
    }

    /// Executes the upgrade action on exit (`NotifyOnly`, `Auto`, or `PromptToUpgrade`)
    /// outside the `SINGLETON` leaf lock, touches the `1h` cooldown timestamp file if and
    /// only if the upgrade succeeded, and consumes `self` into
    /// [`UpgradeJourney<Completed>`].
    #[must_use]
    pub(super) fn execute_action_on_exit(
        self,
        action_on_exit: UpgradeActionOnExit,
    ) -> UpgradeJourney<Completed> {
        let bin_name = get_current_bin_name();
        let current_version_str = self.config.get_current_version().to_string();
        let remote_version_str = self.state.metadata.version.to_string();

        self.config
            .get_user_feedback_channel()
            .println(format_upgrade_available_notice(
                &bin_name,
                &current_version_str,
                &remote_version_str,
            ));

        let resolution = match action_on_exit {
            UpgradeActionOnExit::NotifyOnly => UpgradeResolution::NotifiedOnly,
            UpgradeActionOnExit::Auto => {
                let config = &self.config;
                let metadata = &self.state.metadata;
                match run_in_tokio_logged(config, || perform_upgrade(config, metadata)) {
                    TokioRuntimeOutcome::Completed(Ok(())) => UpgradeResolution::Upgraded,
                    TokioRuntimeOutcome::Completed(Err(err)) => {
                        UpgradeResolution::UpgradeFailed(format!("{err:#}"))
                    }
                    TokioRuntimeOutcome::FailedToStart(err)
                    | TokioRuntimeOutcome::Panicked(err) => {
                        UpgradeResolution::UpgradeFailed(format!("{err:#}"))
                    }
                }
            }
            UpgradeActionOnExit::PromptToUpgrade => {
                let config = &self.config;
                let metadata = &self.state.metadata;
                match run_in_tokio_logged(config, async || match prompt_user_to_upgrade(
                    config.get_crate_name(),
                    &remote_version_str,
                )
                .await
                {
                    PromptUpgradeDecision::SkippedNonInteractive => {
                        UpgradeResolution::SkippedNonInteractive
                    }
                    PromptUpgradeDecision::DeclinedByUser => {
                        UpgradeResolution::DeclinedByUser
                    }
                    PromptUpgradeDecision::UpgradeNow => {
                        match perform_upgrade(config, metadata).await {
                            Ok(()) => UpgradeResolution::Upgraded,
                            Err(err) => {
                                UpgradeResolution::UpgradeFailed(format!("{err:#}"))
                            }
                        }
                    }
                }) {
                    TokioRuntimeOutcome::Completed(res) => res,
                    TokioRuntimeOutcome::FailedToStart(err)
                    | TokioRuntimeOutcome::Panicked(err) => {
                        UpgradeResolution::UpgradeFailed(format!("{err:#}"))
                    }
                }
            }
        };

        if matches!(resolution, UpgradeResolution::Upgraded) {
            record_successful_check(&self.config);
        }

        self.into_completed(resolution)
    }
}

fn log_duplicate_start(config: &UpgradeConfig) -> StartupOutcome {
    DEBUG_SELF_UPGRADE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "UpgradeJourney::start called more than once in this process; ignoring duplicate",
            crate_name = %config.get_crate_name(),
        );
    });
    StartupOutcome::AlreadyStarted
}

/// Wrapper around [`run_in_tokio`] that logs runtime startup or panic failures via
/// [`DEBUG_SELF_UPGRADE`].
fn run_in_tokio_logged<T: Send, F: Send + AsyncFnOnce() -> T>(
    config: &UpgradeConfig,
    async_fn: F,
) -> TokioRuntimeOutcome<T> {
    let outcome = run_in_tokio(async_fn);
    if let TokioRuntimeOutcome::FailedToStart(ref report)
    | TokioRuntimeOutcome::Panicked(ref report) = outcome
    {
        DEBUG_SELF_UPGRADE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "run_in_tokio_logged: Tokio runtime execution failed",
                crate_name = %config.get_crate_name(),
                error = ?report
            );
        });
    }
    outcome
}

/// Runs `async_fn` directly on the dedicated `"self-upgrade-check"` OS thread using a
/// single-threaded Tokio runtime inside [`catch_unwind`], avoiding spawning a nested
/// second OS thread while guaranteeing [`UpgradeCheckStatus::CheckFailed`] on runtime
/// initialization failure or panic.
fn run_on_dedicated_check_thread<F>(checking: UpgradeJourney<Checking>, async_fn: F)
where
    F: AsyncFnOnce(UpgradeJourney<Checking>),
{
    let fallback_checking = checking.clone();
    let unwind_res = catch_unwind(AssertUnwindSafe(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(|runtime| {
                runtime.block_on(async_fn(checking));
            })
    }));

    match unwind_res {
        Ok(Ok(())) => {}
        Ok(Err(io_err)) => {
            let error = format!("Failed to build Tokio runtime: {io_err:#}");
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Dedicated self-upgrade check thread could not build Tokio runtime",
                    crate_name = %fallback_checking.config().get_crate_name(),
                    error = %error
                );
            });
            UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                fallback_checking.into_check_failed(error),
            ));
        }
        Err(panic_payload) => {
            let panic_msg = extract_panic_payload_message(panic_payload.as_ref());
            let error =
                format!("Dedicated self-upgrade check thread panicked: {panic_msg}");
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Dedicated self-upgrade check thread panicked",
                    crate_name = %fallback_checking.config().get_crate_name(),
                    error = %error
                );
            });
            UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                fallback_checking.into_check_failed(error),
            ));
        }
    }
}

fn extract_panic_payload_message<'a>(
    panic_payload: &'a (dyn Any + Send + 'static),
) -> &'a str {
    let try_as_str = panic_payload.downcast_ref::<&str>();
    let try_as_string = panic_payload.downcast_ref::<String>();
    match (try_as_str, try_as_string) {
        (Some(&s), _) => s,
        (_, Some(s)) => s.as_str(),
        (None, None) => "unknown panic payload",
    }
}

#[cfg(test)]
mod tests {
    use super::{super::{CooldownPolicy, CrateIdentity, CrateName, CrateVersion,
                        UpgradeCLIArgs, UserFeedbackChannel},
                *};
    use r3bl_tui::{assert_eq2, try_create_temp_dir};
    use serial_test::serial;
    use std::{fs::File,
              io::{Read, Write},
              net::TcpListener,
              time::{Duration, SystemTime}};

    fn format_manifest_json(version: &str, distribution: &str) -> String {
        format!(
            r#"{{
                "crates": {{
                    "r3bl-cmdr": {{
                        "version": "{version}",
                        "crate_distribution": "{distribution}",
                        "released_on": "2026-04-06T12:00:00Z"
                    }}
                }}
            }}"#
        )
    }

    fn spawn_one_shot_http_server(
        status: &'static str,
        body: String,
        delay: Duration,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut req_buf = [0_u8; 1024];
                drop(stream.read(&mut req_buf));
                if !delay.is_zero() {
                    std::thread::sleep(delay);
                }
                let resp = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                drop(stream.write_all(resp.as_bytes()));
                drop(stream.flush());
            }
        });
        (format!("http://{addr}/versions.jsonc"), handle)
    }

    #[test]
    fn test_upgrade_journey_macro_defaults_and_policy_conversion() {
        use clap::Parser;

        let default_args = UpgradeCLIArgs::try_parse_from(["test-bin"]).unwrap();
        let immediate_args =
            UpgradeCLIArgs::try_parse_from(["test-bin", "--upgrade"]).unwrap();

        let default_journey = upgrade_journey!(UserFeedbackChannel::Quiet, default_args);
        assert_eq2!(
            default_journey.config().get_crate_name(),
            &CrateName::from(env!("CARGO_PKG_NAME"))
        );
        assert_eq2!(
            default_journey.config().get_cooldown_policy(),
            &CooldownPolicy::DefaultCacheDir
        );
        assert_eq2!(
            default_journey.config().get_upgrade_policy(),
            UpgradePolicy::default()
        );
        assert_eq2!(default_journey.state(), &NotStarted);

        let immediate_journey =
            upgrade_journey!(UserFeedbackChannel::Stderr, immediate_args);
        assert_eq2!(
            immediate_journey.config().get_upgrade_policy(),
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate)
        );
    }

    #[test]
    #[serial]
    fn test_duplicate_start_returns_already_started_without_mutating_singleton() {
        UpgradeCheckStatus::reset_for_testing();

        let make_config = |policy: UpgradePolicy| {
            UpgradeConfig::new(
                CrateIdentity::new(
                    "r3bl-cmdr",
                    CrateVersion::try_from("0.0.25").unwrap(),
                ),
                UserFeedbackChannel::Quiet,
                policy,
                None,
                None,
                CooldownPolicy::Disabled,
            )
        };

        let disabled_config = make_config(UpgradePolicy::Disabled);
        let first = UpgradeJourney::new(disabled_config.clone());
        assert_eq2!(first.start(), StartupOutcome::ContinueApplication);

        let expected_status = UpgradeCheckStatus::Disabled(
            UpgradeJourney::new(disabled_config).into_disabled(),
        );
        assert_eq2!(UpgradeCheckStatus::get(), expected_status);

        // Duplicate Immediate call must return AlreadyStarted and leave SINGLETON
        // untouched.
        let dup_immediate = UpgradeJourney::new(make_config(UpgradePolicy::Enabled(
            UpgradeStrategy::Immediate,
        )));
        assert_eq2!(dup_immediate.start(), StartupOutcome::AlreadyStarted);
        assert_eq2!(UpgradeCheckStatus::get(), expected_status);

        // Duplicate DedicatedThread call must also return AlreadyStarted with 0 threads.
        let dup_thread = UpgradeJourney::new(make_config(UpgradePolicy::Enabled(
            UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::Auto),
        )));
        assert_eq2!(dup_thread.start(), StartupOutcome::AlreadyStarted);
        assert_eq2!(UpgradeCheckStatus::get(), expected_status);
    }

    #[test]
    #[serial]
    fn test_path1_immediate_up_to_date_sync_touches_and_bypasses_cooldown() {
        UpgradeCheckStatus::reset_for_testing();
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("r3bl-cmdr.last_check");

        // Pre-populate an active cooldown timestamp file to verify
        // `UpgradeStrategy::Immediate` (`--upgrade`) bypasses it and still performs the
        // live HTTP check.
        let (url, server_handle) = spawn_one_shot_http_server(
            "200 OK",
            format_manifest_json("0.0.25", "open-source"),
            Duration::ZERO,
        );
        let config = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            Some(url),
            None,
            CooldownPolicy::CustomTimestampFilePath(timestamp_file_path.clone()),
        );
        record_successful_check(&config);
        let backdated_within_cooldown = SystemTime::now() - Duration::from_mins(30);
        File::open(&timestamp_file_path)
            .unwrap()
            .set_modified(backdated_within_cooldown)
            .unwrap();

        let outcome = UpgradeJourney::new(config).start();
        server_handle.join().unwrap();

        assert_eq2!(
            outcome,
            StartupOutcome::UpgradedAndExit(ImmediateUpgradeResult::Succeeded)
        );
        assert_eq2!(
            ImmediateUpgradeResult::Succeeded.exit_code(),
            ExitCode::SUCCESS
        );
        assert_eq2!(
            ImmediateUpgradeResult::Succeeded.report(),
            ExitCode::SUCCESS
        );
        assert!(matches!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::UpToDate(_)
        ));

        // Verify the timestamp file `mtime` was refreshed by the live `UpToDate` check.
        let refreshed_mtime = std::fs::metadata(&timestamp_file_path)
            .unwrap()
            .modified()
            .unwrap();
        assert!(refreshed_mtime > backdated_within_cooldown);
    }

    #[tokio::test]
    #[serial]
    async fn test_path1_immediate_up_to_date_async() {
        UpgradeCheckStatus::reset_for_testing();
        let (url, server_handle) = spawn_one_shot_http_server(
            "200 OK",
            format_manifest_json("0.0.25", "open-source"),
            Duration::ZERO,
        );
        let config = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            Some(url),
            None,
            CooldownPolicy::Disabled,
        );

        let outcome = UpgradeJourney::new(config).start();
        server_handle.join().unwrap();

        assert_eq2!(
            outcome,
            StartupOutcome::UpgradedAndExit(ImmediateUpgradeResult::Succeeded)
        );
        assert!(matches!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::UpToDate(_)
        ));
    }

    #[test]
    #[serial]
    fn test_path1_immediate_check_failed_and_upgrade_failed_transitions() {
        UpgradeCheckStatus::reset_for_testing();
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("failed.last_check");

        // 1. Manifest HTTP 500 -> CheckFailed + Failed exit code, no cooldown timestamp
        //    file.
        let (bad_url, bad_handle) = spawn_one_shot_http_server(
            "500 Internal Server Error",
            "server error".to_string(),
            Duration::ZERO,
        );
        let fail_cfg = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            Some(bad_url),
            None,
            CooldownPolicy::CustomTimestampFilePath(timestamp_file_path.clone()),
        );
        let fail_outcome = UpgradeJourney::new(fail_cfg).start();
        bad_handle.join().unwrap();

        assert_eq2!(
            fail_outcome,
            StartupOutcome::UpgradedAndExit(ImmediateUpgradeResult::Failed)
        );
        assert_eq2!(
            ImmediateUpgradeResult::Failed.exit_code(),
            ExitCode::FAILURE
        );
        assert!(!timestamp_file_path.exists());
        match UpgradeCheckStatus::get() {
            UpgradeCheckStatus::CheckFailed(journey) => {
                assert!(!journey.state().error.is_empty());
            }
            other => panic!("Expected CheckFailed, got {other:?}"),
        }

        // 2. Newer ClosedSource version + unreachable release archive server ->
        // transitions Checking -> Upgrading -> Completed(UpgradeFailed), no timestamp
        // file.
        UpgradeCheckStatus::reset_for_testing();
        let (newer_url, newer_handle) = spawn_one_shot_http_server(
            "200 OK",
            format_manifest_json("9.9.9", "closed-source"),
            Duration::ZERO,
        );
        let closed_release_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let closed_release_addr = closed_release_listener.local_addr().unwrap();
        drop(closed_release_listener);

        let upg_fail_cfg = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::Immediate),
            Some(newer_url),
            Some(format!("http://{closed_release_addr}/releases/download")),
            CooldownPolicy::CustomTimestampFilePath(timestamp_file_path.clone()),
        );
        let upg_fail_outcome = UpgradeJourney::new(upg_fail_cfg).start();
        newer_handle.join().unwrap();

        assert_eq2!(
            upg_fail_outcome,
            StartupOutcome::UpgradedAndExit(ImmediateUpgradeResult::Failed)
        );
        assert!(!timestamp_file_path.exists());
        match UpgradeCheckStatus::get() {
            UpgradeCheckStatus::Completed(journey) => {
                assert_eq2!(
                    journey.state().metadata.version,
                    CrateVersion::try_from("9.9.9").unwrap()
                );
                assert!(matches!(
                    journey.state().resolution,
                    UpgradeResolution::UpgradeFailed(ref err) if !err.is_empty()
                ));
            }
            other => panic!("Expected Completed(UpgradeFailed), got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn test_path2_dedicated_thread_cooldown_skip_and_expiry() {
        UpgradeCheckStatus::reset_for_testing();
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("cooldown.last_check");

        // 1. Active 1h cooldown timestamp file -> CooldownSkipped with 0 network/thread
        //    work.
        let active_cfg = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(
                UpgradeActionOnExit::Auto,
            )),
            Some("http://127.0.0.1:0/never_called.jsonc".to_string()),
            None,
            CooldownPolicy::CustomTimestampFilePath(timestamp_file_path.clone()),
        );
        record_successful_check(&active_cfg);

        let skip_outcome = UpgradeJourney::new(active_cfg).start();
        assert_eq2!(skip_outcome, StartupOutcome::ContinueApplication);
        assert!(matches!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::CooldownSkipped(_)
        ));
        UpgradeCheckStatus::handle_upgrade_on_exit();
        assert!(matches!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::CooldownSkipped(_)
        ));

        // 2. Expired timestamp file (`now - 2h`) -> spawns check thread, queries server,
        //    refreshes timestamp file.
        UpgradeCheckStatus::reset_for_testing();
        let expired_time = SystemTime::now() - Duration::from_secs(7200);
        File::open(&timestamp_file_path)
            .unwrap()
            .set_modified(expired_time)
            .unwrap();

        let (url, server_handle) = spawn_one_shot_http_server(
            "200 OK",
            format_manifest_json("0.0.25", "open-source"),
            Duration::ZERO,
        );
        let expired_cfg = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(
                UpgradeActionOnExit::Auto,
            )),
            Some(url),
            None,
            CooldownPolicy::CustomTimestampFilePath(timestamp_file_path.clone()),
        );

        let run_outcome = UpgradeJourney::new(expired_cfg.clone()).start();
        assert_eq2!(run_outcome, StartupOutcome::ContinueApplication);

        UpgradeCheckStatus::handle_upgrade_on_exit();
        server_handle.join().unwrap();

        assert!(matches!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::UpToDate(_)
        ));
        assert_eq2!(check_cooldown_status(&expired_cfg), CooldownStatus::Active);
    }

    #[test]
    #[serial]
    fn test_path2_scenario_2c_exit_joins_inflight_check_thread_and_completes() {
        UpgradeCheckStatus::reset_for_testing();

        // Simulate Scenario 2c: short-lived CLI reaches `handle_upgrade_on_exit()` while
        // the `self-upgrade-check` thread is still awaiting the HTTP response (`50ms`).
        let (url, server_handle) = spawn_one_shot_http_server(
            "200 OK",
            format_manifest_json("0.0.26", "open-source"),
            Duration::from_millis(50),
        );
        let config = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(
                UpgradeActionOnExit::NotifyOnly,
            )),
            Some(url),
            None,
            CooldownPolicy::Disabled,
        );

        let outcome = UpgradeJourney::new(config).start();
        assert_eq2!(outcome, StartupOutcome::ContinueApplication);

        // Immediately invoke exit handler while manifest check is still in flight.
        UpgradeCheckStatus::handle_upgrade_on_exit();
        server_handle.join().unwrap();

        match UpgradeCheckStatus::get() {
            UpgradeCheckStatus::Completed(journey) => {
                assert_eq2!(
                    journey.state().metadata.version,
                    CrateVersion::try_from("0.0.26").unwrap()
                );
                assert_eq2!(journey.state().resolution, UpgradeResolution::NotifiedOnly);
            }
            other => panic!("Expected Completed(NotifiedOnly) after join, got {other:?}"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_path3_disabled_async_and_sync() {
        UpgradeCheckStatus::reset_for_testing();
        let config = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Disabled,
            None,
            None,
            CooldownPolicy::Disabled,
        );
        let outcome = UpgradeJourney::new(config.clone()).start();
        assert_eq2!(outcome, StartupOutcome::ContinueApplication);
        assert_eq2!(
            UpgradeCheckStatus::get(),
            UpgradeCheckStatus::Disabled(UpgradeJourney::new(config).into_disabled())
        );
        UpgradeCheckStatus::handle_upgrade_on_exit();
    }

    #[test]
    #[serial]
    fn test_run_on_dedicated_check_thread_panic_sets_check_failed() {
        UpgradeCheckStatus::reset_for_testing();
        let config = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::default(),
            None,
            None,
            CooldownPolicy::Disabled,
        );
        let checking = UpgradeJourney::new(config)
            .into_checking(UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::Auto));

        run_on_dedicated_check_thread(checking, async |_journey| {
            panic!("simulated panic inside dedicated check thread");
        });

        match UpgradeCheckStatus::get() {
            UpgradeCheckStatus::CheckFailed(failed) => {
                assert!(
                    failed
                        .state()
                        .error
                        .contains("simulated panic inside dedicated check thread")
                );
            }
            other => panic!("Expected CheckFailed after panic, got {other:?}"),
        }
    }

    #[test]
    fn test_extract_panic_payload_message_all_payload_types() {
        let str_payload: Box<dyn Any + Send> = Box::new("static str panic");
        assert_eq2!(
            extract_panic_payload_message(str_payload.as_ref()),
            "static str panic"
        );

        let string_payload: Box<dyn Any + Send> =
            Box::new("owned string panic".to_string());
        assert_eq2!(
            extract_panic_payload_message(string_payload.as_ref()),
            "owned string panic"
        );

        let non_str_payload: Box<dyn Any + Send> = Box::new(42_u32);
        assert_eq2!(
            extract_panic_payload_message(non_str_payload.as_ref()),
            "unknown panic payload"
        );
    }

    #[test]
    #[serial]
    fn test_execute_action_on_exit_auto_upgrade_failure_records_completed_without_cooldown()
     {
        UpgradeCheckStatus::reset_for_testing();
        let temp_dir = try_create_temp_dir().unwrap();
        let timestamp_file_path = temp_dir.join("auto_fail.last_check");

        let closed_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let closed_addr = closed_listener.local_addr().unwrap();
        drop(closed_listener);

        let (manifest_url, server_handle) = spawn_one_shot_http_server(
            "200 OK",
            format_manifest_json("9.9.9", "closed-source"),
            Duration::ZERO,
        );
        let config = UpgradeConfig::new(
            CrateIdentity::new("r3bl-cmdr", CrateVersion::try_from("0.0.25").unwrap()),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::Enabled(UpgradeStrategy::DedicatedThread(
                UpgradeActionOnExit::Auto,
            )),
            Some(manifest_url),
            Some(format!("http://{closed_addr}/releases/download")),
            CooldownPolicy::CustomTimestampFilePath(timestamp_file_path.clone()),
        );

        let outcome = UpgradeJourney::new(config).start();
        assert_eq2!(outcome, StartupOutcome::ContinueApplication);

        UpgradeCheckStatus::handle_upgrade_on_exit();
        server_handle.join().unwrap();

        assert!(!timestamp_file_path.exists());
        match UpgradeCheckStatus::get() {
            UpgradeCheckStatus::Completed(completed) => {
                assert!(matches!(
                    completed.state().resolution,
                    UpgradeResolution::UpgradeFailed(ref err) if !err.is_empty()
                ));
            }
            other => panic!("Expected Completed(UpgradeFailed), got {other:?}"),
        }
    }
}

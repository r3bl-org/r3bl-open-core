// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{Checking, UpgradeJourney};
use crate::{DEBUG_SELF_UPGRADE, ManifestCheckError, UpgradeCheckStatus, UpgradeConfig};
use r3bl_tui::{TokioRuntimeOutcome, run_in_tokio};
use std::{any::Any,
          panic::{AssertUnwindSafe, catch_unwind}};

/// Wrapper around [`run_in_tokio`] that logs runtime startup or panic failures via
/// [`DEBUG_SELF_UPGRADE`].
pub(super) fn run_in_tokio_logged<T: Send, F: Send + AsyncFnOnce() -> T>(
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
pub(super) fn run_on_dedicated_check_thread<F>(
    checking: UpgradeJourney<Checking>,
    async_fn: F,
) where
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
            let error = ManifestCheckError::RuntimeOrThreadInitFailed {
                reason: format!("Failed to build Tokio runtime: {io_err:#}"),
            };
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Dedicated self-upgrade check thread could not build Tokio runtime",
                    crate_name = %fallback_checking.get_config().get_crate_name(),
                    error = %error
                );
            });
            UpgradeCheckStatus::set(UpgradeCheckStatus::CheckFailed(
                fallback_checking.into_check_failed(error),
            ));
        }
        Err(panic_payload) => {
            let panic_msg = extract_panic_payload_message(panic_payload.as_ref());
            let error = ManifestCheckError::CheckThreadPanicked {
                panic_message: panic_msg.to_string(),
            };
            DEBUG_SELF_UPGRADE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "Dedicated self-upgrade check thread panicked",
                    crate_name = %fallback_checking.get_config().get_crate_name(),
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
    use super::*;
    use crate::{CooldownPolicy, CrateVersion, EndpointUrl, JsoncScalar,
                UpgradeActionOnExit, UpgradePolicy, UpgradeStrategy, UserFeedbackChannel};
    use r3bl_tui::assert_eq2;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_run_on_dedicated_check_thread_panic_sets_check_failed() {
        UpgradeCheckStatus::reset_for_testing();
        let config = UpgradeConfig::new(
            "r3bl-cmdr",
            CrateVersion::try_parse("0.0.25").unwrap(),
            UserFeedbackChannel::Quiet,
            UpgradePolicy::default(),
            EndpointUrl::default_manifest_url(),
            CooldownPolicy::Disabled,
        );
        let checking = UpgradeJourney::new(config)
            .into_checking(UpgradeStrategy::DedicatedThread(UpgradeActionOnExit::Auto));

        run_on_dedicated_check_thread(checking, async |_journey| {
            panic!("simulated panic inside dedicated check thread");
        });

        match UpgradeCheckStatus::get() {
            UpgradeCheckStatus::CheckFailed(failed) => {
                assert_eq2!(
                    failed.get_state().error,
                    ManifestCheckError::CheckThreadPanicked {
                        panic_message: "simulated panic inside dedicated check thread"
                            .to_string(),
                    }
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
}

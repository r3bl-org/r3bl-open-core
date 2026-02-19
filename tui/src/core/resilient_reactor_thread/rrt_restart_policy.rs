// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Self-healing restart configuration: [`RestartPolicy`].

use std::time::Duration;

/// Controls restart behavior when your [`RRTWorker`] trait implementation returns
/// [`Continuation::Restart`].
///
/// This design is inspired by [`systemd`]. It adapts the service restart directives
/// (`Restart=`, `RestartSec=`, `StartLimitBurst=`) for in-process thread supervision
/// rather than OS-level process management.
///
/// The [`RRT`] framework applies this policy by sleeping between restart attempts with
/// optional [exponential backoff]. This gives the system time to recover from transient
/// resource exhaustion (e.g., [`fd`] limits, ports in TIME_WAIT).
///
/// See [self-healing restart details] for the full restart lifecycle and two-tier event
/// model.
///
/// # Example Scenarios
///
/// | Scenario                 | What [`create_and_register_os_sources()`] allocates | Why backoff matters                                             |
/// | :----------------------- | :-------------------------------------------------- | :-------------------------------------------------------------- |
/// | Terminal input (current) | [`epoll`] [`fd`], [`eventfd`], [signal] handler     | [`fd`] limit - need time for other processes to release [`fds`] |
/// | Network server           | [`socket`] + `bind` + `listen`                      | Port in [`TIME_WAIT`] - needs kernel timeout to expire          |
/// | Serial/hardware          | `open("/dev/ttyUSB0")` + [`ioctl`]                  | Device busy - other process must release it                     |
///
/// [`Continuation::Restart`]: crate::Continuation::Restart
/// [`RRTWorker`]: super::RRTWorker
/// [`RRT`]: super::RRT
/// [`TIME_WAIT`]: https://en.wikipedia.org/wiki/TCP_TIME-WAIT
/// [`create_and_register_os_sources()`]: super::RRTWorker::create_and_register_os_sources
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`fds`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`ioctl`]: https://man7.org/linux/man-pages/man2/ioctl.2.html
/// [`socket`]: https://man7.org/linux/man-pages/man7/socket.7.html
/// [`systemd`]: https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html
/// [exponential backoff]: https://en.wikipedia.org/wiki/Exponential_backoff
/// [self-healing restart details]: super#self-healing-restart-details
/// [signal]: https://man7.org/linux/man-pages/man7/signal.7.html
#[derive(Debug, Clone)]
pub struct RestartPolicy {
    /// Maximum restart attempts before giving up. `0` means never restart (policy
    /// exhaustion on first [`Continuation::Restart`]).
    ///
    /// [`Continuation::Restart`]: crate::Continuation::Restart
    pub max_restarts: u8,

    /// Delay before the first restart attempt. [`None`] means no delay.
    pub initial_delay: Option<Duration>,

    /// Multiplier applied to the delay after each restart attempt.
    /// [`None`] means constant delay (no growth).
    pub backoff_multiplier: Option<f64>,

    /// Cap on delay growth. [`None`] means unbounded growth.
    pub max_delay: Option<Duration>,
}

/// Defaults tuned for terminal input ([`epoll`] [`fd`], [`eventfd`], [signal] handler
/// creation). The total recovery window is 700ms (100 + 200 + 400), which is long enough
/// for transient [`fd`] pressure to clear but short enough that the user doesn't notice a
/// hiccup.
///
/// ```text
/// Attempt 1: sleep 100ms → F::create_and_register_os_sources()
/// Attempt 2: sleep 200ms → F::create_and_register_os_sources()
/// Attempt 3: sleep 400ms → F::create_and_register_os_sources()
/// Exhausted → send Shutdown → thread exits
/// ```
///
/// For other scenarios (network servers, serial/hardware) where recovery windows are
/// longer, provide your own policy via [`RRTWorker::restart_policy()`].
///
/// [`RRTWorker::restart_policy()`]: super::RRTWorker::restart_policy
/// [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
/// [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [signal]: https://man7.org/linux/man-pages/man7/signal.7.html
impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            initial_delay: Some(Duration::from_millis(100)),
            backoff_multiplier: Some(2.0),
            max_delay: Some(Duration::from_secs(5)),
        }
    }
}

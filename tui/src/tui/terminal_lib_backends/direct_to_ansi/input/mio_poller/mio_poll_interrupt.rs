// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! mio-specific software interrupt for the Resilient Reactor Thread pattern.
//!
//! [`MioSoftwareInterrupt`] wraps a [`mio::Waker`] and implements
//! [`RRTSoftwareInterrupt`] to interrupt the dedicated thread's [`mio::Poll::poll()`]
//! call. The interrupt handle is created from the same [`mio::Poll`] registry as the
//! worker (see [two-phase setup]) and is tightly coupled to it - if the poll is dropped,
//! calling [`trigger_software_interrupt()`] has no effect.
//!
//! [`mio::Poll::poll()`]: mio::Poll::poll
//! [`mio::Poll`]: mio::Poll
//! [`mio::Waker`]: mio::Waker
//! [`RRTSoftwareInterrupt`]: crate::RRTSoftwareInterrupt
//! [`trigger_software_interrupt()`]: MioSoftwareInterrupt::trigger_software_interrupt
//! [two-phase setup]: crate::core::resilient_reactor_thread#two-phase-setup

use crate::core::resilient_reactor_thread::RRTSoftwareInterrupt;
use miette::Diagnostic;

/// Newtype wrapping [`mio::Waker`] to implement [`RRTSoftwareInterrupt`].
///
/// Created from the same [`mio::Poll`] registry as the [`MioPollWorker`] it is paired
/// with. Calling [`trigger_software_interrupt()`] triggers an event on the poll, causing
/// [`mio::Poll::poll()`] to return.
///
/// # How It Works
///
/// See the [Poll -> Registry -> Software Interrupt Chain] diagram on
/// [`RRTSoftwareInterrupt`].
///
/// [`mio::Poll::poll()`]: mio::Poll::poll
/// [`mio::Poll`]: mio::Poll
/// [`mio::Waker`]: mio::Waker
/// [`MioPollWorker`]: super::MioPollWorker
/// [`trigger_software_interrupt()`]: Self::trigger_software_interrupt
/// [Poll -> Registry -> Software Interrupt Chain]:
///     RRTSoftwareInterrupt#poll---registry---software-interrupt-chain
#[derive(Debug)]
pub struct MioSoftwareInterrupt {
    pub mio_waker: mio::Waker,
}

impl MioSoftwareInterrupt {
    /// Creates a synthetic OS event source (like an [`eventfd`] or self-pipe), registers
    /// it with the provided [`mio::Registry`], and returns the handle used to trigger
    /// software interrupts.
    ///
    /// # Errors
    ///
    /// Returns [`SoftwareInterruptCreationError`] if the OS resource cannot be created.
    ///
    /// [`eventfd`]: https://man7.org/linux/man-pages/man2/eventfd.2.html
    pub fn create_and_register_synthetic_software_interrupt_source(
        registry: &mio::Registry,
        token: mio::Token,
    ) -> miette::Result<Self> {
        let mio_waker =
            mio::Waker::new(registry, token).map_err(SoftwareInterruptCreationError)?;
        Ok(Self { mio_waker })
    }
}

impl RRTSoftwareInterrupt for MioSoftwareInterrupt {
    /// Triggers an event on the paired [`mio::Poll`], causing its blocking
    /// [`poll()`] call to return.
    ///
    /// The return value of [`mio::Waker::wake()`] is intentionally discarded - if the
    /// poll has already been dropped (thread exited), the interrupt is a no-op.
    ///
    /// [`mio::Poll`]: mio::Poll
    /// [`mio::Waker::wake()`]: mio::Waker::wake
    /// [`poll()`]: mio::Poll::poll
    fn trigger_software_interrupt(&self) { let _unused = self.mio_waker.wake(); }
}

/// Failed to create [`mio::Waker`] (eventfd/pipe creation failed).
#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("Failed to create mio::Waker")]
#[diagnostic(
    code(r3bl_tui::mio::software_interrupt_creation),
    help("This usually means the system ran out of file descriptors")
)]
pub struct SoftwareInterruptCreationError(#[source] pub std::io::Error);

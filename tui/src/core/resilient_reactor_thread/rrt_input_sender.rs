// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{RRTWorker, ThreadLifecycleMonitor, ThreadState};
use crate::ok;
use std::{future::Future,
          sync::{Arc, MutexGuard}};

/// Sends a message into the blocking worker thread, transparently handling worker
/// lifecycle events.
#[derive(Clone, Debug)]
pub struct InputSender<W: RRTWorker> {
    pub shared_state: Arc<ThreadLifecycleMonitor<W>>,
}

impl<W: RRTWorker> InputSender<W> {
    /// Sends a message into the blocking worker thread, transparently handling worker
    /// lifecycle events.
    ///
    /// If the worker is currently restarting, this method will asynchronously yield and
    /// wait until the new worker is ready before attempting the send.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread is in a stable [`Stopped`] state, meaning the
    /// worker has permanently shut down and can no longer receive messages.
    ///
    /// # Implementation Details
    ///
    /// This method uses a retry loop. If [`smart_sender_notify`] wakes this task up, but
    /// the worker crashes *again* before we acquire the [`Mutex`] lock, the inner match
    /// arm observes the transient state, drops the lock, and goes back to sleep.
    ///
    /// [`Mutex`]: std::sync::Mutex
    /// [`smart_sender_notify`]: ThreadLifecycleMonitor::smart_sender_notify
    /// [`Stopped`]: ThreadState::Stopped
    pub async fn send(&self, msg: W::Input) -> Result<(), miette::Report> {
        loop {
            let guard = self.shared_state.lock();

            match &*guard {
                ThreadState::Running(_, tx) => {
                    match tx.send(msg.clone()) {
                        Ok(_) => {
                            // msg sent!
                            return ok!();
                        }
                        Err(_) => {
                            // Channel disconnected (worker crashed). Loop around to
                            // retry!
                            continue;
                        }
                    }
                }

                ThreadState::Stopped => {
                    return Err(miette::miette!("Worker thread is permanently stopped"));
                }

                // Transient states:
                ThreadState::Starting
                | ThreadState::Restarting
                | ThreadState::Stopping(_) => {
                    self.wait_for_state_change(guard).await;
                }
            }
        }
    }

    /// Handles waiting for transient states safely.
    ///
    /// Consumes the [`MutexGuard`] to enforce that it is dropped *before* we await,
    /// preventing deadlocks and `!Send` future compilation errors.
    ///
    /// # Why this returns a [`Future`] instead of being `async fn`
    ///
    /// If this were an `async fn`, the compiler would capture the `guard` argument into
    /// the generated [`Future`]'s initial state machine *before* executing the body.
    /// Because [`MutexGuard`] is `!Send`, the entire [`Future`] would become `!Send`,
    /// breaking Tokio's concurrency bounds. By using a synchronous `fn` that returns
    /// `impl Future`, the function executes immediately, completely destroying the
    /// `guard` before the safe, [`Send`]-compliant [`Future`] is returned to be awaited.
    ///
    /// # Race Condition Avoidance
    ///
    /// ## Bug 1: The Deadlock (Why we MUST `drop(guard)` before `.await`)
    ///
    /// If we wrote `self.shared_state.smart_sender_notify.wait().await;` without dropping
    /// the guard first, we would be holding the [`MutexGuard`] while we go to sleep.
    ///
    /// 1. We go to sleep holding the lock.
    /// 2. The worker thread finishes booting up and tries to call `set_state(Running)`.
    /// 3. `set_state` tries to grab the lock... but it can't, because we went to sleep
    ///    holding it!
    /// 4. **Deadlock**. We sleep forever waiting for the worker to wake us up, and the
    ///    worker is blocked forever waiting for us to drop the lock.
    ///
    /// (Additionally, Rust's compiler physically prevents this because [`MutexGuard`] is
    /// not allowed to be held across an `.await` point, as the Tokio task might migrate
    /// to another OS thread).
    ///
    /// ## Bug 2: The Missed Wakeup (Why we can't drop the lock before `.wait()`)
    ///
    /// Okay, so we know we must drop the lock before we `.await`. Why not just do this?
    ///
    /// <!-- It is ok to use ignore here - this is just a code fragment -->
    ///
    /// ```ignore
    /// drop(guard);
    /// self.shared_state.smart_sender_notify.wait().await;
    /// ```
    ///
    /// This causes a **missed wakeup** race condition. Imagine this exact timing:
    ///
    /// 1. We `drop(guard)`.
    /// 2. **Context Switch** The OS pauses our task.
    /// 3. The worker thread runs! It grabs the lock, changes the state to `Running`, and
    ///    calls `notify_all()`.
    /// 4. `notify_all()` looks at the queue, sees nobody is waiting, and does nothing.
    /// 5. **Context Switch** Our task resumes!
    /// 6. We call `.wait().await`. We go to sleep waiting for a notification that already
    ///    happened 2 milliseconds ago.
    /// 7. **We sleep forever**.
    fn wait_for_state_change<'a>(
        &'a self,
        guard: MutexGuard<'a, ThreadState<W>>,
    ) -> impl Future<Output = ()> + 'a {
        // RACE CONDITION HANDLING:
        //
        // 1. REGISTER INTEREST WHILE LOCKED
        //
        // By calling `.wait()` here, we put our name on the `Notify` queue. Because we
        // hold the lock, it is impossible for the worker to change the state and fire
        // `notify_all()` yet.
        let wait_fut = self.shared_state.smart_sender_notify.wait();

        // 2. DROP THE LOCK
        //
        // Now that our name is safely on the queue, we release the lock so the worker
        // thread is allowed to progress and change the state.
        drop(guard);

        // 3. RETURN THE FUTURE TO BE AWAITED OUTSIDE
        //
        // We return the future so `send()` can await it. If the worker thread *just*
        // fired `notify_all()` during step 2, the `await` will return instantly because
        // our name was already on the queue. If it hasn't fired yet, we safely go to
        // sleep.
        wait_fut
    }
}

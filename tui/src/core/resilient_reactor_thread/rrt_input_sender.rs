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
    #[allow(clippy::needless_continue)]
    pub async fn send(&self, msg: W::Input) -> Result<(), miette::Report> {
        loop {
            // We use an inner block scope here because the Rust compiler's async
            // state machine generator is conservative and does not realize that
            // `guard` is moved/dropped before the `.await` point if they share the
            // same lexical scope (calling `drop(guard)` manually would not help
            // either, as the check is lexical rather than flow-based). This block
            // ensures `guard` is lexically out of scope before any
            // `.await` point (e.g., `fut.await` below), satisfying the compiler's
            // `Send` bounds analysis.
            let pending_action = {
                let guard = self.shared_state.lock();

                // Early return if the worker is permanently stopped.
                if let ThreadState::Stopped = &*guard {
                    return Err(miette::miette!("Worker thread is permanently stopped"));
                }

                // At this point, we might have a sender or we might have to wait, if the
                // state is transient.
                if let ThreadState::Running(_, tx) = &*guard {
                    // We have a sender; clone it so we can send the message outside the
                    // lock (below).
                    PendingAction::Send(tx.clone())
                } else {
                    // We don't have a sender because the thread is in a transient state.
                    // So we create a future, that will be awaited below, for it to
                    // transition to a state where it has a sender.
                    PendingAction::Wait(self.prepare_wait_future(guard))
                }
            }; // <--- `guard` is (lexically) dropped here.

            match pending_action {
                // We have a sender, try to send the message.
                PendingAction::Send(tx) => {
                    if tx.send(msg.clone()).is_ok() {
                        return ok!();
                    }
                    // If the send failed (channel disconnected), loop to retry.
                    continue;
                }
                // We are in a transient state, use the `fut` to await the state change.
                PendingAction::Wait(fut) => {
                    fut.await;
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
    /// 2. The worker thread finishes booting up and tries to call [`set_state(Running)`].
    /// 3. [`set_state`] tries to grab the lock... but it can't, because we went to sleep
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
    /// 3. The worker thread runs! It grabs the lock, changes the state to [`Running`],
    ///    and calls [`notify_all()`].
    /// 4. [`notify_all()`] looks at the queue, sees nobody is waiting, and does nothing.
    /// 5. **Context Switch** Our task resumes!
    /// 6. We call [`.wait().await`]. We go to sleep waiting for a notification that
    ///    already happened 2 milliseconds ago.
    /// 7. **We sleep forever**.
    ///
    /// [`.wait().await`]: super::AsyncNotify::wait
    /// [`notify_all()`]: super::AsyncNotify::notify_all
    /// [`Running`]: super::ThreadState::Running
    /// [`set_state(Running)`]: super::ThreadLifecycleMonitor::set_state
    /// [`set_state`]: super::ThreadLifecycleMonitor::set_state
    fn prepare_wait_future<'a>(
        &'a self,
        guard: MutexGuard<'_, ThreadState<W>>,
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

enum PendingAction<F, I> {
    Send(tokio::sync::broadcast::Sender<I>),
    Wait(F),
}

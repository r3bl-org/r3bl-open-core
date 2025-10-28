// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::InputEvent;

/// Trait for reading input events asynchronously.
///
/// Provides a unified interface for different input device backends to deliver
/// terminal input events. This abstraction allows switching between crossterm's
/// event polling and direct async I/O implementations.
///
/// # Implementing backends
///
/// Each backend implementation should:
/// - Filter out unsupported event types (Key Release/Repeat, Paste events)
/// - Convert crossterm events to [`InputEvent`]
/// - Return `None` on error or stream closure
///
/// # Why no `+ Send` bound on the returned future?
///
/// This trait intentionally omits `+ Send` from the future return type because:
/// - All implementations hold `&mut self` across `.await` points with non-`Send` streams
/// - The trait is only used within single-task contexts (not sent between threads)
/// - Backend implementations (Crossterm, `DirectToAnsi`) cannot satisfy `Send`
///   constraints
///
/// **IMPORTANT**: The futures returned by this trait are **NOT `Send`** and cannot be
/// used across thread boundaries. This limitation comes from the underlying stream types
/// used by crossterm and async I/O backends.
///
/// # References
///
/// - [Tokio async in depth](https://tokio.rs/tokio/tutorial/async)
/// - [Crossterm event API](https://github.com/crossterm-rs/crossterm/wiki/Upgrade-from-0.13-to-0.14#111-new-event-api)
/// - [Crossterm event polling](https://github.com/crossterm-rs/crossterm/wiki/Upgrade-from-0.13-to-0.14#115-event-polling)
#[allow(
    async_fn_in_trait,
    reason = "Implementations are not Send-safe; trait used only in single-task contexts"
)]
pub trait InputDeviceExt {
    async fn next_input_event(&mut self) -> Option<InputEvent> /* `!Send` */;
}

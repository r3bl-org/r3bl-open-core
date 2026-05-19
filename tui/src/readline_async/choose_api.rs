// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CalculateResizeHint, CaretVerticalViewportLocation, ColWidth,
            DEVELOPMENT_MODE, EventLoopResult, Header, InlineString, InputDevice,
            InputEvent, IntoErr, ItemsOwned, Key, KeyPress, KeyState,
            LineStateControlSignal, ModifierKeysMask, OutputDevice, RowHeight,
            SelectComponent, SharedWriter, SpecialKey, State, StyleSheet,
            TerminalInteractiveStatus, TuiAvailability, ch,
            check_is_terminal_interactive, enter_event_loop_async, fg_green, get_size,
            inline_string, tui::md_parser::md_parser_constants::SPACE_CHAR, usize};
use clap::ValueEnum;
use miette::IntoDiagnostic;
use std::{future::Future, pin::Pin};

pub const DEFAULT_HEIGHT: usize = 5;

/// Type alias for the pinned boxed future returned by the choose function.
pub type ChooseFuture<'a> =
    Pin<Box<dyn Future<Output = miette::Result<ItemsOwned>> + 'a>>;

// XMARK: Box::pin a future that is larger than 16KB.

/// Async function to choose an item from a list of items.
///
/// It takes a list of items, and returns the selected item or items (depending on the
/// selection mode). If the user does not select anything, it returns [`None`]. The
/// function also takes the maximum height and width of the display, and the selection
/// mode (single select or multiple select).
///
/// If the terminal is *fully* un-interactive, it returns [`None`]. This is useful so that
/// it won't block `cargo test` or when run in non-interactive CI/CD environments.
///
/// # Note on [`stderr`] redirection
///
/// This function calls [`emit_stderr_redirection_disclaimer()`] to ensure that if
/// [`stderr`] is redirected, the user is notified that application logs are handled
/// internally.
///
/// # Arguments
///
/// * `arg_header` - The header to display above the list.
/// * `arg_options_to_choose_from` - The list of items to choose from.
/// * `maybe_max_height` - Optional: the maximum height of the list.
/// * `maybe_max_width` - Optional: the maximum width of the list.
/// * `how` - The selection mode.
/// * `stylesheet` - The style to use for the list.
/// * `io` - The input and output devices to use. Call
///   [`DefaultIoDevices::as_mut_tuple()`] if you don't want to specify anything here.
///   * `output_device` - The output device to use.
///   * `input_device` - The input device to use.
///   * `maybe_shared_writer` - The shared writer to use, if `ReadlineAsyncContext` is in
///     use, and the async stdout needs to be paused when this function is running.
///
/// # Returns
///
/// Returns a [`TuiAvailability`] containing a pinned boxed future that resolves to
/// `Ok(ItemsOwned)` with the following behavior:
/// * **Single selection mode** (`HowToChoose::Single`):
///   - If user selects an item and presses Enter: returns an `ItemsOwned` containing the
///     selected item
///   - If user cancels (Escape or Ctrl+C): returns an empty `ItemsOwned`
/// * **Multiple selection mode** (`HowToChoose::Multiple`):
///   - If user selects items (Space to toggle) and presses Enter: returns an `ItemsOwned`
///     containing all selected items
///   - If user presses Enter without selecting any items: returns an empty `ItemsOwned`
///   - If user cancels (Escape or Ctrl+C): returns an empty `ItemsOwned`
/// * **Non-interactive terminal**: returns an empty `ItemsOwned`
///
/// # Errors
///
/// Returns [`miette::Error`] if there are communication errors with the shared writer's
/// line state control channel when sending pause/resume signals. This can occur when:
/// * The shared writer's channel receiver has been dropped
/// * The channel is closed or disconnected
/// * There are other async communication failures with the
///   [`crate::ReadlineAsyncContext`] integration
///
/// # Why return a pinned boxed future?
///
/// This function returns a pinned boxed future ([`Box::pin`]; > 16KB clippy threshold)
/// for safer memory management and better performance characteristics.
///
/// ## Performance Benefits
///
/// * **Without [`Box::pin`]**: The entire > 16KB future gets copied every time it moves
///   between stack frames (function calls, async state transitions, select! operations).
/// * **With [`Box::pin`]**: Only an 8-byte pointer moves, while the actual future data
///   stays fixed on the heap, avoiding expensive > 16KB memory copies.
/// * Reduces stack pressure and improves CPU cache locality.
///
/// ## Safety Benefits
///
/// * This function may be called when the stack already has many frames from the main
///   application logic. Pinning this future to the heap avoids potential stack overflow
///   issues when the stack is deep.
/// * Provides defensive programming "better safe than sorry" approach for stack depth
///   management.
///
/// ## Probably not needed for this function, but done for defensive programming
///
/// It is probably not needed here, but is done just for defensive programming "better
/// safe than sorry" for stack depth management. Generally, the returned pinned boxed
/// future from this function is used in the following contexts:
/// - Single use: The future is created, awaited once, and then dropped - no loops or
///   repeated moves.
/// - Not stored in a struct: The future isn't being stored in a data structure that would
///   require [`std::pin::Pin`].
/// - Direct await: It's immediately awaited, not passed around or stored.
///
/// # Other entry points for interactive terminal apps
///
/// See [interactive terminal application entry points].
///
/// [`check_is_terminal_interactive()`]: crate::check_is_terminal_interactive
/// [`emit_stderr_redirection_disclaimer()`]: crate::emit_stderr_redirection_disclaimer
/// [`stderr`]: std::io::stderr
/// [interactive terminal application entry points]: crate#interactive-terminal-application-entry-points
pub fn choose<'a>(
    arg_header: impl Into<Header>,
    arg_options_to_choose_from: impl Into<ItemsOwned>,
    maybe_max_height: Option<RowHeight>,
    maybe_max_width: Option<ColWidth>,
    how: HowToChoose,
    stylesheet: StyleSheet,
    io: (
        &'a mut OutputDevice,
        &'a mut InputDevice,
        Option<SharedWriter>,
    ),
) -> TuiAvailability<ChooseFuture<'a>> {
    let from = arg_options_to_choose_from.into();
    let header = arg_header.into();

    match check_is_terminal_interactive() {
        TerminalInteractiveStatus::NotAvailable(reason) => {
            TuiAvailability::NotAvailable(reason)
        }

        TerminalInteractiveStatus::Available => {
            let initial_size = match get_size() {
                Ok(size) => size,
                Err(e) => return TuiAvailability::Broken(e),
            };

            TuiAvailability::Available(Box::pin(async move {
                // Destructure the io tuple.
                let (output_device, input_device, maybe_shared_writer) = io;

                // For compatibility with ReadlineAsyncContext (if it is in use).
                if let Some(ref shared_writer) = maybe_shared_writer {
                    // Pause the shared writer while the user is choosing an item.
                    shared_writer
                        .line_state_control_channel_sender
                        .send(LineStateControlSignal::Pause)
                        .await
                        .into_diagnostic()?;
                }

                // - If the max size is None, then set it to DEFAULT_HEIGHT.
                // - If the max size is Some, then this is the max height of the viewport.
                //   - However, if this is 0, then set to DEFAULT_HEIGHT.
                //   - Otherwise, check whether the number of items is less than this max
                //     height and set the max height to the number of items.
                //   - Otherwise, if there are more items than the max height, then clamp
                //     it to the max height.
                let max_display_height = ch({
                    match maybe_max_height {
                        None => DEFAULT_HEIGHT,
                        Some(row_height) => {
                            let row_height = row_height.as_usize();
                            if row_height == 0 {
                                DEFAULT_HEIGHT
                            } else {
                                std::cmp::min(row_height, from.len())
                            }
                        }
                    }
                });

                let max_display_width = ch(match maybe_max_width {
                    None => 0,
                    Some(col_width) => col_width.as_usize(),
                });

                let mut state = State {
                    max_display_height,
                    max_display_width,
                    items: from,
                    header,
                    selection_mode: how,
                    ..Default::default()
                };
                state.set_size(initial_size);

                let mut function_component = SelectComponent {
                    output_device: output_device.clone(),
                    style: stylesheet,
                };

                let res_user_input = enter_event_loop_async(
                    &mut state,
                    &mut function_component,
                    keypress_handler,
                    input_device,
                )
                .await;

                // For compatibility with ReadlineAsyncContext (if it is in use).
                if let Some(ref shared_writer) = maybe_shared_writer {
                    // Resume the shared writer after the user has made their choice.
                    shared_writer
                        .line_state_control_channel_sender
                        .send(LineStateControlSignal::Resume)
                        .await
                        .into_diagnostic()?;
                }

                match res_user_input {
                    Ok(EventLoopResult::ExitWithResult(it)) => Ok(it),
                    _ => Ok(ItemsOwned::default()),
                }
            }))
        }
    }
}

/// Extension trait for [`TuiAvailability<ChooseFuture<'a>>`] to provide a fluent API for
/// common patterns of using the result of the [`choose`] function.
///
/// The are two patterns of using the result of the [`choose`] function:
/// 1. Single selection. For this we have [`get_first_result()`]. Call sites include
///    [`branch_checkout_command.rs`] and [`upgrade_check.rs`].
/// 2. Multiple selection. For this we have [`get_all_results()`].
///
/// [`branch_checkout_command.rs`]:
///     https://github.com/r3bl-org/r3bl-open-core/blob/main/cmdr/src/giti/branch/branch_checkout_command.rs
/// [`get_all_results()`]: Self::get_all_results
/// [`get_first_result()`]: Self::get_first_result
/// [`upgrade_check.rs`]:
///     https://github.com/r3bl-org/r3bl-open-core/blob/main/cmdr/src/analytics_client/upgrade_check.rs
#[allow(async_fn_in_trait)]
pub trait TuiAvailabilityChooseExt {
    /// Propagate errors for a single selection.
    ///
    /// # Returns
    ///
    /// - [`Err`] if the TUI is [`Broken`] or [`NotAvailable`].
    /// - If cancelled return [`Ok(None)`].
    /// - Otherwise returns [`Ok(Some(item))`] on selection if it has a single item (or
    ///   more).
    /// - Otherwise returns [`Ok(None)`] if selection is empty.
    ///
    /// [`Broken`]: TuiAvailability::Broken
    /// [`NotAvailable`]: TuiAvailability::NotAvailable
    /// [`Ok(None)`]: Option::None
    /// [`Ok(Some(item))`]: Option::Some
    /// [`Some(item)`]: Option::Some
    async fn get_first_result(self) -> miette::Result<Option<InlineString>>;

    /// Propagate errors for multiple selections.
    ///
    /// # Returns
    ///
    /// - [`Err`] if the TUI is [`Broken`] or [`NotAvailable`].
    /// - If selection is empty returns [`Ok(None)`].
    /// - Otherwise return [`Ok(Some(items))`] on selection (which contains at least 1
    ///   item).
    ///
    /// [`Broken`]: TuiAvailability::Broken
    /// [`NotAvailable`]: TuiAvailability::NotAvailable
    /// [`Ok(None)`]: Option::None
    /// [`Ok(Some(items))`]: Option::Some
    async fn get_all_results(self) -> miette::Result<Option<ItemsOwned>>;
}

impl TuiAvailabilityChooseExt for TuiAvailability<ChooseFuture<'_>> {
    async fn get_first_result(self) -> miette::Result<Option<InlineString>> {
        match self {
            TuiAvailability::Available(future) => {
                let items = future.await?;
                Ok(items.into_iter().next()) // First item.
            }
            it => it.into_err(),
        }
    }

    async fn get_all_results(self) -> miette::Result<Option<ItemsOwned>> {
        match self {
            TuiAvailability::Available(future) => {
                let items = future.await?;
                if items.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(items))
                }
            }
            it => it.into_err(),
        }
    }
}

/// This struct is provided for convenience to create a default set of IO devices which
/// can be used in the [`choose`] function. The reason this has to be created outside of
/// the [`choose`] function is because mutable references to these devices are passed to
/// it, and it can't take ownership of them.
#[allow(missing_debug_implementations)]
pub struct DefaultIoDevices {
    pub output_device: OutputDevice,
    pub input_device: InputDevice,
    pub maybe_shared_writer: Option<SharedWriter>,
}

impl Default for DefaultIoDevices {
    fn default() -> Self {
        let output_device = OutputDevice::new_stdout();
        let input_device = InputDevice::default();
        DefaultIoDevices {
            output_device,
            input_device,
            maybe_shared_writer: None,
        }
    }
}

impl DefaultIoDevices {
    pub fn as_mut_tuple(
        &mut self,
    ) -> (&mut OutputDevice, &mut InputDevice, Option<SharedWriter>) {
        (
            &mut self.output_device,
            &mut self.input_device,
            self.maybe_shared_writer.clone(),
        )
    }
}

#[allow(clippy::needless_pass_by_value)]
fn keypress_handler(state: &mut State, input_event: InputEvent) -> EventLoopResult {
    DEVELOPMENT_MODE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "🔆🔆🔆 *before* keypress: locate_cursor_in_viewport()",
            cursor_location_in_viewport = ?state.locate_cursor_in_viewport()
        );
    });

    let selection_mode = state.selection_mode;

    let return_it = match input_event {
        // Resize.
        InputEvent::Resize(size) => {
            keypress_handler_helper::handle_resize_event(state, size)
        }

        // Down.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Down),
        }) => keypress_handler_helper::handle_down_key(state),

        // Up.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        }) => keypress_handler_helper::handle_up_key(state),

        // Enter on multi-select.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Enter),
        }) if selection_mode == HowToChoose::Multiple => {
            keypress_handler_helper::handle_enter_key_multi_select(state)
        }

        // Enter.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Enter),
        }) => keypress_handler_helper::handle_enter_key_single_select(state),

        // Escape or Ctrl + c.
        InputEvent::Keyboard(
            KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Esc),
            }
            | KeyPress::WithModifiers {
                key: Key::Character('c'),
                mask:
                    ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        shift_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            },
        ) => keypress_handler_helper::handle_escape_or_ctrl_c(),

        // Space on multi-select.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character(' '),
        }) if selection_mode == HowToChoose::Multiple => {
            keypress_handler_helper::handle_space_key_multi_select(state)
        }

        // Default behavior on Space.
        InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character(SPACE_CHAR),
        }) => keypress_handler_helper::handle_space_key_default(),

        // Ignore other keys.
        _ => keypress_handler_helper::handle_other_keys(),
    };

    DEVELOPMENT_MODE.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = "👉 *after* keypress: locate_cursor_in_viewport()",
            cursor_location_in_viewport = ?state.locate_cursor_in_viewport()
        );
    });

    return_it
}

mod keypress_handler_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn handle_resize_event(state: &mut State, size: crate::Size) -> EventLoopResult {
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "🍎🍎🍎 keypress_handler() resize",
                details = %inline_string!(
                    "New size width:{w} x height:{h}",
                    w = fg_green(&inline_string!("{:?}", size.col_width)),
                    h = fg_green(&inline_string!("{:?}", size.row_height)),
                )
            };
        });
        state.set_resize_hint(size);
        EventLoopResult::ContinueAndRerenderAndClear
    }

    pub fn handle_down_key(state: &mut State) -> EventLoopResult {
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(message = "Down");
        });
        let caret_location = state.locate_cursor_in_viewport();
        match caret_location {
            CaretVerticalViewportLocation::AtAbsoluteTop
            | CaretVerticalViewportLocation::AboveTopOfViewport
            | CaretVerticalViewportLocation::AtTopOfViewport
            | CaretVerticalViewportLocation::InMiddleOfViewport => {
                state.raw_caret_row_index += 1;
            }

            CaretVerticalViewportLocation::AtBottomOfViewport
            | CaretVerticalViewportLocation::BelowBottomOfViewport => {
                state.scroll_offset_row_index += 1;
            }

            CaretVerticalViewportLocation::AtAbsoluteBottom
            | CaretVerticalViewportLocation::NotFound => {
                // Do nothing.
            }
        }
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "enter_event_loop()::state",
                state = ?state
            );
        });

        EventLoopResult::ContinueAndRerender
    }

    pub fn handle_up_key(state: &mut State) -> EventLoopResult {
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(message = "Up");
        });

        match state.locate_cursor_in_viewport() {
            CaretVerticalViewportLocation::NotFound
            | CaretVerticalViewportLocation::AtAbsoluteTop => {
                // Do nothing.
            }
            CaretVerticalViewportLocation::AboveTopOfViewport
            | CaretVerticalViewportLocation::AtTopOfViewport => {
                state.scroll_offset_row_index -= 1;
            }
            CaretVerticalViewportLocation::InMiddleOfViewport
            | CaretVerticalViewportLocation::AtBottomOfViewport
            | CaretVerticalViewportLocation::BelowBottomOfViewport
            | CaretVerticalViewportLocation::AtAbsoluteBottom => {
                state.raw_caret_row_index -= 1;
            }
        }

        EventLoopResult::ContinueAndRerender
    }

    pub fn handle_enter_key_multi_select(state: &State) -> EventLoopResult {
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Enter on multi-select",
                selected_items = ?state.selected_items
            );
        });
        if state.selected_items.is_empty() {
            EventLoopResult::ExitWithoutResult
        } else {
            EventLoopResult::ExitWithResult(state.selected_items.clone())
        }
    }

    pub fn handle_enter_key_single_select(state: &State) -> EventLoopResult {
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Enter",
                focused_index = ?state.get_focused_index()
            );
        });
        let selection_index = usize(state.get_focused_index());
        let maybe_item = state.items.get(selection_index);
        match maybe_item {
            Some(it) => EventLoopResult::ExitWithResult(it.into()),
            None => EventLoopResult::ExitWithoutResult,
        }
    }

    pub fn handle_escape_or_ctrl_c() -> EventLoopResult {
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(message = "Esc");
        });
        EventLoopResult::ExitWithoutResult
    }

    pub fn handle_space_key_multi_select(state: &mut State) -> EventLoopResult {
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Space on multi-select",
                focused_index = ?state.get_focused_index()
            );
        });
        let selection_index = usize(state.get_focused_index());
        let maybe_item = state.items.get(selection_index);
        let maybe_index = state
            .selected_items
            .iter()
            .position(|item| Some(item) == maybe_item);
        match (maybe_item, maybe_index) {
            // No selected_item.
            (None, _) => (),
            // Item already in selected_items so remove it.
            (Some(_), Some(it)) => {
                state.selected_items.remove(it);
            }
            // Item not found in selected_items so add it.
            (Some(it), None) => state.selected_items.push(it.clone()),
        }

        EventLoopResult::ContinueAndRerender
    }

    pub fn handle_space_key_default() -> EventLoopResult {
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(message = "Space");
        });
        EventLoopResult::Continue
    }

    pub fn handle_other_keys() -> EventLoopResult {
        DEVELOPMENT_MODE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(message = "Ignore key event");
        });
        EventLoopResult::Continue
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Hash,
)]
pub enum HowToChoose {
    /// Select only one option from list.
    #[default]
    Single,
    /// Select multiple options from list.
    Multiple,
}

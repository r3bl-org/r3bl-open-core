// Copyright (c) 2023-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DEVELOPMENT_MODE, OutputDevice, RangeExt, ResizeHint, TERMINAL_LIB_BACKEND,
            TermRowDelta, TerminalLibBackend, VPHeight, VPSize, ansi_output, ok,
            queue_commands, throws};
use crossterm::{cursor::{MoveToNextLine, MoveToPreviousLine},
                terminal::{Clear, ClearType}};
use miette::IntoDiagnostic;

pub trait CalculateResizeHint {
    fn set_size(&mut self, new_size: VPSize);
    fn get_resize_hint(&self) -> Option<ResizeHint>;
    fn set_resize_hint(&mut self, new_size: VPSize);
    fn clear_resize_hint(&mut self);
}

pub trait FunctionComponent<S: CalculateResizeHint> {
    fn get_output_device(&mut self) -> OutputDevice;

    fn calculate_header_viewport_height(&self, state: &mut S) -> VPHeight;

    fn calculate_items_viewport_height(&self, state: &mut S) -> VPHeight;

    /// # Errors
    ///
    /// Returns an error if the rendering operation fails.
    fn render(&mut self, state: &mut S) -> miette::Result<()>;

    /// # Errors
    ///
    /// Returns an error if the viewport allocation fails.
    fn allocate_viewport_height_space(&mut self, state: &mut S) -> miette::Result<()> {
        throws!({
            let vp_height =
                /* not including the header */ self.calculate_items_viewport_height(state) +
                /* for header row(s) */ self.calculate_header_viewport_height(state);

            // Allocate space. This is required so that the commands to move the cursor up
            // and down shown below will work.
            for _ in (..vp_height).as_index_iter() {
                println!();
            }

            // Move the cursor back up.
            queue_commands! {
                self.get_output_device(),
                MoveToPreviousLine(vp_height.as_u16()),
            };
        });
    }

    /// # Errors
    ///
    /// Returns an error if clearing the viewport fails.
    fn clear_viewport_for_resize(&mut self, state: &mut S) -> miette::Result<()> {
        throws!({
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "🥑🥑🥑 clear viewport for resize",
                    resize_hint = ?state.get_resize_hint()
                );
            });

            let vp_height = match state.get_resize_hint() {
                // Resize happened.
                Some(
                    ResizeHint::GotBigger | ResizeHint::NoChange | ResizeHint::GotSmaller,
                ) => {
                    /* not including the header */
                    self.calculate_items_viewport_height(state) +
                    /* for header row(s) */
                    self.calculate_header_viewport_height(state)
                }
                // Nothing to do, since resize didn't happen.
                None => return ok!(),
            };

            // Clear the viewport.
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    for _ in (..vp_height).as_index_iter() {
                        queue_commands! {
                            self.get_output_device(),
                            Clear(ClearType::FromCursorDown),
                            MoveToNextLine(1),
                        };
                    }
                    queue_commands! {
                        self.get_output_device(),
                        MoveToPreviousLine(vp_height.as_u16()),
                    };
                }
                TerminalLibBackend::DirectToAnsi => {
                    self.get_output_device()
                        .write(|writer| -> miette::Result<()> {
                            let capacity = ansi_output::estimate_capacity::clear_and_rewind_lines_capacity_hint(
                                vp_height,
                            );
                            let mut buf = String::with_capacity(capacity);

                            if !vp_height.is_empty() {
                                let next_line_seq =
                                    ansi_output::cursor_movement::cursor_next_line(
                                        TermRowDelta::ONE,
                                    );
                                for _ in (..vp_height).as_index_iter() {
                                    buf.push_str(
                                        ansi_output::screen_clearing::clear_to_end_of_screen(),
                                    );
                                    buf.push_str(&next_line_seq);
                                }
                            }

                            if let Some(delta) = TermRowDelta::new(vp_height.as_u16()) {
                                buf.push_str(
                                    &ansi_output::cursor_movement::cursor_previous_line(
                                        delta,
                                    ),
                                );
                            }

                            writer.write_all(buf.as_bytes()).into_diagnostic()?;
                            ok!()
                        })?;
                }
            }

            // Clear resize hint.
            state.clear_resize_hint();
        });
    }

    /// # Errors
    ///
    /// Returns an error if clearing the viewport fails.
    fn clear_viewport(&mut self, state: &mut S) -> miette::Result<()> {
        throws!({
            let vp_height =
                /* not including the header */ self.calculate_items_viewport_height(state) +
                /* for header row(s) */ self.calculate_header_viewport_height(state);

            // Clear the viewport.
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    for _ in (..vp_height).as_index_iter() {
                        queue_commands! {
                            self.get_output_device(),
                            Clear(ClearType::CurrentLine),
                            MoveToNextLine(1),
                        };
                    }
                    queue_commands! {
                        self.get_output_device(),
                        MoveToPreviousLine(vp_height.as_u16()),
                    };
                }
                TerminalLibBackend::DirectToAnsi => {
                    self.get_output_device()
                        .write(|writer| -> miette::Result<()> {
                            let capacity = ansi_output::estimate_capacity::clear_and_rewind_lines_capacity_hint(
                                vp_height,
                            );
                            let mut buf = String::with_capacity(capacity);

                            if !vp_height.is_empty() {
                                let next_line_seq =
                                    ansi_output::cursor_movement::cursor_next_line(
                                        TermRowDelta::ONE,
                                    );
                                for _ in (..vp_height).as_index_iter() {
                                    buf.push_str(
                                        ansi_output::screen_clearing::clear_current_line(
                                        ),
                                    );
                                    buf.push_str(&next_line_seq);
                                }
                            }

                            if let Some(delta) = TermRowDelta::new(vp_height.as_u16()) {
                                buf.push_str(
                                    &ansi_output::cursor_movement::cursor_previous_line(
                                        delta,
                                    ),
                                );
                            }

                            writer.write_all(buf.as_bytes()).into_diagnostic()?;
                            ok!()
                        })?;
                }
            }
        });
    }
}

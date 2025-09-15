// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use std::{fmt::{Debug, Formatter, Result},
          ops::{AddAssign, Deref, DerefMut}};

use super::TERMINAL_LIB_BACKEND;
use crate::{CrosstermDebugFormatRenderOp, InlineString, InlineVec, LockedOutputDevice,
            PaintRenderOp, Pos, RenderOpImplCrossterm, Size, TerminalLibBackend,
            TuiColor, TuiStyle, ok};

/// Convenient macro for creating and manipulating [`RenderOps`] collections.
///
/// This macro provides three main operations:
/// - `@new`: Create a new `RenderOps` with optional initial operations
/// - `@add_to`: Add operations to an existing `RenderOps`
/// - `@render_styled_texts_into`: Render styled text collections into operations
///
/// # Examples
///
/// ## Creating new `RenderOps`
///
/// ```
/// use r3bl_tui::*;
///
/// // Create empty RenderOps
/// let empty_ops = render_ops!();
///
/// // Create RenderOps with initial operations
/// let mut render_ops = render_ops!(
///   @new
///   RenderOp::ClearScreen, RenderOp::ResetColor
/// );
/// let len = render_ops.len();
/// let iter = render_ops.iter();
/// ```
///
/// ## Adding to existing `RenderOps`
///
/// ```
/// use r3bl_tui::*;
///
/// let mut ops = RenderOps::default();
/// render_ops!(
///   @add_to ops =>
///   RenderOp::MoveCursorPositionAbs(Pos::default()),
///   RenderOp::ResetColor
/// );
/// ```
///
/// ## Rendering styled texts
///
/// ```
/// use r3bl_tui::*;
///
/// let mut ops = RenderOps::default();
/// let styled_texts = TuiStyledTexts::default();
/// render_ops!(
///   @render_styled_texts_into ops =>
///   styled_texts
/// );
/// ```
#[macro_export]
macro_rules! render_ops {
  // Empty.
  () => {
    $crate::RenderOps::default()
  };

  // @new: Create a RenderOps. If any ($arg_render_op)* are passed, then add it to its list. Finally
  // return it.
  (
    @new
    $(                      /* Start a repetition. */
      $arg_render_op: expr  /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
    $(,)*                   /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
    {
      let mut render_ops = $crate::RenderOps::default();
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $arg_render_op replaced. */
        render_ops.list.push($arg_render_op);
      )*
      render_ops
    }
  };

  // @add_to: If any ($arg_render_op)* are passed, then add to it.
  (
    @add_to
    $arg_render_ops: expr
    =>
    $(                      /* Start a repetition. */
      $arg_render_op: expr  /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
    $(,)*                   /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
    {
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $arg_render_op replaced. */
        $arg_render_ops.list.push($arg_render_op);
      )*
    }
  };

  // @render_into: If any ($arg_render_op)* are passed, then add to it.
  (
    @render_styled_texts_into
    $arg_render_ops: expr
    =>
    $(                       /* Start a repetition. */
      $arg_styled_texts: expr/* Expression. */
    )                        /* End repetition. */
    ,                        /* Comma separated. */
    *                        /* Zero or more times. */
    $(,)*                    /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
    {
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $arg_render_op replaced. */
        $crate::render_tui_styled_texts_into(&$arg_styled_texts, &mut $arg_render_ops);
      )*
    }
  };
}

/// For ease of use, please use the [`render_ops`!] macro.
///
/// It is a collection of *atomic* paint operations (aka [`RenderOps`] at various
/// [`super::ZOrder`]s); each [`RenderOps`] is made up of a [vec] of [`RenderOp`]. It
/// contains `Map<ZOrder, Vec<RenderOps>>`, eg:
/// - [`super::ZOrder::Normal`] => vec![[`RenderOp::ResetColor`],
///   [`RenderOp::MoveCursorPositionAbs`], [`RenderOp::PaintTextWithAttributes`]]
/// - [`super::ZOrder::Glass`] => vec![[`RenderOp::ResetColor`],
///   [`RenderOp::MoveCursorPositionAbs`], [`RenderOp::PaintTextWithAttributes`]]
/// - etc.
///
/// # What is an atomic paint operation?
/// 1. It moves the cursor using:
///     1. [`RenderOp::MoveCursorPositionAbs`]
///     2. [`RenderOp::MoveCursorPositionRelTo`]
/// 2. And it does not assume that the cursor is in the correct position from some other
///    previously executed operation!
/// 3. So there are no side effects when re-ordering or omitting painting an atomic paint
///    operation (eg in the case where it has already been painted before).
///
/// Here's an example. Consider using the macro for convenience (see [`render_ops`!]).
///
/// ```
/// use r3bl_tui::*;
///
/// let mut render_ops = RenderOps::default();
/// render_ops.push(RenderOp::ClearScreen);
/// render_ops.push(RenderOp::ResetColor);
/// let len = render_ops.len();
/// let iter = render_ops.iter();
/// ```
///
/// # Paint optimization
/// Due to the compositor [`super::OffscreenBuffer`], there is no need to optimize the
/// individual paint operations. You don't have to manage your own whitespace or doing
/// clear before paint! 🎉 The compositor takes care of that for you!
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RenderOps {
    pub list: InlineVec<RenderOp>,
}

impl Default for RenderOps {
    fn default() -> Self {
        Self {
            list: InlineVec::new(),
        }
    }
}

/// Local state tracking for render operations optimization.
///
/// Maintains the last known terminal state to avoid sending redundant
/// escape sequences when the state hasn't changed. This significantly
/// reduces the amount of data sent to the terminal.
#[derive(Default, Debug)]
pub struct RenderOpsLocalData {
    /// Current cursor position in the terminal.
    ///
    /// Used to determine if cursor movement commands need to be sent
    /// when rendering at a new position.
    pub cursor_pos: Pos,

    /// Last known foreground color.
    ///
    /// Tracks the current foreground color to avoid sending redundant
    /// color escape sequences when the color hasn't changed.
    pub fg_color: Option<TuiColor>,

    /// Last known background color.
    ///
    /// Tracks the current background color to avoid sending redundant
    /// color escape sequences when the color hasn't changed.
    pub bg_color: Option<TuiColor>,
}

/// Implementation details for [`RenderOps`] functionality.
///
/// This module contains the core implementation of render operation execution,
/// including methods for processing operation lists, routing to backend implementations,
/// and providing convenient trait implementations for common operations.
pub mod render_ops_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl RenderOps {
        /// Executes all render operations in the collection sequentially.
        ///
        /// This method processes each [`RenderOp`] in the list, maintaining local state
        /// for optimization and routing each operation to the appropriate backend
        /// implementation based on the configured terminal library.
        ///
        /// # Parameters
        /// - `skip_flush`: Mutable reference to control flush behavior
        /// - `window_size`: Current terminal window dimensions
        /// - `locked_output_device`: Locked terminal output for thread-safe writing
        /// - `is_mock`: Whether this is a mock execution for testing
        pub fn execute_all(
            &self,
            skip_flush: &mut bool,
            window_size: Size,
            locked_output_device: LockedOutputDevice<'_>,
            is_mock: bool,
        ) {
            let mut render_local_data = RenderOpsLocalData::default();
            for render_op in &self.list {
                RenderOps::route_paint_render_op_to_backend(
                    &mut render_local_data,
                    skip_flush,
                    render_op,
                    window_size,
                    locked_output_device,
                    is_mock,
                );
            }
        }

        /// Routes a single render operation to the appropriate backend implementation.
        ///
        /// This method acts as a dispatcher, selecting the correct terminal library
        /// backend (currently Crossterm) and delegating the actual rendering work
        /// to the backend-specific implementation.
        ///
        /// # Parameters
        /// - `render_local_data`: Mutable state for render optimization
        /// - `skip_flush`: Mutable reference to control flush behavior
        /// - `render_op`: The specific operation to execute
        /// - `window_size`: Current terminal window dimensions
        /// - `locked_output_device`: Locked terminal output for thread-safe writing
        /// - `is_mock`: Whether this is a mock execution for testing
        pub fn route_paint_render_op_to_backend(
            render_local_data: &mut RenderOpsLocalData,
            skip_flush: &mut bool,
            render_op: &RenderOp,
            window_size: Size,
            locked_output_device: LockedOutputDevice<'_>,
            is_mock: bool,
        ) {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    RenderOpImplCrossterm {}.paint(
                        skip_flush,
                        render_op,
                        window_size,
                        render_local_data,
                        locked_output_device,
                        is_mock,
                    );
                }
                TerminalLibBackend::Termion => unimplemented!(),
            }
        }
    }

    impl Deref for RenderOps {
        type Target = InlineVec<RenderOp>;

        fn deref(&self) -> &Self::Target { &self.list }
    }

    impl DerefMut for RenderOps {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.list }
    }

    impl AddAssign<RenderOp> for RenderOps {
        fn add_assign(&mut self, rhs: RenderOp) { self.list.push(rhs); }
    }

    impl Debug for RenderOps {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            const DELIM: &str = "\n  - ";

            let mut iter = self.iter();

            // We don't care about the result of this operation.
            write!(f, "RenderOps.len(): {}", self.list.len()).ok();

            // First line.
            if let Some(first) = iter.next() {
                // We don't care about the result of this operation.
                write!(f, "[{first:?}]").ok();
            }

            // Subsequent lines.
            for item in iter {
                // We don't care about the result of this operation.
                write!(f, "{DELIM}[{item:?}]").ok();
            }

            ok!()
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub enum RenderOp {
    /// Enables terminal raw mode for direct control over input/output.
    ///
    /// Raw mode disables line buffering and special character processing,
    /// allowing the application to receive keystrokes immediately and
    /// handle all terminal control sequences directly.
    EnterRawMode,

    /// Exits terminal raw mode and restores normal terminal behavior.
    ///
    /// This restores line buffering and standard terminal input processing.
    /// Should always be called before application exit to avoid leaving
    /// the terminal in an unusable state.
    ExitRawMode,

    /// This is always painted on top. [Pos] is the absolute column and row on the
    /// terminal screen. This uses [`super::sanitize_and_save_abs_pos`] to clean up the
    /// given [Pos].
    MoveCursorPositionAbs(/* absolute position */ Pos),

    /// This is always painted on top. 1st [Pos] is the origin column and row, and the
    /// 2nd [Pos] is the offset column and row. They are added together to move the
    /// absolute position on the terminal screen. Then
    /// [`RenderOp::MoveCursorPositionAbs`] is used.
    MoveCursorPositionRelTo(
        /* origin position */ Pos,
        /* relative position */ Pos,
    ),

    /// Clears the entire terminal screen and positions cursor at top-left.
    ///
    /// This operation erases all visible content on the terminal screen
    /// and resets the cursor to position (0, 0). Useful for initializing
    /// a clean display state before rendering new content.
    ClearScreen,

    /// Directly set the fg color for crossterm w/out using [`TuiStyle`].
    SetFgColor(TuiColor),

    /// Directly set the bg color for crossterm w/out using [`TuiStyle`].
    SetBgColor(TuiColor),

    /// Resets terminal colors to their default values.
    ///
    /// This clears any previously set foreground and background colors,
    /// returning the terminal to its default color scheme. Essential for
    /// ensuring clean color state between different rendering operations.
    ResetColor,

    /// Translate [`TuiStyle`] into fg and bg colors for crossterm. Note that this does
    /// not apply attributes (bold, italic, underline, strikethrough, etc). If you
    /// need to apply attributes, use [`RenderOp::PaintTextWithAttributes`] instead.
    ApplyColors(Option<TuiStyle>),

    /// Translate [`TuiStyle`] into *only* attributes for crossterm (bold, italic,
    /// underline, strikethrough, etc) and not colors. If you need to apply color, use
    /// [`RenderOp::ApplyColors`] instead.
    ///
    /// 1. If the [`InlineString`] argument is plain text (no ANSI sequences) then it will
    ///    be clipped available width of the terminal screen).
    ///
    /// 2. If the [`InlineString`] argument contains ANSI sequences then it will be
    ///    printed as-is. You are responsible for handling clipping of the text to the
    ///    bounds of the terminal screen.
    PaintTextWithAttributes(InlineString, Option<TuiStyle>),

    /// This is **not** meant for use directly by apps. It is to be used only by the
    /// [`super::OffscreenBuffer`]. This operation skips the checks for content width
    /// padding & clipping, and window bounds clipping. These are not needed when the
    /// compositor is painting an offscreen buffer, since when the offscreen buffer was
    /// created the two render ops above were used which already handle the clipping and
    /// padding.
    CompositorNoClipTruncPaintTextWithAttributes(InlineString, Option<TuiStyle>),

    /// No-operation render operation that does nothing when executed for `Default` impl.
    ///
    /// Used as a placeholder or default value in situations where a [`RenderOp`] is
    /// required but no actual rendering should occur. Safe to include in operation lists
    /// as it has no side effects.
    #[default]
    Noop,
}

/// Core trait implementations for [`RenderOp`].
///
/// This module provides essential trait implementations for render operations,
/// including default values and debug formatting functionality.
mod render_op_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for RenderOp {
        /// When [`crate::RenderPipeline`] is printed as debug, each [`RenderOp`] is
        /// printed using this method. Also [`crate::queue_render_op`!] does not
        /// use this; it has its own way of logging output.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    CrosstermDebugFormatRenderOp {}.fmt_debug(self, f)
                }
                TerminalLibBackend::Termion => unimplemented!(),
            }
        }
    }
}

/// Flush trait implementation for [`RenderOp`].
///
/// This module implements the [`Flush`] trait for render operations,
/// providing methods to flush terminal output and clear before flushing.
mod render_op_impl_trait_flush {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Flush for RenderOp {
        fn flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    RenderOpImplCrossterm {}.flush(locked_output_device);
                }
                TerminalLibBackend::Termion => unimplemented!(),
            }
        }

        fn clear_before_flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    RenderOpImplCrossterm {}.clear_before_flush(locked_output_device);
                }
                TerminalLibBackend::Termion => unimplemented!(),
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FlushKind {
    JustFlush,
    ClearBeforeFlush,
}

/// Trait for controlling terminal output flushing behavior.
///
/// This trait provides methods to flush pending terminal output and optionally
/// clear the terminal before flushing. Essential for ensuring that render
/// operations are actually displayed on the terminal.
pub trait Flush {
    /// Flushes pending output to the terminal.
    ///
    /// This method ensures that all buffered terminal output is written
    /// and displayed immediately.
    fn flush(&mut self, locked_output_device: LockedOutputDevice<'_>);

    /// Clears the terminal before flushing output.
    ///
    /// This method first clears the terminal screen, then flushes any
    /// pending output. Useful for ensuring a clean display state.
    fn clear_before_flush(&mut self, locked_output_device: LockedOutputDevice<'_>);
}

/// Trait for formatting [`RenderOp`] instances in debug output.
///
/// This trait abstracts debug formatting logic, allowing different
/// terminal backends to provide their own specialized debug representations
/// of render operations.
pub trait DebugFormatRenderOp {
    /// Formats the `RenderOp` for debug output.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to the formatter fails.
    fn fmt_debug(&self, this: &RenderOp, f: &mut Formatter<'_>) -> Result;
}

/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::{fmt::{Debug, Formatter, Result},
          ops::{Deref, DerefMut}};

use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use super::TERMINAL_LIB_BACKEND;
use crate::*;

/// Here's an example. Refer to [RenderOps] for more details.
///
/// ```rust
/// use r3bl_tui::*;
///
/// let mut render_ops = render_ops!(
///   @new
///   RenderOp::ClearScreen, RenderOp::ResetColor
/// );
/// let len = render_ops.len();
/// let iter = render_ops.iter();
/// ```
#[macro_export]
macro_rules! render_ops {
  // Empty.
  () => {
    RenderOps::default()
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
      let mut render_ops = RenderOps::default();
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
        $arg_styled_texts.render_into(&mut $arg_render_ops);
      )*
    }
  };
}

/// For ease of use, please use the [render_ops!] macro.
///
/// It is a collection of *atomic* paint operations (aka [`RenderOps`] at various [`ZOrder`]s); each
/// [`RenderOps`] is made up of a [vec] of [`RenderOp`]. It contains `Map<ZOrder, Vec<RenderOps>>`,
/// eg:
/// - [`ZOrder::Normal`] => vec![[`RenderOp::ResetColor`], [`RenderOp::MoveCursorPositionAbs(..)`],
///   [`RenderOp::PrintTextWithAttributes(..)`]]
/// - [`ZOrder::Glass`] => vec![[`RenderOp::ResetColor`], [`RenderOp::MoveCursorPositionAbs(..)`],
///   [`RenderOp::PrintTextWithAttributes(..)`]]
/// - etc.
///
/// ## What is an atomic paint operation?
/// 1. It moves the cursor using:
///     1. [`RenderOp::MoveCursorPositionAbs`]
///     2. [`RenderOp::MoveCursorPositionRelTo`]
/// 2.  And it does not assume that the cursor is in the correct position from some other previously
///     executed operation!
/// 3. So there are no side effects when re-ordering or omitting painting an atomic paint operation
///    (eg in the case where it has already been painted before).
///
/// Here's an example. Consider using the macro for convenience (see [render_ops!]).
///
/// ```rust
/// use r3bl_tui::*;
///
/// let mut render_ops = RenderOps::default();
/// render_ops.push(RenderOp::ClearScreen);
/// render_ops.push(RenderOp::ResetColor);
/// let len = render_ops.len();
/// let iter = render_ops.iter();
/// ```
///
/// ## Paint optimization
/// Due to the compositor [OffscreenBuffer], there is no need to optimize the individual paint
/// operations. You don't have to manage your own whitespace or doing clear before paint! 🎉 The
/// compositor takes care of that for you!
#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RenderOps {
    pub list: Vec<RenderOp>,
}

#[derive(Default, Debug)]
pub struct RenderOpsLocalData {
    pub cursor_position: Position,
}

pub mod render_ops_impl {
    use std::ops::AddAssign;

    use super::*;

    impl RenderOps {
        pub async fn execute_all(
            &self,
            skip_flush: &mut bool,
            shared_global_data: &SharedGlobalData,
        ) {
            let mut local_data = RenderOpsLocalData::default();
            for render_op in self.list.iter() {
                RenderOps::route_paint_render_op_to_backend(
                    &mut local_data,
                    skip_flush,
                    render_op,
                    shared_global_data,
                )
                .await;
            }
        }

        pub async fn route_paint_render_op_to_backend(
            local_data: &mut RenderOpsLocalData,
            skip_flush: &mut bool,
            render_op: &RenderOp,
            shared_global_data: &SharedGlobalData,
        ) {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    RenderOpImplCrossterm {}
                        .paint(skip_flush, render_op, shared_global_data, local_data)
                        .await;
                }
                TerminalLibBackend::Termion => todo!(), // FUTURE: implement PaintRenderOp trait for termion
            }
        }
    }

    impl Deref for RenderOps {
        type Target = Vec<RenderOp>;

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
            let mut vec_lines: Vec<String> = vec![];

            // First line.
            let first_line: String = format!("RenderOps.len(): {}", self.list.len());
            vec_lines.push(first_line);

            // Subsequent lines (optional).
            for render_op in self.iter() {
                let line: String = format!("[{render_op:?}]");
                vec_lines.push(line);
            }

            // Join all lines.
            write!(f, "\n    - {}", vec_lines.join("\n      - "))
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RenderOp {
    EnterRawMode,

    ExitRawMode,

    /// This is always painted on top. [Position] is the absolute column and row on the terminal
    /// screen. This uses [sanitize_and_save_abs_position] to clean up the given
    /// [Position].
    MoveCursorPositionAbs(/* absolute position */ Position),

    /// This is always painted on top. 1st [Position] is the origin column and row, and the 2nd
    /// [Position] is the offset column and row. They are added together to move the absolute position
    /// on the terminal screen. Then [RenderOp::MoveCursorPositionAbs] is used.
    MoveCursorPositionRelTo(
        /* origin position */ Position,
        /* relative position */ Position,
    ),

    ClearScreen,

    /// Directly set the fg color for crossterm w/out using [Style].
    SetFgColor(TuiColor),

    /// Directly set the bg color for crossterm w/out using [Style].
    SetBgColor(TuiColor),

    ResetColor,

    /// Translate [Style] into fg and bg colors for crossterm. Note that this does not
    /// apply attributes (bold, italic, underline, strikethrough, etc). If you need to
    /// apply attributes, use [RenderOp::PaintTextWithAttributes] instead.
    ApplyColors(Option<Style>),

    /// Translate [Style] into *only* attributes for crossterm (bold, italic, underline,
    /// strikethrough, etc) and not colors. If you need to apply color, use
    /// [RenderOp::ApplyColors] instead.
    ///
    /// 1. If the [String] argument is plain text (no ANSI sequences) then it will be
    ///    clipped available width of the terminal screen).
    ///
    /// 2. If the [String] argument contains ANSI sequences then it will be printed as-is.
    ///    You are responsible for handling clipping of the text to the bounds of the
    ///    terminal screen.
    PaintTextWithAttributes(String, Option<Style>),

    /// This is **not** meant for use directly by apps. It is to be used only by the
    /// [OffscreenBuffer]. This operation skips the checks for content width padding & clipping, and
    /// window bounds clipping. These are not needed when the compositor is painting an offscreen
    /// buffer, since when the offscreen buffer was created the two render ops above were used which
    /// already handle the clipping and padding.
    CompositorNoClipTruncPaintTextWithAttributes(String, Option<Style>),

    /// For [Default] impl.
    Noop,
}

mod render_op_impl {
    use super::*;

    impl Default for RenderOp {
        fn default() -> Self { Self::Noop }
    }

    impl Debug for RenderOp {
        /// When [RenderPipeline] is printed as debug, each [RenderOp] is printed using this method. Also
        /// [exec_render_op!] does not use this; it has its own way of logging output.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    CrosstermDebugFormatRenderOp {}.debug_format(self, f)
                }
                TerminalLibBackend::Termion => todo!(), // FUTURE: implement debug formatter for termion
            }
        }
    }
}

mod render_op_impl_trait_flush {
    use super::*;

    impl Flush for RenderOp {
        fn flush(&mut self) {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    RenderOpImplCrossterm {}.flush();
                }
                TerminalLibBackend::Termion => todo!(), // FUTURE: implement flush for termion
            }
        }

        fn clear_before_flush(&mut self) {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    RenderOpImplCrossterm {}.clear_before_flush();
                }
                TerminalLibBackend::Termion => todo!(), // FUTURE: implement clear_before_flush for termion
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FlushKind {
    JustFlush,
    ClearBeforeFlush,
}

pub trait Flush {
    fn flush(&mut self);
    fn clear_before_flush(&mut self);
}

pub trait DebugFormatRenderOp {
    fn debug_format(&self, this: &RenderOp, f: &mut Formatter<'_>) -> Result;
}

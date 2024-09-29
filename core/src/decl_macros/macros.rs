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

/// Wrap the given block or stmt so that it returns a Result<()>. It is just
/// syntactic sugar that helps having to write Ok(()) repeatedly.
///
/// Here's an example.
/// ```ignore
/// use r3bl_rs_utils_core::CommonResult;
///
/// fn test_simple_2_col_layout() -> CommonResult<()> {
///   throws! {
///     match input_event {
///       InputEvent::DisplayableKeypress(character) => {
///         println_raw!(character);
///       }
///       _ => todo!()
///     }
///   }
/// }
/// ```
///
/// Here's another example.
/// ```ignore
/// use r3bl_rs_utils_core::{CommonResult, throws};
///
/// fn test_simple_2_col_layout() -> CommonResult<()> {
///   throws!({
///     let mut canvas = Canvas::default();
///     canvas.stylesheet = create_stylesheet()?;
///     canvas.canvas_start(
///       CanvasPropsBuilder::new()
///         .set_pos((0, 0).into())
///         .set_size((500, 500).into())
///         .build(),
///     )?;
///     layout_container(&mut canvas)?;
///     canvas.canvas_end()?;
///   });
/// }
/// ```
#[macro_export]
macro_rules! throws {
  ($it: block) => {{
    $it
    return Ok(())
  }};
  ($it: stmt) => {{
    $it
    return Ok(())
  }};
}

/// Wrap the given block or stmt so that it returns a Result<$it>. It is just
/// syntactic sugar that helps having to write Ok($it) repeatedly.
///
/// Here's an example.
/// ```ignore
/// throws_with_return!({
///   println!("⛵ Draw -> draw: {}\r", state);
///   render_pipeline!()
/// });
/// ```
#[macro_export]
macro_rules! throws_with_return {
    ($it: block) => {{
        return Ok($it);
    }};
    ($it: stmt) => {{
        return Ok($it);
    }};
}

/// Syntactic sugar to run a conditional statement. Here's an example.
/// ```ignore
/// const DEBUG: bool = true;
/// call_if_true!(
///   DEBUG,
///   eprintln!(
///     "{} {} {}\r",
///     r3bl_rs_utils_core::style_error("▶"),
///     r3bl_rs_utils_core::style_prompt($msg),
///     r3bl_rs_utils_core::style_dimmed(&format!("{:#?}", $err))
///   )
/// );
/// ```
#[macro_export]
macro_rules! call_if_true {
    ($cond:ident, $block: expr) => {{
        if $cond {
            $block
        }
    }};
}

/// This is a really simple macro to make it effortless to use the color console logger.
///
/// It takes a single identifier as an argument, or any number of them. It simply dumps an
/// arrow symbol, followed by the identifier ([stringify]'d) along with the value that it
/// contains (using the [Debug] formatter). All of the output is colorized for easy
/// readability. You can use it like this.
///
/// ```rust
/// use r3bl_rs_utils_core::console_log;
///
/// let my_string = "Hello World!";
/// console_log!(my_string);
/// let my_number = 42;
/// console_log!(my_string, my_number);
/// ```
///
/// You can also use it in these other forms for terminal raw mode output. This will dump
/// the output to stderr.
///
/// ```ignore
/// if let Err(err) = $cmd {
///   let msg = format!("❌ Failed to {}", stringify!($cmd));
///   console_log!(ERROR_RAW &msg, err);
/// }
/// ```
///
/// This will dump the output to stdout.
///
/// ```rust
/// use r3bl_rs_utils_core::console_log;
///
/// let msg = format!("✅ Did the thing to {}", stringify!($name));
/// console_log!(OK_RAW &msg);
/// ```
///
/// <https://danielkeep.github.io/tlborm/book/mbe-macro-rules.html#repetitions>
#[macro_export]
macro_rules! console_log {
  (ERROR_RAW $msg:expr, $err:expr) => {{
    eprintln!(
      "{} {} {}\r",
      r3bl_rs_utils_core::style_error("▶"),
      r3bl_rs_utils_core::style_prompt($msg),
      r3bl_rs_utils_core::style_underline(&format!("{:#?}", $err))
    );
  }};

  (OK_RAW $msg:expr) => {{
    println!(
      "{} {}\r",
      r3bl_rs_utils_core::style_error("▶"),
      r3bl_rs_utils_core::style_prompt($msg)
    );
  }};

  (
    $(                      /* Start a repetition. */
      $element:expr         /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
    $(,)*                   /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
      {
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $element replaced. */
        println!(
          "{} {} = {}",
          r3bl_rs_utils_core::style_error("▶"),
          r3bl_rs_utils_core::style_prompt(stringify!($element)),
          r3bl_rs_utils_core::style_underline(&format!("{:#?}", $element))
        );
      )*
  }};
}

/// Runs the `$code` block after evaluating the `$eval` expression and assigning
/// it to `$id`.
///
/// # Examples:
/// ```ignore
/// with! {
///   LayoutProps {
///     id: id.to_string(),
///     dir,
///     req_size: RequestedSize::new(width_pc, height_pc),
///   },
///   as it,
///   run {
///     match self.is_layout_stack_empty() {
///       true => self.add_root_layout(it),
///       false => self.add_normal_layout(it),
///     }?;
///   }
/// }
/// ```
#[macro_export]
macro_rules! with {
    ($eval:expr, as $id:ident, run $code:block) => {
        let $id = $eval;
        $code;
    };
}

/// Similar to [`with!`] except `$id` is a mutable reference to the `$eval`
/// expression.
///
/// # Examples:
/// ```ignore
/// with_mut! {
///   StyleFlag::BOLD_SET | StyleFlag::DIM_SET,
///   as mask2,
///   run {
///     assert!(mask2.contains(StyleFlag::BOLD_SET));
///     assert!(mask2.contains(StyleFlag::DIM_SET));
///     assert!(!mask2.contains(StyleFlag::UNDERLINE_SET));
///     assert!(!mask2.contains(StyleFlag::COLOR_FG_SET));
///     assert!(!mask2.contains(StyleFlag::COLOR_BG_SET));
///     assert!(!mask2.contains(StyleFlag::PADDING_SET));
///   }
/// }
/// ```
#[macro_export]
macro_rules! with_mut {
    ($eval:expr, as $id:ident, run $code:block) => {
        let mut $id = $eval;
        $code;
    };
}

/// Similar to [`with_mut!`] except that it returns the value of the `$code`
/// block.
///
/// # Examples:
/// ```ignore
/// let queue = with_mut_returns! {
///   ColumnRenderComponent { lolcat },
///   as it,
///   return {
///     let current_box = surface.current_box()?;
///     it.render_component(current_box, state, shared_store).await?
///   }
/// };
/// ```
#[macro_export]
macro_rules! with_mut_returns {
    ($eval:expr, as $id:ident, return $code:block) => {{
        let mut $id = $eval;
        $code
    }};
}

/// Unwrap the `$option`, and if `None` then run the `$next` closure which must
/// return an error. This macro must be called in a block that returns a
/// `CommonResult<T>`.
///
/// # Example
///
/// ```ignore
/// pub fn from(
///   width_percent: u8,
///   height_percent: u8,
/// ) -> CommonResult<RequestedSize> {
///   let size_tuple = (width_percent, height_percent);
///   let (width_pc, height_pc) = unwrap_option_or_run_fn_returning_err!(
///     convert_to_percent(size_tuple),
///     || LayoutError::new_err(LayoutErrorType::InvalidLayoutSizePercentage)
///   );
///   Ok(Self::new(width_pc, height_pc))
/// }
/// ```
#[macro_export]
macro_rules! unwrap_option_or_run_fn_returning_err {
    ($option:expr, $next:expr) => {
        match $option {
            Some(value) => value,
            None => return $next(),
        }
    };
}

/// Basically a way to compute something lazily when it (the `Option`) is set to `None`.
///
/// Unwrap the `$option`, and if `None` then run the `$next` closure which must return a
/// value that is set to `$option`.
///
/// # Example
///
/// ```ignore
/// use r3bl_rs_utils_core::unwrap_option_or_compute_if_none;
///
/// #[test]
/// fn test_unwrap_option_or_compute_if_none() {
///   struct MyStruct {
///     field: Option<i32>,
///   }
///   let mut my_struct = MyStruct { field: None };
///   assert_eq!(my_struct.field, None);
///   unwrap_option_or_compute_if_none!(my_struct.field, { || 1 });
///   assert_eq!(my_struct.field, Some(1));
/// }
/// ```
#[macro_export]
macro_rules! unwrap_option_or_compute_if_none {
    ($option:expr, $next:expr) => {
        match $option {
            Some(value) => value,
            None => {
                $option = Some($next());
                $option.unwrap()
            }
        }
    };
}

/// Similar to [`assert_eq!`] but automatically prints the left and right hand side
/// variables if the assertion fails.
///
/// Useful for debugging tests, since cargo would just print out the left and right values
/// *w/out* providing information on *what variables* were being compared.
#[macro_export]
macro_rules! assert_eq2_og {
    ($left:expr, $right:expr $(,)?) => {
        assert_eq!(
            $left,
            $right,
            "\n😮 {}\nleft : `{}`\nright: `{}`\nline :",
            $crate::style_prompt("Houston, we have a problem..."),
            $crate::style_error(stringify!($left)),
            $crate::style_error(stringify!($right))
        );
    };
}

/// A wrapper for `pretty_assertions::assert_eq!` macro.
#[macro_export]
macro_rules! assert_eq2 {
    ($($params:tt)*) => {
        pretty_assertions::assert_eq!($($params)*)
    };
}

/// Send a signal to the main thread of app to render. The two things to pass in this macro are
/// 1. Sender
/// 2. AppEvent (Signal to MPSC channel)
#[macro_export]
macro_rules! send_signal {
    (
        $main_thread_channel_sender : expr,
        $signal : expr
    ) => {{
        let sender_clone = $main_thread_channel_sender.clone();

        // Note: make sure to wrap the call to `send` in a `tokio::spawn()` so
        // that it doesn't block the calling thread. More info:
        // <https://tokio.rs/tokio/tutorial/channels>.
        tokio::spawn(async move {
            let _ = sender_clone.send($signal).await;
        });
    }};
}

/// Simple macro to create a [`Result`] with an [`Ok`] variant. It is just syntactic sugar
/// that helps having to write `Ok(())`.
/// - If no arg is passed in then it will return `Ok(())`.
/// - If an arg is passed in then it will return `Ok($arg)`.
#[macro_export]
macro_rules! ok {
    // No args.
    () => {
        Ok(())
    };
    // With arg.
    ($value:expr) => {
        Ok($value)
    };
}

/// A decl macro that generates code to measure the performance of the block that it
/// surrounds.
///
/// # Returns
///
/// If you use `timed!($expr)` then it will return a tuple of `($expr, duration)`.
///
/// # Example
///
/// ```
/// use r3bl_rs_utils_core::timed;
/// use sha2::{Digest, Sha256};
/// let (retval, duration) = timed!({
///     let prompt = "Hello, World!";
///     let mut hasher = Sha256::new();
///     hasher.update(prompt);
///     let result = hasher.finalize();
///     let mut bytes = [0u8; 4];
///     bytes.copy_from_slice(&result.as_slice()[..4]);
///     u32::from_le_bytes(bytes)
/// });
/// ```
#[macro_export]
macro_rules! timed {
    ($block:block) => {{
        let start = std::time::Instant::now();
        let retval = $block;
        let duration = start.elapsed();
        (retval, duration)
    }};
}

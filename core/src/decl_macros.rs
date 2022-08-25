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
/// fn test_simple_2_col_layout() -> CommonResult<()> {
///   throws! {
///     match input_event {
///       TWInputEvent::DisplayableKeypress(character) => {
///         println_raw!(character);
///       }
///       _ => todo!()
///     }
///   }
/// }
/// ```
///
/// Here's another example.
/// ```rust
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
///   CommandQueue::default()
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

/// Declarative macro to surround the given block with a call to
/// [`tokio::spawn`]. This is useful for spawning a task that will run in the
/// background from a function that is NOT async.
///
/// # Examples:
///
/// Your block can be sync and `foo` is not async.
///
/// ```no_run
/// pub fn foo() {
///   fire_and_forget!({
///     println!("Hello");
///   });
/// }
/// ```
///
/// Your block can be async and `foo` is still not async.
///
/// ```no_run
/// pub fn foo() {
///   fire_and_forget!(
///      let fake_data = fake_contact_data_api()
///      .await
///      .unwrap_or_else(|_| FakeContactData {
///        name: "Foo Bar".to_string(),
///        phone_h: "123-456-7890".to_string(),
///        email_u: "foo".to_string(),
///        email_d: "bar.com".to_string(),
///        ..FakeContactData::default()
///      });
///   );
/// }
/// ```
#[macro_export]
macro_rules! fire_and_forget {
  ($block:block) => {
    return tokio::spawn(async move { $block });
  };
}

/// Syntactic sugar to run a conditional statement. Here's an example.
/// ```rust
/// const DEBUG: bool = true;
/// call_if_true!(
///   DEBUG,
///   eprintln!(
///     "{} {} {}\r",
///     r3bl_rs_utils::style_error("▶"),
///     r3bl_rs_utils::style_prompt($msg),
///     r3bl_rs_utils::style_dimmed(&format!("{:#?}", $err))
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

/// This is a really simple macro to make it effortless to use the color console
/// logger. It takes a single identifier as an argument, or any number of them.
/// It simply dumps an arrow symbol, followed by the identifier ([stringify]'d)
/// along with the value that it contains (using the [Debug] formatter). All of
/// the output is colorized for easy readability. You can use it like this.
///
/// ```rust
/// let my_string = "Hello World!";
/// debug!(my_string);
/// let my_number = 42;
/// debug!(my_string, my_number);
/// ```
///
/// You can also use it in these other forms for terminal raw mode output. This
/// will dump the output to stderr.
///
/// ```rust
/// if let Err(err) = $cmd {
///   let msg = format!("❌ Failed to {}", stringify!($cmd));
///   debug!(ERROR_RAW &msg, err);
/// }
/// ```
///
/// This will dump the output to stdout.
///
/// ```rust
/// let msg = format!("✅ Did the thing to {}", stringify!($name));
/// debug!(OK_RAW &msg);
/// ```
///
/// <https://danielkeep.github.io/tlborm/book/mbe-macro-rules.html#repetitions>
#[macro_export]
macro_rules! debug {
  (ERROR_RAW $msg:expr, $err:expr) => {{
    call_if_true!(
      DEBUG,
      eprintln!(
        "{} {} {}\r",
        r3bl_rs_utils_core::style_error("▶"),
        r3bl_rs_utils_core::style_prompt($msg),
        r3bl_rs_utils_core::style_dimmed(&format!("{:#?}", $err))
      )
    );
  }};

  (OK_RAW $msg:expr) => {{
    call_if_true(
      DEBUG,
      println!(
        "{} {}\r",
        r3bl_rs_utils_core::style_error("▶"),
        r3bl_rs_utils_core::style_prompt($msg)
      ),
    )
  }};

  (
    $(                      /* Start a repetition. */
      $element:expr         /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
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
          r3bl_rs_utils_core::style_dimmed(&format!("{:#?}", $element))
        );
      )*
  }};
}

/// Declarative macro to generate the API call functions. This adds the
/// following:
/// - `make_request()` async function to call the API.
/// - `to_string()` function to stringify the struct to JSON.
/// - impl `Display` trait to for the struct using `to_string()` above.
#[macro_export]
macro_rules! make_api_call_for {
  ($IDENT:ident at $ENDPOINT:ident) => {
    pub async fn make_request() -> Result<$IDENT, Box<dyn Error>> {
      let res = reqwest::get($ENDPOINT).await?;
      let res_text = res.text().await?;
      let res_json: $IDENT = serde_json::from_str(&res_text)?;
      Ok(res_json)
    }

    impl $IDENT {
      pub fn to_string(&self) -> String { serde_json::to_string(&self).unwrap() }
    }

    impl Display for $IDENT {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
      }
    }
  };
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
/// let tw_queue = with_mut_returns! {
///   ColumnRenderComponent { lolcat },
///   as it,
///   return {
///     let current_box = tw_surface.current_box()?;
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

/// Unwrap the `$option`, and if `None` then run the `$next` closure which must
/// return a value that is set to `$option`. Basically a way to compute
/// something lazily when it (the `Option`) is set to `None`.
///
/// # Example
///
/// ```
/// use r3bl_rs_utils::unwrap_option_or_compute_if_none;
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

/// Similar to [`assert_eq!`] but automatically prints the left and right hand side variables if the
/// assertion fails. Useful for debugging tests, since cargo would just print out the left and right
/// values *w/out* providing information on *what variables* were being compared.
#[macro_export]
macro_rules! assert_eq2 {
  ($left:expr, $right:expr $(,)?) => {
    assert_eq!(
      $left,
      $right,
      "\n😮 {}\nleft-expr : {}\nright-expr: {}",
      r3bl_rs_utils::style_prompt("Houston, we have a problem..."),
      r3bl_rs_utils::style_error(stringify!($left)),
      r3bl_rs_utils::style_error(stringify!($right))
    );
  };
}

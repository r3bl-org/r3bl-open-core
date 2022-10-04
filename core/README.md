<p align="center">
  <img src="r3bl-term.svg" height="128px">
</p>

# Context
<a id="markdown-context" name="context"></a>


<!-- R3BL TUI library & suite of apps focused on developer productivity -->

<span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span style="color:#F92363">
</span><span style="color:#F82067">T</span><span style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span style="color:#F31874">
</span><span style="color:#F11678">l</span><span style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span style="color:#E10895">
</span><span style="color:#DE0799">&amp;</span><span style="color:#DB069E">
</span><span style="color:#D804A2">s</span><span style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span style="color:#C801B6">
</span><span style="color:#C501B9">o</span><span style="color:#C101BD">f</span><span style="color:#BD01C1">
</span><span style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span style="color:#AA03D2">
</span><span style="color:#A603D5">f</span><span style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span style="color:#890DE8">
</span><span style="color:#850FEB">o</span><span style="color:#8111ED">n</span><span style="color:#7C13EF">
</span><span style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span style="color:#572CFC">r</span><span style="color:#532FFD">
</span><span style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>

We are working on building command line apps in Rust which have rich text user interfaces (TUI). We
want to lean into the terminal as a place of productivity, and build all kinds of awesome apps for
it.

1. 🔮 Instead of just building one app, we are building a library to enable any kind of rich TUI
   development w/ a twist: taking concepts that work really well for the frontend mobile and web
   development world and re-imagining them for TUI & Rust.

   - Taking things like React, JSX, CSS, and Redux, but making everything async (they can be run in
     parallel & concurrent via Tokio).
   - Even the thread running the main event loop doesn't block since it is async.
   - Using proc macros to create DSLs to implement CSS & JSX.

2. 🌎 We are building apps to enhance developer productivity & workflows.

   - The idea here is not to rebuild tmux in Rust (separate processes mux'd onto a single terminal
     window). Rather it is to build a set of integrated "apps" (or "tasks") that run in the same
     process that renders to one terminal window.
   - Inside of this terminal window, we can implement things like "app" switching, routing, tiling
     layout, stacking layout, etc. so that we can manage a lot of TUI apps (which are tightly
     integrated) that are running in the same process, in the same window. So you can imagine that
     all these "app"s have shared application state (that is in a Redux store). Each "app" may also
     have its own Redux store.
   - Here are some examples of the types of "app"s we want to build:
     1. multi user text editors w/ syntax highlighting
     2. integrations w/ github issues
     3. integrations w/ calendar, email, contacts APIs

# r3bl_rs_utils_core
<a id="markdown-r3bl_rs_utils_core" name="r3bl_rs_utils_core"></a>


<!-- TOC depthfrom:2 updateonsave:true orderedlist:false insertanchor:true -->

- [Macros](#macros)
  - [Declarative](#declarative)
    - [assert_eq2!](#assert_eq2)
    - [throws!](#throws)
    - [throws_with_return!](#throws_with_return)
    - [log!](#log)
    - [log_no_err!](#log_no_err)
    - [debug_log_no_err!](#debug_log_no_err)
    - [trace_log_no_err!](#trace_log_no_err)
    - [make_api_call_for!](#make_api_call_for)
    - [fire_and_forget!](#fire_and_forget)
    - [call_if_true!](#call_if_true)
    - [debug!](#debug)
    - [with!](#with)
    - [with_mut!](#with_mut)
    - [with_mut_returns!](#with_mut_returns)
    - [unwrap_option_or_run_fn_returning_err!](#unwrap_option_or_run_fn_returning_err)
    - [unwrap_option_or_compute_if_none!](#unwrap_option_or_compute_if_none)
- [Common](#common)
  - [CommonResult and CommonError](#commonresult-and-commonerror)
- [Other crates that depend on this](#other-crates-that-depend-on-this)
- [Issues, comments, feedback, and PRs](#issues-comments-feedback-and-prs)

<!-- /TOC -->

## Macros
<a id="markdown-macros" name="macros"></a>


### Declarative
<a id="markdown-declarative" name="declarative"></a>


There are quite a few declarative macros that you will find in the library. They tend to be used
internally in the implementation of the library itself. Here are some that are actually externally
exposed via `#[macro_export]`.

#### assert_eq2!
<a id="markdown-assert_eq2!" name="assert_eq2!"></a>


Similar to [`assert_eq!`] but automatically prints the left and right hand side variables if the
assertion fails. Useful for debugging tests, since the cargo would just print out the left and right
values w/out providing information on what variables were being compared.

#### throws!
<a id="markdown-throws!" name="throws!"></a>


Wrap the given `block` or `stmt` so that it returns a `Result<()>`. It is just syntactic sugar that
helps having to write `Ok(())` repeatedly at the end of each block. Here's an example.

```rust
fn test_simple_2_col_layout() -> CommonResult<()> {
  throws! {
    match input_event {
      InputEvent::DisplayableKeypress(character) => {
        println_raw!(character);
      }
      _ => todo!()
    }
  }
}
```

Here's another example.

```rust
fn test_simple_2_col_layout() -> CommonResult<()> {
  throws!({
    let mut canvas = Canvas::default();
    canvas.stylesheet = create_stylesheet()?;
    canvas.canvas_start(
      CanvasPropsBuilder::new()
        .set_pos((0, 0).into())
        .set_size((500, 500).into())
        .build(),
    )?;
    layout_container(&mut canvas)?;
    canvas.canvas_end()?;
  });
}
```

#### throws_with_return!
<a id="markdown-throws_with_return!" name="throws_with_return!"></a>


This is very similar to [`throws!`](#throws) but it also returns the result of the block.

```rust
fn test_simple_2_col_layout() -> CommonResult<RenderPipeline> {
  throws_with_return!({
    println!("⛵ Draw -> draw: {}\r", state);
    RenderPipeline::default()
  });
}
```

#### log!
<a id="markdown-log!" name="log!"></a>


You can use this macro to dump log messages at 3 levels to a file. By default this file is named
`log.txt` and is dumped in the current directory. Here's how you can use it.

Please note that the macro returns a `Result`. A type alias is provided to save some typing called
`CommonResult<T>` which is just a short hand for `std::result::Result<T, Box<dyn Error>>`. The log
file itself is overwritten for each "session" that you run your program.

```rust
use r3bl_rs_utils::{init_file_logger_once, log, CommonResult};

fn run() -> CommonResult<()> {
  let msg = "foo";
  let msg_2 = "bar";

  log!(INFO, "This is a info message");
  log!(INFO, target: "foo", "This is a info message");

  log!(WARN, "This is a warning message {}", msg);
  log!(WARN, target: "foo", "This is a warning message {}", msg);

  log!(ERROR, "This is a error message {} {}", msg, msg_2);
  log!(ERROR, target: "foo", "This is a error message {} {}", msg, msg_2);

  log!(DEBUG, "This is a debug message {} {}", msg, msg_2);
  log!(DEBUG, target: "foo", "This is a debug message {} {}", msg, msg_2);

  log!(TRACE, "This is a debug message {} {}", msg, msg_2);
  log!(TRACE, target: "foo", "This is a debug message {} {}", msg, msg_2);

  Ok(())
}
```

To change the default log file to whatever you choose, you can use the `try_to_set_log_file_path()`
function. If the logger hasn't yet been initialized, this function will set the log file path.
Otherwise it will return an error.

```rust
use r3bl_rs_utils::{try_set_log_file_path, CommonResult, CommonError};
fn run() {
  match try_set_log_file_path("new_log.txt") {
      Ok(path_set) => debug!(path_set),
      Err(error) => debug!(error),
  }
}
```

To change the default log level or to disable the log itself, you can use the
`try_to_set_log_level()` function.

If you want to override the default log level `LOG_LEVEL`, you can use this function. If the logger
has already been initialized, then it will return a an error.

```rust
use r3bl_rs_utils::{try_to_set_log_level, CommonResult, CommonError};
use log::LevelFilter;

fn run() {
  match try_to_set_log_level(LevelFilter::Trace) {
      Ok(level_set) => debug!(level_set),
      Err(error) => debug!(error),
  }
}
```

To disable logging simply set the log level to
[`LevelFilter::Off`](https://docs.rs/log/latest/log/enum.LevelFilter.html).

```rust
use r3bl_rs_utils::{try_to_set_log_level, CommonResult, CommonError};
use log::LevelFilter;

fn run() {
  match try_to_set_log_level(LevelFilter::Off) {
      Ok(level_set) => debug!(level_set),
      Err(error) => debug!(error),
  }
}
```

Please check out the source
[here](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/src/utils/file_logging.rs).

#### log_no_err!
<a id="markdown-log_no_err!" name="log_no_err!"></a>


This macro is very similar to the [log!](#log) macro, except that it won't return any error if the
underlying logging system fails. It will simply print a message to `stderr`. Here's an example.

```rust
pub fn log_state(&self, msg: &str) {
  log_no_err!(INFO, "{:?} -> {}", msg, self.to_string());
  log_no_err!(INFO, target: "foo", "{:?} -> {}", msg, self.to_string());
}
```

#### debug_log_no_err!
<a id="markdown-debug_log_no_err!" name="debug_log_no_err!"></a>


This is a really simple macro to make it effortless to debug into a log file. It outputs `DEBUG`
level logs. It takes a single identifier as an argument, or any number of them. It simply dumps an
arrow symbol, followed by the identifier `stringify`'d along with the value that it contains (using
the `Debug` formatter). All of the output is colorized for easy readability. You can use it like
this.

```rust
let my_string = "Hello World!";
debug_log_no_err!(my_string);
```

#### trace_log_no_err!
<a id="markdown-trace_log_no_err!" name="trace_log_no_err!"></a>


This is very similar to [debug_log_no_err!](#debuglognoerr) except that it outputs `TRACE` level
logs.

```rust
let my_string = "Hello World!";
trace_log_no_err!(my_string);
```

#### make_api_call_for!
<a id="markdown-make_api_call_for!" name="make_api_call_for!"></a>


This macro makes it easy to create simple HTTP GET requests using the `reqwest` crate. It generates
an `async` function called `make_request()` that returns a `CommonResult<T>` where `T` is the type
of the response body. Here's an example.

```rust
use std::{error::Error, fmt::Display};
use r3bl_rs_utils::make_api_call_for;
use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "https://api.namefake.com/english-united-states/female/";

make_api_call_for! {
  FakeContactData at ENDPOINT
}
#[derive(Serialize, Deserialize, Debug, Default)]

pub struct FakeContactData {
  pub name: String,
  pub phone_h: String,
  pub email_u: String,
  pub email_d: String,
  pub address: String,
}

let fake_data = fake_contact_data_api()
            .await
            .unwrap_or_else(|_| FakeContactData {
              name: "Foo Bar".to_string(),
              phone_h: "123-456-7890".to_string(),
              email_u: "foo".to_string(),
              email_d: "bar.com".to_string(),
              ..FakeContactData::default()
            });
```

You can find lots of
[examples here](https://github.com/r3bl-org/address-book-with-redux-tui/blob/main/src/tui/middlewares).

#### fire_and_forget!
<a id="markdown-fire_and_forget!" name="fire_and_forget!"></a>


This is a really simple wrapper around `tokio::spawn()` for the given block. Its just syntactic
sugar. Here's an example of using it for a non-`async` block.

```rust
pub fn foo() {
  fire_and_forget!(
    { println!("Hello"); }
  );
}
```

And, here's an example of using it for an `async` block.

```rust
pub fn foo() {
  fire_and_forget!(
     let fake_data = fake_contact_data_api()
     .await
     .unwrap_or_else(|_| FakeContactData {
       name: "Foo Bar".to_string(),
       phone_h: "123-456-7890".to_string(),
       email_u: "foo".to_string(),
       email_d: "bar.com".to_string(),
       ..FakeContactData::default()
     });
  );
}
```

#### call_if_true!
<a id="markdown-call_if_true!" name="call_if_true!"></a>


Syntactic sugar to run a conditional statement. Here's an example.

```rust
const DEBUG: bool = true;
call_if_true!(
  DEBUG,
  eprintln!(
    "{} {} {}\r",
    r3bl_rs_utils::style_error("▶"),
    r3bl_rs_utils::style_prompt($msg),
    r3bl_rs_utils::style_dimmed(&format!("{:#?}", $err))
  )
);
```

#### debug!
<a id="markdown-debug!" name="debug!"></a>


This is a really simple macro to make it effortless to use the color console logger. It takes a
single identifier as an argument, or any number of them. It simply dumps an arrow symbol, followed
by the identifier (stringified) along with the value that it contains (using the `Debug` formatter).
All of the output is colorized for easy readability. You can use it like this.

```rust
let my_string = "Hello World!";
debug!(my_string);
let my_number = 42;
debug!(my_string, my_number);
```

You can also use it in these other forms for terminal raw mode output. This will dump the output to
stderr.

```rust
if let Err(err) = $cmd {
  let msg = format!("❌ Failed to {}", stringify!($cmd));
  debug!(ERROR_RAW &msg, err);
}
```

This will dump the output to stdout.

```rust
let msg = format!("✅ Did the thing to {}", stringify!($name));
debug!(OK_RAW &msg);
```

#### with!
<a id="markdown-with!" name="with!"></a>


This is a macro that takes inspiration from the `with` scoping function in Kotlin. It just makes it
easier to express a block of code that needs to run after an expression is evaluated and saved to a
given variable. Here's an example.

```rust
with! {
  /* $eval */ LayoutProps {
    id: id.to_string(),
    dir,
    req_size: RequestedSize::new(width_pc, height_pc),
  },
  as /* $id */ it,
  run /* $code */ {
    match self.is_layout_stack_empty() {
      true => self.add_root_layout(it),
      false => self.add_normal_layout(it),
    }?;
  }
}
```

It does the following:

1. Evaluates the `$eval` expression and assigns it to `$id`.
2. Runs the `$code` block.

#### with_mut!
<a id="markdown-with_mut!" name="with_mut!"></a>


This macro is just like [`with!`](#with) but it takes a mutable reference to the `$id` variable.
Here's a code example.

```rust
with_mut! {
  StyleFlag::BOLD_SET | StyleFlag::DIM_SET,
  as mask2,
  run {
    assert!(mask2.contains(StyleFlag::BOLD_SET));
    assert!(mask2.contains(StyleFlag::DIM_SET));
    assert!(!mask2.contains(StyleFlag::UNDERLINE_SET));
    assert!(!mask2.contains(StyleFlag::COLOR_FG_SET));
    assert!(!mask2.contains(StyleFlag::COLOR_BG_SET));
    assert!(!mask2.contains(StyleFlag::PADDING_SET));
  }
}
```

#### with_mut_returns!
<a id="markdown-with_mut_returns!" name="with_mut_returns!"></a>


This macro is just like [`with_mut!`](#withmutreturns) except that it returns the value of the
`$code` block. Here's a code example.

```rust
let tw_queue = with_mut_returns! {
    ColumnRenderComponent { lolcat },
    as it,
    return {
      it.render_component(tw_surface.current_box()?, state, shared_store).await?
    }
};
```

#### unwrap_option_or_run_fn_returning_err!
<a id="markdown-unwrap_option_or_run_fn_returning_err!" name="unwrap_option_or_run_fn_returning_err!"></a>


This macro can be useful when you are working w/ an expression that returns an `Option` and if that
`Option` is `None` then you want to abort and return an error immediately. The idea is that you are
using this macro in a function that returns a `Result<T>` basically.

Here's an example to illustrate.

```rust
pub fn from(
  width_percent: u8,
  height_percent: u8,
) -> CommonResult<RequestedSize> {
  let size_tuple = (width_percent, height_percent);
  let (width_pc, height_pc) = unwrap_option_or_run_fn_returning_err!(
    convert_to_percent(size_tuple),
    || LayoutError::new_err(LayoutErrorType::InvalidLayoutSizePercentage)
  );
  Ok(Self::new(width_pc, height_pc))
}
```

#### unwrap_option_or_compute_if_none!
<a id="markdown-unwrap_option_or_compute_if_none!" name="unwrap_option_or_compute_if_none!"></a>


This macro is basically a way to compute something lazily when it (the `Option`) is set to `None`.
Unwrap the `$option`, and if `None` then run the `$next` closure which must return a value that is
set to `$option`. Here's an example.

```rust
use r3bl_rs_utils::unwrap_option_or_compute_if_none;

#[test]
fn test_unwrap_option_or_compute_if_none() {
  struct MyStruct {
    field: Option<i32>,
  }
  let mut my_struct = MyStruct { field: None };
  assert_eq!(my_struct.field, None);
  unwrap_option_or_compute_if_none!(my_struct.field, { || 1 });
  assert_eq!(my_struct.field, Some(1));
}
```

## Common
<a id="markdown-common" name="common"></a>


### CommonResult and CommonError
<a id="markdown-commonresult-and-commonerror" name="commonresult-and-commonerror"></a>


These two structs make it easier to work w/ `Result`s. They are just syntactic sugar and helper
structs. You will find them used everywhere in the
[`r3bl_rs_utils`](https://crates.io/crates/r3bl_rs_utils) crate.

Here's an example of using them both.

```rust
use r3bl_rs_utils::{CommonError, CommonResult};

#[derive(Default, Debug, Clone)]
pub struct Stylesheet {
  pub styles: Vec<Style>,
}

impl Stylesheet {
  pub fn add_style(
    &mut self,
    style: Style,
  ) -> CommonResult<()> {
    if style.id.is_empty() {
      return CommonError::new_err_with_only_msg("Style id cannot be empty");
    }
    self.styles.push(style);
    Ok(())
  }
}
```

## Other crates that depend on this
<a id="markdown-other-crates-that-depend-on-this" name="other-crates-that-depend-on-this"></a>


This crate is a dependency of the following crates:

1. [`r3bl_rs_utils_macro`](https://crates.io/crates/r3bl_rs_utils_macro) (procedural macros)
2. [`r3bl_rs_utils`](https://crates.io/crates/r3bl_rs_utils) crates (the "main" library)

## Issues, comments, feedback, and PRs
<a id="markdown-issues%2C-comments%2C-feedback%2C-and-prs" name="issues%2C-comments%2C-feedback%2C-and-prs"></a>


Please report any issues to the [issue tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues).
And if you have any feature requests, feel free to add them there too 👍.

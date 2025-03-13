/*
 *   Copyright (c) 2022-2025 R3BL LLC
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
/// # Example 1
///
/// ```
/// use r3bl_core::{CommonResult, throws};
///
/// fn test_simple_2_col_layout() -> CommonResult<()> {
///     let input_event = Some("a");
///     throws! {
///         match input_event {
///             Some(character) => println!("{:?}", character),
///             _ => todo!(),
///         }
///     }
/// }
/// ```
///
/// # Example 2
///
/// ```
/// use r3bl_core::{CommonResult, throws};
///
/// fn test_simple_2_col_layout() -> CommonResult<()> {
///     throws!({
///         let result: miette::Result<&str> = Ok("foo bar");
///         _ = result?;
///         ()
///     });
/// }
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
/// # Example
///
/// ```no_run
/// use r3bl_core::{throws_with_return, CommonResult};
/// fn function_returns_string() -> CommonResult<&'static str> {
///     throws_with_return!({
///         println!("⛵ Draw -> draw: {}\r", "state");
///         "Hello, World!"
///     });
/// }
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

/// Syntactic sugar to run a conditional statement. You can use [bool::then()] instead of
/// this macro in most case, except for when you need to return something from the block.
///
/// # Example
///
/// ```
/// use r3bl_core::call_if_true;
/// const DBG_FLAG: bool = true;
/// call_if_true!(
///     DBG_FLAG,
///     eprintln!(
///         "{} {} {}\r",
///         "one",
///         "two",
///         "three"
///     )
/// );
/// ```
#[macro_export]
macro_rules! call_if_true {
    ($cond:expr, $block: expr) => {{
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
/// # Example 1
///
/// ```rust
/// use r3bl_core::console_log;
///
/// let my_string = "Hello World!";
/// console_log!(my_string);
///
/// let my_number = 42;
/// console_log!(my_string, my_number);
/// ```
///
/// # Example 2
///
/// You can also use it in these other forms for terminal raw mode output. This will dump
/// the output to stderr.
///
/// ```rust
/// use r3bl_core::console_log;
/// let result: miette::Result<String> = Ok("foo".to_string());
/// if let Err(err) = result {
///     let msg = format!("❌ Failed to {}", stringify!($cmd));
///     console_log!(ERROR_RAW &msg, err);
/// }
/// ```
///
/// # Example 3
///
/// This will dump the output to stdout.
///
/// ```rust
/// use r3bl_core::console_log;
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
            r3bl_ansi_color::red("▶"),
            r3bl_ansi_color::green($msg),
            r3bl_ansi_color::underline(&format!("{:#?}", $err))
        );
    }};

    (OK_RAW $msg:expr) => {{
        println!(
            "{} {}\r",
            r3bl_ansi_color::red("▶"),
            r3bl_ansi_color::green($msg)
        );
    }};

    (
        $(                      /* Start a repetition. */
            $element:expr       /* Expression. */
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
                    "{} {} <- {}",
                    r3bl_ansi_color::red("▶"),
                    r3bl_ansi_color::underline(&format!("{:#?}", $element)),
                    r3bl_ansi_color::green(stringify!($element))
                );
            )*
        }
    };
}

/// Runs the `$code` block after evaluating the `$eval` expression and assigning it to
/// `$id`.
/// - It returns the `$id` after running the `$code` block.
/// - Note that `$id` is not leaked into the caller's scope / block.
///
/// # Examples
///
/// ```no_run
/// use r3bl_core::with;
/// let it = with! {
///     Some(12),
///     as it /* This only exists in the scope of the run block below. */,
///     run {
///         match it {
///             Some(val) => assert!(val == 12),
///             _ => todo!()
///         };
///     }
/// };
/// assert!(it == Some(12));
/// ```
#[macro_export]
macro_rules! with {
    ($eval:expr, as $id:ident, run $code:block) => {{
        let $id = $eval;
        $code;
        $id
    }};
}

/// Similar to [`with!`] except `$id` is a mutable reference to the `$eval` expression.
/// - It returns the `$id` after running the `$code` block.
/// - Note that `$id` is not leaked into the caller's scope / block.
///
/// # Example
///
/// ```rust
/// use r3bl_core::with_mut;
/// let it = with_mut! {
///     vec!["one", "two", "three"],
///     as it /* This only exists in the scope of the run block below. */,
///     run {
///         it.push("four");
///         assert_eq!(it.len(), 4);
///     }
/// };
/// assert!(it.len() == 4);
/// ```
#[macro_export]
macro_rules! with_mut {
    ($eval:expr, as $id:ident, run $code:block) => {{
        let mut $id = $eval;
        $code;
        $id
    }};
}

/// Similar to [`with_mut!`] except that it returns the value of the `$code` block.
/// - Note that `$id` is not leaked into the caller's scope / block.
///
/// # Example
///
/// ```rust
/// use r3bl_core::with_mut_returns;
/// let queue = with_mut_returns! {
///     vec![1, 2, 3],
///     as it,
///     return {
///         it.push(4);
///         assert_eq!(it.len(), 4);
///         it[3]
///     }
/// };
/// ```
#[macro_export]
macro_rules! with_mut_returns {
    ($eval:expr, as $id:ident, return $code:block) => {{
        let mut $id = $eval;
        $code
    }};
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
/// use r3bl_core::timed;
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

/// A decl macro that generates a global and mutable singleton instance of a struct that
/// is safe. This struct must implement the [Default] trait.
///
/// This macro also generates Rust docs for the generated code, so you can see the
/// documentation with `cargo doc --no-deps --open`.
///
/// # Arguments
///
/// The arguments to this macro are:
/// 1. The struct type (which must implement [Default] trait).
/// 2. The global variable name.
///
/// # Example
///
/// ```no_run
/// use r3bl_core::create_global_singleton;
///
/// #[derive(Default)]
/// pub struct MyStruct (i32);
///
/// create_global_singleton!(MyStruct, GLOBAL_MY_STRUCT);
///
/// let singleton = MyStruct::get_mut_singleton().unwrap();
/// singleton.lock().unwrap().0 = 42;
/// ```
///
/// More info on generating doc comments in declarative macros:
/// - <https://stackoverflow.com/questions/33999341/generating-documentation-in-macros>
#[macro_export]
macro_rules! create_global_singleton {
    ($struct_type:ty, $global_var_name:ident) => {
        paste::paste! {
            #[doc = concat!(
                "A global [std::sync::Once] instance to ensure that the global
                [", stringify!($global_var_name), "]
                is initialized only once."
            )]
            pub static [<ONCE_ $global_var_name>]: std::sync::Once = std::sync::Once::new();

            #[doc = concat!(
                "A global instance of
                [", stringify!($struct_type), "]
                that is mutable. Even though this is globally mutable `unsafe` is not required,
                since it is protected by a [std::sync::Mutex], wrapped in an [std::sync::Arc]."
            )]
            pub static mut $global_var_name: Option<std::sync::Arc<std::sync::Mutex<$struct_type>>> = None;

            #[allow(dead_code)]
            impl $struct_type {
                /// Returns a mutable reference to the global singleton instance [$global_var_name]
                /// of type [$struct_type].
                #[allow(static_mut_refs)]
                pub fn get_mut_singleton() -> miette::Result<std::sync::Arc<std::sync::Mutex<$struct_type>>> {
                    unsafe {
                        [<ONCE_ $global_var_name>].call_once(|| {
                            $global_var_name = Some(std::sync::Arc::new(std::sync::Mutex::new(<$struct_type>::default())));
                        });

                        if let Some(ref global_var) = $global_var_name {
                            Ok(global_var.clone())
                        } else {
                            let err_msg = concat!("Failed to initialize the global mutable variable: ", stringify!($global_var_name));
                            miette::bail!(err_msg);
                        }
                    }
                }
            }

        }
    };
}

#[cfg(test)]
mod tests_singleton {
    #[test]
    fn test_singleton_macro_once() {
        #[derive(Default)]
        pub struct MyStruct {
            pub field: i32,
        }

        create_global_singleton!(MyStruct, GLOBAL_MY_STRUCT);

        unsafe {
            ONCE_GLOBAL_MY_STRUCT.call_once(|| {
                GLOBAL_MY_STRUCT =
                    Some(std::sync::Arc::new(std::sync::Mutex::new(MyStruct {
                        field: 42,
                    })));
            });

            if let Some(ref global_my_struct) = GLOBAL_MY_STRUCT {
                assert_eq!(global_my_struct.lock().unwrap().field, 42);
            } else {
                panic!("Failed to initialize the global my struct");
            }
        }
    }

    #[test]
    fn test_singleton_macro_get_mut() {
        #[derive(Default)]
        pub struct MyStruct2 {
            pub field: i32,
        }

        create_global_singleton!(MyStruct2, GLOBAL_MY_STRUCT2);

        let singleton = MyStruct2::get_mut_singleton().unwrap();
        let mut instance = singleton.lock().unwrap();
        instance.field = 42;
        assert_eq!(instance.field, 42);
    }
}

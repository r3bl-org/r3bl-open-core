// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Global allocator ([`mimalloc`]), safe-stack startup macro
//! ([`run_in_main_with_safe_stack!`]), and macro re-exports.
//!
//! # Global Allocator (`mimalloc`)
//!
//! [`mimalloc`] (by Microsoft) replaces the default system allocator, which is tuned for
//! single-threaded workloads, with an allocator optimized for multi-threaded workloads
//! where many small objects are created and destroyed. See [Performance notes][1] for
//! more details.
//!
//! # Why `pub use` for `mimalloc` and [`tokio`]?
//!
//! Declarative macros (`macro_rules!`) expand inside the caller's crate, so paths inside
//! them must resolve through `$crate::...`:
//! - `pub use mimalloc;` lets [`set_mimalloc_in_main!`] expand to
//!   `$crate::mimalloc::MiMalloc` without downstream crates declaring `mimalloc` in their
//!   own `Cargo.toml`.
//! - `pub use tokio;` lets the `async =>` arm of [`run_in_main_with_safe_stack!`] expand
//!   to `$crate::tokio::runtime::Builder` without downstream crates declaring [`tokio`]
//!   (or its `rt-multi-thread` / `macros` features) in their own `Cargo.toml`.
//!
//! [1]: https://github.com/microsoft/mimalloc?tab=readme-ov-file#performance
//! [`mimalloc`]: mimalloc
//! [`run_in_main_with_safe_stack!`]: crate::run_in_main_with_safe_stack
//! [`set_mimalloc_in_main!`]: crate::set_mimalloc_in_main
//! [`tokio`]: tokio

pub use mimalloc;
pub use tokio;

/// Ensures consistent stack size invariants and installs [`mimalloc`] as the global
/// allocator for the application process across all operating systems.
///
/// # Background & Cross-Platform Invariants
///
/// Operating systems differ in the default stack size allocated to the main process
/// thread:
/// - Linux & macOS: The operating system allocates an 8 MiB (`8192 KiB`) default stack
///   for the primary thread (governed by POSIX `ulimit -s`).
/// - Windows: The Portable Executable (PE) binary format defaults to only 1 MiB of stack
///   space.
///
/// Applications built with the `r3bl_tui` crate make heavy use of stack allocations for
/// speed (e.g., [`SmallVec`], [`SmallString`], flexbox layout calculation, and recursive
/// parser passes). Code that executes safely on Linux or macOS within the 8 MiB stack
/// limit will trigger a `STATUS_STACK_OVERFLOW` (`0xc00000fd`) crash on Windows when
/// restricted to 1 MiB.
///
/// To ensure the code we write maintains the same invariants across all operating
/// systems:
/// - Installs [`mimalloc`] as the `#[global_allocator]` via [`set_mimalloc_in_main!`] (so
///   binaries do not need to call it separately; invoke this macro at most once per
///   binary crate).
/// - On Windows (`#[cfg(target_os = "windows")]`), spawns the main logic inside a
///   dedicated thread configured with [`SAFE_STACK_SIZE_BYTES`] (`8 MiB`) and joins it.
///   In `async =>` mode, also configures Tokio worker threads with
///   [`SAFE_STACK_SIZE_BYTES`].
/// - On non-Windows platforms (`#[cfg(not(target_os = "windows"))]`), executes the main
///   logic directly on the main thread with zero overhead.
///
/// # Panics
///
/// This macro calls `.expect(...)` on Windows thread spawning/joining and (in `async =>`
/// mode) on Tokio runtime initialization, which will panic if:
/// - Thread spawning fails (e.g., insufficient system resources)
/// - The spawned thread panics
/// - Tokio runtime initialization fails
///
/// These are considered fatal startup errors; `clippy::unwrap_in_result` is suppressed
/// inside the macro expansion so callers returning `Result` need no boilerplate lint
/// attributes.
///
/// # Usage
///
/// **Synchronous entry point (`sync =>`):**
///
/// ```no_run
/// use r3bl_tui::{ok, run_in_main_with_safe_stack};
///
/// fn main() -> miette::Result<()> {
///     run_in_main_with_safe_stack!(sync => try_main_impl())
/// }
///
/// fn try_main_impl() -> miette::Result<()> {
///     // Your actual synchronous main logic here.
///     ok!()
/// }
/// ```
///
/// **Asynchronous Tokio entry point (`async =>`):**
///
/// ```no_run
/// use r3bl_tui::{ok, run_in_main_with_safe_stack};
///
/// fn main() -> miette::Result<()> {
///     run_in_main_with_safe_stack!(async => try_main_impl())
/// }
///
/// async fn try_main_impl() -> miette::Result<()> {
///     // Your actual async main logic here (no `#[tokio::main]` needed).
///     ok!()
/// }
/// ```
///
/// [`SAFE_STACK_SIZE_BYTES`]: crate::SAFE_STACK_SIZE_BYTES
/// [`set_mimalloc_in_main!`]: crate::set_mimalloc_in_main
/// [`SmallString`]: smallstr::SmallString
/// [`SmallVec`]: smallvec::SmallVec
#[macro_export]
macro_rules! run_in_main_with_safe_stack {
    (sync => $main_fn:expr) => {{
        $crate::set_mimalloc_in_main!();

        #[allow(clippy::unwrap_in_result)]
        {
            // On Windows, run the main logic in a thread with an 8 MiB stack.
            #[cfg(target_os = "windows")]
            {
                let handle = std::thread::Builder::new()
                    .stack_size($crate::SAFE_STACK_SIZE_BYTES)
                    .spawn(|| $main_fn)
                    .expect("Failed to spawn main thread with safe stack size");

                handle.join().expect("Main thread panicked")
            }

            #[cfg(not(target_os = "windows"))]
            $main_fn
        }
    }};

    (async => $main_fut:expr) => {{
        $crate::run_in_main_with_safe_stack!(sync => {
            #[allow(clippy::unwrap_in_result)]
            {
                let mut builder = $crate::tokio::runtime::Builder::new_multi_thread();
                builder.enable_all();

                #[cfg(target_os = "windows")]
                builder.thread_stack_size($crate::SAFE_STACK_SIZE_BYTES);

                builder
                    .build()
                    .expect("Failed building the Tokio runtime")
                    .block_on($main_fut)
            }
        })
    }};
}

/// Installs [`mimalloc::MiMalloc`] as the binary's `#[global_allocator]`.
///
/// Prefer [`run_in_main_with_safe_stack!`] in `fn main()`, which invokes this macro
/// automatically alongside Windows 8 MiB stack setup. Invoke directly only when you want
/// [`mimalloc`] without the safe-stack thread wrapper, and at most once per binary crate
/// (Rust allows only one `#[global_allocator]` per binary).
///
/// [`mimalloc::MiMalloc`]: mimalloc::MiMalloc
/// [`run_in_main_with_safe_stack!`]: crate::run_in_main_with_safe_stack
#[macro_export]
macro_rules! set_mimalloc_in_main {
    () => {{
        use $crate::mimalloc::MiMalloc;

        #[global_allocator]
        static GLOBAL: MiMalloc = MiMalloc;
    }};
}

/// Default safe stack size (`8 MiB`, `8_388_608` bytes) matching the Linux and macOS
/// primary thread stack limit (`ulimit -s`).
///
/// Used by [`run_in_main_with_safe_stack!`] on Windows for both the spawned main thread
/// and Tokio worker threads so stack-heavy code never hits Windows' 1 MiB PE default.
///
/// [`run_in_main_with_safe_stack!`]: crate::run_in_main_with_safe_stack
pub const SAFE_STACK_SIZE_BYTES: usize = 8 * 1024 * 1024;

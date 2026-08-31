// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// [`mimalloc`] is a replacement for the default global allocator. It's optimized for
/// multi-threaded use cases where lots of small objects are created and destroyed.
/// The default allocator is the system allocator that's optimized for single threaded
/// use cases.
///
/// [`mimalloc`] (by Microsoft):
/// - <https://github.com/microsoft/mimalloc?tab=readme-ov-file#performance>
///
/// [`mimalloc`]: mimalloc
#[macro_export]
macro_rules! set_mimalloc_in_main {
    () => {{
        use mimalloc::MiMalloc;

        #[global_allocator]
        static GLOBAL: MiMalloc = MiMalloc;
    }};
}

/// Ensures consistent stack size invariants for the application process across all
/// operating systems.
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
/// - On Windows (`#[cfg(target_os = "windows")]`), this macro spawns the main logic
///   inside a dedicated thread configured with an 8 MiB (`8 * 1024 * 1024` bytes) stack
///   and joins it.
/// - On non-Windows platforms (`#[cfg(not(target_os = "windows"))]`), it executes the
///   main logic directly on the main thread with zero overhead.
///
/// # Panics
///
/// This macro calls `.expect("conversion error")` on thread creation and join operations,
/// which will panic if:
/// - Thread spawning fails (e.g., insufficient system resources)
/// - The spawned thread panics
///
/// These are considered fatal errors for application startup, similar to how the
/// `#[tokio::main]` macro handles runtime initialization failures. If you need to use
/// this macro in a function that returns `Result`, suppress the
/// `clippy::unwrap_in_result` lint on that function.
///
/// # Usage
///
/// ```no_run
/// use r3bl_tui::{CommonResult, ok, run_with_safe_stack};
///
/// fn main() -> CommonResult {
///     run_with_safe_stack!(main_impl())
/// }
///
/// // Note: tokio::main also uses .expect("conversion error") internally,
/// // so the lint suppression is needed regardless of this macro's implementation.
/// #[tokio::main]
/// #[allow(clippy::unwrap_in_result)]
/// async fn main_impl() -> CommonResult {
///     // Your actual main logic here.
///     ok!()
/// }
/// ```
///
/// [`SmallString`]: smallstr::SmallString
/// [`SmallVec`]: smallvec::SmallVec
#[macro_export]
macro_rules! run_with_safe_stack {
    ($main_fn:expr) => {{
        // On Windows, run the main logic in a thread with larger stack.
        #[cfg(target_os = "windows")]
        {
            let handle = std::thread::Builder::new()
                .stack_size(8 * 1024 * 1024) // 8 MiB stack.
                .spawn(|| $main_fn)
                .expect("conversion error");

            handle.join().expect("conversion error")
        }

        #[cfg(not(target_os = "windows"))]
        $main_fn
    }};
}

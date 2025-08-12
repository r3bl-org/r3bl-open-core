// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// `mimalloc` is a replacement for the default global allocator. It's optimized for
/// multi-threaded use cases where lots of small objects are created and destroyed.
/// The default allocator is the system allocator that's optimized for single threaded
/// use cases.
///
/// mimalloc (by Microsoft):
/// - <https://github.com/microsoft/mimalloc?tab=readme-ov-file#performance>
///
/// jemalloc (originally by Facebook, now archived):
/// - <https://www.svix.com/blog/heap-fragmentation-in-rust-applications/>
/// - <https://news.ycombinator.com/item?id=35473271>
/// - <https://crates.io/crates/jemallocator>
#[macro_export]
macro_rules! set_mimalloc_in_main {
    () => {{
        use mimalloc::MiMalloc;

        #[global_allocator]
        static GLOBAL: MiMalloc = MiMalloc;
    }};
}

/// On Windows, the default stack size is 1MB which can cause stack overflow errors
/// in TUI applications that use large stack allocations (e.g., SmallVec/SmallString).
/// This macro wraps the main function to run it in a thread with an 8MB stack on Windows.
///
/// # Usage
///
/// ```no_run
/// use r3bl_tui::{CommonResult, run_with_safe_stack};
///
/// fn main() -> CommonResult<()> {
///     run_with_safe_stack!(main_impl())
/// }
///
/// #[tokio::main]
/// async fn main_impl() -> CommonResult<()> {
///     // Your actual main logic here
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! run_with_safe_stack {
    ($main_fn:expr) => {{
        // On Windows, run the main logic in a thread with larger stack.
        #[cfg(target_os = "windows")]
        {
            let handle = std::thread::Builder::new()
                .stack_size(8 * 1024 * 1024) // 8MB stack
                .spawn(|| $main_fn)
                .unwrap();

            handle.join().unwrap()
        }

        #[cfg(not(target_os = "windows"))]
        $main_fn
    }};
}

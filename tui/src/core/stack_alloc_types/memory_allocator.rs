/*
 *   Copyright (c) 2025 R3BL LLC
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

/// `jemalloc` is a replacement for the default global allocator. It's optimized for
/// multi-threaded use cases where lots of small objects are created and destroyed.
/// The default allocator is the system allocator that's optimized for single threaded
/// use cases.
/// - <https://www.svix.com/blog/heap-fragmentation-in-rust-applications/>
/// - <https://news.ycombinator.com/item?id=35473271>
/// - <https://crates.io/crates/jemallocator>
/// - <https://engineering.fb.com/2011/01/03/core-infra/scalable-memory-allocation-using-jemalloc/>
#[macro_export]
macro_rules! set_jemalloc_in_main {
    () => {{
        #[cfg(not(target_env = "msvc"))]
        use tikv_jemallocator::Jemalloc;

        #[cfg(not(target_env = "msvc"))]
        #[global_allocator]
        static GLOBAL: Jemalloc = Jemalloc;
    }};
}

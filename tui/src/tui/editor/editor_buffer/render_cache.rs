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

//! This module contains the implementation of the render cache for the editor buffer.
//! Currently the cache can only hold 1 entry at a time. The cache is invalidated if the
//! content of the editor buffer changes, or if the scroll offset or window size changes.
//!
//! - The key is derived from the scroll offset and window size.
//! - The value is a [RenderOps] struct that contains the render operations to render the
//!   content of the editor buffer to the screen.
//!
//! In the future, if there is a need to store multiple entries in the cache, the cache
//! can be implemented as a [r3bl_core::RingBuffer] or [r3bl_core::InlineVec] of
//! [CacheEntry] structs.

use std::ops::{Deref, DerefMut};

use r3bl_core::{Dim, ScrOfs, Size};

use super::EditorBuffer;
use crate::{engine_public_api, EditorEngine, HasFocus, RenderArgs, RenderOps};

pub(in crate::tui::editor::editor_buffer) mod key {
    use super::*;

    /// Cache key is combination of scroll_offset and window_size.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Key((ScrOfs, Dim));

    impl Key {
        pub fn new(scr_ofs: ScrOfs, window_size: Dim) -> Self {
            (scr_ofs, window_size).into()
        }
    }

    impl From<(ScrOfs, Dim)> for Key {
        fn from((scr_ofs, window_size): (ScrOfs, Dim)) -> Self {
            Self((scr_ofs, window_size))
        }
    }
}
pub use key::*; // Allow code below to all the symbols in this mod.

pub(in crate::tui::editor::editor_buffer) mod cache_entry {
    use super::{key::Key, *};

    /// Cache entry is a combination of a single key and single value.
    #[derive(Clone, Debug, PartialEq)]
    pub struct CacheEntry(pub Key, pub RenderOps);

    impl CacheEntry {
        pub fn new(arg_key: impl Into<Key>, value: RenderOps) -> Self {
            Self(arg_key.into(), value)
        }
    }
}
pub use cache_entry::*; // Allow code below to all the symbols in this mod.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UseRenderCache {
    Yes,
    No,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct RenderCache {
    pub entry: Option<cache_entry::CacheEntry>,
}

mod render_cache_impl_block {
    use super::*;

    impl Deref for RenderCache {
        type Target = Option<cache_entry::CacheEntry>;

        fn deref(&self) -> &Self::Target { &self.entry }
    }

    impl DerefMut for RenderCache {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.entry }
    }
    impl RenderCache {
        pub fn clear(&mut self) { self.entry = None; }

        pub fn get(&self, arg_key: impl Into<Key>) -> Option<&RenderOps> {
            let key: Key = arg_key.into();
            if key == self.entry.as_ref()?.0 {
                Some(&self.entry.as_ref()?.1)
            } else {
                None
            }
        }

        pub fn insert(&mut self, arg_key: impl Into<Key>, value: RenderOps) {
            let key: Key = arg_key.into();
            self.entry = Some(CacheEntry::new(key, value));
        }

        /// Render the content of the editor buffer to the screen from the cache if the content
        /// has not been modified.
        ///
        /// The cache miss occurs if
        /// - Scroll Offset changes
        /// - Window size changes
        /// - Content of the editor changes
        pub fn render_content(
            buffer: &mut EditorBuffer,
            engine: &mut EditorEngine,
            window_size: Size,
            has_focus: &mut HasFocus,
            render_ops: &mut RenderOps,
            use_cache: UseRenderCache,
        ) {
            match use_cache {
                UseRenderCache::Yes => {
                    if let Some(cached_output) =
                        buffer.render_cache.get((buffer.get_scr_ofs(), window_size))
                    {
                        // Cache hit
                        *render_ops = cached_output.clone();
                        return;
                    }

                    // Cache miss, due to either:
                    // - Content has been modified.
                    // - Scroll Offset or Window size has been modified.
                    buffer.render_cache.clear();
                    let render_args = RenderArgs {
                        engine,
                        buffer,
                        has_focus,
                    };

                    // Re-render content, generate & write to render_ops.
                    engine_public_api::render_content(render_args, render_ops);

                    // Snapshot the render_ops in the cache.
                    buffer
                        .render_cache
                        .insert((buffer.get_scr_ofs(), window_size), render_ops.clone());
                }
                UseRenderCache::No => {
                    buffer.render_cache.clear();
                    let render_args = RenderArgs {
                        engine,
                        buffer,
                        has_focus,
                    };
                    // Re-render content, generate & write to render_ops.
                    engine_public_api::render_content(render_args, render_ops);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use r3bl_core::{assert_eq2, col, height, row, scr_ofs, width};

    use super::*;
    use crate::{render_ops, RenderOp};

    /// Fake render_ops to be used in the tests.
    fn get_render_ops_og() -> RenderOps {
        render_ops!(
            @new
            RenderOp::ClearScreen, RenderOp::ResetColor
        )
    }

    /// Fake window size to be used in the tests.
    fn get_window_size_og() -> Dim { height(70) + width(15) }

    #[test]
    fn test_cache_can_be_disabled() {
        let buffer = &mut EditorBuffer::default();
        let engine = &mut EditorEngine::default();
        let has_focus = &mut HasFocus::default();

        // Cache should be empty.
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            None
        );

        // The very first request to cache is always missed since cache is empty.
        let render_ops_mut = &mut get_render_ops_og();
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );

        // Cache should have been populated with the render_ops_og.
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            Some(&get_render_ops_og())
        );

        // Disable cache and re-render content.
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::No,
        );

        // Cache should have been cleared.
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            None
        );
    }

    #[test]
    fn test_assert_cache_hit_for_multiple_renders() {
        let buffer = &mut EditorBuffer::default();
        let engine = &mut EditorEngine::default();
        let has_focus = &mut HasFocus::default();

        // Cache should be empty.
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            None
        );

        // The very first request to cache is always missed since cache is empty.
        let render_ops_mut = &mut get_render_ops_og();
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );

        // Cache should have been populated with the render_ops_og.
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            Some(&get_render_ops_og())
        );

        // Subsequent requests to cache should be hits.
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            Some(&get_render_ops_og())
        );

        // Modify the `render_ops_mut` manually (eg: when the caret is added using
        // `render_caret`). This should not change the content and result in a cache
        // hit.
        render_ops_mut.clear();
        assert!(render_ops_mut.is_empty());
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );
        // `render_ops_mut` should have been restored to `render_ops_og` by
        // render_content(.., UseRenderCache::Yes).
        assert!(!render_ops_mut.is_empty());
        assert_eq2!(render_ops_mut, &get_render_ops_og());
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            Some(&get_render_ops_og())
        );
    }

    #[test]
    fn test_assert_cache_miss_for_first_render() {
        let buffer = &mut EditorBuffer::default();
        let engine = &mut EditorEngine::default();
        let has_focus = &mut HasFocus::default();

        // Cache should be empty.
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            None
        );

        // The very first request to cache is always missed since cache is empty.
        let render_ops_mut = &mut get_render_ops_og();
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );

        // Cache should have been populated with the render_ops_og.
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            Some(&get_render_ops_og())
        );

        // Modify the `render_ops_mut` manually (eg: when the caret is added using
        // `render_caret`). This should not change the content and result in a cache
        // hit.
        render_ops_mut.clear();
        assert!(render_ops_mut.is_empty());
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );
        // `render_ops_mut` should have been restored to `render_ops_og` by
        // render_content(.., UseRenderCache::Yes).
        assert!(!render_ops_mut.is_empty());
        assert_eq2!(render_ops_mut, &get_render_ops_og());
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            Some(&get_render_ops_og())
        );
    }

    #[test]
    fn test_window_size_change_causes_cache_miss() {
        let buffer = &mut EditorBuffer::default();
        let engine = &mut EditorEngine::default();
        let has_focus = &mut HasFocus::default();

        // The very first request to cache is always missed since cache is empty.
        let render_ops_mut = &mut get_render_ops_og();
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );

        // Change in window size should invalidate the cache and result in a cache miss.
        let window_size_new = height(50) + width(15);
        assert!(window_size_new != get_window_size_og());
        RenderCache::render_content(
            buffer,
            engine,
            window_size_new,
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og())),
            None
        );
        assert_eq2!(
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), window_size_new)),
            Some(&get_render_ops_og())
        );
    }

    #[test]
    fn test_scroll_offset_change_causes_cache_miss() {
        let buffer = &mut EditorBuffer::default();
        let engine = &mut EditorEngine::default();
        let has_focus = &mut HasFocus::default();

        // The very first request to cache is always missed since cache is empty.
        let render_ops_mut = &mut get_render_ops_og();
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );

        // Change in scroll_offset should invalidate the cache and result in a cache miss.
        let scr_ofs_old = buffer.get_scr_ofs();
        let scr_ofs_new = scr_ofs(col(1) + row(1));
        assert!(scr_ofs_new != scr_ofs_old);

        buffer.content.scr_ofs = scr_ofs_new;
        RenderCache::render_content(
            buffer,
            engine,
            get_window_size_og(),
            has_focus,
            render_ops_mut,
            UseRenderCache::Yes,
        );
        assert_eq2!(
            buffer.render_cache.get((scr_ofs_old, get_window_size_og())),
            None
        );
        assert_eq2!(
            buffer.render_cache.get((scr_ofs_new, get_window_size_og())),
            Some(&get_render_ops_og())
        );
    }

    #[test]
    fn test_content_change_invalidates_cache() {
        let buffer = &mut EditorBuffer::default();
        let engine = &mut EditorEngine::default();
        let has_focus = &mut HasFocus::default();

        // Change in content should invalidate the cache.
        let snapshot_1 = {
            buffer.set_lines(["r3bl"]);
            RenderCache::render_content(
                buffer,
                engine,
                get_window_size_og(),
                has_focus,
                &mut get_render_ops_og(),
                UseRenderCache::Yes,
            );
            assert!(buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og()))
                .is_some());
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og()))
                .unwrap()
                .clone()
        };

        // Change in content should invalidate the cache.
        let snapshot_2 = {
            buffer.set_lines(["r3bl", "r3bl"]);
            RenderCache::render_content(
                buffer,
                engine,
                get_window_size_og(),
                has_focus,
                &mut get_render_ops_og(),
                UseRenderCache::Yes,
            );
            assert!(buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og()))
                .is_some());
            buffer
                .render_cache
                .get((buffer.get_scr_ofs(), get_window_size_og()))
                .unwrap()
                .clone()
        };

        assert!(snapshot_1 != snapshot_2);
    }
}

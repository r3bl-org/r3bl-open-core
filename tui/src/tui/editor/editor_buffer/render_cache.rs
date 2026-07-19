// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module contains the implementation of the render cache for the editor buffer.
//! Currently the cache can only hold 1 entry at a time. The cache is invalidated if the
//! content of the editor buffer changes, or if the viewport origin or window size
//! changes.
//!
//! - The key is derived from the viewport origin and window size.
//! - The value is a [`RenderOpIRVec`] struct that contains the render operations to
//!   render the content of the editor buffer to the screen.
//!
//! In the future, if there is a need to store multiple entries in the cache, the cache
//! can be implemented as a [`RingBuffer`] or [`InlineVec`] of [`CacheEntry`].
//!
//! [`InlineVec`]: crate::InlineVec
//! [`RingBuffer`]: crate::RingBuffer

use crate::{EditorBuffer, EditorEngine, HasFocus, RenderArgs, RenderOpIRVec, VPSize,
            Viewport, engine_public_api};

/// Holds a single cache entry that represents the render operations to render the content
/// of the editor buffer to the current viewport and (terminal) screen size.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct RenderCache {
    pub entry: Option<CacheEntry>,
}

impl RenderCache {
    /// Clears the single cache entry.
    pub fn clear(&mut self) { self.entry = None; }

    /// Returns the cached [`RenderOpIRVec`] if the entry matches the given [`Viewport`].
    #[must_use]
    pub fn get(&self, viewport: Viewport) -> Option<&RenderOpIRVec> {
        let (key, value) = self.entry.as_ref()?;

        if *key == viewport {
            return Some(value);
        }

        None
    }

    /// This cache only holds a single entry. So if there is an existing entry, it is
    /// replaced with the new entry.
    pub fn insert(&mut self, viewport: Viewport, value: RenderOpIRVec) {
        self.entry = Some((viewport, value));
    }

    /// Render the content of the editor buffer to the screen from the cache if the
    /// content has not been modified.
    ///
    /// The cache miss occurs if
    /// - Viewport origin changes
    /// - Window size changes
    /// - Content of the editor changes
    pub fn render_content(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        window_size: VPSize,
        has_focus: &mut HasFocus,
        render_ops: &mut RenderOpIRVec,
        use_cache: UseRenderCache,
    ) {
        use UseRenderCache::{No, Yes};

        let viewport = Viewport::new(buffer.get_vp_origin(), window_size);

        // Cache enabled & hit so early return.
        let cache_entry = buffer.get_render_cache().get(viewport);
        match (use_cache, cache_entry) {
            (Yes, Some(value)) => {
                *render_ops = value.clone();
                return;
            }
            _ => { /* Cache disabled, or cache miss. */ }
        }

        // Cached disabled, or miss due to:
        // - Content has been modified.
        // - Viewport origin or Window size has been modified.
        // So re-render content, generate & write to render_ops.
        engine_public_api::render_content(
            RenderArgs::new(engine, buffer, has_focus),
            render_ops,
        );

        match use_cache {
            // Cache is enabled, so update it.
            Yes => buffer
                .get_render_cache_mut()
                .insert(viewport, render_ops.clone()),
            // Cache is disabled, so invalidate it (it should contain nothing at this
            // point).
            No => buffer.get_render_cache_mut().clear(),
        }
    }
}

pub type CacheEntry = (Viewport, RenderOpIRVec);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UseRenderCache {
    Yes,
    No,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RenderOpCommon, assert_eq2, c_pos,
                editor::test_fixtures_editor::mock_real_objects_for_editor, vp_height,
                vp_width};

    /// Fake `render_ops` to be used in the tests.
    fn get_render_ops_og() -> RenderOpIRVec {
        let mut ops = RenderOpIRVec::new();
        ops.push(RenderOpCommon::ClearScreen);
        ops.push(RenderOpCommon::ResetColor);
        ops
    }

    /// Fake window size to be used in the tests.
    fn get_window_size_og() -> VPSize { vp_height(70) + vp_width(15) }

    #[test]
    fn test_cache_can_be_disabled() {
        let buffer = &mut EditorBuffer::default();
        let engine = &mut EditorEngine::default();
        let has_focus = &mut HasFocus::default();

        // Cache should be empty.
        assert_eq2!(
            buffer
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
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
        let window_size_new = vp_height(50) + vp_width(15);
        assert_ne!(window_size_new, get_window_size_og());
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
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og())),
            None
        );
        assert_eq2!(
            buffer
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), window_size_new)),
            Some(&get_render_ops_og())
        );
    }

    #[test]
    fn test_vp_origin_change_causes_cache_miss() {
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

        // Change in vp_origin should invalidate the cache and result in a cache miss.
        let vp_origin_old = buffer.get_vp_origin();
        let vp_origin_new = c_pos(1, 1);
        assert_ne!(vp_origin_new, vp_origin_old);

        {
            let buffer_mut = buffer.get_mut_no_drop(get_window_size_og());
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = vp_origin_new);
        }
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
                .get_render_cache()
                .get(Viewport::new(vp_origin_old, get_window_size_og())),
            None
        );
        assert_eq2!(
            buffer
                .get_render_cache()
                .get(Viewport::new(vp_origin_new, get_window_size_og())),
            Some(&get_render_ops_og())
        );
    }

    #[test]
    fn test_content_change_invalidates_cache() {
        let buffer = &mut EditorBuffer::default();
        let engine = &mut mock_real_objects_for_editor::make_editor_engine();
        let has_focus = &mut HasFocus::default();

        // Change in content should invalidate the cache.
        let snapshot_1 = {
            buffer.init_with(["r3bl"]);
            RenderCache::render_content(
                buffer,
                engine,
                get_window_size_og(),
                has_focus,
                &mut get_render_ops_og(),
                UseRenderCache::Yes,
            );
            assert!(
                buffer
                    .get_render_cache()
                    .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og()))
                    .is_some()
            );
            buffer
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og()))
                .expect("conversion error")
                .clone()
        };

        // Change in content should invalidate the cache.
        let snapshot_2 = {
            buffer.init_with(["r3bl", "r3bl"]);
            RenderCache::render_content(
                buffer,
                engine,
                get_window_size_og(),
                has_focus,
                &mut get_render_ops_og(),
                UseRenderCache::Yes,
            );
            assert!(
                buffer
                    .get_render_cache()
                    .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og()))
                    .is_some()
            );
            buffer
                .get_render_cache()
                .get(Viewport::new(buffer.get_vp_origin(), get_window_size_og()))
                .expect("conversion error")
                .clone()
        };

        assert_ne!(snapshot_1, snapshot_2);
    }
}

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

use std::{collections::HashMap,
          ops::{Deref, DerefMut}};

use r3bl_core::{string_storage, Size, StringStorage};

use super::EditorBuffer;
use crate::{engine_public_api, EditorEngine, HasFocus, RenderArgs, RenderOps};

pub type RenderCacheMap = HashMap<StringStorage, RenderOps>;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct RenderCache(pub RenderCacheMap);

pub enum UseRenderCache {
    Yes,
    No,
}

impl RenderCache {
    pub fn clear(&mut self) { self.0.clear(); }

    pub fn get(&self, key: &StringStorage) -> Option<&RenderOps> { self.0.get(key) }

    pub fn insert(&mut self, key: StringStorage, value: RenderOps) {
        self.0.insert(key, value);
    }

    /// Cache key is combination of scroll_offset and window_size.
    pub fn generate_key(buffer: &EditorBuffer, window_size: Size) -> StringStorage {
        string_storage!(
            "{offset:?}{size:?}",
            offset = buffer.get_scr_ofs(),
            size = window_size,
        )
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
                let key = RenderCache::generate_key(buffer, window_size);
                if let Some(cached_output) = buffer.render_cache.get(&key) {
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
                engine_public_api::render_content(&render_args, render_ops);

                // Snapshot the render_ops in the cache.
                buffer.render_cache.insert(key, render_ops.clone());
            }
            UseRenderCache::No => {
                buffer.render_cache.clear();
                let render_args = RenderArgs {
                    engine,
                    buffer,
                    has_focus,
                };
                // Re-render content, generate & write to render_ops.
                engine_public_api::render_content(&render_args, render_ops);
            }
        }
    }
}

impl From<RenderCacheMap> for RenderCache {
    fn from(map: RenderCacheMap) -> Self { Self(map) }
}

impl Deref for RenderCache {
    type Target = RenderCacheMap;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for RenderCache {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

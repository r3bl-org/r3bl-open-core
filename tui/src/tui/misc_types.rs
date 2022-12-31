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

use std::{fmt::Debug,
          ops::{Deref, DerefMut}};

use int_enum::IntEnum;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use strum_macros::AsRefStr;

use crate::*;

pub mod args {
    use super::*;

    pub struct RenderArgs<'a, S, A>
    where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        pub editor_engine: &'a mut EditorEngine,
        pub editor_buffer: &'a EditorBuffer,
        pub component_registry: &'a ComponentRegistry<S, A>,
    }

    pub struct EditorArgsMut<'a> {
        pub editor_engine: &'a mut EditorEngine,
        pub editor_buffer: &'a mut EditorBuffer,
    }

    pub struct EditorArgs<'a> {
        pub editor_engine: &'a EditorEngine,
        pub editor_buffer: &'a EditorBuffer,
    }

    /// Global scope args struct that holds references.
    ///
    /// ![Editor component lifecycle
    /// diagram](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/docs/memory-architecture.drawio.svg)
    pub struct GlobalScopeArgs<'a, S, A>
    where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        pub shared_global_data: &'a SharedGlobalData,
        pub shared_store: &'a SharedStore<S, A>,
        pub state: &'a S,
        pub window_size: &'a Size,
    }

    /// Component scope args struct that holds references.
    ///
    /// ![Editor component lifecycle
    /// diagram](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/docs/memory-architecture.drawio.svg)
    pub struct ComponentScopeArgs<'a, S, A>
    where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        pub shared_global_data: &'a SharedGlobalData,
        pub shared_store: &'a SharedStore<S, A>,
        pub state: &'a S,
        pub component_registry: &'a mut ComponentRegistry<S, A>,
        pub window_size: &'a Size,
    }

    /// [EditorEngine] args struct that holds references.
    ///
    /// ![Editor component lifecycle
    /// diagram](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/docs/memory-architecture.drawio.svg)
    pub struct EditorEngineArgs<'a, S, A>
    where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        pub shared_global_data: &'a SharedGlobalData,
        pub shared_store: &'a SharedStore<S, A>,
        pub state: &'a S,
        pub component_registry: &'a mut ComponentRegistry<S, A>,
        pub self_id: FlexBoxId,
        pub editor_buffer: &'a EditorBuffer,
        pub editor_engine: &'a mut EditorEngine,
    }

    /// [DialogEngine] args struct that holds references.
    ///
    /// ![Editor component lifecycle
    /// diagram](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/docs/memory-architecture.drawio.svg)
    pub struct DialogEngineArgs<'a, S, A>
    where
        S: Default + Clone + PartialEq + Debug + Sync + Send,
        A: Default + Clone + Sync + Send,
    {
        pub shared_global_data: &'a SharedGlobalData,
        pub shared_store: &'a SharedStore<S, A>,
        pub state: &'a S,
        pub component_registry: &'a mut ComponentRegistry<S, A>,
        pub self_id: FlexBoxId,
        pub dialog_buffer: &'a DialogBuffer,
        pub dialog_engine: &'a mut DialogEngine,
        pub window_size: &'a Size,
    }
}
pub use args::*;

pub mod aliases {
    use super::*;

    pub type ScrollOffset = Position;
}
pub use aliases::*;

pub mod pretty_print_option {
    use super::*;

    #[macro_export]
    macro_rules! format_option {
        ($opt:expr) => {
            match ($opt) {
                Some(v) => v,
                None => &FormatMsg::None,
            }
        };
    }

    #[derive(Clone, Copy, Debug)]
    pub enum FormatMsg {
        None,
    }
}
pub use pretty_print_option::*;

pub mod global_constants {
    use super::*;

    #[repr(u8)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq, IntEnum)]
    pub enum MinSize {
        Col = 65,
        Row = 11,
    }

    #[repr(usize)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq, IntEnum)]
    pub enum DefaultSize {
        GlobalDataCacheSize = 1000,
    }

    #[derive(Debug, Eq, PartialEq, AsRefStr)]
    pub enum BorderGlyphCharacter {
        #[strum(to_string = "╮")]
        TopRight,
        #[strum(to_string = "╭")]
        TopLeft,
        #[strum(to_string = "╯")]
        BottomRight,
        #[strum(to_string = "╰")]
        BottomLeft,
        #[strum(to_string = "─")]
        Horizontal,
        #[strum(to_string = "│")]
        Vertical,
    }

    pub const SPACER: &str = " ";
    pub const DEFAULT_CURSOR_CHAR: char = '▒';
    pub const DEFAULT_SYN_HI_FILE_EXT: &str = "md";
}
pub use global_constants::*;

pub mod list_of {
    use super::*;
    /// Redundant struct to [Vec]. Added so that [From] trait can be implemented for for [List] of `T`.
    /// Where `T` is any number of types in the tui crate.
    #[derive(Debug, Clone, Default)]
    pub struct List<T> {
        pub items: Vec<T>,
    }

    impl<T> Deref for List<T> {
        type Target = Vec<T>;
        fn deref(&self) -> &Self::Target { &self.items }
    }

    impl<T> DerefMut for List<T> {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.items }
    }
}
pub use list_of::*;

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
          ops::{AddAssign, Deref, DerefMut}};

use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use strum_macros::AsRefStr;

use crate::*;

pub mod args {
    use super::*;

    pub struct RenderArgs<'a, S, A>
    where
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
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
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
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
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
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
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
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
        S: Debug + Default + Clone + PartialEq + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
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

pub mod misc_type_aliases {
    use super::*;

    pub type ScrollOffset = Position;
    pub type US = UnicodeString;
}
pub use misc_type_aliases::*;

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
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum MinSize {
        Col = 65,
        Row = 11,
    }

    #[repr(usize)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum DefaultSize {
        GlobalDataCacheSize = 1_000_000,
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

        #[strum(to_string = "┤")]
        LineUpDownLeft,

        #[strum(to_string = "├")]
        LineUpDownRight,
    }

    pub const SPACER: &str = " ";
    pub const DEFAULT_CURSOR_CHAR: char = '▒';
    pub const DEFAULT_SYN_HI_FILE_EXT: &str = "md";
}
pub use global_constants::*;

pub mod list_of {
    use super::*;

    #[macro_export]
    macro_rules! list {
        (
            $($item: expr),*
            $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
        ) => {
            {
                #[allow(unused_mut)]
                let mut it = List::new();
                $(
                    it.items.push($item);
                )*
                it
            }
        };
    }

    /// Redundant struct to [Vec]. Added so that [From] trait can be implemented for for [List] of
    /// `T`. Where `T` is any number of types in the tui crate.
    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct List<T> {
        pub items: Vec<T>,
    }

    impl<T> List<T> {
        pub fn with_capacity(size: usize) -> Self {
            Self {
                items: Vec::with_capacity(size),
            }
        }

        pub fn new() -> Self { Self { items: Vec::new() } }
    }

    /// Add (other) item to list (self).
    impl<T> AddAssign<T> for List<T> {
        fn add_assign(&mut self, other_item: T) { self.push(other_item); }
    }

    /// Add (other) list to list (self).
    impl<T> AddAssign<List<T>> for List<T> {
        fn add_assign(&mut self, other_list: List<T>) { self.extend(other_list.items); }
    }

    /// Add (other) vec to list (self).
    impl<T> AddAssign<Vec<T>> for List<T> {
        fn add_assign(&mut self, other_vec: Vec<T>) { self.extend(other_vec); }
    }

    impl<T> From<List<T>> for Vec<T> {
        fn from(list: List<T>) -> Self { list.items }
    }

    impl<T> From<Vec<T>> for List<T> {
        fn from(items: Vec<T>) -> Self { Self { items } }
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

mod cli_args {
    use super::*;

    /// Helper trait and impl to convert [std::env::Args] to a [`Vec<String>`] after removing the first
    /// item (which is the path to the executable).
    pub trait ArgsToStrings {
        fn filter_and_convert_to_strings(&self) -> Vec<String>;
        fn as_str(my_vec: &[String]) -> Vec<&str>;
    }

    impl ArgsToStrings for std::env::Args {
        fn filter_and_convert_to_strings(&self) -> Vec<String> {
            let mut list = std::env::args().collect::<Vec<String>>();
            if !list.is_empty() {
                list.remove(0);
            }
            list
        }

        fn as_str(my_vec: &[String]) -> Vec<&str> { List::from(my_vec).items }
    }

    impl<'a> From<&'a [String]> for List<&'a str> {
        /// The [From] trait is implemented for [List] instead of [Vec].
        /// 1. [List] is defined in this crate.
        /// 2. [Vec] is not.
        ///
        /// The [`Vec<String>`] reference is converted to a [List<&str>]. Which can then be converted into a
        /// [Vec<&str>] if needed.
        ///
        /// More info on converting [`Vec<String>`] to [Vec<&str>]:
        /// <https://users.rust-lang.org/t/is-this-the-best-way-to-go-from-vec-string-to-vec-str/37838>
        fn from(my_vec: &'a [String]) -> Self {
            let items = my_vec.iter().map(String::as_str).collect::<Vec<&str>>();
            List { items }
        }
    }
}
pub use cli_args::*;

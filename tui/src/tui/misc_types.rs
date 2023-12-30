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

pub use args::*;
pub use cli_args::*;
pub use dialog_component_traits::*;
pub use editor_component_traits::*;
pub use global_constants::*;
pub use list_of::*;
pub use misc_type_aliases::*;
pub use pretty_print_option::*;
pub use pretty_print_traits::*;
use r3bl_rs_utils_core::*;
use strum_macros::AsRefStr;

use crate::*;

pub mod args {
    use super::*;

    pub struct RenderArgs<'a> {
        pub editor_engine: &'a mut EditorEngine,
        pub editor_buffer: &'a EditorBuffer,
        pub has_focus: &'a mut HasFocus,
    }

    pub struct EditorArgsMut<'a> {
        pub editor_engine: &'a mut EditorEngine,
        pub editor_buffer: &'a mut EditorBuffer,
    }

    pub struct EditorArgs<'a> {
        pub editor_engine: &'a EditorEngine,
        pub editor_buffer: &'a EditorBuffer,
    }

    /// [DialogEngine] args struct that holds references.
    ///
    /// ![Editor component lifecycle
    /// diagram](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/docs/memory-architecture.drawio.svg)
    pub struct DialogEngineArgs<'a, S, AS>
    where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        pub self_id: FlexBoxId,
        pub global_data: &'a mut GlobalData<S, AS>,
        pub dialog_engine: &'a mut DialogEngine,
        pub has_focus: &'a mut HasFocus,
    }
}

pub mod misc_type_aliases {
    use super::*;

    pub type ScrollOffset = Position;
    pub type US = UnicodeString;
}

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

pub mod list_of {
    use get_size::GetSize;
    use serde::{Deserialize, Serialize};

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
    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, GetSize)]
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
        fn from(other: Vec<T>) -> Self { Self { items: other } }
    }

    impl<T> Deref for List<T> {
        type Target = Vec<T>;
        fn deref(&self) -> &Self::Target { &self.items }
    }

    impl<T> DerefMut for List<T> {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.items }
    }
}

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

mod pretty_print_traits {
    use crossterm::style::Stylize;

    use crate::US;

    /// Marker trait to "remember" which types can be printed to the console w/ color.
    pub trait ConsoleLogInColor {
        fn console_log_fg(&self);
        fn console_log_bg(&self);
    }

    fn console_log_fg(this: &str) {
        if this.is_empty() {
            println!("\n{}", "← empty →".yellow());
        } else {
            println!("\n{}", this.yellow());
        }
    }

    fn console_log_bg(this: &str) {
        if this.is_empty() {
            println!("\n{}", "← empty →".red().on_white());
        } else {
            println!("\n{}", this.red().on_white());
        }
    }

    impl<T: PrettyPrintDebug> ConsoleLogInColor for T {
        fn console_log_fg(&self) { console_log_fg(&self.pretty_print_debug()); }

        fn console_log_bg(&self) { console_log_bg(&self.pretty_print_debug()); }
    }

    impl ConsoleLogInColor for &str {
        fn console_log_fg(&self) { console_log_fg(self); }

        fn console_log_bg(&self) { console_log_bg(self); }
    }

    impl ConsoleLogInColor for String {
        fn console_log_fg(&self) { console_log_fg(self); }

        fn console_log_bg(&self) { console_log_bg(self); }
    }

    /// Marker trait to "remember" which types support pretty printing for debugging.
    pub trait PrettyPrintDebug {
        fn pretty_print_debug(&self) -> String;
    }

    /// Marker trait to "remember" which types can be converted to plain text.
    pub trait ConvertToPlainText {
        fn to_plain_text_us(&self) -> US;
    }
}

mod editor_component_traits {
    use super::*;

    /// This marker trait is meant to be implemented by whatever state struct is being used to store
    /// the editor buffer for this re-usable editor component. It is used in the `where` clause of
    /// the [EditorComponent] to ensure that the generic type `S` implements this trait,
    /// guaranteeing that it holds a hash map of [EditorBuffer]s w/ key of [FlexBoxId].
    pub trait HasEditorBuffers {
        fn get_mut_editor_buffer(&mut self, id: FlexBoxId) -> Option<&mut EditorBuffer>;
        fn insert_editor_buffer(&mut self, id: FlexBoxId, buffer: EditorBuffer);
        fn contains_editor_buffer(&self, id: FlexBoxId) -> bool;
    }
}

pub mod dialog_component_traits {
    use tokio::sync::mpsc::Sender;

    use super::*;

    /// This marker trait is meant to be implemented by whatever state struct is being used to store the
    /// dialog buffer for this re-usable editor component. It is used in the `where` clause of the
    /// [DialogComponent] to ensure that the generic type `S` implements this trait, guaranteeing that
    /// it holds a single [DialogBuffer].
    pub trait HasDialogBuffers {
        fn get_mut_dialog_buffer(&mut self, id: FlexBoxId) -> Option<&mut DialogBuffer>;
    }

    #[derive(Debug)]
    pub enum DialogChoice {
        Yes(String),
        No,
    }

    pub type OnDialogPressFn<S, AS> = fn(
        DialogChoice,
        &mut S,
        main_thread_channel_sender: &mut Sender<TerminalWindowMainThreadSignal<AS>>,
    );

    pub type OnDialogEditorChangeFn<S, AS> = fn(
        &mut S,
        main_thread_channel_sender: &mut Sender<TerminalWindowMainThreadSignal<AS>>,
    );
}

const PET_NAMES: [&str; 5] = ["Buddy", "Max", "Bella", "Charlie", "Lucy"];
const FRUIT_NAMES: [&str; 5] = ["Apple", "Banana", "Orange", "Pear", "Peach"];

pub fn generate_random_friendly_id() -> String {
    use rand::Rng;

    // Generate friendly pet and fruit name combination.
    let pet = {
        let mut rng = rand::thread_rng();
        let pet = PET_NAMES[rng.gen_range(0..PET_NAMES.len())];
        pet.to_lowercase()
    };

    let fruit = {
        let mut rng = rand::thread_rng();
        let fruit = FRUIT_NAMES[rng.gen_range(0..FRUIT_NAMES.len())];
        fruit.to_lowercase()
    };

    let random_number = {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..1000)
    };

    format!("{pet}-{fruit}-{random_number}")
}

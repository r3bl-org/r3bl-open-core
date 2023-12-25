/*
 *   Copyright (c) 2023 R3BL LLC
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

use r3bl_tui::*;

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Id {
    Editor = 1,
    SimpleDialog = 2,
    AutocompleteDialog = 3,
    EditorStyleNameDefault = 4,
    DialogStyleNameBorder = 5,
    DialogStyleNameTitle = 6,
    DialogStyleNameEditor = 7,
    DialogStyleNameResultsPanel = 8,
}

mod id_impl {
    use super::*;

    impl From<Id> for u8 {
        fn from(id: Id) -> u8 { id as u8 }
    }

    impl From<Id> for FlexBoxId {
        fn from(id: Id) -> FlexBoxId { FlexBoxId(id as u8) }
    }
}

// 00: app_main - copy from ex_editor

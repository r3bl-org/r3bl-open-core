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

// Attach.
pub mod caret_locate;
pub mod editor_buffer_clipboard_support;
pub mod editor_buffer_selection_support;
pub mod editor_buffer_struct;
pub mod selection_list;
pub mod system_clipboard_service_provider;

// Re-export.
pub use caret_locate::*;
pub use editor_buffer_clipboard_support::*;
pub use editor_buffer_selection_support::*;
pub use editor_buffer_struct::*;
pub use selection_list::*;
pub use system_clipboard_service_provider::*;

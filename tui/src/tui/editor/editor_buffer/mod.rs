/*
 *   Copyright (c) 2022-2025 R3BL LLC
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
pub mod buffer_struct;
pub mod caret_locate;
pub mod clipboard_support;
pub mod cur_index; // Not re-exported.
pub mod history; // Not re-exported.
pub mod render_cache; // Not re-exported.
pub mod selection_list;
pub mod selection_range;
pub mod selection_support;
pub mod sizing; // Not re-exported.
pub mod system_clipboard_service_provider;

// Re-export.
pub use buffer_struct::*;
pub use caret_locate::*;
pub use clipboard_support::*;
pub use selection_list::*;
pub use selection_range::*;
pub use selection_support::*;
pub use system_clipboard_service_provider::*;

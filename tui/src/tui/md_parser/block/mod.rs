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

// Attach.
pub mod parse_block_code;
pub mod parse_block_heading;
pub mod parse_block_markdown_text_until_eol;
pub mod parse_block_ordered_list;
pub mod parse_block_unordered_list;

// Re-export.
pub use parse_block_code::*;
pub use parse_block_heading::*;
pub use parse_block_markdown_text_until_eol::*;
pub use parse_block_ordered_list::*;
pub use parse_block_unordered_list::*;

// Tests.
pub mod test_data;

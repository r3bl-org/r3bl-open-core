/*
 *   Copyright (c) 2024 R3BL LLC
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

// Attach sources.
pub mod plain_parser_catch_all;
pub mod parse_fragments_in_a_line;
pub mod specialized_parser_delim_matchers;
pub mod specialized_parsers;

// Re-export.
pub use plain_parser_catch_all::*;
pub use parse_fragments_in_a_line::*;
pub use specialized_parser_delim_matchers::*;
pub use specialized_parsers::*;
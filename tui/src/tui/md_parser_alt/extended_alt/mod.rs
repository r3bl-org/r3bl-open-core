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

// Attach.
pub mod parse_metadata_k_csv_alt;
pub mod parse_metadata_k_v_alt;
pub mod parser_take_text_until_eol_or_eoi_alt;

// Re-export.
pub use parse_metadata_k_csv_alt::*;
pub use parse_metadata_k_v_alt::*;
pub use parser_take_text_until_eol_or_eoi_alt::*;

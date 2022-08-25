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

use r3bl_rs_utils_core::*;
use syn::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Attrib {
  Bold,
  Dim,
  Underline,
  Reverse,
  Hidden,
  Strikethrough,
}

/// Docs: https://docs.rs/syn/1.0.98/syn/parse/struct.ParseBuffer.html
#[derive(Debug, Clone)]
pub(crate) struct StyleMetadata {
  pub id: Expr,                  /* Only required field. */
  pub attrib_vec: Vec<Attrib>,   /* Attributes are optional. */
  pub padding: Option<UnitType>, /* Optional. */
  pub color_fg: Option<Expr>,    /* Optional. */
  pub color_bg: Option<Expr>,    /* Optional. */
}

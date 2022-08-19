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

// REFACTOR: macro for easy box_start call
// REFACTOR: add test for this macro
#[macro_export]
macro_rules! box_start {
  (
    in: $arg_surface : expr, // Eg: in: tw_surface,
    $arg_id : expr,          // Eg: "foo",
    $arg_dir : expr,         // Eg: Direction::Horizontal,
    $arg_req_size : expr,    // Eg: (50, 100).try_into()?,
    [$($args:tt)*]           // Eg: [ "style1" , "style2" ]
  ) => {
    $arg_surface.box_start(box_props! {
      $arg_id,
      $arg_dir,
      $arg_req_size,
      get_styles! { from: $arg_surface.stylesheet => [$($args)*] }
    })?
  };
}

// REFACTOR: macro for easy tw_box_props creation
#[macro_export]
macro_rules! box_props {
  (
    $arg_id : expr,       // Eg: "foo",
    $arg_dir : expr,      // Eg: Direction::Horizontal,
    $arg_req_size : expr, // Eg: (50, 100).try_into()?,
    $arg_styles: expr     // Eg: get_styles! { from: stylesheet => ["style1", "style2"] };
  ) => {
    TWBoxProps {
      id: $arg_id.to_string(),
      dir: $arg_dir,
      req_size: $arg_req_size,
      styles: $arg_styles,
    }
  };
  (
    $arg_id : expr,       // Eg: "foo",
    $arg_dir : expr,      // Eg: Direction::Horizontal,
    $arg_req_size : expr, // Eg: (50, 100).try_into()?,
    [$($args:tt)*]        // Eg: [ style! {...} , style! {...} ]
  ) => {
    TWBoxProps {
      id: $arg_id.to_string(),
      dir: $arg_dir,
      req_size: $arg_req_size,
      styles: Some(vec![$($args)*]),
    }
  };
  (
    $arg_id : expr,       // Eg: "foo",
    $arg_dir : expr,      // Eg: Direction::Horizontal,
    $arg_req_size : expr, // Eg: (50, 100).try_into()?,
  ) => {
    TWBoxProps {
      id: $arg_id.to_string(),
      dir: $arg_dir,
      req_size: $arg_req_size,
      styles: None,
    }
  };
}

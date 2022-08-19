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

// REFACTOR: move this to top_level_macros.rs
// REFACTOR: macro to make box easily
/// Use incremental TT munching: https://veykril.github.io/tlborm/decl-macros/patterns/tt-muncher.html
#[macro_export]
macro_rules! make_box {
  (
    in:     $arg_surface : expr,   // Eg: in: tw_surface,
    id:     $arg_id : expr,        // Eg: "foo",
    dir:    $arg_dir : expr,       // Eg: Direction::Horizontal,
    size:   $arg_req_size : expr,  // Eg: (50, 100).try_into()?,
    style:  [$($args:tt)*],        // Eg: [ "style1" , "style2" ]
    render: {$($tail:tt)*}         // Eg: render! args
  ) => {
    box_start! {
      in: $arg_surface,
      $arg_id,
      $arg_dir,
      $arg_req_size,
      [$($args)*]
    };

    render! {
      in: $arg_surface,
      $($tail)*
    };

    $arg_surface.box_end()?;
  };
}

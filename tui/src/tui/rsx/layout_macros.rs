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

#[macro_export]
macro_rules! box_end {
    (
        in: $arg_surface : expr // Eg: in: surface,
        $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
        $arg_surface.box_end()?;
    };
}

/// When calling this, make sure to make a corresponding call to [box_end!].
#[macro_export]
macro_rules! box_start {
    (
        in:                     $arg_surface : expr,                // Eg: in: surface,
        id:                     $arg_id : expr,                     // Eg: 0,
        dir:                    $arg_dir : expr,                    // Eg: Direction::Horizontal,
        requested_size_percent: $arg_requested_size_percent : expr, // Eg: (50, 100).try_into()?,
        styles:                 [$($args:tt)*]                      // Eg: [ "style1" , "style2" ]
        $(,)*                   /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
      $arg_surface.box_start(box_props! {
            id:                     $arg_id,
            dir:                    $arg_dir,
            requested_size_percent: $arg_requested_size_percent,
            maybe_styles:           get_tui_styles! { @from: $arg_surface.stylesheet, [$($args)*] }
        })?
    };
}

#[macro_export]
macro_rules! box_props {
  (
    id:                     $arg_id : expr,                     // Eg: 0,
    dir:                    $arg_dir : expr,                    // Eg: Direction::Horizontal,
    requested_size_percent: $arg_requested_size_percent : expr, // Eg: (50, 100).try_into()?,
    maybe_styles:           $arg_styles: expr                   // Eg: get_tui_styles! {
                                                                //     from: stylesheet,
                                                                //     ["style1", "style2"] };
    $(,)*                   /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
  ) => {
    $crate::FlexBoxProps {
      id: $arg_id,
      dir: $arg_dir,
      requested_size_percent: $arg_requested_size_percent,
      maybe_styles: $arg_styles,
    }
  };

  (
    id:                     $arg_id : expr,                     // Eg: 0,
    dir:                    $arg_dir : expr,                    // Eg: Direction::Horizontal,
    requested_size_percent: $arg_requested_size_percent : expr, // Eg: (50, 100).try_into()?,
    maybe_styles:           [$($args:tt)*]                      // Eg: [style!{...} , style!{...}]
    $(,)*                   /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
  ) => {
    $crate::FlexBoxProps {
      id: $arg_id,
      dir: $arg_dir,
      requested_size_percent: $arg_requested_size_percent,
      maybe_styles: Some(vec![$($args)*]),
    }
  };

  (
    id:                     $arg_id : expr,                     // Eg:0,
    dir:                    $arg_dir : expr,                    // Eg: Direction::Horizontal,
    requested_size_percent: $arg_requested_size_percent : expr, // Eg: (50, 100).try_into()?,
    $(,)*                   /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
  ) => {
    $crate::FlexBoxProps {
      id: $arg_id,
      dir: $arg_dir,
      requested_size_percent: $arg_requested_size_percent,
      maybe_styles: None,
    }
  };
}

/// [Use incremental TT munching](https://veykril.github.io/tlborm/decl-macros/patterns/tt-muncher.html)
#[deprecated]
#[macro_export]
macro_rules! box_start_with_component {
  (
    in:                     $arg_surface : expr,                 // Eg: in: surface,
    id:                     $arg_id : expr,                      // Eg: 0,
    dir:                    $arg_dir : expr,                     // Eg: Direction::Horizontal,
    requested_size_percent: $arg_requested_size_percent : expr,  // Eg: (50, 100).try_into()?,
    styles:                 [$($args:tt)*],                      // Eg: [ "style1" , "style2" ]
    render:                 {$($tail:tt)*}                       // Eg: render! args
  ) => {
    box_start! {
      in:                     $arg_surface,
      id:                     $arg_id,
      dir:                    $arg_dir,
      requested_size_percent: $arg_requested_size_percent,
      styles:                 [$($args)*]
    };

    render_component_in_surface! {
      in:           $arg_surface,
      component_id: $arg_id,
      $($tail)*
    };

    $arg_surface.box_end()?;
  };
}

/// `self` has to be passed into `$arg_renderer` because this macro has a `let` statement
/// that requires it to have a block.
///
/// And in the block generated by the macro, `self` is not available from the calling
/// scope.
#[deprecated]
#[macro_export]
macro_rules! box_start_with_surface_renderer {
  (
    in:                     $arg_surface        : expr,           // Eg: in: surface,
    surface_renderer:       $arg_renderer       : expr,           // Eg: surface_renderer: two_col_layout,
    id:                     $arg_id             : expr,           // Eg: 0,
    dir:                    $arg_dir            : expr,           // Eg: Direction::Horizontal,
    requested_size_percent: $arg_requested_size_percent : expr,   // Eg: (50, 100).try_into()?,
    styles:                 [$($args_styles:tt)*],                // Eg: [ "style1" , "style2" ]
    state:                  $arg_state          : expr,           // Eg: state,
    shared_store:           $arg_shared_store   : expr,           // Eg: shared_store
    shared_global_data:     $arg_shared_global_data : expr,       // Eg: shared_global_data
    window_size:            $arg_window_size    : expr            // Eg: window_size
  ) => {
    box_start! {
      in:                     $arg_surface,
      id:                     $arg_id,
      dir:                    $arg_dir,
      requested_size_percent: $arg_requested_size_percent,
      styles:                 [$($args_styles)*]
    };

    $arg_renderer
      .render_in_surface(
        $crate::GlobalScopeArgs {
          shared_global_data: $arg_shared_global_data,
          shared_store:       $arg_shared_store,
          state:              $arg_state,
          window_size:        $arg_window_size
        },
        $arg_surface)
      .await?;

    $arg_surface.box_end()?;
  };
}

// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[macro_export]
macro_rules! box_end {
    (
        in: $arg_surface : expr // Eg: in: surface,
        $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
        $arg_surface.box_end()?;
    };
}

/// When calling this, make sure to make a corresponding call to [`box_end`!].
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
      $arg_surface.box_start($crate::FlexBoxProps {
            id:                     $arg_id,
            dir:                    $arg_dir,
            requested_size_percent: $arg_requested_size_percent,
            maybe_styles:           $crate::get_tui_styles! { @from: $arg_surface.stylesheet, [$($args)*.into()] }
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

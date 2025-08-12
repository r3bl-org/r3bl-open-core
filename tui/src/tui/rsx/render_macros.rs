// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Render the component in the current box (which is retrieved from the surface). This is
/// the "normal" way to render a component, in the `FlexBox` that is currently being laid
/// out.
#[macro_export]
macro_rules! render_component_in_current_box {
    (
        in:                  $arg_surface                 : expr,   // Eg: in: surface
        component_id:        $arg_component_id            : expr,   // Eg: "0"
        from:                $arg_component_registry_map  : expr,   // Eg: from: component_registry_map
        global_data:         $arg_global_data             : expr,   // Eg: global_data
        has_focus:           $arg_has_focus               : expr    // Eg: has_focus
    ) => {
        let maybe_component_ref = $crate::ComponentRegistry::try_to_get_component_by_id(
            $arg_component_registry_map,
            $arg_component_id,
        );

        if let Some(component_ref) = maybe_component_ref {
            let surface_bounds = $crate::SurfaceBounds::from(&*($arg_surface));
            let current_box = $arg_surface.current_box()?;
            let queue = component_ref.render(
                $arg_global_data,
                *current_box,
                surface_bounds,
                $arg_has_focus,
            )?;
            $arg_surface.render_pipeline += queue;
        }
    };
}

/// Render the component in the given box (which is not retrieved from the surface).
///
/// This is usually to do "absolute positioned" rendering of components (like for a modal
/// dialog box that paints on top of everything else in the window).
#[macro_export]
macro_rules! render_component_in_given_box {
    (
        in:           $arg_surface                  : expr, // Eg: in: surface
        box:          $arg_box                      : expr, // Eg: box: FlexBox::default()
        component_id: $arg_component_id             : expr, // Eg: "0"
        from:         $arg_component_registry_map   : expr, // Eg: from: component_registry_map
        global_data:  $arg_global_data              : expr, // Eg: global_data
        has_focus:    $arg_has_focus                : expr  // Eg: has_focus
     ) => {{
        let maybe_component_ref = $crate::ComponentRegistry::try_to_get_component_by_id(
            $arg_component_registry_map,
            $arg_component_id,
        );

        if let Some(component_ref) = maybe_component_ref {
            let surface_bounds = $crate::SurfaceBounds::from(&*($arg_surface));
            let queue: $crate::RenderPipeline = component_ref.render(
                $arg_global_data,
                $arg_box,
                surface_bounds,
                $arg_has_focus,
            )?;
            $arg_surface.render_pipeline += queue;
        }
    }};
}

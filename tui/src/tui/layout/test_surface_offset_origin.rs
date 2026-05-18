// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Regression test for `update_insertion_pos_for_next_box` – verify that sibling
/// boxes in a Horizontal container at a non‑zero origin all get the same row
/// component (preserved from the parent), instead of the second sibling having
/// its row zeroed.
#[cfg(test)]
mod tests {
    use crate::{CommonResult, FlexBoxId, FlexBoxProps, LayoutDirection,
                LayoutManagement, Surface, SurfaceProps, col, height, row, width};

    #[test]
    fn test_surface_horizontal_non_zero_origin() -> CommonResult<()> {
        fn make_container_assertions(surface: &Surface) -> CommonResult<()> {
            throws!({
                let container = surface.stack_of_boxes.first().unwrap();
                assert_eq2!(container.origin_pos, col(0) + row(5));
                assert_eq2!(container.dir, LayoutDirection::Horizontal);
            });
        }

        fn make_child1_assertions(surface: &Surface) -> CommonResult<()> {
            throws!({
                let child = surface.stack_of_boxes.last().unwrap();
                // First child starts at the container's insertion start = (0, 5).
                assert_eq2!(child.origin_pos, col(0) + row(5));
            });
        }

        fn make_child2_assertions(surface: &Surface) -> CommonResult<()> {
            throws!({
                let child = surface.stack_of_boxes.last().unwrap();
                // Second child row must equal the container's row (5), NOT 0.
                // Before the fix this was `col(50) + row(0)`.
                assert_eq2!(child.origin_pos, col(50) + row(5));
            });
        }

        throws!({
            let mut surface = Surface::default();

            // Surface starts at row 5 – the existing tests all start at (0, 0).
            surface.surface_start(SurfaceProps {
                pos: col(0) + row(5),
                size: width(100) + height(40),
            })?;

            // Horizontal container – no padding, fills the surface.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(0),
                dir: LayoutDirection::Horizontal,
                requested_size_percent: req_size_pc!(width: 100, height: 100),
                maybe_styles: None,
            })?;
            make_container_assertions(&surface)?;

            // First child: 50 % width.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(1),
                dir: LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width: 50, height: 100),
                maybe_styles: None,
            })?;
            make_child1_assertions(&surface)?;
            surface.box_end()?;

            // Second child: 50 % width.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(2),
                dir: LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width: 50, height: 100),
                maybe_styles: None,
            })?;
            make_child2_assertions(&surface)?;
            surface.box_end()?;

            surface.box_end()?;
            surface.surface_end()?;
        });
    }

    #[test]
    fn test_surface_vertical_non_zero_origin() -> CommonResult<()> {
        fn make_container_assertions(surface: &Surface) -> CommonResult<()> {
            throws!({
                let container = surface.stack_of_boxes.first().unwrap();
                assert_eq2!(container.origin_pos, col(5) + row(0));
                assert_eq2!(container.dir, LayoutDirection::Vertical);
            });
        }

        fn make_child1_assertions(surface: &Surface) -> CommonResult<()> {
            throws!({
                let child = surface.stack_of_boxes.last().unwrap();
                assert_eq2!(child.origin_pos, col(5) + row(0));
            });
        }

        fn make_child2_assertions(surface: &Surface) -> CommonResult<()> {
            throws!({
                let child = surface.stack_of_boxes.last().unwrap();
                // Second child column must equal the container's column (5), NOT 0.
                // Before the fix for Vertical this would have been `col(0) + row(20)`.
                assert_eq2!(child.origin_pos, col(5) + row(20));
            });
        }

        throws!({
            let mut surface = Surface::default();

            surface.surface_start(SurfaceProps {
                pos: col(5) + row(0),
                size: width(100) + height(40),
            })?;

            // Vertical container - no padding, fills the surface.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(0),
                dir: LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width: 100, height: 100),
                maybe_styles: None,
            })?;
            make_container_assertions(&surface)?;

            // First child: 50 % height.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(1),
                dir: LayoutDirection::Horizontal,
                requested_size_percent: req_size_pc!(width: 100, height: 50),
                maybe_styles: None,
            })?;
            make_child1_assertions(&surface)?;
            surface.box_end()?;

            // Second child: 50 % height.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(2),
                dir: LayoutDirection::Horizontal,
                requested_size_percent: req_size_pc!(width: 100, height: 50),
                maybe_styles: None,
            })?;
            make_child2_assertions(&surface)?;
            surface.box_end()?;

            surface.box_end()?;
            surface.surface_end()?;
        });
    }
}

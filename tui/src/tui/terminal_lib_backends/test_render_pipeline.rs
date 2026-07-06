// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[cfg(test)]
mod tests {
    use crate::{RenderOpCommon, RenderOpIR, RenderOpIRVec, RenderPipeline, ZOrder,
                assert_eq2, render_pipeline};

    #[test]
    fn render_ops_macro() {
        let mut render_ops = RenderOpIRVec::new();
        render_ops += RenderOpCommon::ClearScreen;
        render_ops += RenderOpCommon::ResetColor;
        assert_eq2!(render_ops.len(), 2);
    }

    #[test]
    fn render_pipeline_macro() {
        // Single pipeline.
        let mut pipeline = render_pipeline!();

        render_pipeline!(
          @push_into pipeline
          at ZOrder::Normal =>
            RenderOpIR::Common(RenderOpCommon::ClearScreen),
            RenderOpIR::Common(RenderOpCommon::ResetColor)
        );
        let _render_ops_set = pipeline.get(&ZOrder::Normal);

        let render_op_count = pipeline.get_all_render_op_in(ZOrder::Normal);

        assert_eq2!(render_op_count, 2);
    }

    #[test]
    fn merge_pipelines() {
        // Merge multiple pipelines.
        let pipeline_1: RenderPipeline = {
            let mut it = render_pipeline!(@new ZOrder::Normal
              =>
                RenderOpIR::Common(RenderOpCommon::ClearScreen),
                RenderOpIR::Common(RenderOpCommon::ResetColor)
            );

            render_pipeline!(@push_into it at ZOrder::High =>
              RenderOpIR::Common(RenderOpCommon::ResetColor)
            );

            assert_eq2!(it.get_all_render_op_in(ZOrder::Normal), 2);

            assert_eq2!(it.get_all_render_op_in(ZOrder::High), 1);

            it
        };

        // This is a duplicate of the above pipeline.
        let pipeline_2: RenderPipeline = {
            let it = render_pipeline!(@new ZOrder::Normal
              =>
                RenderOpIR::Common(RenderOpCommon::ClearScreen),
                RenderOpIR::Common(RenderOpCommon::ResetColor)
            );

            assert_eq2!(it.get_all_render_op_in(ZOrder::Normal), 2);

            it
        };

        let _pipeline_merged: RenderPipeline = {
            let pipeline_merged = render_pipeline!(@join_and_drop pipeline_1, pipeline_2);
            let _normal_set = pipeline_merged.get(&ZOrder::Normal);
            let _caret_set = pipeline_merged.get(&ZOrder::High);

            assert_eq2!(pipeline_merged.get_all_render_op_in(ZOrder::Normal), 4);
            assert_eq2!(pipeline_merged.get_all_render_op_in(ZOrder::High), 1);

            pipeline_merged
        };
    }

    #[test]
    fn hoist_z_order_in_pipeline() {
        let mut pipeline = render_pipeline!();

        render_pipeline!(@push_into pipeline at ZOrder::Normal =>
          RenderOpIR::Common(RenderOpCommon::ClearScreen),
          RenderOpIR::Common(RenderOpCommon::ResetColor)
        );

        pipeline.hoist(ZOrder::Normal, ZOrder::Glass);

        assert!(pipeline.get(&ZOrder::Normal).is_empty());
        assert_eq2!(pipeline.get_all_render_op_in(ZOrder::Glass), 2);
    }
}

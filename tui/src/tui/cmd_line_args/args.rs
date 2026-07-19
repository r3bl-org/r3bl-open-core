// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DialogEngine, EditorBuffer, EditorEngine, FlexBoxId, GlobalData, HasFocus};
use std::fmt::Debug;

#[derive(Debug)]
pub struct RenderArgs<'a> {
    pub engine: &'a mut EditorEngine,
    pub buffer: &'a EditorBuffer,
    pub has_focus: &'a mut HasFocus,
}

mod impl_render_args {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl<'a> RenderArgs<'a> {
        pub fn new(
            engine: &'a mut EditorEngine,
            buffer: &'a EditorBuffer,
            has_focus: &'a mut HasFocus,
        ) -> Self {
            Self {
                engine,
                buffer,
                has_focus,
            }
        }
    }
}

#[derive(Debug)]
pub struct EditorArgsMut<'a> {
    pub buffer: &'a mut EditorBuffer,
    pub engine: &'a mut EditorEngine,
}

mod impl_editor_args_mut {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl<'a> EditorArgsMut<'a> {
        pub fn new(buffer: &'a mut EditorBuffer, engine: &'a mut EditorEngine) -> Self {
            Self { buffer, engine }
        }
    }
}

/// [`DialogEngine`] args struct that holds references.
///
/// ![Editor component lifecycle
/// diagram](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/docs/memory-architecture.drawio.svg)
#[derive(Debug)]
pub struct DialogEngineArgs<'a, S, AS>
where
    S: Debug + Default + Clone + Sync + Send,
    AS: Debug + Default + Clone + Sync + Send,
{
    pub self_id: FlexBoxId,
    pub global_data: &'a mut GlobalData<S, AS>,
    pub engine: &'a mut DialogEngine,
    pub has_focus: &'a mut HasFocus,
}

mod impl_dialog_engine_args {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl<'a, S, AS> DialogEngineArgs<'a, S, AS>
    where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        pub fn new(
            self_id: FlexBoxId,
            global_data: &'a mut GlobalData<S, AS>,
            engine: &'a mut DialogEngine,
            has_focus: &'a mut HasFocus,
        ) -> Self {
            Self {
                self_id,
                global_data,
                engine,
                has_focus,
            }
        }
    }
}

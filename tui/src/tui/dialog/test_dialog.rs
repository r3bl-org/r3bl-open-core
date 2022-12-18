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

#[cfg(test)]
mod test_dialog_event {
    use r3bl_rs_utils_core::*;

    use crate::*;

    #[test]
    fn dialog_event_handles_enter() {
        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Enter));
        let dialog_event = DialogEvent::from(&input_event);
        assert_eq2!(dialog_event, DialogEvent::EnterPressed);
    }

    #[test]
    fn dialog_event_handles_esc() {
        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Esc));
        let dialog_event = DialogEvent::from(&input_event);
        assert_eq2!(dialog_event, DialogEvent::EscPressed);
    }

    #[test]
    fn dialog_event_handles_modal_keypress() {
        let modal_keypress = keypress!(@char ModifierKeysMask::CTRL, 'l');
        let input_event = InputEvent::Keyboard(modal_keypress);
        let dialog_event = DialogEvent::should_activate_modal(&input_event, modal_keypress);
        assert_eq2!(dialog_event, DialogEvent::ActivateModal);
    }
}

#[cfg(test)]
mod test_dialog_api_make_flex_box_for_dialog {
    use std::error::Error;

    use r3bl_rs_utils_core::*;

    use crate::*;

    /// More info on `is` and downcasting:
    /// - https://stackoverflow.com/questions/71409337/rust-how-to-match-against-any
    /// - https://ysantos.com/blog/downcast-rust
    #[test]
    fn make_flex_box_for_dialog_display_size_too_small() {
        let surface = Surface::default();
        let window_size = Size::default();
        let dialog_id: FlexBoxId = 0;

        // The window size is too small and will result in this error.
        // Err(
        //   CommonError {
        //       err_type: DisplaySizeTooSmall,
        //       err_msg: Some(
        //           "Window size is too small. Min size is 65 cols x 10 rows",
        //       ),
        //   },
        let result_flex_box = dbg!(DialogEngineApi::make_flex_box_for_dialog(
            dialog_id,
            &surface,
            &window_size
        ));

        // Assert that a general `CommonError` is returned.
        let my_err: Box<dyn Error + Send + Sync> = result_flex_box.err().unwrap();
        assert_eq2!(my_err.is::<CommonError>(), true);

        // Assert that this specific error is returned.
        let result = matches!(
            my_err.downcast_ref::<CommonError>(),
            Some(CommonError {
                err_type: CommonErrorType::DisplaySizeTooSmall,
                err_msg: _,
            })
        );

        assert_eq2!(result, true);
    }

    #[test]
    fn make_flex_box_for_dialog() {
        // 1. The surface and window_size are not the same width and height.
        // 2. The surface is also not starting from the top left corner of the window.
        let surface = Surface {
            origin_pos: position! { col_index: 2, row_index: 2 },
            box_size: size!( col_count: 65, row_count: 10 ),
            ..Default::default()
        };
        let window_size = size!( col_count: 70, row_count: 15 );
        let self_id: FlexBoxId = 0;

        // The dialog box should be centered inside the surface.
        let result_flex_box = dbg!(DialogEngineApi::make_flex_box_for_dialog(
            self_id,
            &surface,
            &window_size
        ));

        assert_eq2!(result_flex_box.is_ok(), true);

        let flex_box = result_flex_box.unwrap();
        assert_eq2!(flex_box.id, self_id);
        assert_eq2!(
            flex_box.style_adjusted_bounds_size,
            size!( col_count: 58, row_count: 4 )
        );
        assert_eq2!(
            flex_box.style_adjusted_origin_pos,
            position!( col_index: 5, row_index: 5 )
        );
    }
}

#[cfg(test)]
mod test_dialog_api_apply_event {
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::*;

    #[tokio::test]
    async fn apply_event_esc() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects::make_dialog_engine();
        let shared_store = &mock_real_objects::create_store();
        let state = &String::new();
        let shared_global_data =
            &test_editor::mock_real_objects::make_shared_global_data((*window_size).into());
        let component_registry = &mut test_editor::mock_real_objects::make_component_registry();

        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Esc));
        let response = dbg!(DialogEngineApi::apply_event(args, &input_event)
            .await
            .unwrap());
        assert!(matches!(
            response,
            DialogEngineApplyResponse::DialogChoice(DialogChoice::No)
        ));
    }

    #[tokio::test]
    async fn apply_event_enter() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects::make_dialog_engine();
        let shared_store = &mock_real_objects::create_store();
        let state = &String::new();
        let shared_global_data =
            &test_editor::mock_real_objects::make_shared_global_data((*window_size).into());
        let component_registry = &mut test_editor::mock_real_objects::make_component_registry();

        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let input_event = InputEvent::Keyboard(keypress!(@special SpecialKey::Enter));
        let response = dbg!(DialogEngineApi::apply_event(args, &input_event)
            .await
            .unwrap());
        if let DialogEngineApplyResponse::DialogChoice(DialogChoice::Yes(value)) = &response {
            assert_eq2!(value, "");
        }
        assert!(matches!(
            response,
            DialogEngineApplyResponse::DialogChoice(DialogChoice::Yes(_))
        ));
    }

    #[tokio::test]
    async fn apply_event_other_key() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects::make_dialog_engine();
        let shared_store = &mock_real_objects::create_store();
        let state = &String::new();
        let shared_global_data =
            &test_editor::mock_real_objects::make_shared_global_data((*window_size).into());
        let component_registry = &mut test_editor::mock_real_objects::make_component_registry();

        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        let input_event = InputEvent::Keyboard(keypress!(@char 'a'));
        let response = dbg!(DialogEngineApi::apply_event(args, &input_event)
            .await
            .unwrap());
        if let DialogEngineApplyResponse::UpdateEditorBuffer(editor_buffer) = &response {
            assert_eq2!(editor_buffer.get_as_string(), "a");
        }
    }
}

#[cfg(test)]
mod test_dialog_api_render_engine {
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::*;

    #[tokio::test]
    async fn render_engine() {
        let self_id: FlexBoxId = 0;
        let window_size = &size!( col_count: 70, row_count: 15 );
        let dialog_buffer = &mut DialogBuffer::new_empty();
        let dialog_engine = &mut mock_real_objects::make_dialog_engine();
        let shared_store = &mock_real_objects::create_store();
        let state = &String::new();
        let shared_global_data =
            &test_editor::mock_real_objects::make_shared_global_data((*window_size).into());
        let component_registry = &mut test_editor::mock_real_objects::make_component_registry();

        let args = DialogEngineArgs {
            shared_global_data,
            shared_store,
            state,
            component_registry,
            window_size,
            self_id,
            dialog_buffer,
            dialog_engine,
        };

        // 1. The surface and window_size are not the same width and height.
        // 2. The surface is also not starting from the top left corner of the window.
        let surface = Surface {
            origin_pos: position! { col_index: 2, row_index: 2 },
            box_size: size!( col_count: 65, row_count: 10 ),
            ..Default::default()
        };
        let window_size = size!( col_count: 70, row_count: 15 );
        let self_id: FlexBoxId = 0;

        // The dialog box should be centered inside the surface.
        let result_flex_box = dbg!(DialogEngineApi::make_flex_box_for_dialog(
            self_id,
            &surface,
            &window_size
        ))
        .unwrap();

        let pipeline = dbg!(DialogEngineApi::render_engine(args, &result_flex_box)
            .await
            .unwrap());
        assert_eq2!(pipeline.len(), 1);
        let render_ops = pipeline.get(&ZOrder::Glass).unwrap();
        assert!(!render_ops.is_empty());
    }
}

pub mod mock_real_objects {
    use std::sync::Arc;

    use r3bl_redux::{SharedStore, Store};
    use tokio::sync::RwLock;

    use crate::{test_editor::mock_real_objects, *};

    pub fn create_store() -> Arc<RwLock<Store<String, String>>> {
        let mut _store = Store::<String, String>::default();
        let shared_store: SharedStore<String, String> = Arc::new(RwLock::new(_store));
        shared_store
    }

    pub fn make_dialog_engine() -> DialogEngine {
        DialogEngine {
            editor_engine: mock_real_objects::make_editor_engine(),
            ..Default::default()
        }
    }
}

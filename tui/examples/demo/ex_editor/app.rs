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
 *   Unless required by applicable law or agreed &to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::fmt::Debug;

use async_trait::async_trait;
use int_enum::IntEnum;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;
use strum_macros::AsRefStr;

use super::*;

/// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntEnum)]
enum Id {
  Editor = 1,
  Dialog = 2,
}

#[derive(Debug, Eq, PartialEq, AsRefStr)]
pub enum DialogStyleId {
  Border,
  Title,
  Editor,
}

/// Async trait object that implements the [TWApp] trait.
pub struct AppWithLayout {
  pub component_registry: ComponentRegistry<State, Action>,
}

mod app_trait_impl {
  use super::*;

  #[async_trait]
  impl App<State, Action> for AppWithLayout {
    // ┏━━━━━━━━━━━━━━━━━━┓
    // ┃ app_handle_event ┃
    // ┛                  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    async fn app_handle_event(
      &mut self,
      args: GlobalScopeArgs<'_, State, Action>,
      input_event: &InputEvent,
    ) -> CommonResult<EventPropagation> {
      let GlobalScopeArgs {
        state,
        shared_store,
        shared_tw_data,
        window_size,
      } = args;

      call_if_true!(DEBUG_TUI_MOD, {
        log_no_err!(DEBUG, "🐝 focus: {:?}", self.component_registry.has_focus);
        log_no_err!(DEBUG, "💾 user_data: {:?}", self.component_registry.user_data);
      });

      if let EventPropagation::Consumed = self.try_input_event_activate_modal(args, input_event) {
        return Ok(EventPropagation::Consumed);
      }

      route_event_to_focused_component!(
        registry:       self.component_registry,
        has_focus:      self.has_focus,
        input_event:    input_event,
        state:          state,
        shared_store:   shared_store,
        shared_tw_data: shared_tw_data,
        window_size:    window_size
      )
    }

    // ┏━━━━━━━━━━━━┓
    // ┃ app_render ┃
    // ┛            ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    async fn app_render(&mut self, args: GlobalScopeArgs<'_, State, Action>) -> CommonResult<RenderPipeline> {
      throws_with_return!({
        let GlobalScopeArgs {
          state,
          shared_store,
          shared_tw_data,
          window_size,
        } = args;
        let adjusted_window_size = size!(cols: window_size.cols, rows: window_size.rows - 1);

        // Render container component.
        let mut surface = surface_start_with_surface_renderer! {
          surface_renderer: container_layout_render::RenderContainer{ app_ref: self },
          stylesheet:       style_helpers::create_stylesheet()?,
          pos:              position!(col:0, row:0),
          size:             adjusted_window_size, // Bottom row for status bar.
          state:            state,
          shared_store:     shared_store,
          shared_tw_data:   shared_tw_data,
          window_size:      window_size
        };

        // Render status bar.
        status_bar_helpers::render_status_bar(&mut surface.render_pipeline, window_size);

        // Return RenderOps pipeline (which will actually be painted elsewhere).
        surface.render_pipeline
      });
    }
  }
}

mod detect_modal_dialog_activation_from_input_event {
  use super::*;

  impl AppWithLayout {
    /// If `input_event` matches <kbd>Ctrl+l</kbd>, then toggle the modal dialog.
    pub fn try_input_event_activate_modal(
      &mut self,
      args: GlobalScopeArgs<'_, State, Action>,
      input_event: &InputEvent,
    ) -> EventPropagation {
      let maybe_input_event_matches_modal_activation_key = DialogEvent::try_from(
        input_event,
        Some(KeyPress::WithModifiers {
          key: Key::Character('l'),
          mask: ModifierKeysMask::CTRL,
        }),
      );

      match maybe_input_event_matches_modal_activation_key {
        Some(DialogEvent::ActivateModal) => {
          self.activate_modal(args);
          EventPropagation::Consumed
        }
        _ => EventPropagation::Propagate,
      }
    }

    fn activate_modal(&mut self, args: GlobalScopeArgs<State, Action>) {
      self.component_registry.has_focus.set_modal_id(Id::Dialog.int_value());

      let text = {
        if let Some(editor_buffer) = args.state.get_editor_buffer(Id::Editor.int_value()) {
          editor_buffer.get_as_string()
        } else {
          "Press <Esc> to close, or <Enter> to accept".to_string()
        }
      };

      let title = "Modal Dialog Title";

      spawn_dispatch_action!(
        args.shared_store,
        Action::SetDialogBufferTitleAndText(title.to_string(), text.to_string())
      );

      call_if_true!(DEBUG_TUI_MOD, {
        log_no_err!(DEBUG, "📣 activate modal: {:?}", self.component_registry.has_focus);
      });
    }
  }
}

mod container_layout_render {
  use super::*;

  pub struct RenderContainer<'a> {
    pub app_ref: &'a mut AppWithLayout,
  }

  #[async_trait]
  impl SurfaceRenderer<State, Action> for RenderContainer<'_> {
    async fn render_in_surface(
      &mut self,
      args: GlobalScopeArgs<'_, State, Action>,
      surface: &mut Surface,
    ) -> CommonResult<()> {
      throws!({
        let GlobalScopeArgs {
          state,
          shared_store,
          shared_tw_data,
          window_size,
        } = args;

        populate_component_registry::init(self.app_ref, surface).await;

        // Layout editor component, and render it.
        box_start_with_component! {
          in:                     surface,
          id:                     Id::Editor.int_value(),
          dir:                    Direction::Vertical,
          requested_size_percent: requested_size_percent!(width: 100, height: 100),
          styles:                 [&Id::Editor.int_value().to_string()],
          render: {
            from:           self.app_ref.component_registry,
            state:          state,
            shared_store:   shared_store,
            shared_tw_data: shared_tw_data,
            window_size:    window_size
          }
        }

        // Then, render modal dialog (if it is active).
        if self
          .app_ref
          .component_registry
          .has_focus
          .is_modal_id(Id::Dialog.int_value())
        {
          render_component_in_box! {
            in:             surface,
            box:            DialogEngineApi::make_flex_box_for_dialog(Id::Dialog.int_value(), surface, window_size)?,
            component_id:   Id::Dialog.int_value(),
            from:           self.app_ref.component_registry,
            state:          state,
            shared_store:   shared_store,
            shared_tw_data: shared_tw_data,
            window_size:    window_size
          };
        }
      });
    }
  }
}

mod populate_component_registry {

  use super::*;

  pub async fn init(app_ref: &mut AppWithLayout, surface: &mut Surface) {
    let editor_id = Id::Editor.int_value();
    let dialog_id = Id::Dialog.int_value();

    try_insert_editor_component(app_ref, editor_id);
    try_insert_dialog_component(app_ref, dialog_id, surface);
    try_init_has_focus(app_ref, editor_id);
  }

  /// Switch focus to the editor component if focus is not set.
  fn try_init_has_focus(app_ref: &mut AppWithLayout, id: FlexBoxIdType) {
    if app_ref.component_registry.has_focus.is_set() {
      return;
    }

    app_ref.component_registry.has_focus.set_id(id);
    call_if_true!(DEBUG_TUI_MOD, {
      log_no_err!(DEBUG, "🪙 {} = {}", "init component_registry.has_focus", id);
    });
  }

  /// Insert dialog component into registry if it's not already there.
  fn try_insert_dialog_component(app_ref: &mut AppWithLayout, id: FlexBoxIdType, surface: &mut Surface) {
    if app_ref.component_registry.contains(id) {
      return;
    }

    let shared_dialog_component = {
      fn on_dialog_press(dialog_choice: DialogChoice, shared_store: &SharedStore<State, Action>) {
        match dialog_choice {
          DialogChoice::Yes(text) => {
            spawn_dispatch_action!(
              shared_store,
              Action::SetDialogBufferTitleAndText("Yes".to_string(), text)
            );
          }
          DialogChoice::No => {
            spawn_dispatch_action!(
              shared_store,
              Action::SetDialogBufferTitleAndText("No".to_string(), "".to_string())
            );
          }
        }
      }

      fn on_dialog_editor_change_handler(editor_buffer: EditorBuffer, shared_store: &SharedStore<State, Action>) {
        spawn_dispatch_action!(shared_store, Action::UpdateDialogBuffer(editor_buffer));
      }

      DialogComponent::new_shared(
        id,
        on_dialog_press,
        on_dialog_editor_change_handler,
        get_style! { from: surface.stylesheet , DialogStyleId::Border.as_ref() },
        get_style! { from: surface.stylesheet , DialogStyleId::Title.as_ref() },
        get_style! { from: surface.stylesheet , DialogStyleId::Editor.as_ref() },
      )
    };

    app_ref.component_registry.put(id, shared_dialog_component);

    call_if_true!(DEBUG_TUI_MOD, {
      log_no_err!(DEBUG, "🪙 {}", "construct DialogComponent { on_dialog_press }");
    });
  }

  /// Insert editor component into registry if it's not already there.
  fn try_insert_editor_component(app_ref: &mut AppWithLayout, id: FlexBoxIdType) {
    if app_ref.component_registry.contains(id) {
      return;
    }

    let shared_editor_component = {
      fn on_buffer_change(shared_store: &SharedStore<State, Action>, my_id: FlexBoxIdType, buffer: EditorBuffer) {
        spawn_dispatch_action!(shared_store, Action::UpdateEditorBufferById(my_id, buffer));
      }

      let config_options = EditorEngineConfigOptions::default();
      EditorComponent::new_shared(id, config_options, on_buffer_change)
    };

    app_ref.component_registry.put(id, shared_editor_component);

    call_if_true!(DEBUG_TUI_MOD, {
      log_no_err!(DEBUG, "🪙 {}", "construct EditorComponent { on_buffer_change }");
    });
  }
}

mod debug_helpers {
  use super::*;

  impl Default for AppWithLayout {
    fn default() -> Self {
      // Potentially do any other initialization here.
      call_if_true!(DEBUG_TUI_MOD, {
        log_no_err!(
          DEBUG,
          "🪙 {}",
          "construct ex_editor::AppWithLayout { ComponentRegistry }"
        );
      });

      Self {
        component_registry: Default::default(),
      }
    }
  }

  impl Debug for AppWithLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_struct("AppWithLayout")
        .field("component_registry", &self.component_registry)
        .finish()
    }
  }
}

mod style_helpers {
  use super::*;

  pub fn create_stylesheet() -> CommonResult<Stylesheet> {
    throws_with_return!({
      stylesheet! {
        style! {
          id: Id::Editor.int_value().to_string()
          attrib: [bold]
          padding: 1
          color_fg: TWColor::Blue
        },
        style! {
          id: DialogStyleId::Title.as_ref()
          attrib: [bold]
          color_fg: TWColor::Yellow
          lolcat: true
        },
        style! {
          id: DialogStyleId::Border.as_ref()
          attrib: [dim]
          color_fg: TWColor::Green
          lolcat: true
        },
        style! {
          id: DialogStyleId::Editor.as_ref()
          attrib: [bold]
          color_fg: TWColor::Magenta
        }
      }
    })
  }
}

mod status_bar_helpers {
  use super::*;

  /// Shows helpful messages at the bottom row of the screen.
  pub fn render_status_bar(render_pipeline: &mut RenderPipeline, size: &Size) {
    let st_vec = styled_texts! {
      styled_text! { "Hints:",                       style!(attrib: [dim])  },
      styled_text! { " Ctrl + x : Exit ⛔ ",         style!(attrib: [bold]) },
      styled_text! { " … ",                          style!(attrib: [dim])  },
      styled_text! { " Type content 🖖 ",            style!(attrib: [bold]) },
      styled_text! { " … ",                          style!(attrib: [dim])  },
      styled_text! { " Ctrl + l : Modal dialog 📣 ", style!(attrib: [bold]) }
    };

    let display_width = st_vec.display_width();
    let col_center: ChUnit = (size.cols - display_width) / 2;
    let row_bottom: ChUnit = size.rows - 1;
    let center: Position = position!(col: col_center, row: row_bottom);

    *render_pipeline += (ZOrder::Normal, RenderOp::MoveCursorPositionAbs(center));
    *render_pipeline += st_vec.render(ZOrder::Normal);
  }
}

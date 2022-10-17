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
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;
use strum_macros::Display;

use super::*;

/// Constants for the ids.
#[derive(Display, Debug)]
enum Id {
  Container,
  Editor,
  Dialog,
}

/// Async trait object that implements the [TWApp] trait.
pub struct AppWithLayout {
  pub component_registry: ComponentRegistry<State, Action>,
}

mod constructor {
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
        component_registry: ComponentRegistry::default(),
      }
    }
  }
}

mod impl_app {
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

      self.try_activate(input_event);

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
        let mut surface = surface_start_with_runnable! {
          runnable:       app_render::AppWithLayoutRenderer(self),
          stylesheet:     style_helpers::create_stylesheet()?,
          pos:            position!(col:0, row:0),
          size:           adjusted_window_size, // Bottom row for status bar.
          state:          state,
          shared_store:   shared_store,
          shared_tw_data: shared_tw_data,
          window_size:    window_size
        };

        // Render status bar.
        status_bar_helpers::render(&mut surface.render_pipeline, window_size);

        // Return RenderOps pipeline (which will actually be painted elsewhere).
        surface.render_pipeline
      });
    }
  }
}

mod modal_dialog {
  use super::*;

  impl AppWithLayout {
    /// If `input_event` matches <kbd>Ctrl+l</kbd>, then toggle the modal dialog.
    pub fn try_activate(&mut self, input_event: &InputEvent) {
      if let Ok(DialogEvent::ActivateModal) = DialogEvent::try_from(
        input_event,
        Keypress::WithModifiers {
          key: Key::Character('l'),
          mask: ModifierKeysMask::CTRL,
        },
      ) {
        call_if_true!(DEBUG_TUI_MOD, {
          log_no_err!(DEBUG, "🅰️ activate modal");
        });

        self.component_registry.has_focus.set_modal_id(&Id::Dialog.to_string());
      };
    }
  }
}

mod app_render {
  use super::*;

  pub struct AppWithLayoutRenderer<'a>(pub &'a mut AppWithLayout);

  #[async_trait]
  impl SurfaceRunnable<State, Action> for AppWithLayoutRenderer<'_> {
    async fn run_on_surface(
      &mut self,
      args: GlobalScopeArgs<'_, State, Action>,
      surface: &mut Surface,
    ) -> CommonResult<()> {
      let GlobalScopeArgs {
        state,
        shared_store,
        shared_tw_data,
        window_size,
      } = args;

      component_registry::populate(self.0).await;

      let editor_id = &Id::Editor.to_string();
      throws!({
        box_start_with_component! {
          in:                     surface,
          id:                     editor_id,
          dir:                    Direction::Vertical,
          requested_size_percent: requested_size_percent!(width: 100, height: 100),
          styles:                 [editor_id],
          render: {
            from:           self.0.component_registry,
            state:          state,
            shared_store:   shared_store,
            shared_tw_data: shared_tw_data,
            window_size:    window_size
          }
        }
      })
    }
  }
}

mod component_registry {
  use super::*;

  pub async fn populate(this: &mut AppWithLayout) {
    let editor_id = &Id::Editor.to_string();
    let dialog_id = &Id::Dialog.to_string();

    try_insert_editor_component(this, editor_id);
    try_insert_dialog_component(this, dialog_id);
    try_init_has_focus(this, editor_id);
  }

  /// Switch focus to the editor component if focus is not set.
  fn try_init_has_focus(this: &mut AppWithLayout, id: &str) {
    if this.component_registry.has_focus.is_set() {
      return;
    }

    this.component_registry.has_focus.set_id(id);
    call_if_true!(DEBUG_TUI_MOD, {
      log_no_err!(DEBUG, "🪙 {} = {}", "init component_registry.has_focus", id);
    });
  }

  /// Insert dialog component into registry if it's not already there.
  fn try_insert_dialog_component(this: &mut AppWithLayout, id: &str) {
    if this.component_registry.has(id) {
      return;
    }

    let shared_dialog_component = {
      fn on_dialog_press(
        my_id: String,
        dialog_response: DialogResponse,
        _prev_focus_id: String,
        shared_store: &SharedStore<State, Action>,
        _component_registry: &ComponentRegistry<State, Action>,
      ) {
        match dialog_response {
          DialogResponse::Yes(text) => {
            spawn_dispatch_action!(shared_store, Action::SetDialog(my_id, text));
          }
          DialogResponse::No => {
            spawn_dispatch_action!(shared_store, Action::SetDialog(my_id, "".to_string()));
          }
        }
      }

      DialogComponent::new_shared(id, on_dialog_press)
    };

    this.component_registry.put(id, shared_dialog_component);

    call_if_true!(DEBUG_TUI_MOD, {
      log_no_err!(DEBUG, "🪙 {}", "construct DialogComponent { on_dialog_press }");
    });
  }

  /// Insert editor component into registry if it's not already there.
  fn try_insert_editor_component(this: &mut AppWithLayout, id: &str) {
    if this.component_registry.has(id) {
      return;
    }

    let shared_editor_component = {
      fn on_buffer_change(shared_store: &SharedStore<State, Action>, my_id: String, buffer: EditorBuffer) {
        spawn_dispatch_action!(shared_store, Action::InsertEditorBuffer(my_id, buffer));
      }

      let config_options = EditorEngineConfigOptions::default();
      EditorComponent::new_shared(id, config_options, on_buffer_change)
    };

    this.component_registry.put(id, shared_editor_component);

    call_if_true!(DEBUG_TUI_MOD, {
      log_no_err!(DEBUG, "🪙 {}", "construct EditorComponent { on_buffer_change }");
    });
  }
}

mod debug_helpers {
  use super::*;

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
    let container_id = &Id::Container.to_string();
    let editor_id = &Id::Editor.to_string();

    throws_with_return!({
      stylesheet! {
        style! {
          id: container_id
        },
        style! {
          id: editor_id
          attrib: [bold]
          padding: 1
          color_fg: TWColor::Blue
        }
      }
    })
  }
}

mod status_bar_helpers {
  use super::*;

  /// Shows helpful messages at the bottom row of the screen.
  pub fn render(render_pipeline: &mut RenderPipeline, size: &Size) {
    let st_vec = styled_texts! {
      styled_text! { "Hints:",               style!(attrib: [dim])       },
      styled_text! { " Ctrl + x : Exit ⛔ ", style!(attrib: [bold])      },
      styled_text! { " … ",                  style!(attrib: [dim])       },
      styled_text! { " char : add ",         style!(attrib: [underline]) },
      styled_text! { " … ",                  style!(attrib: [dim])       },
      styled_text! { " TK / TK : TK ",        style!(attrib: [underline]) }
    };

    let display_width = st_vec.display_width();
    let col_center: ChUnit = (size.cols - display_width) / 2;
    let row_bottom: ChUnit = size.rows - 1;
    let center: Position = position!(col: col_center, row: row_bottom);

    *render_pipeline += (ZOrder::Normal, RenderOp::MoveCursorPositionAbs(center));
    *render_pipeline += st_vec.render(ZOrder::Normal);
  }
}

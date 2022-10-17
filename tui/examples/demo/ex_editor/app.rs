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

mod app_impl {
  use super::*;

  #[async_trait]
  impl App<State, Action> for AppWithLayout {
    // ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
    // │ app_handle_event │
    // ╯                  ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
    async fn app_handle_event(
      &mut self,
      args: GlobalScopeArgs<'_, State, Action>,
      input_event: &InputEvent,
    ) -> CommonResult<EventPropagation> {
      let GlobalScopeArgs {
        state,
        shared_store,
        shared_tw_data,
        ..
      } = args;

      route_event_to_focused_component!(
        registry:       self.component_registry,
        has_focus:      self.has_focus,
        input_event:    input_event,
        state:          state,
        shared_store:   shared_store,
        shared_tw_data: shared_tw_data
      )
    }

    // ╭┄┄┄┄┄┄┄┄┄┄┄┄╮
    // │ app_render │
    // ╯            ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
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
          runnable:       self,
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

  #[async_trait]
  impl SurfaceRunnable<State, Action> for AppWithLayout {
    async fn run_on_surface(
      &mut self,
      args: GlobalScopeArgs<'_, State, Action>,
      surface: &mut Surface,
    ) -> CommonResult<()> {
      let GlobalScopeArgs {
        state,
        shared_store,
        shared_tw_data,
        ..
      } = args;

      self.create_components_populate_registry_init_focus().await;
      self
        .create_main_container(surface, state, shared_store, shared_tw_data)
        .await
    }
  }
}

// Handle component registry.
mod construct_components {
  use super::*;

  impl AppWithLayout {
    pub async fn create_components_populate_registry_init_focus(&mut self) {
      let editor_id = &Id::Editor.to_string();

      // Insert editor component into registry.
      if self.component_registry.id_does_not_exist(editor_id) {
        let shared_editor_component = {
          let on_buffer_change: OnEditorBufferChangeFn<_, _> = |shared_store, my_id, buffer| {
            spawn_dispatch_action!(shared_store, Action::UpdateEditorBuffer(my_id, buffer));
          };
          let config_options = EditorEngineConfigOptions::default();
          EditorComponent::new_shared(editor_id, config_options, on_buffer_change)
        };

        self.component_registry.put(editor_id, shared_editor_component);

        call_if_true!(DEBUG_TUI_MOD, {
          log_no_err!(DEBUG, "🪙 {}", "construct EditorComponent { on_buffer_change }");
        });
      }

      // Init has focus.
      if self.component_registry.has_focus.get_id().is_none() {
        self.component_registry.has_focus.set_id(editor_id);
        call_if_true!(DEBUG_TUI_MOD, {
          log_no_err!(DEBUG, "🪙 {} = {}", "init component_registry.has_focus", editor_id);
        });
      }
    }

    /// Main container CONTAINER_ID.
    pub async fn create_main_container(
      &mut self,
      surface: &mut Surface,
      state: &State,
      shared_store: &SharedStore<State, Action>,
      shared_tw_data: &SharedTWData,
    ) -> CommonResult<()> {
      let editor_id = &Id::Editor.to_string();

      throws!({
        box_start_with_component! {
          in:                     surface,
          id:                     editor_id,
          dir:                    Direction::Vertical,
          requested_size_percent: requested_size_percent!(width: 100, height: 100),
          styles:                 [editor_id],
          render: {
            from:           self.component_registry,
            state:          state,
            shared_store:   shared_store,
            shared_tw_data: shared_tw_data
          }
        }
      });
    }
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

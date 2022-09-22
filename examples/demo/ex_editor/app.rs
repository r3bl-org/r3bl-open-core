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

use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use r3bl_rs_utils::*;
use tokio::sync::RwLock;

use super::*;

// Constants for the ids.
const CONTAINER_ID: &str = "container";
const EDITOR_ID: &str = "editor";

/// Async trait object that implements the [TWApp] trait.
#[derive(Default)]
pub struct AppWithLayout {
  pub component_registry: ComponentRegistry<State, Action>,
}

mod app_impl {
  use super::*;

  #[async_trait]
  impl App<State, Action> for AppWithLayout {
    async fn app_handle_event(
      &mut self, input_event: &InputEvent, state: &State,
      shared_store: &SharedStore<State, Action>, _terminal_size: Size,
      shared_tw_data: &SharedTWData,
    ) -> CommonResult<EventPropagation> {
      route_event_to_focused_component!(
        registry:       self.component_registry,
        has_focus:      self.has_focus,
        input_event:    input_event,
        state:          state,
        shared_store:   shared_store,
        shared_tw_data: shared_tw_data
      )
    }

    async fn app_render(
      &mut self, state: &State, shared_store: &SharedStore<State, Action>,
      shared_tw_data: &SharedTWData,
    ) -> CommonResult<RenderPipeline> {
      throws_with_return!({
        let window_size = shared_tw_data.read().await.get_size();
        let adjusted_window_size = size!(col: window_size.col, row: window_size.row - 1);

        // Render container component.
        let mut surface = surface_start_with_runnable! {
          runnable:       self,
          stylesheet:     style_helpers::create_stylesheet()?,
          pos:            position!(col:0, row:0),
          size:           adjusted_window_size, // Bottom row for status bar.
          state:          state,
          shared_store:   shared_store,
          shared_tw_data: shared_tw_data
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
      &mut self, surface: &mut Surface, state: &State, shared_store: &SharedStore<State, Action>,
      shared_tw_data: &SharedTWData,
    ) -> CommonResult<()> {
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
      let _component = EditorComponent::new(EDITOR_ID);
      let shared_component_r1 = Arc::new(RwLock::new(_component));

      // Construct EDITOR_ID component.
      if self.component_registry.id_does_not_exist(EDITOR_ID) {
        self.component_registry.put(EDITOR_ID, shared_component_r1);
      }

      // Init has focus.
      if self.component_registry.has_focus.get_id().is_none() {
        self.component_registry.has_focus.set_id(EDITOR_ID);
      }
    }

    /// Main container CONTAINER_ID.
    pub async fn create_main_container(
      &mut self, surface: &mut Surface, state: &State, shared_store: &SharedStore<State, Action>,
      shared_tw_data: &SharedTWData,
    ) -> CommonResult<()> {
      throws!({
        box_start_with_component! {
          in:                     surface,
          id:                     EDITOR_ID,
          dir:                    Direction::Vertical,
          requested_size_percent: requested_size_percent!(width: 100, height: 100),
          styles:                 [EDITOR_ID],
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
    throws_with_return!({
      stylesheet! {
        style! {
          id: CONTAINER_ID
        },
        style! {
          id: EDITOR_ID
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
  pub fn render(render_pipeline: &mut RenderPipeline, size: Size) {
    let st_vec = styled_texts! {
      styled_text! { "Hints:",               style!(attrib: [dim])       },
      styled_text! { " Ctrl + x : Exit ⛔ ", style!(attrib: [bold])      },
      styled_text! { " … ",                  style!(attrib: [dim])       },
      styled_text! { " char : add ",         style!(attrib: [underline]) },
      styled_text! { " … ",                  style!(attrib: [dim])       },
      styled_text! { " TK / TK : TK ",        style!(attrib: [underline]) }
    };

    let display_width = st_vec.display_width();
    let col_center: ChUnit = (size.col - display_width) / 2;
    let row_bottom: ChUnit = size.row - 1;
    let center: Position = position!(col: col_center, row: row_bottom);

    *render_pipeline += (ZOrder::Normal, RenderOp::MoveCursorPositionAbs(center));
    *render_pipeline += st_vec.render(ZOrder::Normal);
  }
}

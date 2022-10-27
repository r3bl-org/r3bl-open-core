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
use int_enum::IntEnum;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use r3bl_tui::*;
use tokio::sync::RwLock;

use super::*;

// Constants for the ids.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntEnum)]
pub enum Id {
  Container = 1,
  Col1 = 2,
  Col2 = 3,
}

/// Async trait object that implements the [TWApp] trait.
#[derive(Default)]
pub struct AppWithLayout {
  pub component_registry: ComponentRegistry<State, Action>,
}

mod app_trait_impl {
  use super::*;

  #[async_trait]
  impl App<State, Action> for AppWithLayout {
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

      // Try to handle left and right arrow key input events & return if handled.
      if let Continuation::Return = self.handle_focus_switch(input_event) {
        return Ok(EventPropagation::ConsumedRerender);
      }

      // Route any unhandled event to the component that has focus.
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

    async fn app_render(&mut self, args: GlobalScopeArgs<'_, State, Action>) -> CommonResult<RenderPipeline> {
      throws_with_return!({
        let GlobalScopeArgs {
          state,
          shared_store,
          shared_tw_data,
          window_size,
        } = args;

        // Render container component.
        let mut surface = surface_start_with_surface_renderer! {
          surface_renderer: self,
          stylesheet:       style_helpers::create_stylesheet()?,
          pos:              position!(col:0, row:0),
          size:             size!(cols: window_size.cols, rows: window_size.rows - 1), // Bottom row for status bar.
          state:            state,
          shared_store:     shared_store,
          shared_tw_data:   shared_tw_data,
          window_size:      window_size
        };

        // Render status bar.
        status_bar_helpers::render(&mut surface.render_pipeline, window_size);

        // Return RenderOps pipeline (which will actually be painted elsewhere).
        surface.render_pipeline
      });
    }
  }

  #[async_trait]
  impl SurfaceRenderer<State, Action> for AppWithLayout {
    async fn render_in_surface(
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

      self.init().await;
      self
        .create_main_container(surface, state, shared_store, shared_tw_data, window_size)
        .await
    }
  }
}

// Handle focus.
mod handle_focus {
  use super::*;

  impl AppWithLayout {
    pub fn handle_focus_switch(&mut self, input_event: &InputEvent) -> Continuation<String> {
      let mut event_consumed = false;

      // Handle Left, Right to switch focus between columns.
      if let InputEvent::Keyboard(keypress) = input_event {
        match keypress {
          KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Left),
          } => {
            event_consumed = true;
            self.switch_focus(SpecialKey::Left);
            debug_log_has_focus(
              stringify!(AppWithLayout::app_handle_event).into(),
              &self.component_registry.has_focus,
            );
          }
          KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Right),
          } => {
            event_consumed = true;
            self.switch_focus(SpecialKey::Right);
            debug_log_has_focus(
              stringify!(AppWithLayout::app_handle_event).into(),
              &self.component_registry.has_focus,
            );
          }
          _ => {}
        }
      }

      if event_consumed {
        Continuation::Return
      } else {
        Continuation::Continue
      }
    }

    fn switch_focus(&mut self, special_key: SpecialKey) {
      if let Some(_id) = self.component_registry.has_focus.get_id() {
        if special_key == SpecialKey::Left {
          self.component_registry.has_focus.set_id(Id::Col1.int_value())
        } else {
          self.component_registry.has_focus.set_id(Id::Col2.int_value())
        }
      } else {
        log_no_err!(ERROR, "No focus id has been set, and it should be set!");
      }
    }
  }
}

// Handle component registry.
mod component_registry {
  use super::*;

  impl AppWithLayout {
    pub async fn init(&mut self) {
      // Construct COL_1_ID.
      let col1_id = Id::Col1.int_value();
      if self.component_registry.does_not_contain(col1_id) {
        let component = ColumnRenderComponent::new(col1_id);
        let shared_component = Arc::new(RwLock::new(component));
        self.component_registry.put(col1_id, shared_component);
      }

      // Construct COL_2_ID.
      let col2_id = Id::Col2.int_value();
      if self.component_registry.does_not_contain(col2_id) {
        let component = ColumnRenderComponent::new(col2_id);
        let shared_component = Arc::new(RwLock::new(component));
        self.component_registry.put(col2_id, shared_component);
      }

      // Init has focus.
      if self.component_registry.has_focus.get_id().is_none() {
        self.component_registry.has_focus.set_id(col1_id);
      }
    }
  }
}

mod container_layout_render {
  use super::*;

  impl AppWithLayout {
    /// Main container CONTAINER_ID.
    pub async fn create_main_container(
      &mut self,
      surface: &mut Surface,
      state: &State,
      shared_store: &SharedStore<State, Action>,
      shared_tw_data: &SharedTWData,
      window_size: &Size,
    ) -> CommonResult<()> {
      throws!({
        box_start_with_surface_renderer! {
          in:                     surface,
          surface_renderer:       container_layout_render::TwoColLayout { app_with_layout: self },
          id:                     Id::Container.int_value(),
          dir:                    Direction::Horizontal,
          requested_size_percent: requested_size_percent!(width: 100, height: 100),
          styles:                 [&Id::Container.int_value().to_string()],
          state:                  state,
          shared_store:           shared_store,
          shared_tw_data:         shared_tw_data,
          window_size:            window_size
        };
      });
    }
  }

  pub(crate) struct TwoColLayout<'a> {
    pub(crate) app_with_layout: &'a mut AppWithLayout,
  }

  #[async_trait]
  impl<'a> SurfaceRenderer<State, Action> for TwoColLayout<'a> {
    async fn render_in_surface(
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

      throws!({
        box_start_with_component! {
          in:                     surface,
          id:                     Id::Col1.int_value(),
          dir:                    Direction::Vertical,
          requested_size_percent: requested_size_percent!(width: 50, height: 100),
          styles:                 [&Id::Col1.int_value().to_string()],
          render: {
            from:           self.app_with_layout.component_registry,
            state:          state,
            shared_store:   shared_store,
            shared_tw_data: shared_tw_data,
            window_size:    window_size
          }
        }

        box_start_with_component! {
          in:                     surface,
          id:                     Id::Col2.int_value(),
          dir:                    Direction::Vertical,
          requested_size_percent: requested_size_percent!(width: 50, height: 100),
          styles:                 [&Id::Col2.int_value().to_string()],
          render: {
            from:           self.app_with_layout.component_registry,
            state:          state,
            shared_store:   shared_store,
            shared_tw_data: shared_tw_data,
            window_size:    window_size
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
          id: Id::Container.int_value().to_string()
          padding: 1
        },
        style! {
          id: Id::Col1.int_value().to_string()
          padding: 1
          color_bg: TWColor::Rgb { r: 55, g: 55, b: 100 }
        },
        style! {
          id: Id::Col2.int_value().to_string()
          padding: 1
          color_bg: TWColor::Rgb { r: 55, g: 55, b: 248 }
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
      styled_text! { "Hints:",          style!(attrib: [dim])       },
      styled_text! { " x : Exit ⛔ ",   style!(attrib: [bold])      },
      styled_text! { " … ",             style!(attrib: [dim])       },
      styled_text! { " ↑ / + : inc ",   style!(attrib: [underline]) },
      styled_text! { " … ",             style!(attrib: [dim])       },
      styled_text! { " ↓ / - : dec ",   style!(attrib: [underline]) },
      styled_text! { " … ",             style!(attrib: [dim])       },
      styled_text! { " ← / → : focus ", style!(attrib: [underline]) }
    };

    let display_width = st_vec.display_width();
    let col_center: ChUnit = (size.cols - display_width) / 2;
    let row_bottom: ChUnit = size.rows - 1;
    let center: Position = position!(col: col_center, row: row_bottom);

    *render_pipeline += (ZOrder::Normal, RenderOp::MoveCursorPositionAbs(center));
    *render_pipeline += st_vec.render(ZOrder::Normal);
  }
}

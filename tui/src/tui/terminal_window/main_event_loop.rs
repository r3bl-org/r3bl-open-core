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

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use async_trait::async_trait;
use int_enum::IntEnum;
use r3bl_redux::*;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;
use tokio::sync::RwLock;

use crate::*;

/// Controls whether input events are process by spawning a new task or by blocking the current task.
const SPAWN_PROCESS_INPUT: bool = true;

/// These are global state values for the entire application:
/// - The size holds the width and height of the terminal window.
/// - The cursor_position (for purposes of drawing via [RenderOp], [RenderOps], and
///   [RenderPipeline]).
///   1. This is used for low level painting operations and are not meant to be used by code that
///      renders components.
///   2. The contract that must be respected is that the cursor position is only valid inside a
///      given [RenderOps] list.
///   3. So when you create a [RenderPipeline] and populate it w/ [RenderOps] you can use the
///      following ops [RenderOp::MoveCursorPositionAbs], [RenderOp::MoveCursorPositionRelTo] And
///      they will be valid only for that [RenderOps] list.
#[derive(Clone, Debug, Default)]
pub struct GlobalData {
  pub size: Size,
  pub cursor_position: Position,
  pub maybe_render_pipeline: Option<RenderPipeline>,
  // FUTURE: 🐵 use global_user_data (contains key: String, value: HashMap<String, String>).
  pub global_user_data: HashMap<String, HashMap<String, String>>,
}

impl GlobalData {
  fn try_to_create_instance() -> CommonResult<GlobalData> {
    let mut global_data = GlobalData::default();
    global_data.set_size(terminal_lib_operations::lookup_size()?);
    Ok(global_data)
  }

  pub fn set_size(&mut self, new_size: Size) {
    self.size = new_size;
    self.dump_state_to_log("main_event_loop -> Resize");
  }

  pub fn get_size(&self) -> Size { self.size }

  pub fn dump_state_to_log(&self, msg: &str) {
    call_if_true!(DEBUG_TUI_MOD, log_no_err!(INFO, "{} -> {:?}", msg, self));
  }
}

pub struct TerminalWindow;

impl TerminalWindow {
  /// The where clause needs to match up w/ the trait bounds for [Store].
  ///
  /// ```ignore
  /// where
  /// S: Default + Clone + PartialEq + Debug + Sync + Send,
  /// A: Default + Clone + Sync + Send,
  /// ```
  pub async fn main_event_loop<S, A>(
    shared_app: SharedApp<S, A>,
    store: Store<S, A>,
    exit_keys: Vec<InputEvent>,
  ) -> CommonResult<()>
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send + 'static,
    A: Default + Clone + Sync + Send + 'static,
  {
    // Initialize the terminal window data struct.
    let _global_data = GlobalData::try_to_create_instance()?;
    let shared_global_data: SharedGlobalData = Arc::new(RwLock::new(_global_data));

    // Start raw mode.
    let my_raw_mode = RawMode::start(&shared_global_data).await;

    // Move the store into an Arc & RwLock.
    let shared_store: SharedStore<S, A> = Arc::new(RwLock::new(store));

    // Create a subscriber (AppManager) & attach it to the store.
    let _subscriber = AppManager::new_box(&shared_app, &shared_store, &shared_global_data);
    shared_store.write().await.add_subscriber(_subscriber).await;

    // Create a new event stream (async).
    let mut async_event_stream = AsyncEventStream::default();

    // Perform first render.
    AppManager::render_app(&shared_store, &shared_app, &shared_global_data, None).await?;

    shared_global_data
      .read()
      .await
      .dump_state_to_log("main_event_loop -> Startup 🚀");

    // Main event loop.
    loop {
      // Try and get the next event if available (asynchronously).
      let maybe_input_event = async_event_stream.try_to_get_input_event().await;

      // Process the input_event.
      let input_event = match maybe_input_event {
        Some(it) => it,
        _ => continue,
      };
      call_if_true!(
        DEBUG_TUI_MOD,
        log_no_err!(INFO, "main_event_loop -> Tick: ⏰ {}", input_event)
      );

      // Before any app gets to process the input_event, perform special handling in case it is a
      // resize event. Even if TerminalWindow::process_input_event consumes the event, if the event
      // is a resize event, then we still need to update the size of the terminal window. It also
      // needs to be re-rendered.
      if let InputEvent::Resize(new_size) = input_event {
        shared_global_data.write().await.set_size(new_size);
        shared_global_data.write().await.maybe_render_pipeline = None;
        AppManager::render_app(&shared_store, &shared_app, &shared_global_data, None).await?;
      }

      // Pass the input_event to the app for processing.
      let propagation_result_from_app =
        TerminalWindow::process_input_event(&shared_global_data, &shared_store, &shared_app, &input_event).await?;

      // If event not consumed by app, propagate to the default input handler.
      match propagation_result_from_app {
        EventPropagation::Propagate => {
          if let Continuation::Exit = DefaultInputEventHandler::no_consume(input_event, &exit_keys).await {
            break;
          };
        }
        EventPropagation::ConsumedRerender => {
          AppManager::render_app(&shared_store, &shared_app, &shared_global_data, None).await?;
        }
        EventPropagation::Consumed => {}
      }
    }

    // End raw mode.
    my_raw_mode.end(&shared_global_data).await;

    Ok(())
  }

  async fn process_input_event<S, A>(
    shared_global_data: &SharedGlobalData,
    shared_store: &SharedStore<S, A>,
    shared_app: &SharedApp<S, A>,
    input_event: &InputEvent,
  ) -> CommonResult<EventPropagation>
  where
    S: Default + Clone + PartialEq + Debug + Sync + Send + 'static,
    A: Default + Clone + Sync + Send + 'static,
  {
    let propagation_result_from_app = match SPAWN_PROCESS_INPUT {
      true => {
        // Tokio spawn.
        let propagation_result_from_app = {
          let shared_global_data_clone = shared_global_data.clone();
          let shared_store_clone = shared_store.clone();
          let shared_app_clone = shared_app.clone();
          let input_event_clone = input_event.clone();
          let join_handle = tokio::spawn(async move {
            AppManager::route_input_to_app(
              &shared_global_data_clone,
              &shared_store_clone,
              &shared_app_clone,
              &input_event_clone,
            )
            .await
          });
          join_handle.await??
        };
        call_if_true!(
          DEBUG_TUI_MOD,
          log_no_err!(
            INFO,
            "main_event_loop -> 🚥 SPAWN propagation_result_from_app: {:?}",
            propagation_result_from_app
          )
        );
        propagation_result_from_app
      }
      false => {
        // Blocking call.
        let propagation_result_from_app =
          AppManager::route_input_to_app(shared_global_data, shared_store, shared_app, input_event).await?;
        call_if_true!(
          DEBUG_TUI_MOD,
          log_no_err!(
            INFO,
            "main_event_loop -> 🚥 NO_SPAWN propagation_result_from_app: {:?}",
            propagation_result_from_app
          )
        );
        propagation_result_from_app
      }
    };
    Ok(propagation_result_from_app)
  }
}

struct AppManager<S, A>
where
  S: Default + Clone + PartialEq + Debug + Sync + Send + 'static,
  A: Default + Clone + Sync + Send + 'static,
{
  shared_app: SharedApp<S, A>,
  shared_store: SharedStore<S, A>,
  shared_global_data: SharedGlobalData,
}

#[async_trait]
impl<S, A> AsyncSubscriber<S> for AppManager<S, A>
where
  S: Default + Clone + PartialEq + Debug + Sync + Send + 'static,
  A: Default + Clone + Sync + Send,
{
  async fn run(&self, my_state: S) {
    let result = AppManager::render_app(
      &self.shared_store,
      &self.shared_app,
      &self.shared_global_data,
      my_state.into(),
    )
    .await;
    if let Err(e) = result {
      call_if_true!(DEBUG_TUI_MOD, log_no_err!(ERROR, "MySubscriber::run -> Error: {}", e))
    }
  }
}

impl<S, A> AppManager<S, A>
where
  S: Default + Clone + PartialEq + Debug + Sync + Send + 'static,
  A: Default + Clone + Sync + Send,
{
  fn new_box(
    shared_app: &SharedApp<S, A>,
    shared_store: &SharedStore<S, A>,
    shared_global_data: &SharedGlobalData,
  ) -> Box<Self> {
    Box::new(AppManager {
      shared_app: shared_app.clone(),
      shared_store: shared_store.clone(),
      shared_global_data: shared_global_data.clone(),
    })
  }

  /// Pass the event to the `shared_app` for further processing.
  pub async fn route_input_to_app(
    shared_global_data: &SharedGlobalData,
    shared_store: &SharedStore<S, A>,
    shared_app: &SharedApp<S, A>,
    input_event: &InputEvent,
  ) -> CommonResult<EventPropagation> {
    throws_with_return!({
      // Create global scope args.
      let state = shared_store.read().await.get_state();
      let window_size = shared_global_data.read().await.get_size();
      let global_scope_args = GlobalScopeArgs {
        shared_global_data,
        shared_store,
        state: &state,
        window_size: &window_size,
      };

      // Call app_handle_event.
      shared_app
        .write()
        .await
        .app_handle_event(global_scope_args, input_event)
        .await?
    });
  }

  pub async fn render_app(
    shared_store: &SharedStore<S, A>,
    shared_app: &SharedApp<S, A>,
    shared_global_data: &SharedGlobalData,
    maybe_state: Option<S>,
  ) -> CommonResult<()> {
    throws!({
      // Create global scope args.
      let window_size = shared_global_data.read().await.get_size();
      let state: S = if let Some(state) = maybe_state {
        state
      } else {
        shared_store.read().await.get_state()
      };
      let global_scope_args = GlobalScopeArgs {
        state: &state,
        shared_store,
        shared_global_data,
        window_size: &window_size,
      };

      // Check to see if the window_size is large enough to render.
      let render_result: CommonResult<RenderPipeline> =
        if window_size.is_too_small_to_display(MinSize::Col.int_value(), MinSize::Row.int_value()) {
          shared_global_data.write().await.maybe_render_pipeline = None;
          Ok(render_window_size_too_small(window_size))
        } else {
          // Call app_render.
          shared_app.write().await.app_render(global_scope_args).await
        };

      match render_result {
        Err(error) => {
          RenderOp::default().flush();
          call_if_true!(
            DEBUG_TUI_MOD,
            log_no_err!(ERROR, "MySubscriber::render() error ❌: {}", error)
          );
        }
        Ok(render_pipeline) => {
          render_pipeline
            .paint(FlushKind::ClearBeforeFlush, shared_global_data)
            .await;
          call_if_true!(DEBUG_TUI_MOD, {
            log_no_err!(
              INFO,
              "🎨 MySubscriber::paint() ok ✅: \n size: {:?}\n state: {:?}\n",
              window_size,
              state,
            );
          });
        }
      }
    });
  }
}

fn render_window_size_too_small(window_size: Size) -> RenderPipeline {
  // Show warning message that window_size is too small.
  let display_msg = UnicodeString::from(format!(
    "Window size is too small. Minimum size is {} cols x {} rows",
    MinSize::Col.int_value(),
    MinSize::Row.int_value()
  ));
  let trunc_display_msg = UnicodeString::from(display_msg.truncate_to_fit_size(window_size));
  let trunc_display_msg_len = ch!(trunc_display_msg.len());

  let row_pos = window_size.rows / 2;
  let col_pos = (window_size.cols - trunc_display_msg_len) / 2;

  render_pipeline!(@new ZOrder::Normal =>
    RenderOp::ResetColor,
    RenderOp::MoveCursorPositionAbs(position! {col: col_pos, row: row_pos}),
    RenderOp::SetFgColor(TuiColor::DarkRed),
    RenderOp::PrintTextWithAttributes(
      lolcat_each_char_in_unicode_string(&trunc_display_msg, None),
      Some(style! {attrib: [bold]}))
  )
}

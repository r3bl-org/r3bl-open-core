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

use std::{fmt::{Debug, Display},
          sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::*;

/// These are global state values for the entire application:
/// 1. The size holds the width and height of the terminal window.
/// 2. The cursor_position (for purposes of drawing via [RenderOp], [RenderOps], and
///    [RenderPipeline]). This is used for low level painting operations and are not meant to be
///    used by code that renders components.
#[derive(Clone, Debug, Default)]
pub struct TWData {
  pub size: Size,
  pub cursor_position: Position,
}

impl TWData {
  fn try_to_create_instance() -> CommonResult<TWData> {
    let mut tw_data = TWData::default();
    tw_data.set_size(terminal_lib_operations::lookup_size()?);
    Ok(tw_data)
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
    store: Store<S, A>, shared_app: SharedApp<S, A>, exit_keys: Vec<InputEvent>,
  ) -> CommonResult<()>
  where
    S: Display + Default + Clone + PartialEq + Debug + Sync + Send + 'static,
    A: Display + Default + Clone + Sync + Send + 'static,
  {
    {
      // Initialize the terminal window data struct.
      let _tw_data = TWData::try_to_create_instance()?;
      let shared_tw_data: SharedTWData = Arc::new(RwLock::new(_tw_data));

      // Start raw mode.
      let my_raw_mode = RawMode::start(&shared_tw_data).await;

      // Move the store into an Arc & RwLock.
      let shared_store: SharedStore<S, A> = Arc::new(RwLock::new(store));

      // Create a subscriber (AppManager) & attach it to the store.
      let _subscriber = AppManager::new_box(&shared_app, &shared_store, &shared_tw_data);
      shared_store.write().await.add_subscriber(_subscriber).await;

      // Create a new event stream (async).
      let mut async_event_stream = AsyncEventStream::default();

      // Perform first render.
      AppManager::render_app(&shared_store, &shared_app, &shared_tw_data, None).await?;

      shared_tw_data
        .read()
        .await
        .dump_state_to_log("main_event_loop -> Startup 🚀");

      // Main event loop.
      loop {
        // Try and get the next event if available (asynchronously).
        let maybe_input_event = async_event_stream.try_to_get_input_event().await;

        // Process the input_event.
        if let Some(input_event) = maybe_input_event {
          call_if_true!(
            DEBUG_TUI_MOD,
            log_no_err!(INFO, "main_event_loop -> Tick: ⏰ {}", input_event)
          );

          // Pass event to the app first. It has greater specificity than the default handler.
          let propagation_result_from_app = AppManager::route_input_to_app(
            &shared_tw_data,
            &shared_store,
            &shared_app,
            &input_event,
          )
          .await?;

          // If event not consumed by app, propagate to the default input handler.
          match propagation_result_from_app {
            EventPropagation::ConsumedRerender => {
              AppManager::render_app(&shared_store, &shared_app, &shared_tw_data, None).await?;
            }
            EventPropagation::Propagate => {
              let continuation_result_from_default_handler =
                DefaultInputEventHandler::no_consume(input_event, &exit_keys).await;
              match continuation_result_from_default_handler {
                Continuation::Exit => {
                  break;
                }
                Continuation::ResizeAndContinue(new_size) => {
                  shared_tw_data.write().await.set_size(new_size);
                  AppManager::render_app(&shared_store, &shared_app, &shared_tw_data, None).await?;
                }
                _ => {}
              };
            }
            EventPropagation::Consumed => {}
          }
        }
      }

      // End raw mode.
      my_raw_mode.end(&shared_tw_data).await;

      Ok(())
    }
  }
}

struct AppManager<S, A>
where
  S: Display + Default + Clone + PartialEq + Debug + Sync + Send + 'static,
  A: Display + Default + Clone + Sync + Send + 'static,
{
  shared_app: SharedApp<S, A>,
  shared_store: SharedStore<S, A>,
  shared_tw_data: SharedTWData,
}

#[async_trait]
impl<S, A> AsyncSubscriber<S> for AppManager<S, A>
where
  S: Display + Default + Clone + PartialEq + Debug + Sync + Send + 'static,
  A: Display + Default + Clone + Sync + Send,
{
  async fn run(&self, my_state: S) {
    let result = AppManager::render_app(
      &self.shared_store,
      &self.shared_app,
      &self.shared_tw_data,
      my_state.into(),
    )
    .await;
    if let Err(e) = result {
      call_if_true!(
        DEBUG_TUI_MOD,
        log_no_err!(ERROR, "MySubscriber::run -> Error: {}", e)
      )
    }
  }
}

impl<S, A> AppManager<S, A>
where
  S: Display + Default + Clone + PartialEq + Debug + Sync + Send + 'static,
  A: Display + Default + Clone + Sync + Send,
{
  fn new_box(
    shared_app: &SharedApp<S, A>, shared_store: &SharedStore<S, A>, shared_window: &SharedTWData,
  ) -> Box<Self> {
    Box::new(AppManager {
      shared_app: shared_app.clone(),
      shared_store: shared_store.clone(),
      shared_tw_data: shared_window.clone(),
    })
  }

  /// Pass the event to the `shared_app` for further processing.
  pub async fn route_input_to_app(
    shared_window: &SharedTWData, shared_store: &SharedStore<S, A>, shared_app: &SharedApp<S, A>,
    input_event: &InputEvent,
  ) -> CommonResult<EventPropagation> {
    throws_with_return!({
      let latest_state = shared_store.read().await.get_state();
      let window_size = shared_window.read().await.get_size();
      shared_app
        .write()
        .await
        .app_handle_event(input_event, &latest_state, shared_store, window_size)
        .await?
    });
  }

  pub async fn render_app(
    shared_store: &SharedStore<S, A>, shared_app: &SharedApp<S, A>, shared_tw_data: &SharedTWData,
    maybe_state: Option<S>,
  ) -> CommonResult<()> {
    throws!({
      let state: S = if maybe_state.is_none() {
        shared_store.read().await.get_state()
      } else {
        maybe_state.unwrap()
      };

      let render_result = shared_app
        .write()
        .await
        .app_render(&state, shared_store, shared_tw_data)
        .await;

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
            .paint(FlushKind::ClearBeforeFlush, shared_tw_data)
            .await;
          let window_size = shared_tw_data.read().await.get_size();
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

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
use crossterm::{event::*, terminal::*};
use tokio::sync::RwLock;

use crate::*;

#[derive(Clone, Debug)]
pub struct TWData {
  pub size: Size,
}

/// Create a new [Size] from [crossterm::terminal::size()].
pub fn try_to_get_from_crossterm_terminal() -> CommonResult<Size> {
  let size: Size = size()?.into();
  Ok(size)
}

impl TWData {
  fn try_to_create_instance() -> CommonResult<TWData> {
    Ok(TWData {
      size: try_to_get_from_crossterm_terminal()?,
    })
  }

  pub fn dump_state_to_log(&self, msg: &str) {
    call_if_true!(DEBUG, log_no_err!(INFO, "{} -> {:?}", msg, self));
  }

  pub fn set_size(&mut self, new_size: Size) {
    self.size = new_size;
    self.dump_state_to_log("main_event_loop -> Resize");
  }
}

pub struct TerminalWindow;

impl TerminalWindow {
  /// The where clause needs to match up w/ the trait bounds for [Store].
  ///
  /// ```ignore
  /// where
  /// S: Default + Clone + PartialEq + Eq + Debug + Sync + Send,
  /// A: Default + Clone + Sync + Send,
  /// ```
  pub async fn main_event_loop<S, A>(
    store: Store<S, A>, shared_app: SharedTWApp<S, A>, exit_keys: Vec<TWInputEvent>,
  ) -> CommonResult<()>
  where
    S: Display + Default + Clone + PartialEq + Eq + Debug + Sync + Send + 'static,
    A: Display + Default + Clone + Sync + Send + 'static,
  {
    raw_mode!({
      // Initialize the terminal window data struct.
      let _tw_data = TWData::try_to_create_instance()?;
      let shared_window: SharedWindow = Arc::new(RwLock::new(_tw_data));

      // Move the store into an Arc & RwLock.
      let shared_store: SharedStore<S, A> = Arc::new(RwLock::new(store));

      // Create a subscriber (AppManager) & attach it to the store.
      let _subscriber = AppManager::new_box(&shared_app, &shared_store, &shared_window);
      shared_store.write().await.add_subscriber(_subscriber).await;

      // Create a new event stream (async).
      let mut stream = EventStream::new();

      // Perform first render.
      AppManager::render_app(
        &shared_store,
        &shared_app,
        shared_window.read().await.size,
        None,
      )
      .await?;

      shared_window
        .read()
        .await
        .dump_state_to_log("main_event_loop -> Startup 🚀");

      // Main event loop.
      loop {
        // Try and get the next event if available (asynchronously).
        let maybe_input_event = stream.try_to_get_input_event().await;

        // Process the input_event.
        if let Some(input_event) = maybe_input_event {
          call_if_true!(
            DEBUG,
            log_no_err!(INFO, "main_event_loop -> Tick: ⏰ {}", input_event)
          );

          // Pass event to the app first. It has greater specificity than the default handler.
          let propagation_result_from_app = AppManager::route_input_to_app(
            &shared_window,
            &shared_store,
            &shared_app,
            &input_event,
          )
          .await?;

          // If event not consumed by app, propagate to the default input handler.
          match propagation_result_from_app {
            EventPropagation::ConsumedRerender => {
              let size = shared_window.read().await.size;
              AppManager::render_app(&shared_store, &shared_app, size, None).await?;
            }
            EventPropagation::Propagate => {
              let continuation_result_from_default_handler =
                TWDefaultInputEventHandler::no_consume(input_event, &exit_keys).await;
              match continuation_result_from_default_handler {
                Continuation::Exit => {
                  break;
                }
                Continuation::ResizeAndContinue(new_size) => {
                  shared_window.write().await.set_size(new_size);
                  AppManager::render_app(&shared_store, &shared_app, new_size, None).await?;
                }
                _ => {}
              };
            }
            EventPropagation::Consumed => {}
          }
        }

        // Flush.
        TWCommand::flush();
      }
    })
  }
}

struct AppManager<S, A>
where
  S: Display + Default + Clone + PartialEq + Eq + Debug + Sync + Send + 'static,
  A: Display + Default + Clone + Sync + Send + 'static,
{
  shared_app: SharedTWApp<S, A>,
  shared_store: SharedStore<S, A>,
  shared_window: SharedWindow,
}

#[async_trait]
impl<S, A> AsyncSubscriber<S> for AppManager<S, A>
where
  S: Display + Default + Clone + PartialEq + Eq + Debug + Sync + Send + 'static,
  A: Display + Default + Clone + Sync + Send,
{
  async fn run(&self, my_state: S) {
    let window_size = self.shared_window.read().await.size;
    let result = AppManager::render_app(
      &self.shared_store,
      &self.shared_app,
      window_size,
      my_state.into(),
    )
    .await;
    if let Err(e) = result {
      call_if_true!(
        DEBUG,
        log_no_err!(ERROR, "MySubscriber::run -> Error: {}", e)
      )
    }
  }
}

impl<S, A> AppManager<S, A>
where
  S: Display + Default + Clone + PartialEq + Eq + Debug + Sync + Send + 'static,
  A: Display + Default + Clone + Sync + Send,
{
  fn new_box(
    shared_app: &SharedTWApp<S, A>, shared_store: &SharedStore<S, A>, shared_window: &SharedWindow,
  ) -> Box<Self> {
    Box::new(AppManager {
      shared_app: shared_app.clone(),
      shared_store: shared_store.clone(),
      shared_window: shared_window.clone(),
    })
  }

  /// Pass the event to the `shared_app` for further processing.
  pub async fn route_input_to_app(
    shared_window: &SharedWindow, shared_store: &SharedStore<S, A>, shared_app: &SharedTWApp<S, A>,
    input_event: &TWInputEvent,
  ) -> CommonResult<EventPropagation> {
    throws_with_return!({
      let latest_state = shared_store.read().await.get_state();
      let window_size = shared_window.read().await.size;
      shared_app
        .write()
        .await
        .app_handle_event(input_event, &latest_state, shared_store, window_size)
        .await?
    });
  }

  pub async fn render_app(
    shared_store: &SharedStore<S, A>, shared_app: &SharedTWApp<S, A>, window_size: Size,
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
        .app_render(&state, shared_store, window_size)
        .await;
      match render_result {
        Err(error) => {
          TWCommand::flush();
          call_if_true!(
            DEBUG,
            log_no_err!(ERROR, "MySubscriber::render() error ❌: {}", error)
          );
        }
        Ok(tw_command_queue) => {
          tw_command_queue.flush(true);
          call_if_true!(
            DEBUG,
            log_no_err!(
              INFO,
              "MySubscriber::render() ok ✅: {}, {}",
              window_size,
              state
            )
          );
        }
      }
    });
  }
}

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

use core::fmt::Debug;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{redux::{AsyncMiddlewareSpawnsVec,
                    AsyncMiddlewareVec,
                    AsyncReducerVec,
                    AsyncSubscriberVec},
            AsyncMiddleware,
            AsyncMiddlewareSpawns,
            AsyncReducer,
            AsyncSubscriber};

pub type SharedStore<S, A> = Arc<RwLock<Store<S, A>>>;

#[macro_export]
macro_rules! spawn_dispatch_action {
    ($store: expr, $action: expr) => {{
        let store_copy = $store.clone();
        tokio::spawn(async move {
            store_copy.write().await.dispatch_action($action).await;
        });
    }};
}

/// Thread safe and async Redux store (using [`tokio`]).
pub struct Store<S, A>
where
    S: Clone + Default + PartialEq + Debug + Sync + Send,
    A: Clone + Default + Send + Sync,
{
    pub state: S,
    pub history: Vec<S>,
    pub middleware_vec: AsyncMiddlewareVec<S, A>,
    pub middleware_spawns_vec: AsyncMiddlewareSpawnsVec<S, A>,
    pub subscriber_vec: AsyncSubscriberVec<S>,
    pub reducer_vec: AsyncReducerVec<S, A>,
    pub maybe_previous_state: Option<S>,
}

impl<S, A> Default for Store<S, A>
where
    S: Clone + Default + PartialEq + Debug + Sync + Send,
    A: Clone + Default + Send + Sync,
{
    fn default() -> Store<S, A> {
        Store {
            state: Default::default(),
            history: Default::default(),
            middleware_vec: Default::default(),
            middleware_spawns_vec: Default::default(),
            reducer_vec: Default::default(),
            subscriber_vec: Default::default(),
            maybe_previous_state: None,
        }
    }
}

// FUTURE: make history implementation more comprehensive (eg: max history size) & add tests.

// Handle subscriber, middleware, reducer management.
impl<S, A> Store<S, A>
where
    S: Clone + Default + PartialEq + Debug + Sync + Send,
    A: Clone + Default + Send + Sync,
{
    pub async fn add_subscriber(
        &mut self,
        subscriber_fn: Box<dyn AsyncSubscriber<S> + Send + Sync>,
    ) -> &mut Store<S, A> {
        self.subscriber_vec.push(subscriber_fn);
        self
    }

    pub async fn clear_subscribers(&mut self) -> &mut Store<S, A> {
        self.subscriber_vec.clear();
        self
    }

    pub async fn add_middleware(
        &mut self,
        middleware_fn: Box<dyn AsyncMiddleware<S, A> + Send + Sync>,
    ) -> &mut Store<S, A> {
        self.middleware_vec.push(middleware_fn);
        self
    }

    pub async fn add_middleware_spawns(
        &mut self,
        middleware_fn: Box<dyn AsyncMiddlewareSpawns<S, A> + Send + Sync>,
    ) -> &mut Store<S, A> {
        self.middleware_spawns_vec.push(middleware_fn);
        self
    }

    pub async fn clear_middlewares(&mut self) -> &mut Store<S, A> {
        self.middleware_vec.clear();
        self
    }

    pub async fn add_reducer(
        &mut self,
        reducer_fn: Box<dyn AsyncReducer<S, A> + Send + Sync>,
    ) -> &mut Store<S, A> {
        self.reducer_vec.push(reducer_fn);
        self
    }

    pub async fn clear_reducers(&mut self) -> &mut Store<S, A> {
        self.reducer_vec.clear();
        self
    }
}

// Handle dispatch & history.
impl<S, A> Store<S, A>
where
    S: Clone + Default + PartialEq + Debug + Sync + Send,
    A: Clone + Default + Send + Sync,
{
    pub fn get_state(&self) -> S { self.state.clone() }

    pub fn get_history(&self) -> Vec<S> { self.history.clone() }

    pub async fn dispatch_spawn(&'static mut self, action: A) {
        tokio::spawn(async move {
            self.dispatch_action(action).await;
        });
    }

    pub async fn dispatch_action(&mut self, action: A) {
        // Run middlewares.
        self.middleware_runner(action.clone()).await;

        // Dispatch the action.
        self.actually_dispatch_action(&action.clone()).await;
    }

    async fn actually_dispatch_action(&mut self, action: &A) {
        self.run_reducers(action).await;
        self.run_subscribers().await;
    }

    fn has_state_changed(&self) -> bool {
        if let Some(previous_state) = &self.maybe_previous_state {
            *previous_state != self.state
        } else {
            true
        }
    }

    fn save_state_to_previous_state(&mut self) {
        self.maybe_previous_state = Some(self.state.clone());
    }

    /// Run these in parallel.
    async fn run_subscribers(&mut self) {
        // Early return if state hasn't changed.
        if !self.has_state_changed() {
            return;
        }

        // Update previous state, for next time.
        self.save_state_to_previous_state();

        // Actually run the subscribers.
        let mut vec_fut = vec![];
        let state_clone = self.get_state();
        for fun in &self.subscriber_vec {
            vec_fut.push(fun.run(state_clone.clone()));
        }
        futures::future::join_all(vec_fut).await;
    }

    /// Run these in sequence.
    async fn run_reducers(&mut self, action: &A) {
        if self.reducer_vec.is_empty() {
            return;
        }
        for reducer in &self.reducer_vec {
            let new_state = reducer.run(action, &self.state).await;
            self.state = new_state;
        }
        self.update_history();
    }

    // Update history.
    fn update_history(&mut self) {
        let new_state = self.get_state();

        // Update history.
        let mut update_history = false;
        if self.history.is_empty() {
            update_history = true;
        } else if let Some(last_known_state) = self.history.last() {
            if *last_known_state != new_state {
                update_history = true;
            }
        }
        if update_history {
            self.history.push(new_state)
        };
    }

    /// Run these in parallel.
    pub async fn middleware_runner(&mut self, action: A) {
        self.run_middleware_vec(action.clone()).await;

        self.run_middleware_spawns_vec(action.clone()).await;
    }

    /// Run concurrently (cooperatively on a single thread).
    async fn run_middleware_vec(&mut self, my_action: A) {
        let mut vec_fut = vec![];

        for item in &self.middleware_vec {
            let value = item.run(my_action.clone(), self.get_state());
            vec_fut.push(value);
        }

        let vec_opt_action = futures::future::join_all(vec_fut).await;

        for action in vec_opt_action.into_iter().flatten() {
            self.actually_dispatch_action(&action).await;
        }
    }

    /// Run in parallel (on multiple threads, if using Tokio's multithreaded
    /// executor).
    async fn run_middleware_spawns_vec(&mut self, my_action: A) {
        let mut vec_join_handle = vec![];

        for item in &self.middleware_spawns_vec {
            let fut = item.run(my_action.clone(), self.get_state()).await;
            vec_join_handle.push(fut);
        }

        let vec_results = futures::future::join_all(vec_join_handle).await;

        for join_handle in vec_results {
            let result = join_handle;
            if let Ok(Some(action)) = result {
                self.actually_dispatch_action(&action).await;
            }
        }
    }
}

/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use super::{App, Component};

// App.
pub type SafeApp<S, AS> = dyn App<S = S, AS = AS> + Send + Sync;
pub type BoxedSafeApp<S, AS> = Box<SafeApp<S, AS>>;

// Component.
pub type SafeComponent<S, AS> = dyn Component<S, AS> + Send + Sync;
pub type BoxedSafeComponent<S, AS> = Box<SafeComponent<S, AS>>;

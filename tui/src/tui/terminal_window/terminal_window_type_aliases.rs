// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{App, Component};

// App.
pub type SafeApp<S, AS> = dyn App<S = S, AS = AS> + Send + Sync;
pub type BoxedSafeApp<S, AS> = Box<SafeApp<S, AS>>;

// Component.
pub type SafeComponent<S, AS> = dyn Component<S, AS> + Send + Sync;
pub type BoxedSafeComponent<S, AS> = Box<SafeComponent<S, AS>>;

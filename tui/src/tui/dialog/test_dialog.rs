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


pub mod mock_real_objects_for_dialog {
    use std::sync::Arc;

    use r3bl_redux::{SharedStore, Store};
    use tokio::sync::RwLock;

    use crate::{test_editor::mock_real_objects_for_editor, *};

    pub fn create_store() -> Arc<RwLock<Store<String, String>>> {
        let mut _store = Store::<String, String>::default();
        let shared_store: SharedStore<String, String> = Arc::new(RwLock::new(_store));
        shared_store
    }

    pub fn make_dialog_engine() -> DialogEngine {
        DialogEngine {
            editor_engine: mock_real_objects_for_editor::make_editor_engine(),
            ..Default::default()
        }
    }
}

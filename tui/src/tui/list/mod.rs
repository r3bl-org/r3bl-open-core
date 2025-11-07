/*
 *   Copyright (c) 2025 R3BL LLC
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

//! Reusable list component with trait-based items, multi-select, and batch operations.
//!
//! # Overview
//!
//! The `ListComponent` provides a flexible, trait-based approach to building list UIs in
//! terminal applications. Items implement the [`ListItem`] trait to define their rendering
//! and behavior, while the component handles navigation, selection, and event routing.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │  ListComponent<S, AS, I>                │
//! │  - Viewport management                   │
//! │  - Navigation (up/down/page/home/end)   │
//! │  - Selection tracking (single/multi)    │
//! │  - Event priority routing                │
//! └─────────────────────────────────────────┘
//!           │ delegates to
//!           ▼
//! ┌─────────────────────────────────────────┐
//! │  I: ListItem<S, AS>                     │
//! │  - Custom rendering                      │
//! │  - Event handling when focused           │
//! │  - Item-specific logic                   │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Event Priority System
//!
//! Events are handled in this order:
//! 1. **Navigation keys** (arrows, page up/down, home/end) - always handled by list
//! 2. **Multi-select toggle** (Space key) - when multi-select enabled
//! 3. **Batch actions** - when 2+ items selected
//! 4. **Item delegation** - when exactly 1 item selected
//! 5. **Propagate** - event not handled
//!
//! # Phase 1: Fixed-Height Items
//!
//! This initial implementation assumes all items are exactly 1 row tall, which
//! simplifies viewport calculations and scrolling. Phase 2 will add variable height support.

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

mod list_item_trait;
mod list_component_struct;
mod list_component_impl;

#[cfg(test)]
mod test_list_component;

pub use list_item_trait::*;
pub use list_component_struct::*;

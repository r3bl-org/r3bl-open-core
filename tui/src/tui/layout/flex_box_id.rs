/*
 *   Copyright (c) 2024 R3BL LLC
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
          ops::Deref};

use serde::{Deserialize, Serialize};

/// This works w/ the [int-enum](https://crates.io/crates/int-enum) crate in order to
/// allow for the definition of enums that are represented in memory as [u8]s.
#[derive(Default, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct FlexBoxId(pub u8);


impl From<FlexBoxId> for u8 {
    fn from(id: FlexBoxId) -> Self { id.0 }
}

impl From<u8> for FlexBoxId {
    fn from(id: u8) -> Self { Self(id) }
}

impl Deref for FlexBoxId {
    type Target = u8;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl FlexBoxId {
    fn pretty_print(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "🔑┆id: {}┆", self.0)
    }
}

impl Debug for FlexBoxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pretty_print(f)
    }
}

impl Display for FlexBoxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.pretty_print(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_box_id_default() {
        let id = FlexBoxId::default();
        assert_eq!(id.0, 0);
    }

    #[test]
    fn test_flex_box_id_from_u8() {
        let id = FlexBoxId::from(42u8);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_u8_from_flex_box_id() {
        let id = FlexBoxId(42);
        let value: u8 = id.into();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_flex_box_id_deref() {
        let id = FlexBoxId(42);
        assert_eq!(*id, 42);
    }

    #[test]
    fn test_flex_box_id_debug() {
        let id = FlexBoxId(42);
        assert_eq!(format!("{:?}", id), "🔑┆id: 42┆");
    }

    #[test]
    fn test_flex_box_id_display() {
        let id = FlexBoxId(42);
        assert_eq!(format!("{}", id), "🔑┆id: 42┆");
    }

    #[test]
    fn test_flex_box_id_serialize() {
        let id = FlexBoxId(42);
        let serialized = serde_json::to_string(&id).unwrap();
        assert_eq!(serialized, "42");
    }

    #[test]
    fn test_flex_box_id_deserialize() {
        let serialized = "42";
        let id: FlexBoxId = serde_json::from_str(serialized).unwrap();
        assert_eq!(id.0, 42);
    }
}

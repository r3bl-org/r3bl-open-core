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

use std::fmt::Debug;

use r3bl_rs_utils_core::*;

use crate::*;

/// This is a global (scoped to an [App]) struct that is used to store the `id` of the [FlexBox]
/// that has keyboard focus.
///
/// There are 2 types of keyboard focus:
/// 1. Non modal focus - This is just a single `id` that is stored. To change focus a new `id` is
///    set in its place. Internally a `Vec` of capacity 2 is used to store this and the modal `id`.
/// 2. Modal focus - There can only be one modal at a time. When a modal is active, the `id` of the
///    [FlexBox] that had focus before the modal was activated is saved. When the modal is closed,
///    the `id` of the [FlexBox] that had focus before the modal was activated is restored.
///
/// ## Modal `id`, which is used by modal dialog box
///
/// 1. Only one modal can be active at any time.
/// 2. When a modal is active, the `id` of the [FlexBox] that had focus before the modal was
///    activated is saved.
/// 3. When the modal is closed, the `id` of the [FlexBox] that had focus before the modal was
///    activated is restored.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HasFocus {
    /// This `id` has keyboard focus. This is global.
    id_vec: Vec<FlexBoxId>,
}

impl Default for HasFocus {
    fn default() -> Self {
        Self {
            id_vec: Vec::with_capacity(2),
        }
    }
}

impl HasFocus {
    /// Check to see whether [set_id][HasFocus::set_id] has been called.
    pub fn is_empty(&self) -> bool { self.id_vec.is_empty() }

    /// Check to see whether [set_id][HasFocus::set_id] has been called.
    pub fn is_set(&self) -> bool { !self.is_empty() }

    /// Get the `id` of the [FlexBox] that has keyboard focus.
    pub fn get_id(&self) -> Option<FlexBoxId> {
        if self.id_vec.is_empty() {
            None
        } else {
            let it = self.id_vec.last().unwrap();
            Some(*it)
        }
    }

    /// Set the `id` of the [FlexBox] that has keyboard focus.
    pub fn set_id(&mut self, id: FlexBoxId) {
        if self.id_vec.is_empty() {
            self.id_vec.push(id);
        } else {
            let it = self.id_vec.last_mut().unwrap();
            *it = id;
        }
    }

    /// Check whether the given `id` currently has keyboard focus.
    pub fn does_id_have_focus(&self, id: FlexBoxId) -> bool {
        if self.id_vec.is_empty() {
            false
        } else {
            let it = self.id_vec.last().unwrap();
            *it == id
        }
    }

    /// Check whether the `id` of the [FlexBox] currently has keyboard focus.
    pub fn does_current_box_have_focus(&self, current_box: &FlexBox) -> bool {
        self.does_id_have_focus(current_box.id)
    }
}

impl HasFocus {
    /// Pushes the `id` to the `id_vec`. The previous `id` is saved and can be restored with
    /// [reset_modal_id](HasFocus::reset_modal_id).
    pub fn try_set_modal_id(&mut self, id: FlexBoxId) -> CommonResult<()> {
        // Must have a non modal id already set.
        if !self.is_set() {
            let msg = "Modal id can only be set if id is already set. id is not set.";
            return CommonError::new_err_with_only_msg(msg);
        }

        // Must not have a modal id already set.
        if self.is_modal_set() {
            let msg = format!(
                "Modal id is already set to {}. Can't set it to {id}.",
                self.get_id().unwrap()
            );
            return CommonError::new_err_with_only_msg(&msg);
        }

        // Ok to set modal id.
        self.id_vec.push(id);
        Ok(())
    }

    /// Checks whether any modal `id` is set.
    pub fn is_modal_set(&self) -> bool { self.id_vec.len() == 2 }

    /// Checks whether the given `id` is the modal `id`.
    pub fn is_modal_id(&self, id: FlexBoxId) -> bool {
        self.is_modal_set() && self.does_id_have_focus(id)
    }

    /// Restores the modal `id` to the previous non-modal `id`. It does nothing if there's no modal
    /// `id` set.
    pub fn reset_modal_id(&mut self) {
        if self.is_modal_set() {
            self.id_vec.pop();
        }
    }
}

#[cfg(test)]
mod has_focus_tests {
    use super::*;

    #[test]
    fn works_with_normal_id() {
        let mut has_focus = HasFocus::default();
        assert!(has_focus.is_empty());
        assert!(!has_focus.is_set());

        has_focus.set_id(1);
        assert!(!has_focus.is_empty());
        assert!(has_focus.is_set());
        assert_eq2!(has_focus.get_id(), Some(1));
        assert!(has_focus.does_id_have_focus(1));
        let current_box_1 = FlexBox {
            id: 1,
            ..Default::default()
        };
        assert!(has_focus.does_current_box_have_focus(&current_box_1));

        has_focus.set_id(2);
        assert!(!has_focus.is_empty());
        assert!(has_focus.is_set());
        assert_eq2!(has_focus.get_id(), Some(2));
        assert!(has_focus.does_id_have_focus(2));
        let current_box_2 = FlexBox {
            id: 2,
            ..Default::default()
        };
        assert!(has_focus.does_current_box_have_focus(&current_box_2));
        assert!(!has_focus.does_id_have_focus(1));
        assert!(!has_focus.does_current_box_have_focus(&current_box_1));
    }

    #[test]
    fn fails_with_modal_id_with_no_id_set() {
        let mut has_focus = HasFocus::default();
        assert!(has_focus.is_empty());
        assert!(!has_focus.is_set());

        let my_err_box = has_focus.try_set_modal_id(1).err().unwrap();
        assert_eq2!(my_err_box.is::<CommonError>(), true);

        let my_err = my_err_box.downcast_ref::<CommonError>().unwrap();
        let CommonError { err_msg: msg, .. } = my_err;
        assert_eq2!(
            msg.as_ref().unwrap(),
            "Modal id can only be set if id is already set. id is not set."
        );

        assert!(!has_focus.is_modal_set());
        assert!(!has_focus.is_modal_id(1));
        has_focus.reset_modal_id();
    }

    #[test]
    fn works_with_modal_id_when_id_is_set() {
        let mut has_focus = HasFocus::default();
        assert!(has_focus.is_empty());
        assert!(!has_focus.is_set());

        has_focus.set_id(1);
        assert!(has_focus.try_set_modal_id(2).is_ok());

        assert!(has_focus.is_modal_set());
        assert!(has_focus.is_modal_id(2));
        assert!(!has_focus.is_modal_id(1));
        assert_eq2!(has_focus.get_id(), Some(2));

        assert!(has_focus.try_set_modal_id(3).is_err());
        assert!(has_focus.is_modal_set());
        assert!(has_focus.is_modal_id(2));

        has_focus.reset_modal_id();
        assert!(!has_focus.is_modal_set());
        assert!(!has_focus.is_modal_id(1));
        assert_eq2!(has_focus.get_id(), Some(1));
        assert_eq2!(has_focus.does_id_have_focus(1), true);
        let current_box_1 = FlexBox {
            id: 1,
            ..Default::default()
        };
        assert!(has_focus.does_current_box_have_focus(&current_box_1));
        assert!(has_focus.is_set());
        assert!(!has_focus.is_empty());
    }
}

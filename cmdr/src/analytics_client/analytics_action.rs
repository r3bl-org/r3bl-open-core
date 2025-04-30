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

use std::fmt::{Formatter, Result};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnalyticsAction {
    GitiBranchDelete,
    GitiFailedToRun,
    GitiAppStart,
    EdiAppStart,
    EdiFileNew,
    EdiFileOpenSingle,
    EdiFileOpenMultiple,
    EdiFileSave,
    MachineIdProxyCreate,
}

impl std::fmt::Display for AnalyticsAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #[rustfmt::skip]
        let action = match self {
            AnalyticsAction::GitiAppStart =>          "giti app start",
            AnalyticsAction::GitiBranchDelete =>      "giti branch delete",
            AnalyticsAction::GitiFailedToRun =>       "giti failed to run",
            AnalyticsAction::EdiAppStart =>           "edi app start",
            AnalyticsAction::EdiFileNew =>            "edi file new",
            AnalyticsAction::EdiFileOpenSingle =>     "edi file open one file",
            AnalyticsAction::EdiFileOpenMultiple =>   "edi file open many files",
            AnalyticsAction::EdiFileSave =>           "edi file save",
            AnalyticsAction::MachineIdProxyCreate =>  "proxy machine id create",
        };
        write!(f, "{action}")
    }
}

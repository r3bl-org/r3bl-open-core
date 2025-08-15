// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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
    ChAppStart,
    ChFailedToRun,
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
            AnalyticsAction::ChAppStart =>            "ch app start",
            AnalyticsAction::ChFailedToRun =>         "ch failed to run",
            AnalyticsAction::MachineIdProxyCreate =>  "proxy machine id create",
        };
        write!(f, "{action}")
    }
}

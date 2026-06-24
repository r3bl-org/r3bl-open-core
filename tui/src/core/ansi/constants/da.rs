// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Attributes ([`DA`]) request and response sequence constants.
//!
//! These constants are used by [`DaSequence`] for building [`DA`] responses, and by
//! the parsing system for recognizing incoming [`DA`] requests.
//!
//! See [constants module design] for the three-tier architecture.
//!
//! [`DA`]: crate::DaSequence
//! [`DaSequence`]: crate::DaSequence
//! [constants module design]: mod@crate::constants#design

use crate::define_ansi_const;

// DA response sequence components.

define_ansi_const!(@da_str : DA1_VT220_COLOR_RESPONSE_STR = ["?62;22c"] =>
    "Primary Device Attributes Response (DA1)" :
    "Complete VT220 response indicating ANSI color support: `ESC [ ? 62 ; 22 c`."
);

// DA request sequences.

define_ansi_const!(@da_str : DA1_REQUEST_NO_PARAM = ["c"] =>
    "Primary Device Attributes Request" :
    "Application asks terminal for device attributes. Terminal replies with `ESC [ ? 62 ; 22 c`."
);

define_ansi_const!(@da_str : DA1_REQUEST_PARAM_0 = ["0c"] =>
    "Primary Device Attributes Request (with 0 parameter)" :
    "Application asks terminal for device attributes. Terminal replies with `ESC [ ? 62 ; 22 c`."
);

define_ansi_const!(@da_str : DA2_REQUEST = [">c"] =>
    "Secondary Device Attributes Request" :
    "Application asks terminal for secondary device attributes. We currently ignore this."
);

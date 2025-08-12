// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[derive(Debug, Clone, PartialEq, Copy, Default)]
pub enum ContainsResult {
    #[default]
    DoesNotContain,
    DoesContain,
}

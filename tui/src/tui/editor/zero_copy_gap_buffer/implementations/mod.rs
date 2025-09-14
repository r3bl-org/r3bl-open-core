// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Implementation modules - these extend ZeroCopyGapBuffer with specialized capabilities.
mod access;
mod basic;
mod delete;
mod insert;
mod segment_builder;

// Note: These modules extend ZeroCopyGapBuffer through `impl` blocks
// They are not re-exported as they provide specialized, not universal, capabilities.

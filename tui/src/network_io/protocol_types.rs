// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Type alias for type to read from the stream to get the length prefix.
pub type LengthPrefixType = u64;
/// Type aliases for the payload buffer type.
pub type Buffer = Vec<BufferAtom>;
pub type BufferAtom = u8;

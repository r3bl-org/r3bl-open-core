/// Fast string conversion using stack-allocated buffers.
///
/// [`SmallString`] would be better if the buffer was:
/// - **Long-lived**: Amortizes stack allocation over many operations
/// - **Predictable size**: 90%+ of strings fit in inline capacity
/// - **Hot loop**: Allocator pressure becomes a bottleneck
///
/// But for this use case (short-lived, unpredictable size), [`String`] is optimal.
///
/// This type alias allows us to easily experiment with different string-like data
/// structures in the future without impacting the rest of the codebase.
///
/// [`String`]: std::string::String
/// [`Display::fmt`]: Display::fmt
/// [`SmallString<[u8; 64]>`]: smallstr::SmallString
/// [`SmallString<[u8; 256]>`]: smallstr::SmallString
pub type BufTextStorage = String;

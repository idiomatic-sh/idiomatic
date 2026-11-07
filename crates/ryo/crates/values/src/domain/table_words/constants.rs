//! The names the table_words concept gives text.

/// Diagnostic phase before the decoder reserves cells.
pub const BEFORE_RESERVE: &str = "decode_before_reserve";
/// Diagnostic phase after the decoder reserves cells.
pub const AFTER_RESERVE: &str = "decode_after_reserve";
/// Diagnostic phase before the decoder shrinks its buffers.
pub const BEFORE_SHRINK: &str = "decode_before_shrink";
/// Diagnostic phase after the decoder shrinks its buffers.
pub const AFTER_SHRINK: &str = "decode_after_shrink";
/// The target lifecycle memory diagnostics are written to.
pub const LIFECYCLE_TARGET: &str = "ryo::diagnostic::lifecycle_memory";
/// The target execution diagnostics are written to.
pub const EXECUTION_TARGET: &str = "ryo::diagnostic::execution";
/// The owner named in the decoding diagnostic.
pub const DECODING_OWNER: &str = "decoding_input";
/// The phase named in the compact admission diagnostic.
pub const COMPACT_ADMISSION: &str = "compact_admission";

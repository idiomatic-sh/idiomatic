//! Stable preparation refusals.
/// Sealed source files and declared missing paths must cover the requested inventory.
pub const CONTEXT_SOURCE_MISMATCH: &str = "context source seal differs from requested inventory";
/// Context modes must match the presence of an explicit manifest.
pub const CONTEXT_MODE_MISMATCH: &str = "worker context mode requires a matching explicit manifest";

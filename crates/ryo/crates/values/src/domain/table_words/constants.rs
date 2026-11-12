//! The names the table_words concept gives text.

/// The target lifecycle memory diagnostics are written to.
pub const LIFECYCLE_TARGET: &str = "ryo::diagnostic::lifecycle_memory";
/// The target execution diagnostics are written to.
pub const EXECUTION_TARGET: &str = "ryo::diagnostic::execution";
/// The target resource-route diagnostics are written to.
pub const RESOURCE_ROUTES_TARGET: &str = "ryo::diagnostic::resource_routes";
/// The phase named in the compact admission diagnostic.
pub(crate) const COMPACT_ADMISSION: &str = "compact_admission";

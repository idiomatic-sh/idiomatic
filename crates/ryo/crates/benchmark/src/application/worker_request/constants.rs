//! Stable preparation refusals.
/// Sealed source files and declared missing paths must cover the requested inventory.
pub const CONTEXT_SOURCE_MISMATCH: &str = "context source seal differs from requested inventory";
/// Context modes must match the presence of an explicit manifest.
pub const CONTEXT_MODE_MISMATCH: &str = "worker context mode requires a matching explicit manifest";
/// Text of the worker requires a resolved diagnostic select in this part.
pub(super) const WORKER_REQUIRES_A_RESOLVED_DIAGNOSTIC_SELECT: &str =
    "worker requires a resolved diagnostic selection";
/// Text of the complete given project inventory requires de in this part.
pub(super) const COMPLETE_GIVEN_PROJECT_INVENTORY_REQUIRES_DE: &str =
    "complete Given project inventory requires declared search roots";
/// Text of the given project inventory cannot override a in this part.
pub(super) const GIVEN_PROJECT_INVENTORY_CANNOT_OVERRIDE_A: &str =
    "Given project inventory cannot override a sealed disk context";
/// Text of the loaded engine session belongs to another in this part.
pub(super) const LOADED_ENGINE_SESSION_BELONGS_TO_ANOTHER: &str =
    "loaded engine session belongs to another received request";

//! Refusals of certifying a registered case context.
/// The registered defaults must be exactly the measured profile's defaults.
pub(super) const CASE_INVENTORY_DIFFERS_FROM_MEASURED: &str =
    "case inventory differs from measured complete rule identities";
/// Only an execution authority can certify registered cases.
pub(super) const REGISTERED_CERTIFICATION_REQUIRES_ACTUAL_EXE: &str =
    "registered certification requires actual execution authority";
/// The registered authority must name the measured profile's bytes.
pub(super) const REGISTERED_AUTHORITY_DIFFERS_FROM_MEASURED_C: &str =
    "registered authority differs from measured canonical profile";
/// The selected rules must be the measured profile's rule directory.
pub(super) const SELECTED_RULE_ROOT_DIFFERS_FROM_MEASURED: &str =
    "selected rule root differs from measured profile";
/// The registered executables must be the measured contestants.
pub(super) const CASE_INVENTORY_EXECUTABLE_IDENTITIES_DIFFER: &str =
    "case inventory executable identities differ from measured contestants";
/// The registered context must hold the tested query.
pub(super) const REGISTERED_CONTEXT_LACKS_TESTED_QUERY: &str =
    "registered context lacks tested query";

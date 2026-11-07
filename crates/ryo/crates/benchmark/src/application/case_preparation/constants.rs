//! Names owned by this suite operation.
pub(super) const CACHE: &str = "cache";
pub(super) const NO_EXECUTABLE_CASES: &str = crate::domain::suite::constants::NO_CASES;
pub(super) const MISSING_PROFILE_PARENT: &str = "case rules have no profile parent";
pub(super) const REQUIRED_METADATA: &str =
    "file-observation-required profile needs explicit metadata for every source";
pub(super) const COMPLETE_GIVEN_ROOTS: &str =
    "complete Given project inventory requires declared search roots";
pub(super) const GIVEN_OVERRIDES_SEAL: &str =
    "Given project inventory cannot override a sealed disk context";
pub(super) const CONTEXT_MODE: &str = "case context must agree with explicit profile context mode";

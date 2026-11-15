//! Refusals of declared input observations.
/// A case root without a profile directory cannot be checked against its profile.
pub(crate) const MISSING_CASE_PROFILE_BOUNDARY: &str = "missing case profile boundary";
/// A profile that requires file observations cannot be verified where the host does not honour file permissions.
pub(crate) const HOST_DOES_NOT_HONOUR_FILE_PERMISSIONS: &str =
    "the pinned reference does not evaluate file permissions on this host";

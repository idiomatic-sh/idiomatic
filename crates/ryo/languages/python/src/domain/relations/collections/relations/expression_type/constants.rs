//! Stable expression-type relation vocabulary.
/// The `atom` relation label.
pub(super) const ATOM: &str = "atom";
/// The `union` relation label.
pub(super) const UNION: &str = "union";
/// The `unknown` relation label.
pub(super) const UNKNOWN: &str = "unknown";
/// The `type-error` relation label.
pub(super) const TYPE_ERROR: &str = "type-error";
/// The `empty` relation label.
pub(super) const NO_TYPE: &str = "";

/// Required literal payloads were not supplied completely.
pub(super) const INCOMPLETE: &str = "basic expression types require complete parser payloads";

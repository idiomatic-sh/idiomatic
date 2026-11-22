//! Stable expression-type relation vocabulary.
/// The `atom` relation label.
pub(crate) const ATOM: &str = "atom";
/// The `union` relation label.
pub(crate) const UNION: &str = "union";
/// The `unknown` relation label.
pub(crate) const UNKNOWN: &str = crate::domain::language::constants::UNKNOWN;
/// The `type-error` relation label.
pub(crate) const TYPE_ERROR: &str = "type-error";
/// The `empty` relation label.
pub(crate) const NO_TYPE: &str = "";

/// Required literal payloads were not supplied completely.
pub(crate) const INCOMPLETE: &str = "basic expression types require complete parser payloads";
/// The texts of this concept a filler writes that are the same in every file: fixed names of the run.
pub(crate) const FIXED_NAMES: &[&str] = &[ATOM, UNION, UNKNOWN, TYPE_ERROR, NO_TYPE];

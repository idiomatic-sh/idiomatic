//! Stable operation vocabulary and artifact identities.
pub(super) const EMPTY_AUDIT: &str = "empty audit is not default coverage";
pub(crate) const EMPTY_SUBSET: &str = "no authored default subset";
pub(super) const RULE_SEPARATOR: &str = ", ";
pub(super) const MISSING_POLARITY: &str = "a rule requires positive and negative cases";
pub(super) const EMPTY_CASES: &str = "case evidence must be nonempty";
pub(super) const INVALID_CASE_IDENTITY: &str = "case names must be unique and sources nonempty";
pub(super) const MISSING_SOURCE: &str = "expected finding outside case";
pub(super) const INVALID_SOURCE_RANGE: &str = "expected finding has invalid UTF-8 span";
pub(super) const INVALID_SELECTION: &str =
    "selected subset must contain distinct authored default rules";

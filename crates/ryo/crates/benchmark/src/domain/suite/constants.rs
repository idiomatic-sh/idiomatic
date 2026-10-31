//! Stable operation vocabulary and artifact identities.
/// An empty inventory cannot establish default coverage.
pub(super) const EMPTY_AUDIT: &str = "empty audit is not default coverage";
/// A benchmark must execute at least one authored default rule.
pub(crate) const EMPTY_SUBSET: &str = "no authored default subset";
/// Readable separator for missing or extra rule identities.
pub(super) const RULE_SEPARATOR: &str = ", ";
/// Every authored rule needs both positive and negative evidence.
pub(super) const MISSING_POLARITY: &str = "a rule requires positive and negative cases";
/// An empty fixture file provides no evidence.
pub(super) const EMPTY_CASES: &str = "case evidence must be nonempty";
/// Refuse duplicate case names or cases without sources.
pub(super) const INVALID_CASE_IDENTITY: &str = "case names must be unique and sources nonempty";
/// Expected findings must refer to an included source.
pub(super) const MISSING_SOURCE: &str = "expected finding outside case";
/// Expected offsets must be ordered UTF-8 boundaries inside the source.
pub(super) const INVALID_SOURCE_RANGE: &str = "expected finding has invalid UTF-8 span";
/// Only distinct authored default rules may enter a selected profile.
pub(super) const INVALID_SELECTION: &str =
    "selected subset must contain distinct authored default rules";

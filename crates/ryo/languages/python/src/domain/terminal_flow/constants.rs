//! The names of this concept.
/// The text `implicit`.
pub(crate) const IMPLICIT: &str = "implicit";
/// The text `raise`.
pub(crate) const RAISE: &str = "raise";
/// The text `return-or-raise`.
pub(crate) const RETURN_OR_RAISE: &str = "return-or-raise";
/// The text `conditional-return`.
pub(crate) const CONDITIONAL_RETURN: &str = "conditional-return";
/// How a function's body ends, as a filler writes it: the same in every file, so fixed names of the run.
pub(crate) const FIXED_NAMES: &[&str] = &[IMPLICIT, RAISE, RETURN_OR_RAISE, CONDITIONAL_RETURN];

//! The names of this concept.
/// Evaluated when its definition or statement runs.
pub(crate) const EAGER: &str = "eager";
/// Kept as text and not evaluated unless asked.
pub(crate) const POSTPONED: &str = "postponed";
/// Evaluated when first asked.
pub(crate) const DEFERRED: &str = "deferred";
/// Never evaluated.
pub(crate) const UNEVALUATED: &str = "unevaluated";
/// The annotations of the file are evaluated when first asked.
pub(crate) const LAZY: &str = "lazy";
/// The kind of the root of a Python file.
pub(crate) const MODULE: &str = "module";
/// The classes of an annotation's root, by their place in a root's bits: the evaluation and the mode of each.
pub(crate) const CLASSES: [(&str, &str); 5] =
    [(EAGER, EAGER), (POSTPONED, LAZY), (DEFERRED, LAZY), (UNEVALUATED, LAZY), (UNEVALUATED, EAGER)];
/// The texts of this concept a filler writes that are the same in every file: fixed names of the run.
pub(crate) const FIXED_NAMES: &[&str] = &[EAGER, POSTPONED, DEFERRED, UNEVALUATED, LAZY];

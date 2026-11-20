//! The names of this concept.
/// The text `string`.
pub(crate) const STRING: &str = "string";
/// The text `0`.
pub(crate) const ZERO_PAYLOAD: &str = "0";
/// The text `null`.
pub(crate) const NULL: &str = "null";
/// The kinds of a primitive literal, as a filler writes them: the same in every file, so fixed names of the run.
pub(crate) const FIXED_NAMES: &[&str] = &[
    STRING,
    NULL,
    crate::domain::syntax::constants::BYTES_SCALAR,
    crate::domain::syntax::constants::INTEGER_SCALAR,
    crate::domain::syntax::constants::FLOAT_SCALAR,
    crate::domain::syntax::constants::COMPLEX_SCALAR,
    crate::domain::syntax::constants::BOOLEAN_SCALAR,
    crate::domain::syntax::constants::ELLIPSIS_SCALAR,
];

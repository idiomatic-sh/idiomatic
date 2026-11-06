//! The names of this concept.
/// The label of a specification that is a valid standard one.
pub(crate) const RECOGNIZED: &str = "recognized";
/// The label of a specification built from dynamic input.
pub(crate) const DYNAMIC: &str = "dynamic";
/// The label of a specification with an invalid presentation type.
pub(crate) const INVALID_TYPE: &str = "invalid-type";
/// The label of a specification with any other error.
pub(crate) const OTHER_ERROR: &str = "other-error";
/// The cause of a reading that is only partial.
pub(crate) const PARTIAL_CAUSE: &str =
    "format specifications require complete parser-decoded inputs";

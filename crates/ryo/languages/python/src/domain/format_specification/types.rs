//! One source-backed standard mini-language observation.

use ryo::languages::Position;

/// The place of a specification among the fields of its string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpecOrdinal(pub usize);







/// One interpolation of an f-string, with the f-string that owns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OwnedField {
    /// The f-string.
    pub(crate) owner: Position,
    /// The interpolation.
    pub(crate) field: Position,
}







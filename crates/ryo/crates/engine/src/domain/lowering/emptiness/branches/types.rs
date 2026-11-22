//! The types of the `branches` part: what a definition's rows hold of the elements of the file's tree in its columns,
//! read once a definition.

use ryo_language::domain::facts::types::CellWord;

/// A set of the words of names a cell may hold; none: any.
pub(crate) type Words = Option<Vec<CellWord>>;

/// What every row holds of one element of the file's tree: the kinds and fields it may have, and the kinds its parent
/// may have. Each part is a condition every row meets; none: any.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColumnShape {
    /// The element's kinds.
    pub(crate) kinds: Words,
    /// The element's fields in its parent.
    pub(crate) fields: Words,
    /// Its parent's kinds.
    pub(crate) parent_kinds: Words,
}

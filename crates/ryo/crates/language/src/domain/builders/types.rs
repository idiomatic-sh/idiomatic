//! The types of the `builders` concept: where the next element goes, and the formatter that writes a property into retained storage.

use ryo_relations::domain::relations::types::Position;
use ryo_relations::domain::terms::types::ChildOrdinal;

/// The parent of an element about to be added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parent {
    /// The element is a root: it is its own parent.
    Root,
    /// The element is a child of this one.
    Of(Position),
}

/// Where the next element goes: its parent and its place among the parent's children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementPlacement {
    /// The parent.
    pub parent: Parent,
    /// The element's place among its parent's children.
    pub ordinal: ChildOrdinal,
}

/// A formatter writing directly into retained parser-property storage.
pub(crate) struct PropertyText<'a>(pub &'a mut Vec<u8>);

impl std::fmt::Write for PropertyText<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

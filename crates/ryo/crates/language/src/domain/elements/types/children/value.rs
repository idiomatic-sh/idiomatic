//! The children of one element, in order, found by skipping each child's subtree.
use super::super::Elements;
use ryo_relations::domain::relations::types::Position;

/// The children of one element, in order, found by skipping each child's subtree.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
/// use ryo_language::domain::elements::service::children;
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_relations::domain::relations::types::{KindBits, Offset, Position};
///
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab"));
/// enter_node(&mut builder, KindBits(1), Offset(0));
/// token(&mut builder, KindBits(2), Offset(0), Offset(1));
/// token(&mut builder, KindBits(2), Offset(1), Offset(2));
/// leave_node(&mut builder, Offset(2));
/// assert_eq!(children(&elements, Position(0)).collect::<Vec<_>>(), [Position(1), Position(2)]);
/// ```
#[derive(Clone, Debug)]
pub struct Children<'a> {
    /// The table.
    pub elements: &'a Elements,
    /// The next child.
    pub next: Position,
    /// The last position inside the parent.
    pub last: Position,
}

impl Iterator for Children<'_> {
    type Item = Position;

    fn next(&mut self) -> Option<Position> {
        if self.next > self.last {
            return None;
        }
        let child = self.next;
        if let Some(native) = crate::domain::elements::service::native_table(self.elements) {
            self.next = crate::domain::node_table::service::subtree_end_of(native, child);
            return Some(child);
        }
        let size = self
            .elements
            .size
            .as_ref()
            .values
            .get(usize::try_from(child.0).unwrap_or(usize::MAX))?;
        self.next = Position(child.0 + size.0 + 1);
        Some(child)
    }
}

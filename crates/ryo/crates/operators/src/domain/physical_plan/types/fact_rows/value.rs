//! The `FactRows` value.

use ryo_relations::domain::relations::types::ColumnsView;

/// A relation's rows as the facts lend them for the open file.
#[derive(Clone, Copy, Debug)]
pub enum FactRows<'a> {
    /// The rows, borrowed in place: read as they are, never decoded into a table.
    Lent(ColumnsView<'a>),
    /// The facts refused the relation: a fault, never empty rows.
    Refused,
}

impl<'a> FactRows<'a> {
    /// The columns, when lent.
    #[must_use]
    pub const fn lent(self) -> Option<ColumnsView<'a>> {
        match self {
            Self::Lent(view) => Some(view),
            Self::Refused => None,
        }
    }
}

//! The types of the `keyed` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::contracts::types::{Declaration, DeclarationName};

/// A declaration within a table, and its name from the table: `id` for `tools.[0].id`.
#[derive(Clone, Debug)]
pub struct TableMember<'d> {
    /// The declaration.
    pub declaration: &'d Declaration,
    /// Its name from the table.
    pub rest: DeclarationName,
}

/// A table of the file's that takes another name: the template table's it meets, or one aside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Renaming {
    /// Its name.
    pub from: DeclarationName,
    /// The name it takes.
    pub to: DeclarationName,
}

/// How deep a declaration of a keyed document is, or where an element is in its sequence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Depth(pub usize);

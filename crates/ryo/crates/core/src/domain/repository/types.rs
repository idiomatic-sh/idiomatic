//! The types of the `repository` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::{CellKind, NameRef};

/// A column of a relation as the digest of the catalog sees it: its name and what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DigestColumn<'a> {
    /// What the column is called in the query language.
    pub name: NameRef<'a>,
    /// What the column holds.
    pub kind: CellKind,
}

/// A relation as the digest of the catalog sees it: its name and its columns in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DigestRelation<'a> {
    /// What the relation is called in the query language.
    pub name: NameRef<'a>,
    /// Its columns, in the order the relation has them.
    pub columns: Vec<DigestColumn<'a>>,
}

/// A count or a length that the digest of the catalog writes as eight bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SchemaCount(pub usize);

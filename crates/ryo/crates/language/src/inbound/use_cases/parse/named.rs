//! File-local tables declared by a linked provider in addition to the shared catalog.
use crate::domain::{
    elements::types::Elements,
    symbols::types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::types::{CellKind, FactColumn, FileId};

/// Ordered public schema. Cells use the same file, node, span and symbol identities as
/// the provider's other file-scoped answers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileRelation {
    /// Query-visible name.
    pub name: &'static str,
    /// Column labels and scalar kinds in physical order.
    pub columns: &'static [(&'static str, CellKind)],
}

/// Additional relations from the same numbered parse. An error means the provider
/// could not establish a complete answer for this file.
pub trait FileRelations: Send + Sync {
    /// Every relation this provider can supply.
    fn declarations(&self) -> &'static [FileRelation];

    /// Returns complete columns. A table with no rows has one empty column per label.
    /// Symbols are interned into the same dictionary as the numbered tree.
    fn read(
        &self,
        relation: &FileRelation,
        file: FileId,
        elements: &Elements,
        tree: &crate::domain::node_table::types::NodeTable,
        bytes: ByteSlice<'_>,
        symbols: &mut Symbols,
    ) -> Result<Vec<FactColumn>, super::errors::ParseError>;
}

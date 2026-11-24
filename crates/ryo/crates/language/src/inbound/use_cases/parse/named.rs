//! File-local tables declared by a linked provider in addition to the shared catalog.
use crate::domain::{
    elements::types::Elements,
    symbols::types::Symbols,
};
use ryo_relations::domain::relations::types::{CellKind, FactColumn};

/// Ordered public schema. Cells use the same file, node, span and symbol identities as
/// the provider's other file-scoped answers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileRelation {
    /// Query-visible name.
    pub name: &'static str,
    /// Column labels and scalar kinds in physical order.
    pub columns: &'static [(&'static str, CellKind)],
}

/// Repository observations available to a provider while its numbered file is borrowed.
pub trait FileProject {
    /// Ordered repository-relative search roots, when the caller declared them.
    fn roots(&self) -> Option<&[String]>;
    /// Observe an entry within the effective repository. Missing and unknown are distinct.
    fn entry(&self, path: &std::path::Path) -> crate::domain::project_context::types::ObservedEntryState;
}

/// Additional relations from the same numbered parse. An error means the provider
/// could not establish a complete answer for this file.
pub trait FileRelations: Send + Sync {
    /// Whether any of these producers consult repository observations.
    fn uses_project(&self) -> bool { false }

    /// Every relation this provider can supply.
    fn declarations(&self) -> &'static [FileRelation];

    /// Answers all requested tables from the same numbered file, in declaration order.
    /// A refusal applies to its relation. A complete empty table has one column per label.
    fn read(
        &self,
        relations: &[&FileRelation],
        request: super::types::ParseRequest<'_>,
        elements: &Elements,
        tree: &crate::domain::node_table::types::NodeTable,
        symbols: &mut Symbols,
        project: Option<&dyn FileProject>,
    ) -> Vec<Result<Vec<FactColumn>, super::errors::ParseError>>;
}

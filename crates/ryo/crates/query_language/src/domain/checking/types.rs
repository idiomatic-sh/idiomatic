//! The types of the `checking` concept: plain data, public fields and derives.

use ryo_values::domain::program::types::ImportDeclaration;
use std::collections::BTreeMap;

/// The local alias an import is visible under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportAlias(pub Box<str>);

/// Imported aliases visible from one source document.
#[derive(Clone, Debug, Default)]
pub(crate) struct ImportIndex(pub BTreeMap<ImportAlias, ImportDeclaration>);

impl std::borrow::Borrow<str> for ImportAlias {
    fn borrow(&self) -> &str {
        &self.0
    }
}

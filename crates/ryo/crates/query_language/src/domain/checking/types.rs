//! The types of the `checking` concept: plain data, public fields and derives.

use crate::domain::linking::errors::LinkingError;
use ryo_values::domain::program::types::ImportDeclaration;
use ryo_values::domain::query_tokens::types::QuerySourceId;
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

/// What the checks of a catalog's fragments answered, made once for the catalog and read by every rule linked over it: a
/// fragment's names, signature, aliases, import targets and import cycles are the same whichever rule imports it.
#[derive(Clone, Debug, Default)]
pub(crate) struct FragmentChecks {
    /// Each fragment's imports by alias.
    pub imports: BTreeMap<QuerySourceId, ImportIndex>,
    /// The first fragment, in the catalog's order, whose step names, signature or aliases are refused; the checks after
    /// it were not made.
    pub refused: Option<LinkingError>,
    /// The first fragment, in the catalog's order, that imports a module the bundle does not hold, with why.
    pub target: Option<(QuerySourceId, LinkingError)>,
    /// The import cycle among the fragments, when there is one.
    pub cycle: Option<LinkingError>,
}

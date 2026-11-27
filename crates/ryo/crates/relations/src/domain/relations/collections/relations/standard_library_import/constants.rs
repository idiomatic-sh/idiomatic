//! Published spelling of the versioned import-membership fact.
use super::item::StandardLibraryImportColumn;
use crate::domain::relations::constants::{OCCURRENCE, TARGET};

/// Relation name in RyoQL.
#[must_use]
pub fn name() -> &'static str {
    "standard-library-import"
}

/// Relation meaning.
#[must_use]
pub fn summary() -> &'static str {
    "An import occurrence whose absolute module root is in the configured language release's standard library."
}

/// Column name in RyoQL.
#[must_use]
pub fn column_name(column: StandardLibraryImportColumn) -> &'static str {
    match column {
        StandardLibraryImportColumn::Occurrence => OCCURRENCE,
        StandardLibraryImportColumn::Target => TARGET,
        StandardLibraryImportColumn::File => "file",
    }
}

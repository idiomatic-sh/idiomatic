//! An imported module whose absolute root belongs to the selected language release's standard library.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId};
use crate::domain::terms::types::Symbol;

/// Positive membership only. A complete empty answer proves no matching import;
/// an unavailable answer remains Unknown to consumers.
#[derive(Clone, Copy, Debug)]
pub struct StandardLibraryImport;
impl super::super::interface::sealed::Sealed for StandardLibraryImport {}

/// Declare one real import occurrence with its absolute imported path.
#[macro_export]
#[doc(hidden)]
macro_rules! standard_library_import {
    ($callback:path) => {
        $callback! {
            item StandardLibraryImport,
            row { /// A module import recognized by the selected language catalog.
                StandardLibraryImportRow },
            column { /// Membership columns.
                StandardLibraryImportColumn },
            expected { /// Independently expected membership.
                StandardLibraryImportExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The imported alias in the parser's actual syntax tree.
                occurrence Occurrence: NodeId => Node;
                /// The normalized absolute target, including any submodule suffix.
                target Target: Symbol => Symbol;
                /// The file containing the import occurrence.
                file File: FileId => File;
            }
        }
    };
}
crate::standard_library_import!(crate::relation_row);

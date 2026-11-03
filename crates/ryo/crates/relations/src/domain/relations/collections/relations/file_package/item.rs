//! The `file_package` relation: the package each file of the repository belongs to, the innermost one whose directory holds it; the
//! language's `project` use case writes it.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::FileId;

/// `file_package`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct FilePackage;

impl super::super::interface::sealed::Sealed for FilePackage {}

/// The declaration of `file_package`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! file_package {
    ($callback:path) => {
        $callback! {
            item FilePackage,
            row {
                /// One row of `file_package`.
                FilePackageRow
            },
            column {
                /// The columns of `file_package`.
                FilePackageColumn
            },
            expected {
                /// One row of `file_package` as a fixture states it.
                FilePackageExpectedRow
            },
            class Manifest,
            columns {
                /// The file.
                path Path: FileId => File;
                /// The package: its manifest, relative to the repository root.
                package Package: FileId => File;
            }
        }
    };
}

crate::file_package!(crate::relation_row);

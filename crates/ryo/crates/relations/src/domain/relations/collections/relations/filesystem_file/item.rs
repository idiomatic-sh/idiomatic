//! The `filesystem_file` relation: one row for each file the run inventories, whatever kind of file it is.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::FileId;

/// `filesystem_file`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct FilesystemFile;

impl super::super::interface::sealed::Sealed for FilesystemFile {}

/// The declaration of `filesystem_file`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! filesystem_file {
    ($callback:path) => {
        $callback! {
            item FilesystemFile,
            row {
                /// One row of `filesystem_file`.
                FilesystemFileRow
            },
            column {
                /// The columns of `filesystem_file`.
                FilesystemFileColumn
            },
            expected {
                /// One row of `filesystem_file` as a fixture states it.
                FilesystemFileExpectedRow
            },
            class Engine,
            scope File,
            notes Unneeded,
            columns {
                /// The file.
                path Path: FileId => File;
            }
        }
    };
}

crate::filesystem_file!(crate::relation_row);

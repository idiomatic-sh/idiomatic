//! The `filesystem_executable` relation: known POSIX executable-bit observations supplied at the input edge.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag};

/// `filesystem_executable`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct FilesystemExecutable;

impl super::super::interface::sealed::Sealed for FilesystemExecutable {}

/// The declaration of `filesystem_executable`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! filesystem_executable {
    ($callback:path) => {
        $callback! {
            item FilesystemExecutable,
            row {
                /// One row of `filesystem_executable`.
                FilesystemExecutableRow
            },
            column {
                /// The columns of `filesystem_executable`.
                FilesystemExecutableColumn
            },
            expected {
                /// One row of `filesystem_executable` as a fixture states it.
                FilesystemExecutableExpectedRow
            },
            class Engine,
            scope File,
            notes Unneeded,
            columns {
                /// The file.
                path Path: FileId => File;
                /// Whether the observed POSIX mode intersects execute bits.
                executable Executable: Flag => Flag;
            }
        }
    };
}

crate::filesystem_executable!(crate::relation_row);

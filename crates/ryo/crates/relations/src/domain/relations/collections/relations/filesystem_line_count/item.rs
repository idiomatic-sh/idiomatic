//! The `filesystem_line_count` relation: how many lines each inventoried file holds, whatever kind of file it is.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer};

/// `filesystem_line_count`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct FilesystemLineCount;

impl super::super::interface::sealed::Sealed for FilesystemLineCount {}

/// The declaration of `filesystem_line_count`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! filesystem_line_count {
    ($callback:path) => {
        $callback! {
            item FilesystemLineCount,
            row {
                /// One row of `filesystem_line_count`.
                FilesystemLineCountRow
            },
            column {
                /// The columns of `filesystem_line_count`.
                FilesystemLineCountColumn
            },
            expected {
                /// One row of `filesystem_line_count` as a fixture states it.
                FilesystemLineCountExpectedRow
            },
            class Engine,
            scope File,
            notes Unneeded,
            columns {
                /// The file.
                path Path: FileId => File;
                /// Its lines, blank lines included; a last line with no line break counts.
                lines Lines: Integer => Number;
            }
        }
    };
}

crate::filesystem_line_count!(crate::relation_row);

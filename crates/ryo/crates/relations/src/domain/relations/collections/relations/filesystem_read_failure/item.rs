//! The `filesystem_read_failure` relation: primary inventoried paths the input edge could not read.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Span};
use crate::domain::terms::types::Symbol;

/// `filesystem_read_failure`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct FilesystemReadFailure;

impl super::super::interface::sealed::Sealed for FilesystemReadFailure {}

/// The declaration of `filesystem_read_failure`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! filesystem_read_failure {
    ($callback:path) => {
        $callback! {
            item FilesystemReadFailure,
            row {
                /// One row of `filesystem_read_failure`.
                FilesystemReadFailureRow
            },
            column {
                /// The columns of `filesystem_read_failure`.
                FilesystemReadFailureColumn
            },
            expected {
                /// One row of `filesystem_read_failure` as a fixture states it.
                FilesystemReadFailureExpectedRow
            },
            class Engine,
            scope File,
            notes Unneeded,
            columns {
                /// The file.
                path Path: FileId => File;
                /// The original input-edge failure detail.
                detail Detail: Symbol => Symbol;
                /// The genuine zero-length file origin, independent of source availability.
                span Span: Span => Span;
            }
        }
    };
}

crate::filesystem_read_failure!(crate::relation_row);

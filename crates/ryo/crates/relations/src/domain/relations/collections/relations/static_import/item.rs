//! The `static_import` relation: a name an import brings into scope.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `static_import`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct StaticImport;

impl super::super::interface::sealed::Sealed for StaticImport {}

/// The declaration of `static_import`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! static_import {
    ($callback:path) => {
        $callback! {
            item StaticImport,
            row {
                /// One row of `static_import`.
                StaticImportRow
            },
            column {
                /// The columns of `static_import`.
                StaticImportColumn
            },
            expected {
                /// One row of `static_import` as a fixture states it.
                StaticImportExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The import: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// What encloses it: its file and where it starts.
                owner Owner: NodeId => Node;
                /// The name bound.
                binding Binding: Symbol => Symbol;
                /// The path imported, normalized.
                target Target: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the import is.
                span Span: Span => Span;
            }
        }
    };
}

crate::static_import!(crate::relation_row);

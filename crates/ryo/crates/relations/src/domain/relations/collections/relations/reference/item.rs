//! The `reference` relation: a name used where it is not declared.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `reference`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Reference;

impl super::super::interface::sealed::Sealed for Reference {}

/// The declaration of `reference`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! reference {
    ($callback:path) => {
        $callback! {
            item Reference,
            row {
                /// One row of `reference`.
                ReferenceRow
            },
            column {
                /// The columns of `reference`.
                ReferenceColumn
            },
            expected {
                /// One row of `reference` as a fixture states it.
                ReferenceExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The use: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// What encloses it: its file and where it starts.
                owner Owner: NodeId => Node;
                /// The name used.
                target Target: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the use is.
                span Span: Span => Span;
            }
        }
    };
}

crate::reference!(crate::relation_row);

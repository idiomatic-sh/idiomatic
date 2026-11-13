//! The `resolves` relation: a name occurrence, with the declaration or import that binds it in the file.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `resolves`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Resolves;

impl super::super::interface::sealed::Sealed for Resolves {}

/// The declaration of `resolves`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! resolves {
    ($callback:path) => {
        $callback! {
            item Resolves,
            row {
                /// One row of `resolves`.
                ResolvesRow
            },
            column {
                /// The columns of `resolves`.
                ResolvesColumn
            },
            expected {
                /// One row of `resolves` as a fixture states it.
                ResolvesExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The name occurrence: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// What holds the occurrence: its lexical scope or enclosing declaration.
                owner Owner: NodeId => Node;
                /// The name, as written.
                name Name: Symbol => Symbol;
                /// The `id` of the `defines` row that binds the name where it is written: a declaration of the file, or the import that brings
                /// it in.
                definition Definition: NodeId => Node;
                /// The file it is in.
                file File: FileId => File;
                /// Where the name is.
                span Span: Span => Span;
            }
        }
    };
}

crate::resolves!(crate::relation_row);

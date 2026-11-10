//! The `visibility` relation: the visibility a declaration is written with (`pub`, `pub(crate)`, `export`, `private`), as
//! written, on the declaration that carries it.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `visibility`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Visibility;

impl super::super::interface::sealed::Sealed for Visibility {}

/// The declaration of `visibility`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! visibility {
    ($callback:path) => {
        $callback! {
            item Visibility,
            row {
                /// One row of `visibility`.
                VisibilityRow
            },
            column {
                /// The columns of `visibility`.
                VisibilityColumn
            },
            expected {
                /// One row of `visibility` as a fixture states it.
                VisibilityExpectedRow
            },
            class Code,
            columns {
                /// The visibility: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// The declaration it is written on: the node `defines.id` gives the declaration.
                declaration Declaration: NodeId => Node;
                /// The visibility as written, its white space one space. Free text: the language owns the words.
                text Text: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the visibility is.
                span Span: Span => Span;
            }
        }
    };
}

crate::visibility!(crate::relation_row);

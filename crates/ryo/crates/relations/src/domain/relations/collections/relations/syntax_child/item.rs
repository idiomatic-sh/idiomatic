//! The `syntax_child` relation: an edge of the syntax tree, ordered under its parent.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId};
use crate::domain::terms::types::Symbol;

/// `syntax_child`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct SyntaxChild;

impl super::super::interface::sealed::Sealed for SyntaxChild {}

/// The declaration of `syntax_child`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! syntax_child {
    ($callback:path) => {
        $callback! {
            item SyntaxChild,
            row {
                /// One row of `syntax_child`.
                SyntaxChildRow
            },
            column {
                /// The columns of `syntax_child`.
                SyntaxChildColumn
            },
            expected {
                /// One row of `syntax_child` as a fixture states it.
                SyntaxChildExpectedRow
            },
            class Structure,
            scope File,
            notes Unneeded,
            columns {
                /// The parent node: its file and where it starts.
                parent Parent: NodeId => Node;
                /// The child, a node or a token: its file and where it starts.
                child Child: NodeId => Node;
                /// The child's place under the parent, from zero.
                ordinal Ordinal: Integer => Number;
                /// The child's role under the parent, in the language's own vocabulary of roles.
                field Field: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
            }
        }
    };
}

crate::syntax_child!(crate::relation_row);
